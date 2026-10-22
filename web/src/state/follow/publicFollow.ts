import { isFollowPlatform, type FollowPlatform } from '@/platforms/common/platformRegistry';

export type FollowLiveStatus = 'UNKNOWN' | 'LIVE' | 'OFFLINE';

export type FollowedStreamer = {
  id: string;
  platform: FollowPlatform;
  nickname: string;
  avatarUrl: string;
  roomTitle?: string;
  currentRoomId: string;
  liveStatus: FollowLiveStatus;
  stableUserId?: string;
  redId?: string;
};

const LIVE_STATUSES = new Set<string>(['UNKNOWN', 'LIVE', 'OFFLINE']);

function optionalString(value: unknown): string | undefined {
  return typeof value === 'string' ? value : undefined;
}

export function sanitizeFollowedStreamersForTransfer(values: readonly unknown[]): FollowedStreamer[] {
  const result: FollowedStreamer[] = [];

  for (const value of values) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) continue;
    const source = value as Record<string, unknown>;
    const id = optionalString(source.id)?.trim();
    const platform = optionalString(source.platform)?.toUpperCase();
    const nickname = optionalString(source.nickname);
    const avatarUrl = optionalString(source.avatarUrl);
    const currentRoomId = optionalString(source.currentRoomId);
    const liveStatus = optionalString(source.liveStatus);

    if (!id || !platform || !isFollowPlatform(platform)) continue;
    if (nickname === undefined || avatarUrl === undefined || currentRoomId === undefined) continue;
    if (!liveStatus || !LIVE_STATUSES.has(liveStatus)) continue;

    const item: FollowedStreamer = {
      id,
      platform,
      nickname,
      avatarUrl,
      currentRoomId,
      liveStatus: liveStatus as FollowLiveStatus,
    };

    const roomTitle = optionalString(source.roomTitle);
    const stableUserId = optionalString(source.stableUserId);
    const redId = optionalString(source.redId);
    if (roomTitle !== undefined) item.roomTitle = roomTitle;
    if (stableUserId !== undefined) item.stableUserId = stableUserId;
    if (redId !== undefined) item.redId = redId;
    result.push(item);
  }

  return result;
}
