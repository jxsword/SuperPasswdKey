// content script 入口（SP-F-704/707/708）：接收 fill 命令，写值后立即弃引用；
// 登录提交捕获（SP-F-708）——值在提交时点读取、仅经内存消息直传 SW，不落任何存储。
// 原生 setter + input/change 事件派发（React/Vue 受控组件兼容，design_docs/12 §4）。
import { collectInputs, inputAt } from "./collect";
import { detectLoginFields } from "./detect";
import { removeSaveBanner, showSaveBanner } from "./save-banner";
import type {
  ContentMessage,
  ContentToSwMessage,
  FillResponse,
  PageInfoResponse,
  SaveConfirmResponse,
} from "../shared/messages";

function setNativeValue(el: HTMLInputElement, value: string): void {
  const proto =
    el instanceof HTMLInputElement
      ? HTMLInputElement.prototype
      : HTMLTextAreaElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(proto, "value")?.set;
  if (setter) {
    setter.call(el, value);
  } else {
    el.value = value;
  }
  el.dispatchEvent(new Event("input", { bubbles: true }));
  el.dispatchEvent(new Event("change", { bubbles: true }));
}

function performFill(
  username?: string,
  password?: string,
  totp?: string,
): FillResponse {
  const fields = detectLoginFields(collectInputs());
  if (!fields) {
    return { filled: false, reason: "no-login-form" };
  }
  if (username !== undefined && fields.usernameIndex !== null) {
    const el = inputAt(fields.usernameIndex);
    if (el) setNativeValue(el, username);
  }
  if (password !== undefined) {
    const el = inputAt(fields.passwordIndex);
    if (el) setNativeValue(el, password);
  }
  if (totp !== undefined && fields.totpIndex !== null) {
    const el = inputAt(fields.totpIndex);
    if (el) setNativeValue(el, totp);
  }
  // SP-F-707：用后即弃——本函数返回后对值的引用仅存在于调用栈，函数不缓存
  return { filled: true };
}

const bannerCallbacks = {
  onConfirm: async (): Promise<SaveConfirmResponse> => {
    const res = (await chrome.runtime.sendMessage({
      type: "sp.saveConfirmed",
    })) as SaveConfirmResponse;
    return res ?? { ok: false, error: "no response" };
  },
  onDismiss: () => {
    void chrome.runtime.sendMessage({ type: "sp.saveDismissed" }).catch(() => {});
  },
};

function showBannerFromSw(url: string, username: string): void {
  showSaveBanner({ url, username }, bannerCallbacks);
}

chrome.runtime.onMessage.addListener(
  (msg: ContentMessage, _sender, sendResponse) => {
    if (msg?.type === "sp.fill") {
      sendResponse(performFill(msg.username, msg.password, msg.totp));
      return;
    }
    if (msg?.type === "sp.pageInfo") {
      const res: PageInfoResponse = { url: location.href };
      sendResponse(res);
      return;
    }
    if (msg?.type === "sp.showSaveBanner") {
      // 同页路径（SPA / preventDefault demo）：SW 捕获后立即让本页弹浮层
      showBannerFromSw(msg.url, msg.username);
      sendResponse({ ok: true });
      return;
    }
  },
);

// ---- SP-F-708：登录提交捕获（document 级 capture——stopPropagation/preventDefault
// 场景均可命中；值在提交时点读取，提交后送 SW 内存暂存）----
document.addEventListener(
  "submit",
  () => {
    try {
      const fields = detectLoginFields(collectInputs());
      if (!fields) return;
      const passwordEl = inputAt(fields.passwordIndex);
      if (!passwordEl || !passwordEl.value) return; // 空密码不捕获
      const usernameEl =
        fields.usernameIndex !== null ? inputAt(fields.usernameIndex) : null;
      const login = {
        url: location.href,
        username: usernameEl?.value ?? "",
        password: passwordEl.value,
      };
      // SW 收到后：内存暂存 + 立即让本页弹浮层；跨导航场景由新页面加载时查询
      void chrome.runtime
        .sendMessage({ type: "sp.captured", login } satisfies ContentToSwMessage)
        .catch(() => {}); // 页面正在卸载等场景静默
    } catch {
      // 探测失败不阻塞页面自身的登录流程
    }
  },
  true,
);

// ---- 跨导航路径：本页面刚完成导航加载，向 SW 查询是否存在待保存项 ----
void (async () => {
  try {
    const pending = (await chrome.runtime.sendMessage({
      type: "sp.queryPendingSave",
    })) as { url: string; username: string } | null;
    if (pending?.url && pending.username !== undefined) {
      showBannerFromSw(pending.url, pending.username);
    }
  } catch {
    // SW 不可用（刚启动/回收）——无浮层即无保存，属可接受降级
  }
})();

// 供测试/调试：外部可显式移除浮层
export { removeSaveBanner };

