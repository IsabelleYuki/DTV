import { invoke } from '@tauri-apps/api/core';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { refreshXiaohongshuFollowedStreamer, toXiaohongshuFollowPatch } from './followListHelper';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

afterEach(() => vi.clearAllMocks());

const followed = {
  id: 'stable-user',
  stableUserId: 'stable-user',
  redId: 'red-id',
  platform: 'XIAOHONGSHU' as const,
  nickname: '主播',
  avatarUrl: '',
  currentRoomId: 'room-one',
  liveStatus: 'LIVE' as const,
};

describe('Xiaohongshu follow refresh', () => {
  it('updates a dynamic room without returning an id mutation', () => {
    const patch = toXiaohongshuFollowPatch(followed, {
      stableUserId: 'stable-user',
      redId: 'red-id',
      nickname: '主播',
      avatarUrl: 'avatar',
      currentRoomId: 'room-two',
      liveStatus: 'LIVE',
    });
    expect(patch.currentRoomId).toBe('room-two');
    expect(patch).not.toHaveProperty('id');
    expect(followed.id).toBe('stable-user');
  });

  it('rejects a response for a different stable identity', () => {
    expect(() => toXiaohongshuFollowPatch(followed, {
      stableUserId: 'other-user',
      nickname: '另一个人',
      avatarUrl: '',
      liveStatus: 'OFFLINE',
    })).toThrow('稳定身份不匹配');
  });

  it('sends the saved nickname as a refresh fallback while preserving stable identity', async () => {
    vi.mocked(invoke).mockResolvedValue({
      stableUserId: 'stable-user',
      redId: 'red-id',
      nickname: '主播',
      avatarUrl: '',
      liveStatus: 'OFFLINE',
    });

    await refreshXiaohongshuFollowedStreamer(followed);

    expect(invoke).toHaveBeenCalledWith('refresh_xiaohongshu_creator', {
      stableUserId: 'stable-user',
      redId: 'red-id',
      nickname: '主播',
    });
  });
});
