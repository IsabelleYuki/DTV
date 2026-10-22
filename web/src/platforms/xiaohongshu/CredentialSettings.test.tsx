// @vitest-environment jsdom

import React from 'react';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { CredentialSettings } from './CredentialSettings';
import type { XiaohongshuCredentialService } from './credentialService';

afterEach(cleanup);

function service(): XiaohongshuCredentialService {
  return {
    status: vi.fn().mockResolvedValue({ storage: 'absent', valid: null }),
    save: vi.fn().mockResolvedValue({ storage: 'secure', valid: null }),
    test: vi.fn().mockResolvedValue({ storage: 'secure', valid: true }),
    clear: vi.fn().mockResolvedValue({ storage: 'absent', valid: null }),
  };
}

describe('Xiaohongshu credential settings', () => {
  it('hides the Cookie by default and only reveals it on demand', async () => {
    render(<CredentialSettings service={service()} />);
    const input = await screen.findByLabelText('小红书 Cookie', { selector: 'input' });

    expect(input).toHaveProperty('type', 'password');
    fireEvent.click(screen.getByRole('button', { name: '显示 Cookie' }));
    expect(input).toHaveProperty('type', 'text');
  });

  it('clears component state after saving and never displays the secret in status text', async () => {
    const api = service();
    render(<CredentialSettings service={api} />);
    const input = await screen.findByLabelText('小红书 Cookie', { selector: 'input' }) as HTMLInputElement;
    const secret = 'a1=abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop; web_session=session-secret';

    fireEvent.change(input, { target: { value: secret } });
    fireEvent.click(screen.getByRole('button', { name: '安全保存' }));

    await waitFor(() => expect(api.save).toHaveBeenCalledWith(secret, 'secure'));
    expect(input.value).toBe('');
    expect(document.body.textContent).not.toContain('session-secret');
  });

  it('shows a secure-store failure instead of silently reporting an ordinary absent state', async () => {
    const api = service();
    vi.mocked(api.status).mockResolvedValue({
      storage: 'absent',
      valid: null,
      error_code: 'secure_store_unavailable',
      message: '系统安全存储不可用，可改用仅本次会话。',
    });

    render(<CredentialSettings service={api} />);

    expect((await screen.findByRole('status')).textContent).toContain('系统安全存储不可用');
  });

  it('labels a successful unsaved test without pretending it was stored', async () => {
    const api = service();
    vi.mocked(api.test).mockResolvedValue({
      storage: 'absent',
      valid: true,
      message: '输入的 Cookie 连接正常，尚未保存。',
    });
    render(<CredentialSettings service={api} />);
    const input = await screen.findByLabelText('小红书 Cookie', { selector: 'input' });
    fireEvent.change(input, { target: { value: 'test-cookie' } });
    fireEvent.click(screen.getByRole('button', { name: '测试连接' }));

    expect((await screen.findByRole('status')).textContent).toContain('尚未保存');
    expect(input).toHaveProperty('value', 'test-cookie');
  });
});
