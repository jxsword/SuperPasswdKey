// SuperPasswdKey 桌面壳前端（静态零依赖；经 withGlobalTauri 暴露的 API 调 Rust command）。
// 机密纪律：密码值只经 invoke 参数传给 Rust（SP-F-804：JS 侧输入瞬态，不写日志/存储）。
const { invoke } = window.__TAURI__.core;

const statusEl = document.getElementById("status");
const unlockForm = document.getElementById("unlockForm");
const lockBtn = document.getElementById("lockBtn");
const unlockBtn = document.getElementById("unlockBtn");
const passwordEl = document.getElementById("password");
const keyfileEl = document.getElementById("keyfile");

function fmtRemaining(ms) {
  if (ms <= 0) return "0s";
  const s = Math.ceil(ms / 1000);
  return s >= 60 ? `${Math.floor(s / 60)}m${s % 60}s` : `${s}s`;
}

function render(st) {
  const vaultLine = `库：${st.vault_exists ? st.vault_path : st.vault_path + "（不存在，可用 sp init 创建）"}`;
  if (st.unlocked) {
    statusEl.textContent =
      `会话：已解锁\n空闲剩余 ${fmtRemaining(st.idle_remaining_ms)} · 绝对剩余 ${fmtRemaining(st.absolute_remaining_ms)}\n${vaultLine}`;
    unlockForm.hidden = true;
    lockBtn.hidden = false;
  } else {
    statusEl.textContent = `会话：已锁定\n${vaultLine}`;
    unlockForm.hidden = false;
    lockBtn.hidden = true;
  }
}

async function refresh() {
  try {
    render(await invoke("session_status"));
  } catch (e) {
    statusEl.textContent = `状态获取失败：${e}`;
  }
}

unlockBtn.addEventListener("click", async () => {
  unlockBtn.disabled = true;
  statusEl.textContent = "解锁中（KDF 派生，约 1–3 秒）…";
  try {
    // 密码经 invoke 直达 Rust，调用后 JS 侧不再持有引用
    render(await invoke("unlock", { password: passwordEl.value, keyfile: keyfileEl.value || null }));
    passwordEl.value = "";
  } catch (e) {
    statusEl.textContent = String(e);
  } finally {
    unlockBtn.disabled = false;
  }
});

passwordEl.addEventListener("keydown", (ev) => {
  if (ev.key === "Enter") unlockBtn.click();
});

lockBtn.addEventListener("click", async () => {
  render(await invoke("lock_now"));
});

// ---- 设置（任务 2：settings.json + HKCU 自启；超时配置重启生效）----
const idleMinEl = document.getElementById("idleMin");
const absMinEl = document.getElementById("absMin");
const autostartChk = document.getElementById("autostartChk");
const saveBtn = document.getElementById("saveBtn");
const settingsMsg = document.getElementById("settingsMsg");

const msToMin = (ms) => Math.max(1, Math.round(ms / 60000));
const minToMs = (min) => Math.max(60000, Math.round(min * 60000));

async function loadSettings() {
  try {
    const s = await invoke("get_settings");
    idleMinEl.value = msToMin(s.idle_ms);
    absMinEl.value = msToMin(s.absolute_ms);
    autostartChk.checked = s.autostart;
  } catch (e) {
    settingsMsg.textContent = `设置读取失败：${e}`;
  }
}

saveBtn.addEventListener("click", async () => {
  saveBtn.disabled = true;
  settingsMsg.textContent = "保存中…";
  try {
    await invoke("save_settings", {
      s: {
        idle_ms: minToMs(Number(idleMinEl.value)),
        absolute_ms: minToMs(Number(absMinEl.value)),
        autostart: autostartChk.checked,
      },
    });
    settingsMsg.textContent = "已保存：自启即时生效；超时配置重启后生效。";
  } catch (e) {
    settingsMsg.textContent = `保存失败：${e}`;
  } finally {
    saveBtn.disabled = false;
  }
});

refresh();
loadSettings();
// 每 5s 刷新倒计时展示；机密操作以后端惰性检查为准（展示不构成安全边界）
setInterval(refresh, 5000);
