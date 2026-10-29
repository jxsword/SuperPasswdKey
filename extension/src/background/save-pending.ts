// 登录保存暂存队列（SP-F-708）：**纯内存**——绝不写 chrome.storage（SP-F-707 纪律）；
// MV3 SW 被回收即清空（可接受：浮层随之消失，用户重新提交登录即可）。
// 纯逻辑模块：时间注入，vitest 直测。
//
// 语义：
// - capture：登录提交时存入（每 tab 一项，新捕获覆盖旧捕获）
// - peek：跨导航浮层查询（不移除；锁定重试需要保留），过期（TTL）即清
// - confirm：用户确认保存 → 移除并返回
// - dismiss：用户忽略 → 移除 + 本 SW 会话内该 host 不再询问

export interface PendingSave {
  url: string;
  username: string;
  password: string;
  tabId: number;
  capturedAt: number;
}

/** 待保存项有效期：过期浮层不再出现（避免陈旧提示） */
export const PENDING_TTL_MS = 60_000;

/** 提取 URL 的 host（非法 URL → null）；纯函数 */
export function urlHost(url: string): string | null {
  try {
    return new URL(url).host;
  } catch {
    return null;
  }
}

export class SavePendingStore {
  private item: PendingSave | null = null;
  private dismissedHosts = new Set<string>();
  private nowMs: () => number;

  constructor(nowMs: () => number = Date.now) {
    this.nowMs = nowMs;
  }

  capture(
    login: { url: string; username: string; password: string },
    tabId: number,
  ): void {
    this.item = { ...login, tabId, capturedAt: this.nowMs() };
  }

  /** 查询该 tab 的待保存项（不移除；过期即清）；host 已被本会话忽略 → null */
  peek(tabId: number): PendingSave | null {
    const it = this.item;
    if (!it || it.tabId !== tabId) return null;
    if (this.nowMs() - it.capturedAt > PENDING_TTL_MS) {
      this.item = null;
      return null;
    }
    const host = urlHost(it.url);
    if (host !== null && this.dismissedHosts.has(host)) return null;
    return it;
  }

  /** 用户确认：移除并返回待保存项（无 → null） */
  confirm(tabId: number): PendingSave | null {
    const it = this.peek(tabId);
    this.item = null;
    return it;
  }

  /** 用户忽略：清待保存项 + 本会话不再询问该 host */
  dismiss(tabId: number): void {
    const it = this.peek(tabId);
    if (it) {
      const host = urlHost(it.url);
      if (host !== null) this.dismissedHosts.add(host);
    }
    this.item = null;
  }
}
