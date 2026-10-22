import { describe, expect, it } from 'vitest';

import { isXiaohongshuOfflineError, playbackType, type XiaohongshuPlayback } from './playerHelper';

describe('Xiaohongshu playback selection', () => {
  it('uses the format attached to the response-derived primary URL', () => {
    const playback: XiaohongshuPlayback = {
      stableUserId: 'stable',
      currentRoomId: 'room',
      title: 'title',
      nickname: 'name',
      avatarUrl: '',
      primaryUrl: 'https://live-source-play.xhscdn.com/live/a.m3u8?token=1',
      variants: [
        { url: 'https://live-source-play.xhscdn.com/live/a.m3u8?token=1', format: 'hls', label: 'hls' },
      ],
    };
    expect(playbackType(playback)).toBe('hls');
  });

  it('does not misreport authentication, network, or schema failures as offline', () => {
    expect(isXiaohongshuOfflineError({ code: 'offline', message: '主播未开播。' })).toBe(true);
    expect(isXiaohongshuOfflineError({ code: 'network', message: '无法读取小红书直播间。' })).toBe(false);
    expect(isXiaohongshuOfflineError({ code: 'authenticationExpired', message: '请重新登录。' })).toBe(false);
    expect(isXiaohongshuOfflineError({ code: 'malformedResponse', message: '响应格式已变化。' })).toBe(false);
  });
});
