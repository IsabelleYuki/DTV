import { describe, expect, it } from 'vitest';

import { toXiaohongshuFollowPatch } from './followListHelper';

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
});
