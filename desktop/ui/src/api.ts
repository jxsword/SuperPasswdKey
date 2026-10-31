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
  tags: string[];
}

export interface CustomField {
  name: string;
  value: string;
  protected: boolean;
}

export interface EntryDetail {
  title: string;
  username: string | null;
  password: string | null;
  url: string | null;
  notes: string | null;
  otp: string | null;
  custom_fields: CustomField[];
  tags: string[];
}

/** 条目草稿（创建/编辑共用；编辑时 custom_fields 原样回传保真） */
export interface EntryDraft {
  title: string;
  username: string | null;
  password: string | null;
  url: string | null;
  notes: string | null;
  otp: string | null;
  custom_fields: CustomField[];
  tags: string[];
}

export interface SettingsFile {
  idle_ms: number;
  absolute_ms: number;
  autostart: boolean;
  /** 上次成功解锁使用的 keyfile 路径（空=未使用；文件不存在时后端返回空串） */
  keyfile_path: string;
}

export const sessionStatus = () => invoke<SessionStatus>("session_status");

export const unlock = (password: string, keyfile: string | null) =>
  invoke<SessionStatus>("unlock", { password, keyfile });

export const lockNow = () => invoke<SessionStatus>("lock_now");

/** 编辑态开关（EntryFormModal 打开期间暂停 idle 超时，absolute 保留） */
export const setEditing = (active: boolean) => invoke<void>("set_editing", { active });

/** keyfile 文件浏览（系统对话框；取消返回空串） */
export const keyfilePick = () => invoke<string>("keyfile_pick");

export const touch = () => invoke<void>("touch");

export const getSettings = () => invoke<SettingsFile>("get_settings");

export const saveSettings = (s: SettingsFile) =>
  invoke<SettingsFile>("save_settings", { s });

export const entryList = () => invoke<EntrySummary[]>("entry_list");

export const entryGet = (title: string) => invoke<EntryDetail>("entry_get", { title });

export const entryTotp = (title: string) => invoke<string>("entry_totp", { title });

/** 新建条目（Rust 侧 add_entry + save_session；同名允许，对齐 CLI 口径） */
export const entryCreate = (draft: EntryDraft) => invoke<void>("entry_create", { draft });

/** 编辑条目（按 oldTitle 定位；标题可经 draft.title 改写；旧快照进 history） */
export const entryEdit = (oldTitle: string, draft: EntryDraft) =>
  invoke<void>("entry_edit", { oldTitle, draft });

/** 删除条目 */
export const entryDelete = (title: string) => invoke<void>("entry_delete", { title });

/** 列条目附件名 */
export const attachmentList = (title: string) => invoke<string[]>("attachment_list", { title });

/** 添加附件（同名覆盖，对齐 CLI 语义；data 为原始字节） */
export const attachmentAdd = (title: string, name: string, data: Uint8Array) =>
  invoke<void>("attachment_add", { title, name, data: Array.from(data) });

/**
 * 下载附件：Rust 弹系统保存对话框 → 落盘 + 限权收敛（icacls/0600，CLI 同语义）。
 * 返回落盘路径；用户取消返回空串。
 */
export const attachmentDownload = (title: string, name: string) =>
  invoke<string>("attachment_download", { title, name });

/** 复制到剪贴板（Rust 侧 arboard + 30s 超时清空，对齐 CLI clip 语义） */
export const clipCopy = (text: string) => invoke<void>("clip_copy", { text });
