<script setup lang="ts">
// 解锁面板：主密码 + keyfile（复用 v0.3.0 静态页语义；密码经 invoke 直达 Rust，不落前端存储）。
// keyfile 路径自动记忆：预填上次成功解锁的路径（文件不存在时后端返回空），
// 支持系统文件浏览选择（2026-10-31 验收反馈）。
import { onMounted, ref } from "vue";
import * as api from "../api";
import { useSession } from "../composables/useSession";

const props = defineProps<{ error: string }>();
const emit = defineEmits<{ unlocked: [] }>();

const session = useSession();
const password = ref("");
const keyfile = ref("");
const busy = ref(false);
const localError = ref("");

onMounted(async () => {
  try {
    const s = await api.getSettings();
    keyfile.value = s.keyfile_path;
  } catch {
    // 预填失败不阻断解锁
  }
});

async function browseKeyfile() {
  try {
    const p = await api.keyfilePick();
    if (p) keyfile.value = p;
  } catch (e) {
    localError.value = String(e);
  }
}

async function submit() {
  if (!password.value) {
    localError.value = "请输入主密码";
    return;
  }
  busy.value = true;
  localError.value = "";
  try {
    await session.unlock(password.value, keyfile.value);
    password.value = "";
    keyfile.value = "";
    emit("unlocked");
  } catch (e) {
    localError.value = String(e);
  } finally {
    busy.value = false;
  }
}

function onKeydown(ev: KeyboardEvent) {
  if (ev.key === "Enter") submit();
}
</script>

<template>
  <div class="unlock-panel">
    <div class="status" v-if="props.error">{{ props.error }}</div>
    <label for="unlock-password">主密码</label>
    <input
      id="unlock-password"
      type="password"
      v-model="password"
      autocomplete="off"
      @keydown="onKeydown"
    />
    <label for="unlock-keyfile">keyfile 路径（可选；自动记忆上次使用的路径）</label>
    <div class="kf-row">
      <input id="unlock-keyfile" type="text" v-model="keyfile" autocomplete="off" />
      <button class="browse" type="button" @click="browseKeyfile">浏览…</button>
    </div>
    <div class="error" v-if="localError">{{ localError }}</div>
    <button class="submit" :disabled="busy" @click="submit">
      {{ busy ? "解锁中（KDF 派生，约 1–3 秒）…" : "解锁" }}
    </button>
    <p class="hint">
      库路径：%APPDATA%\SuperPasswdKey\vault.kdbx<br />
      创建：<code>sp init --db "%APPDATA%\SuperPasswdKey\vault.kdbx"</code>
    </p>
  </div>
</template>

<style scoped>
.unlock-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 32px;
  max-width: 420px;
  margin: 0 auto;
  height: 100%;
  justify-content: center;
}

.status {
  font-size: 12px;
  color: var(--danger);
}

label {
  font-size: 12px;
  color: var(--muted);
}

input {
  padding: 8px 10px;
  font-size: 14px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--panel);
}

input:focus {
  outline: 2px solid rgba(0, 150, 136, 0.2);
  border-color: var(--teal);
}

.kf-row {
  display: flex;
  gap: 6px;
}

.kf-row input {
  flex: 1;
}

.browse {
  border: 1px solid var(--border);
  background: var(--panel);
  border-radius: 6px;
  padding: 0 12px;
  font-size: 12px;
  cursor: pointer;
  flex-shrink: 0;
}

.browse:hover {
  border-color: var(--teal);
}

.error {
  font-size: 12px;
  color: var(--danger);
}

button {
  margin-top: 12px;
  padding: 10px;
  font-size: 14px;
  border: none;
  border-radius: 6px;
  background: var(--teal);
  color: #fff;
  cursor: pointer;
}

button:disabled {
  opacity: 0.6;
  cursor: default;
}

.hint {
  font-size: 11px;
  color: var(--muted);
  margin-top: 16px;
  line-height: 1.6;
}

code {
  background: #eceff1;
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 11px;
}
</style>
