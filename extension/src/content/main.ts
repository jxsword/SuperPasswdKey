// content script 入口（SP-F-704/707）：接收 fill 命令，写值后立即弃引用。
// 原生 setter + input/change 事件派发（React/Vue 受控组件兼容，design_docs/12 §4）。
import { collectInputs, inputAt } from "./collect";
import { detectLoginFields } from "./detect";
import type { ContentMessage, FillResponse, PageInfoResponse } from "../shared/messages";

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
  },
);
