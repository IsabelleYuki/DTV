"use client";

import React, { useMemo } from "react";

import { MainPlayer } from "@/components/player/MainPlayer";
import { platformFromInput } from "@/platforms/common/platformRegistry";

export function PlayerPage({
  platform,
  roomId,
  onRequestCloseAction
}: {
  platform: string;
  roomId: string;
  onRequestCloseAction?: () => void;
}) {
  const plat = useMemo(() => platformFromInput(platform), [platform]);
  if (!plat) {
    return (
      <div style={{ padding: 18, color: "var(--secondary-text)", fontWeight: 700 }}>
        不支持的直播平台。
      </div>
    );
  }
  if (!roomId) {
    return (
      <div style={{ padding: 18, color: "var(--secondary-text)", fontWeight: 700 }}>
        无效的房间 ID。
      </div>
    );
  }
  return <MainPlayer platform={plat} roomId={roomId} onRequestCloseAction={onRequestCloseAction} />;
}
