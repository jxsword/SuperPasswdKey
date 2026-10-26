// 消息类型共享定义（SW ↔ host 帧结构 / SW ↔ content / SW ↔ popup）。
// 纪律（SP-F-707）：机密只在内存消息中直传，任何组件不得写入 storage。

export const HOST_NAME = "com.superpasswdkey.host";

/** SW → host 帧信封（design_docs/11 §2，P1: codec=plain） */
export interface HostEnvelope {
  v: 1;
  codec: "plain";
  payload: HostRequest;
}

export interface HostRequest {
  action: string;
  requestID: string;
  [k: string]: unknown;
}

export interface HostError {
  code: "badrequest" | "locked" | "credentials" | "notfound" | "internal";
  message: string;
}

/** get-logins 响应条目（不含密码，design_docs/11 §3） */
export interface LoginMatch {
  id: string;
  title: string;
  username: string | null;
  url: string | null;
  hasOtp: boolean;
}

export interface HostResponse {
  action: string;
  requestID: string;
  success: boolean;
  data?: unknown;
  error?: HostError;
}

/** SW → content script */
export type FillRequest = {
  type: "sp.fill";
  username?: string;
  password?: string;
  totp?: string;
};
export type PageInfoRequest = { type: "sp.pageInfo" };
export type ContentMessage = FillRequest | PageInfoRequest;

export interface FillResponse {
  filled: boolean;
  reason?: string;
}
export interface PageInfoResponse {
  url: string;
}

/** popup → SW */
export type PopupMessage =
  | { type: "status" }
  | { type: "unlock"; password: string }
  | { type: "lock" }
  | { type: "logins" }
  | { type: "fill"; id: string; username?: string | null; withTotp: boolean };

export interface StatusInfo {
  connected: boolean;
  locked: boolean;
  dbPath: string | null;
  entryCount: number | null;
}
