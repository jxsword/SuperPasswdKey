// service worker（SP-F-701/702/703）：native messaging 端口管理、消息路由、
// MV3 保活（chrome.alarms 周期 sp.ping，design_docs/11 §7）。
// 机密（密码/TOTP）只在消息链路内存直传，绝不写 chrome.storage（SP-F-707）。
// 标签页辅助见 tabs.ts。
import type {
  ContentMessage,
  HostEnvelope,
  HostResponse,
  PopupMessage,
  StatusInfo,
} from "../shared/messages";
import { HOST_NAME } from "../shared/messages";
import { activeTabId, pageUrl } from "./tabs";

const KEEPALIVE_ALARM = "sp.keepalive";
const RESPONSE_TIMEOUT_MS = 15_000;

let port: chrome.runtime.Port | null = null;
let sessionActive = false;
let seq = 0;
const pending = new Map<
  string,
  { resolve: (v: HostResponse["data"]) => void; reject: (e: Error) => void }
>();

function rejectAll(err: Error): void {
  for (const p of pending.values()) p.reject(err);
  pending.clear();
}

function ensurePort(): chrome.runtime.Port {
  if (port) return port;
  const p = chrome.runtime.connectNative(HOST_NAME);
  p.onMessage.addListener((resp: HostResponse) => {
    const entry = pending.get(resp.requestID);
    if (!entry) return;
    pending.delete(resp.requestID);
    if (resp.success) entry.resolve(resp.data);
    else
      entry.reject(
        new Error(`[${resp.error?.code}] ${resp.error?.message ?? "failed"}`),
      );
  });
  p.onDisconnect.addListener(() => {
    port = null;
    sessionActive = false;
    stopKeepalive();
    rejectAll(new Error("host disconnected (locked or not running)"));
  });
  port = p;
  return p;
}

/** 发送一帧 host 请求并等待响应（design_docs/11 §2 帧信封） */
function hostCall<T>(
  action: string,
  fields: Record<string, unknown> = {},
): Promise<T> {
  const p = ensurePort();
  const requestID = `r${Date.now().toString(36)}${(seq++).toString(36)}`;
  const envelope: HostEnvelope = {
    v: 1,
    codec: "plain",
    payload: { action, requestID, ...fields },
  };
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(requestID);
      reject(new Error(`[timeout] ${action} no response`));
    }, RESPONSE_TIMEOUT_MS);
    pending.set(requestID, {
      resolve: (v) => {
        clearTimeout(timer);
        resolve(v as T);
      },
      reject: (e) => {
        clearTimeout(timer);
        reject(e);
      },
    });
    p.postMessage(envelope);
  });
}

// ---- MV3 保活（design_docs/11 §7）：解锁期间周期 ping（收发双向重置 SW 30s 计时） ----

function startKeepalive(): void {
  chrome.alarms.create(KEEPALIVE_ALARM, { periodInMinutes: 0.5 });
}

function stopKeepalive(): void {
  void chrome.alarms.clear(KEEPALIVE_ALARM);
}

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === KEEPALIVE_ALARM && sessionActive) {
    hostCall("sp.ping").catch(() => {
      sessionActive = false;
      stopKeepalive();
    });
  }
});

// ---- popup 消息路由 ----

chrome.runtime.onMessage.addListener(
  (msg: PopupMessage, _sender, sendResponse) => {
    (async () => {
      switch (msg?.type) {
        case "status": {
          try {
            const st = await hostCall<{
              locked: boolean;
              dbPath: string | null;
              entryCount?: number;
            }>("sp.status");
            sessionActive = !st.locked;
            const info: StatusInfo = {
              connected: true,
              locked: st.locked,
              dbPath: st.dbPath,
              entryCount: st.entryCount ?? null,
            };
            sendResponse(info);
          } catch {
            const info: StatusInfo = {
              connected: false,
              locked: true,
              dbPath: null,
              entryCount: null,
            };
            sendResponse(info);
          }
          return;
        }
        case "unlock": {
          try {
            const st = await hostCall<{ entryCount: number }>("sp.unlock", {
              password: msg.password,
            });
            sessionActive = true;
            startKeepalive();
            sendResponse({ ok: true, entryCount: st.entryCount });
          } catch (e) {
            sendResponse({ ok: false, error: (e as Error).message });
          }
          return;
        }
        case "lock": {
          try {
            await hostCall("sp.lock");
          } catch {
            // 端口已断开等场景视为已锁定
          }
          sessionActive = false;
          stopKeepalive();
          sendResponse({ ok: true });
          return;
        }
        case "logins": {
          try {
            const url = await pageUrl();
            if (!url) {
              sendResponse({
                ok: false,
                error: "this page has no fillable URL",
              });
              return;
            }
            const st = await hostCall<{
              matches: import("../shared/messages").LoginMatch[];
            }>("sp.get-logins", { url });
            sendResponse({ ok: true, url, matches: st.matches });
          } catch (e) {
            sendResponse({ ok: false, error: (e as Error).message });
          }
          return;
        }
        case "fill": {
          try {
            const tabId = await activeTabId();
            if (tabId === null) throw new Error("no active tab");
            const pw = await hostCall<{ value: string }>("sp.get-secret", {
              id: msg.id,
              field: "password",
            });
            let totp: string | undefined;
            if (msg.withTotp) {
              try {
                totp = (
                  await hostCall<{ value: string }>("sp.get-secret", {
                    id: msg.id,
                    field: "totp",
                  })
                ).value;
              } catch {
                totp = undefined; // 条目无有效 otp 配置时静默跳过
              }
            }
            const fillMsg: ContentMessage = {
              type: "sp.fill",
              username: msg.username ?? undefined,
              password: pw.value,
              totp,
            };
            const res = await chrome.tabs.sendMessage(tabId, fillMsg);
            sendResponse(res);
          } catch (e) {
            sendResponse({ filled: false, reason: (e as Error).message });
          }
          return;
        }
        default:
          sendResponse({ ok: false, error: "unknown message" });
      }
    })();
    return true; // async sendResponse
  },
);
