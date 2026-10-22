// @vitest-environment jsdom

import React from 'react';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { CreatorSearch } from './CreatorSearch';
import type { XiaohongshuCreatorService } from './creatorService';

afterEach(cleanup);

describe('Xiaohongshu creator search', () => {
  it('keeps query and add actions out of the legacy transparent button reset', async () => {
    const service: XiaohongshuCreatorService = {
      resolve: vi.fn().mockResolvedValue([
        { stableUserId: 'stable-a', redId: 'red-one', nickname: '清晰可见的主播', avatarUrl: '' },
      ]),
    };
    render(<CreatorSearch service={service} onAdd={vi.fn()} isAdded={() => false} />);

    fireEvent.change(screen.getByLabelText('主播身份'), { target: { value: '清晰可见的主播' } });
    const queryButton = screen.getByRole('button', { name: '查询' });
    expect(queryButton.getAttribute('data-slot')).toBe('button');
    fireEvent.click(queryButton);

    const addButton = await screen.findByRole('button', { name: '添加' });
    expect(addButton.getAttribute('data-slot')).toBe('button');
  });

  it('keeps ambiguous same-name creators selectable by stable identity', async () => {
    const service: XiaohongshuCreatorService = {
      resolve: vi.fn().mockResolvedValue([
        { stableUserId: 'stable-a', redId: 'red-one', nickname: '同名主播', avatarUrl: '' },
        { stableUserId: 'stable-b', redId: 'red-two', nickname: '同名主播', avatarUrl: '' },
      ]),
    };
    const onAdd = vi.fn();
    render(<CreatorSearch service={service} onAdd={onAdd} isAdded={() => false} />);

    fireEvent.change(screen.getByLabelText('主播身份'), { target: { value: '同名主播' } });
    fireEvent.click(screen.getByRole('button', { name: '查询' }));

    await waitFor(() => expect(screen.getAllByText('同名主播')).toHaveLength(2));
    expect(screen.getByText('小红书号：red-one')).toBeTruthy();
    expect(screen.getByText('小红书号：red-two')).toBeTruthy();
    fireEvent.click(screen.getAllByRole('button', { name: '添加' })[1]);
    expect(onAdd).toHaveBeenCalledWith(expect.objectContaining({ stableUserId: 'stable-b' }));
  });
});
