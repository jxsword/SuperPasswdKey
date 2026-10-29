// 页内保存确认浮层（SP-F-708）：不回显密码；确认后经 SW 调 sp.save-login；
// 拒绝路径零请求零写入。DOM 层薄壳——状态机（保存中/已保存/需解锁/旧 host）在此处理。
// 样式内联（扩展无外部 CSS 依赖），fixed 右下角，不阻塞页面交互。

export interface SaveConfirmResult {
  ok: boolean;
  saved?: "added" | "updated";
  title?: string;
  error?: string;
  staleHost?: boolean;
}

export interface SaveBannerCallbacks {
  /** 用户点“保存”；实现方调 SW 的 sp.saveConfirmed */
  onConfirm: () => Promise<SaveConfirmResult>;
  /** 用户点“忽略”；实现方调 SW 的 sp.saveDismissed */
  onDismiss: () => void;
}

const BANNER_ID = "sp-save-banner";

function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/** 移除既有浮层（幂等：同页重复触发/已保存后再触发） */
export function removeSaveBanner(): void {
  document.getElementById(BANNER_ID)?.remove();
}

export function showSaveBanner(
  opts: { url: string; username: string },
  cb: SaveBannerCallbacks,
): void {
  removeSaveBanner();

  const root = document.createElement("div");
  root.id = BANNER_ID;
  root.style.cssText = [
    "position:fixed", "right:16px", "bottom:16px", "z-index:2147483647",
    "max-width:340px", "padding:12px 14px", "border-radius:10px",
    "background:#263238", "color:#eceff1",
    "font:13px/1.5 'Segoe UI','Microsoft YaHei',sans-serif",
    "box-shadow:0 6px 24px rgba(0,0,0,.35)",
  ].join(";");

  const title = document.createElement("div");
  title.textContent = "SuperPasswdKey · 检测到登录提交";
  title.style.cssText = "font-weight:600;margin-bottom:4px";
  root.appendChild(title);

  const line = document.createElement("div");
  line.textContent = `将用户名「${opts.username}」保存到 ${hostOf(opts.url)} 的密码库？`;
  line.style.cssText = "margin-bottom:10px;word-break:break-all";
  root.appendChild(line);

  const status = document.createElement("div");
  status.style.cssText = "margin-bottom:8px;color:#b0bec5;min-height:1em";
  root.appendChild(status);

  const btnRow = document.createElement("div");
  btnRow.style.cssText = "display:flex;gap:8px";
  const saveBtn = document.createElement("button");
  saveBtn.textContent = "保存";
  saveBtn.style.cssText =
    "flex:1;padding:6px 10px;border:none;border-radius:6px;background:#009688;color:#fff;cursor:pointer;font-size:13px";
  const dismissBtn = document.createElement("button");
  dismissBtn.textContent = "忽略";
  dismissBtn.style.cssText =
    "flex:1;padding:6px 10px;border:none;border-radius:6px;background:#546e7a;color:#fff;cursor:pointer;font-size:13px";
  btnRow.appendChild(saveBtn);
  btnRow.appendChild(dismissBtn);
  root.appendChild(btnRow);
  document.documentElement.appendChild(root);

  dismissBtn.addEventListener("click", () => {
    removeSaveBanner();
    cb.onDismiss();
  });

  saveBtn.addEventListener("click", async () => {
    saveBtn.disabled = true;
    status.textContent = "保存中…";
    let res: SaveConfirmResult;
    try {
      res = await cb.onConfirm();
    } catch (e) {
      res = { ok: false, error: (e as Error).message };
    }
    if (res.ok) {
      status.textContent =
        res.saved === "updated"
          ? `已更新条目「${res.title ?? ""}」（旧密码已存入历史）`
          : `已保存为条目「${res.title ?? ""}」`;
      status.style.color = "#a5d6a7";
      dismissBtn.remove();
      saveBtn.remove();
      setTimeout(removeSaveBanner, 2500);
      return;
    }
    saveBtn.disabled = false;
    if (res.staleHost) {
      status.textContent = "当前 host 版本过旧，不支持自动保存（请更新 sp-desktop/sp-nmhost）";
      status.style.color = "#ffcc80";
      btnRow.remove();
      return;
    }
    if ((res.error ?? "").startsWith("[locked]")) {
      status.textContent = "密码库已锁定：请点击扩展图标解锁后，再次提交登录或点“保存”重试";
      status.style.color = "#ffcc80";
      return; // 保留浮层与 pending，可重试
    }
    status.textContent = `保存失败：${res.error ?? "unknown"}`;
    status.style.color = "#ef9a9a";
  });
}
