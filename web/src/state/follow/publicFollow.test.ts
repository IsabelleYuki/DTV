import { describe, expect, it } from 'vitest';

import { sanitizeFollowedStreamersForTransfer } from './publicFollow';

describe('public follow serialization', () => {
  it('keeps Xiaohongshu public identity and dynamic room fields', () => {
    const result = sanitizeFollowedStreamersForTransfer([
      {
        id: 'stable-user-1',
        stableUserId: 'stable-user-1',
        redId: 'creator-red-id',
        platform: 'XIAOHONGSHU',
        nickname: 'Creator',
        avatarUrl: 'https://example.test/avatar.jpg',
        currentRoomId: 'dynamic-room-2',
        liveStatus: 'LIVE',
        roomTitle: 'Live now',
      },
    ]);

    expect(result).toEqual([
      {
        id: 'stable-user-1',
        stableUserId: 'stable-user-1',
        redId: 'creator-red-id',
        platform: 'XIAOHONGSHU',
        nickname: 'Creator',
        avatarUrl: 'https://example.test/avatar.jpg',
        currentRoomId: 'dynamic-room-2',
        liveStatus: 'LIVE',
        roomTitle: 'Live now',
      },
    ]);
  });

  it('drops unknown and credential-shaped fields', () => {
    const result = sanitizeFollowedStreamersForTransfer([
      {
        id: 'stable-user-1',
        platform: 'XIAOHONGSHU',
        nickname: 'Creator',
        avatarUrl: '',
        currentRoomId: '',
        liveStatus: 'UNKNOWN',
        cookie: 'a1=secret',
        token: 'secret-token',
        signedHeaders: { 'X-s': 'secret-signature' },
      },
    ]);

    expect(JSON.stringify(result)).not.toContain('secret');
    expect(result[0]).not.toHaveProperty('cookie');
    expect(result[0]).not.toHaveProperty('token');
    expect(result[0]).not.toHaveProperty('signedHeaders');
  });
});
