// popup（SP-F-705，design_docs/12 §5）：状态条 / 解锁 / 匹配列表 + 本地搜索 / 填充。
// 主密码输入后立即清空；机密不进任何存储。
import type { LoginMatch, PopupMessage, StatusInfo } from "../shared/messages";

function send<T = unknown>(msg: PopupMessage): Promise<T> {
  return chrome.runtime.sendMessage(msg) as Promise<T>;
}

const $ = (id: string): HTMLElement =>
  document.getElementById(id) as HTMLElement;

let allMatches: LoginMatch[] = [];

function render(): void {
  const list = $("matchList");
  list.textContent = "";
  const q = ($("search") as HTMLInputElement).value.trim().toLowerCase();
  const filtered = allMatches.filter(
    (m) =>
      !q ||
      m.title.toLowerCase().includes(q) ||
      (m.username ?? "").toLowerCase().includes(q),
  );
  if (filtered.length === 0) {
    const empty = document.createElement("div");
    empty.className = "hint";
    empty.textContent =
      allMatches.length === 0
        ? "当前页面没有匹配条目"
        : "无匹配搜索结果";
    list.appendChild(empty);
    return;
  }
  for (const m of filtered) {
    const row = document.createElement("div");
    row.className = "item";
    const meta = document.createElement("div");
    meta.className = "meta";
    const title = document.createElement("div");
    title.className = "title";
    title.textContent = m.title;
    const user = document.createElement("div");
    user.className = "user";
    user.textContent = m.username ?? "(无用户名)";
    meta.append(title, user);
    const btn = document.createElement("button");
    btn.className = "fill";
    btn.textContent = "填充";
    btn.addEventListener("click", () => void doFill(m));
    row.append(meta, btn);
    list.appendChild(row);
  }
}

async function doFill(m: LoginMatch): Promise<void> {
  btnFlash(m.id, "填充中…");
  const res = await send<{ filled: boolean; reason?: string }>({
    type: "fill",
    id: m.id,
    username: m.username,
    withTotp: m.hasOtp,
    url: m.url, // S3 审计：SW fill 前与目标 tab 当前 URL 复核 host
  });
  if (res.filled) {
    window.close();
  } else {
    btnFlash(m.id, `失败: ${res.reason ?? "unknown"}`);
  }
}

function btnFlash(id: string, text: string): void {
  const btns = document.querySelectorAll<HTMLButtonElement>(".fill");
  // 填充按钮按渲染序对应条目；简单场景直接刷新列表态
  const idx = allMatches.findIndex((m) => m.id === id);
  if (btns[idx]) btns[idx].textContent = text;
}

async function refreshStatus(): Promise<void> {
  const st = await send<StatusInfo>({ type: "status" });
  const status = $("status");
  if (!st.connected) {
    status.textContent =
      "无法连接 host（sp-nmhost 未注册/未运行）。请按 native-host/README 注册。";
    status.classList.add("error");
    show("unlockSection", false);
    show("listSection", false);
    return;
  }
  status.classList.remove("error");
  if (st.locked) {
    status.textContent = st.dbPath
      ? `库已锁定：${st.dbPath}`
      : "库已锁定（未配置默认库）";
    show("unlockSection", true);
    show("listSection", false);
    show("lockBtn", false);
  } else {
    status.textContent = `已解锁${st.entryCount !== null ? `（${st.entryCount} 条目）` : ""}`;
    show("unlockSection", false);
    show("listSection", true);
    show("lockBtn", true);
    await refreshMatches();
  }
}

async function refreshMatches(): Promise<void> {
  const res = await send<{
    ok: boolean;
    matches?: LoginMatch[];
    error?: string;
  }>({ type: "logins" });
  if (res.ok && res.matches) {
    allMatches = res.matches;
  } else {
    allMatches = [];
    $("status").textContent = res.error ?? "获取匹配失败";
  }
  render();
}

function show(id: string, visible: boolean): void {
  $(id).hidden = !visible;
}

async function doUnlock(): Promise<void> {
  const input = $("masterPassword") as HTMLInputElement;
  const err = $("unlockError");
  err.textContent = "";
  if (!input.value) return;
  const res = await send<{ ok: boolean; error?: string }>({
    type: "unlock",
    password: input.value,
  });
  input.value = ""; // SP-F-707：用后即清
  if (res.ok) {
    await refreshStatus();
  } else {
    err.textContent = res.error ?? "解锁失败";
  }
}

$("unlockBtn").addEventListener("click", () => void doUnlock());
$("masterPassword").addEventListener("keydown", (e) => {
  if ((e as KeyboardEvent).key === "Enter") void doUnlock();
});
$("lockBtn").addEventListener("click", async () => {
  await send({ type: "lock" });
  await refreshStatus();
});
$("search").addEventListener("input", render);

void refreshStatus();
