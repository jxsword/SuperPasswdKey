// Rust 侧 command 的 TypeScript 类型镜像（desktop/src-tauri DTO 单一来源口径）。
// 机密纪律（SP-F-707 口径）：密码值仅经 invoke 参数/返回值瞬态传递，不落任何前端存储。
import { invoke } from "@tauri-apps/api/core";

export interface SessionStatus {
  unlocked: boolean;
  idle_remaining_ms: number;
  absolute_remaining_ms: number;
  vault_path: string;
  vault_exists: boolean;
}

export interface EntrySummary {
  title: string;
  username: string | null;
  url: string | null;
  has_otp: boolean;
}

export interface EntryDetail {
  title: string;
  username: string | null;
  password: string | null;
  url: string | null;
  notes: string | null;
  otp: string | null;
}

export interface SettingsFile {
  idle_ms: number;
  absolute_ms: number;
  autostart: boolean;
}

export const sessionStatus = () => invoke<SessionStatus>("session_status");

export const unlock = (password: string, keyfile: string | null) =>
  invoke<SessionStatus>("unlock", { password, keyfile });

export const lockNow = () => invoke<SessionStatus>("lock_now");

export const touch = () => invoke<void>("touch");

export const getSettings = () => invoke<SettingsFile>("get_settings");

export const saveSettings = (s: SettingsFile) =>
  invoke<SettingsFile>("save_settings", { s });

export const entryList = () => invoke<EntrySummary[]>("entry_list");

export const entryGet = (title: string) => invoke<EntryDetail>("entry_get", { title });

export const entryTotp = (title: string) => invoke<string>("entry_totp", { title });

/** 复制到剪贴板（Rust 侧 arboard + 30s 超时清空，对齐 CLI clip 语义） */
export const clipCopy = (text: string) => invoke<void>("clip_copy", { text });
