import { describe, expect, it } from 'vitest';

import { collectPortableConfigEntries, isExportableStorageKey } from './configTransfer';

function storageFrom(entries: Record<string, string>): Storage {
  const keys = Object.keys(entries);
  return {
    get length() {
      return keys.length;
    },
    key(index: number) {
      return keys[index] ?? null;
    },
    getItem(key: string) {
      return entries[key] ?? null;
    },
    setItem() {},
    removeItem() {},
    clear() {},
  } as Storage;
}

describe('portable config safety', () => {
  it('never exports Xiaohongshu credential storage keys', () => {
    expect(isExportableStorageKey('xiaohongshu_cookie')).toBe(false);
    expect(isExportableStorageKey('xiaohongshu_token')).toBe(false);
    expect(isExportableStorageKey('XIAOHONGSHU_preferred_quality')).toBe(true);
  });

  it('allowlists public follow fields inside exported JSON', () => {
    const entries = collectPortableConfigEntries(
      storageFrom({
        followedStreamers: JSON.stringify([
          {
            id: 'stable-user-1',
            stableUserId: 'stable-user-1',
            redId: 'creator-red-id',
            platform: 'XIAOHONGSHU',
            nickname: 'Creator',
            avatarUrl: '',
            currentRoomId: 'dynamic-room-2',
            liveStatus: 'LIVE',
            cookie: 'a1=secret',
            token: 'secret-token',
          },
        ]),
        followListOrder: JSON.stringify([
          {
            type: 'streamer',
            data: {
              id: 'stable-user-1',
              platform: 'XIAOHONGSHU',
              nickname: 'Creator',
              avatarUrl: '',
              currentRoomId: 'dynamic-room-2',
              liveStatus: 'LIVE',
              cookie: 'a1=secret-in-order',
            },
          },
        ]),
        xiaohongshu_cookie: 'a1=secret',
      }),
    );

    expect(entries).not.toHaveProperty('xiaohongshu_cookie');
    expect(entries.followedStreamers).toBeDefined();
    expect(entries.followedStreamers).not.toContain('secret');
    expect(entries.followListOrder).not.toContain('secret');
    expect(JSON.parse(entries.followedStreamers)).toEqual([
      {
        id: 'stable-user-1',
        stableUserId: 'stable-user-1',
        redId: 'creator-red-id',
        platform: 'XIAOHONGSHU',
        nickname: 'Creator',
        avatarUrl: '',
        currentRoomId: 'dynamic-room-2',
        liveStatus: 'LIVE',
      },
    ]);
  });
});
