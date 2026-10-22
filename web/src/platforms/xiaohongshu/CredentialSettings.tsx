"use client";

import React, { useEffect, useMemo, useState } from 'react';
import { Clock3, Eye, EyeOff, PlugZap, Save, ShieldCheck, Trash2 } from 'lucide-react';

import styles from './CredentialSettings.module.css';
import {
  xiaohongshuCredentialService,
  type CredentialSaveMode,
  type CredentialStatus,
  type XiaohongshuCredentialService,
} from './credentialService';

function statusLabel(status: CredentialStatus | null): string {
  if (status?.error_code && status.message) return status.message;
  if (!status || status.storage === 'absent') return '未配置';
  if (status.valid === false) return status.message || 'Cookie 已失效';
  if (status.storage === 'secure') return status.valid ? '安全存储 · 连接正常' : '已安全存储';
  return status.valid ? '仅本次会话 · 连接正常' : '仅本次会话';
}

function errorText(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object' && 'message' in error) return String(error.message);
  return '操作失败，请稍后重试。';
}

export function CredentialSettings({
  service = xiaohongshuCredentialService,
}: {
  service?: XiaohongshuCredentialService;
}) {
  const [cookie, setCookie] = useState('');
  const [visible, setVisible] = useState(false);
  const [status, setStatus] = useState<CredentialStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    service.status()
      .then((next) => {
        if (alive) setStatus(next);
      })
      .catch((cause) => {
        if (alive) setError(errorText(cause));
      });
    return () => {
      alive = false;
    };
  }, [service]);

  const label = useMemo(() => statusLabel(status), [status]);

  const run = async (action: () => Promise<CredentialStatus>, clearInput = false) => {
    setBusy(true);
    setError(null);
    try {
      const next = await action();
      setStatus(next);
      if (clearInput) {
        setCookie('');
        setVisible(false);
      }
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  const save = (storage: CredentialSaveMode) => {
    const value = cookie.trim();
    if (!value) {
      setError('请输入完整 Cookie。');
      return;
    }
    void run(() => service.save(value, storage), true);
  };

  return (
    <section className={styles.panel} aria-labelledby="xiaohongshu-cookie-heading">
      <div className={styles.headingRow}>
        <div>
          <div className={styles.titleLine}>
            <ShieldCheck size={18} aria-hidden="true" />
            <h2 id="xiaohongshu-cookie-heading">小红书 Cookie</h2>
          </div>
          <p className={styles.subtitle}>凭据只交给本机后端，不进入浏览器存储、导出或局域网同步。</p>
        </div>
        <span className={styles.status} role="status">{label}</span>
      </div>
      <p className={styles.help}>
        在浏览器登录自己的小红书账号后，从开发者工具的请求标头中复制完整 Cookie。完整功能需要
        <code>a1</code> 与 <code>web_session</code>；DTV 不会读取浏览器凭据。
      </p>
      <div className={styles.field}>
        <label htmlFor="xiaohongshu-cookie">小红书 Cookie</label>
        <div className={styles.inputRow}>
          <input
            className={styles.input}
            id="xiaohongshu-cookie"
            type={visible ? 'text' : 'password'}
            autoComplete="off"
            spellCheck={false}
            value={cookie}
            onChange={(event) => setCookie(event.target.value)}
            placeholder="a1=…; web_session=…"
          />
          <button className={styles.revealButton} type="button" onClick={() => setVisible((value) => !value)}>
            {visible ? <EyeOff size={16} aria-hidden="true" /> : <Eye size={16} aria-hidden="true" />}
            {visible ? '隐藏 Cookie' : '显示 Cookie'}
          </button>
        </div>
      </div>
      {error ? <p className={styles.error} role="alert">{error}</p> : null}
      <div className={styles.actions}>
        <button className={styles.secondaryButton} type="button" disabled={busy} onClick={() => void run(() => service.test(cookie.trim() || undefined))}>
          <PlugZap size={16} aria-hidden="true" />
          测试连接
        </button>
        <button className={styles.primaryButton} type="button" disabled={busy} onClick={() => save('secure')}>
          <Save size={16} aria-hidden="true" />
          安全保存
        </button>
        <button className={styles.secondaryButton} type="button" disabled={busy} onClick={() => save('session')}>
          <Clock3 size={16} aria-hidden="true" />
          仅本次会话
        </button>
        <button className={styles.dangerButton} type="button" disabled={busy || status?.storage === 'absent'} onClick={() => void run(() => service.clear(), true)}>
          <Trash2 size={16} aria-hidden="true" />
          清除
        </button>
      </div>
      <p className={styles.footnote}>未配置 Cookie 时仍可打开小红书页面，但主播搜索、长期状态追踪和取流不可用。</p>
    </section>
  );
}
