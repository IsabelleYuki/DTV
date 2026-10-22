import { invoke } from '@tauri-apps/api/core';

export type XiaohongshuCreator = {
  stableUserId: string;
  redId?: string | null;
  nickname: string;
  avatarUrl: string;
  currentRoomId?: string | null;
  liveStatus?: boolean | null;
};

export interface XiaohongshuCreatorService {
  resolve(input: string): Promise<XiaohongshuCreator[]>;
}

export const xiaohongshuCreatorService: XiaohongshuCreatorService = {
  resolve: (input) => invoke<XiaohongshuCreator[]>('search_xiaohongshu_creators', { input }),
};
