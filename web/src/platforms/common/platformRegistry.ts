import { Platform } from './types';

export const STREAMING_UI_PLATFORMS = [
  'douyu',
  'douyin',
  'huya',
  'bilibili',
  'xiaohongshu',
] as const;

export type StreamingUiPlatform = (typeof STREAMING_UI_PLATFORMS)[number];
export type UiPlatform = StreamingUiPlatform | 'custom';
export type FollowPlatform = `${Platform}`;

export const PLATFORM_BY_UI_PLATFORM: Record<StreamingUiPlatform, Platform> = {
  douyu: Platform.DOUYU,
  douyin: Platform.DOUYIN,
  huya: Platform.HUYA,
  bilibili: Platform.BILIBILI,
  xiaohongshu: Platform.XIAOHONGSHU,
};

export const FOLLOW_PLATFORM_BY_UI_PLATFORM: Record<StreamingUiPlatform, FollowPlatform> = {
  douyu: Platform.DOUYU,
  douyin: Platform.DOUYIN,
  huya: Platform.HUYA,
  bilibili: Platform.BILIBILI,
  xiaohongshu: Platform.XIAOHONGSHU,
};

export const UI_PLATFORM_BY_ROUTE: Readonly<Record<string, UiPlatform>> = {
  '/': 'douyu',
  '/douyin': 'douyin',
  '/huya': 'huya',
  '/bilibili': 'bilibili',
  '/xiaohongshu': 'xiaohongshu',
  '/custom': 'custom',
};

export const ROUTE_BY_UI_PLATFORM: Readonly<Record<UiPlatform, string>> = {
  douyu: '/',
  douyin: '/douyin/',
  huya: '/huya/',
  bilibili: '/bilibili/',
  xiaohongshu: '/xiaohongshu/',
  custom: '/custom/',
};

const UI_PLATFORM_SET = new Set<string>([...STREAMING_UI_PLATFORMS, 'custom']);
const FOLLOW_PLATFORM_SET = new Set<string>(Object.values(FOLLOW_PLATFORM_BY_UI_PLATFORM));

export function isUiPlatform(value: string): value is UiPlatform {
  return UI_PLATFORM_SET.has(value);
}

export function isFollowPlatform(value: string): value is FollowPlatform {
  return FOLLOW_PLATFORM_SET.has(value);
}

export function followPlatformFromPlatform(value: string): FollowPlatform | null {
  const normalized = value.trim().toUpperCase();
  return isFollowPlatform(normalized) ? normalized : null;
}

export function platformFromInput(value: string): Platform | null {
  const normalized = value.trim().toLowerCase();
  if (!STREAMING_UI_PLATFORMS.includes(normalized as StreamingUiPlatform)) return null;
  return PLATFORM_BY_UI_PLATFORM[normalized as StreamingUiPlatform];
}

export function routeForUiPlatform(platform: UiPlatform): string {
  return ROUTE_BY_UI_PLATFORM[platform];
}

export function uiPlatformFromPath(pathname: string): UiPlatform {
  const normalized = `/${pathname.trim().replace(/^\/+|\/+$/g, '')}`;
  if (normalized === '/') return 'douyu';
  const segment = `/${normalized.slice(1).split('/')[0]}`;
  return UI_PLATFORM_BY_ROUTE[segment] ?? 'douyu';
}
