import { invoke } from '@tauri-apps/api/core';

export type XiaohongshuStreamVariant = {
  url: string;
  format: 'flv' | 'hls';
  label: string;
};

export type XiaohongshuPlayback = {
  stableUserId: string;
  currentRoomId: string;
  title: string;
  nickname: string;
  avatarUrl: string;
  primaryUrl: string;
  variants: XiaohongshuStreamVariant[];
};

let proxyActive = false;

export function isXiaohongshuOfflineError(error: unknown): boolean {
  const code = error && typeof error === 'object' && 'code' in error ? String(error.code) : '';
  const message =
    error && typeof error === 'object' && 'message' in error
      ? String(error.message)
      : typeof error === 'string'
        ? error
        : '';
  return code === 'offline' || /未开播|直播已结束|not live|offline/i.test(message);
}

export function playbackType(playback: XiaohongshuPlayback): 'flv' | 'hls' {
  const primary = playback.variants.find((variant) => variant.url === playback.primaryUrl);
  if (primary) return primary.format;
  return playback.primaryUrl.toLowerCase().includes('.m3u8') ? 'hls' : 'flv';
}

export async function getXiaohongshuStreamConfig(
  stableUserId: string,
  redId?: string,
): Promise<XiaohongshuPlayback & { streamUrl: string; streamType: 'flv' | 'hls' }> {
  const playback = await invoke<XiaohongshuPlayback>('get_xiaohongshu_playback', {
    stableUserId,
    redId: redId || null,
    preferredFormat: 'flv',
  });
  if (!playback.primaryUrl || !playback.variants.some((variant) => variant.url === playback.primaryUrl)) {
    throw new Error('直播响应中没有可验证的播放地址。');
  }
  const streamType = playbackType(playback);
  if (streamType === 'flv') {
    await invoke('set_stream_url_cmd', { url: playback.primaryUrl });
    const streamUrl = await invoke<string>('start_proxy');
    proxyActive = true;
    return { ...playback, streamUrl, streamType };
  }
  return { ...playback, streamUrl: playback.primaryUrl, streamType };
}

export async function stopXiaohongshuProxy(): Promise<void> {
  if (!proxyActive) return;
  try {
    await invoke('stop_proxy');
  } finally {
    proxyActive = false;
  }
}
