import { describe, expect, it } from 'vitest';

import { hlsProxyUrl, isXiaohongshuOfflineError, playbackType, type XiaohongshuPlayback } from './playerHelper';

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

  it('routes HLS through the local relay without putting credentials in the proxy URL', () => {
    const url = hlsProxyUrl(
      'http://127.0.0.1:34719/live.flv',
      'https://live-source-play.xhscdn.com/live/a.m3u8?token=upstream-only',
    );
    expect(url).toBe(
      'http://127.0.0.1:34719/hls?url=https%3A%2F%2Flive-source-play.xhscdn.com%2Flive%2Fa.m3u8%3Ftoken%3Dupstream-only',
    );
    expect(url).not.toContain('Cookie');
  });
});
