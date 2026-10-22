import { describe, expect, it } from 'vitest';

import {
  FOLLOW_PLATFORM_BY_UI_PLATFORM,
  PLATFORM_BY_UI_PLATFORM,
  UI_PLATFORM_BY_ROUTE,
  isFollowPlatform,
  isUiPlatform,
  platformFromInput,
  routeForUiPlatform,
  uiPlatformFromPath,
  followPlatformFromPlatform,
} from './platformRegistry';

describe('platform registry', () => {
  it('maps Xiaohongshu explicitly across UI, player and follow domains', () => {
    expect(isUiPlatform('xiaohongshu')).toBe(true);
    expect(isFollowPlatform('XIAOHONGSHU')).toBe(true);
    expect(PLATFORM_BY_UI_PLATFORM.xiaohongshu).toBe('XIAOHONGSHU');
    expect(FOLLOW_PLATFORM_BY_UI_PLATFORM.xiaohongshu).toBe('XIAOHONGSHU');
    expect(UI_PLATFORM_BY_ROUTE['/xiaohongshu']).toBe('xiaohongshu');
  });

  it('rejects unknown values instead of falling back to another platform', () => {
    expect(isUiPlatform('unknown')).toBe(false);
    expect(isFollowPlatform('UNKNOWN')).toBe(false);
    expect(UI_PLATFORM_BY_ROUTE['/missing']).toBeUndefined();
    expect(platformFromInput('unknown')).toBeNull();
  });

  it('parses Xiaohongshu player input without a legacy fallback', () => {
    expect(platformFromInput('xiaohongshu')).toBe('XIAOHONGSHU');
    expect(platformFromInput('XIAOHONGSHU')).toBe('XIAOHONGSHU');
  });

  it('round-trips Xiaohongshu navigation routes', () => {
    expect(routeForUiPlatform('xiaohongshu')).toBe('/xiaohongshu/');
    expect(uiPlatformFromPath('/xiaohongshu')).toBe('xiaohongshu');
    expect(uiPlatformFromPath('/xiaohongshu/')).toBe('xiaohongshu');
  });

  it('maps player platforms to follows without a Bilibili fallback', () => {
    expect(followPlatformFromPlatform('XIAOHONGSHU')).toBe('XIAOHONGSHU');
    expect(followPlatformFromPlatform('UNKNOWN')).toBeNull();
  });
});
