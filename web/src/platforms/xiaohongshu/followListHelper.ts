import { invoke } from '@tauri-apps/api/core';

import type { FollowedStreamer } from '@/state/follow/FollowProvider';

export type XiaohongshuCreatorRefresh = {
  stableUserId: string;
  redId?: string | null;
  nickname: string;
  avatarUrl: string;
  currentRoomId?: string | null;
  liveStatus: 'LIVE' | 'OFFLINE';
  roomTitle?: string | null;
};

export function toXiaohongshuFollowPatch(
  streamer: FollowedStreamer,
  refresh: XiaohongshuCreatorRefresh,
): Partial<FollowedStreamer> {
  if (refresh.stableUserId !== streamer.id) {
    throw new Error('小红书主播稳定身份不匹配。');
  }
  return {
    stableUserId: streamer.id,
    redId: refresh.redId || streamer.redId,
    nickname: refresh.nickname || streamer.nickname,
    avatarUrl: refresh.avatarUrl || streamer.avatarUrl,
    currentRoomId: refresh.currentRoomId || '',
    liveStatus: refresh.liveStatus,
    roomTitle: refresh.roomTitle || streamer.roomTitle,
  };
}

export async function refreshXiaohongshuFollowedStreamer(
  streamer: FollowedStreamer,
): Promise<Partial<FollowedStreamer>> {
  const refresh = await invoke<XiaohongshuCreatorRefresh>('refresh_xiaohongshu_creator', {
    stableUserId: streamer.stableUserId || streamer.id,
    redId: streamer.redId || null,
  });
  return toXiaohongshuFollowPatch(streamer, refresh);
}
