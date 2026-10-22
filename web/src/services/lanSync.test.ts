import { describe, expect, it } from 'vitest';

import { applyIncrementalLanSyncImport, collectLanSyncEntries, parseLanSyncPayload } from './lanSync';

function memoryStorage(initial: Record<string, string> = {}): Storage {
  const values = new Map(Object.entries(initial));
  return {
    get length() { return values.size; },
    key(index: number) { return Array.from(values.keys())[index] ?? null; },
    getItem(key: string) { return values.get(key) ?? null; },
    setItem(key: string, value: string) { values.set(key, value); },
    removeItem(key: string) { values.delete(key); },
    clear() { values.clear(); },
  } as Storage;
}

const xhsStreamer = {
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
};

describe('LAN sync Xiaohongshu safety and merge', () => {
  it('allowlists public fields when collecting and parsing sync payloads', () => {
    const entries = collectLanSyncEntries(memoryStorage({
      followedStreamers: JSON.stringify([xhsStreamer]),
      followListOrder: JSON.stringify([{ type: 'streamer', data: xhsStreamer }]),
    }));

    expect(entries.followedStreamers).not.toContain('secret');
    expect(entries.followListOrder).not.toContain('secret');

    const parsed = parseLanSyncPayload({
      kind: 'dtv-lan-sync',
      version: 1,
      exportedAt: new Date(0).toISOString(),
      source: { client: 'test' },
      entries: { followedStreamers: JSON.stringify([xhsStreamer]) },
    });
    expect(parsed.entries.followedStreamers).not.toContain('secret');
  });

  it('merges Xiaohongshu stable identity, dynamic room, and folder membership', () => {
    const storage = memoryStorage();
    const result = applyIncrementalLanSyncImport(storage, {
      followedStreamers: JSON.stringify([xhsStreamer]),
      followFolders: JSON.stringify([
        { id: 'folder-1', name: '收藏', streamerIds: ['XIAOHONGSHU:stable-user-1'] },
      ]),
      followListOrder: JSON.stringify([{ type: 'streamer', data: xhsStreamer }]),
    });

    expect(result.addedStreamers).toBe(1);
    expect(JSON.parse(storage.getItem('followedStreamers') ?? '[]')).toEqual([
      {
        id: 'stable-user-1',
        stableUserId: 'stable-user-1',
        redId: 'creator-red-id',
        platform: 'XIAOHONGSHU',
        nickname: 'Creator',
        avatarUrl: '',
        roomTitle: '',
        currentRoomId: 'dynamic-room-2',
        liveStatus: 'LIVE',
      },
    ]);
    expect(storage.getItem('followFolders')).toContain('XIAOHONGSHU:stable-user-1');
    expect(storage.getItem('followedStreamers')).not.toContain('secret');
  });
});
