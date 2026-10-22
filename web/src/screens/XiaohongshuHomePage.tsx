"use client";

import React, { useMemo } from 'react';
import { Radio, UserRoundPlus } from 'lucide-react';

import styles from './XiaohongshuHomePage.module.css';
import { CredentialSettings } from '@/platforms/xiaohongshu/CredentialSettings';
import { useFollow } from '@/state/follow/FollowProvider';

export function XiaohongshuHomePage() {
  const { followedStreamers } = useFollow();
  const creators = useMemo(
    () => followedStreamers.filter((streamer) => streamer.platform === 'XIAOHONGSHU'),
    [followedStreamers],
  );

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <h1>小红书直播</h1>
          <p>保存固定主播身份，每次播放时重新查找当前直播间。</p>
        </div>
        <div className={styles.creatorCount} aria-label={`已关注 ${creators.length} 位小红书主播`}>
          <Radio size={16} aria-hidden="true" />
          {creators.length} 位主播
        </div>
      </header>

      <CredentialSettings />

      <section className={styles.creators} aria-labelledby="xiaohongshu-creators-heading">
        <div className={styles.creatorsHeading}>
          <div>
            <h2 id="xiaohongshu-creators-heading">已关注主播</h2>
            <p>收藏会参与文件夹、拖拽排序、平台筛选、导入导出和局域网同步。</p>
          </div>
        </div>

        {creators.length ? (
          <div className={styles.creatorGrid}>
            {creators.map((creator) => (
              <article className={styles.creatorRow} key={`${creator.platform}:${creator.id}`}>
                <div className={styles.avatar} aria-hidden="true">
                  {creator.avatarUrl ? <img src={creator.avatarUrl} alt="" /> : creator.nickname.slice(0, 1)}
                </div>
                <div className={styles.creatorMeta}>
                  <strong>{creator.nickname}</strong>
                  <span>{creator.redId || creator.stableUserId || creator.id}</span>
                </div>
                <span className={styles.liveState}>{creator.liveStatus === 'LIVE' ? '直播中' : creator.liveStatus === 'OFFLINE' ? '未开播' : '状态未知'}</span>
              </article>
            ))}
          </div>
        ) : (
          <div className={styles.emptyState}>
            <UserRoundPlus size={24} aria-hidden="true" />
            <div>
              <strong>还没有小红书主播</strong>
              <p>配置 Cookie 后，可通过主页链接、分享链接、UID 或小红书号添加。</p>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
