// 活动标签页辅助（design_docs/12 §1）：不用 tabs 权限——URL 由 content script 提供。
import type { ContentMessage, PageInfoResponse } from "../shared/messages";

export async function activeTabId(): Promise<number | null> {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab?.id ?? null;
}

export async function pageUrl(): Promise<string | null> {
  const tabId = await activeTabId();
  if (tabId === null) return null;
  try {
    const res = await chrome.tabs.sendMessage(tabId, {
      type: "sp.pageInfo",
    } satisfies ContentMessage);
    return (res as PageInfoResponse).url;
  } catch {
    return null; // chrome:// 页等无 content script 场景
  }
}
