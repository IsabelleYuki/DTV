import { invoke } from '@tauri-apps/api/core';

export type CredentialStorage = 'absent' | 'session' | 'secure';

export type CredentialStatus = {
  storage: CredentialStorage;
  valid: boolean | null;
  message?: string | null;
  error_code?: string | null;
};

export type CredentialSaveMode = Exclude<CredentialStorage, 'absent'>;

export interface XiaohongshuCredentialService {
  status(): Promise<CredentialStatus>;
  save(cookie: string, storage: CredentialSaveMode): Promise<CredentialStatus>;
  test(cookie?: string): Promise<CredentialStatus>;
  clear(): Promise<CredentialStatus>;
}

export const xiaohongshuCredentialService: XiaohongshuCredentialService = {
  status: () => invoke<CredentialStatus>('xiaohongshu_credential_status'),
  save: (cookie, storage) => invoke<CredentialStatus>('xiaohongshu_set_cookie', { cookie, storage }),
  test: (cookie) => invoke<CredentialStatus>('xiaohongshu_test_cookie', { cookie: cookie?.trim() || null }),
  clear: () => invoke<CredentialStatus>('xiaohongshu_clear_cookie'),
};
