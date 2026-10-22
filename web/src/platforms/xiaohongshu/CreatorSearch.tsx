"use client";

import React, { FormEvent, useState } from 'react';
import { Check, LoaderCircle, Search, UserRoundPlus } from 'lucide-react';

import styles from './CreatorSearch.module.css';
import {
  xiaohongshuCreatorService,
  type XiaohongshuCreator,
  type XiaohongshuCreatorService,
} from './creatorService';

function errorText(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object' && 'message' in error) return String(error.message);
  return '查询失败，请稍后重试。';
}

export function CreatorSearch({
  service = xiaohongshuCreatorService,
  onAdd,
  isAdded,
}: {
  service?: XiaohongshuCreatorService;
  onAdd: (creator: XiaohongshuCreator) => void;
  isAdded: (stableUserId: string) => boolean;
}) {
  const [input, setInput] = useState('');
  const [results, setResults] = useState<XiaohongshuCreator[]>([]);
  const [busy, setBusy] = useState(false);
  const [searched, setSearched] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const value = input.trim();
    if (!value) return;
    setBusy(true);
    setError(null);
    try {
      setResults(await service.resolve(value));
      setSearched(true);
    } catch (cause) {
      setResults([]);
      setSearched(true);
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className={styles.panel} aria-labelledby="xiaohongshu-add-heading">
      <div className={styles.heading}>
        <div>
          <h2 id="xiaohongshu-add-heading">添加主播</h2>
          <p>支持主页、分享链接、24 位 UID、小红书号或昵称。重名时请按小红书号选择。</p>
        </div>
      </div>
      <form className={styles.form} onSubmit={submit}>
        <label className={styles.srOnly} htmlFor="xiaohongshu-creator-input">主播身份</label>
        <input
          id="xiaohongshu-creator-input"
          value={input}
          onChange={(event) => setInput(event.target.value)}
          placeholder="粘贴主页 / 分享链接，或输入 UID、小红书号、昵称"
          autoComplete="off"
          spellCheck={false}
        />
        <button type="submit" disabled={busy || !input.trim()}>
          {busy ? <LoaderCircle className={styles.spinner} size={17} aria-hidden="true" /> : <Search size={17} aria-hidden="true" />}
          {busy ? '正在查询' : '查询'}
        </button>
      </form>
      {error ? <p className={styles.error} role="alert">{error}</p> : null}
      {searched && !busy && !error && results.length === 0 ? (
        <p className={styles.empty}>没有找到匹配主播，请检查输入后重试。</p>
      ) : null}
      {results.length > 0 ? (
        <div className={styles.results} aria-label="小红书主播查询结果">
          {results.map((creator) => {
            const added = isAdded(creator.stableUserId);
            return (
              <article className={styles.result} key={creator.stableUserId}>
                <div className={styles.avatar} aria-hidden="true">
                  {creator.avatarUrl ? <img src={creator.avatarUrl} alt="" /> : creator.nickname.slice(0, 1)}
                </div>
                <div className={styles.identity}>
                  <strong>{creator.nickname}</strong>
                  <span>小红书号：{creator.redId || '未公开'}</span>
                  <small>UID {creator.stableUserId}</small>
                </div>
                <button type="button" disabled={added} onClick={() => onAdd(creator)}>
                  {added ? <Check size={16} aria-hidden="true" /> : <UserRoundPlus size={16} aria-hidden="true" />}
                  {added ? '已添加' : '添加'}
                </button>
              </article>
            );
          })}
        </div>
      ) : null}
    </section>
  );
}
