<script setup lang="ts">
// 详情只读面板（MVP）：字段展示 + 密码掩码/复制 + TOTP 动态码 30s 轮询/复制。
// 密码值仅瞬态展示（掩码），复制走 Rust clip_copy（arboard + 30s 超时清空）。
import { onUnmounted, ref, watch } from "vue";
import * as api from "../api";
import type { EntryDetail } from "../api";

const props = defineProps<{ title: string; touch: () => void }>();

const detail = ref<EntryDetail | null>(null);
const totpCode = ref("");
const totpError = ref("");
const error = ref("");
const copied = ref("");
let totpTimer: number | undefined;

async function load() {
  error.value = "";
  totpError.value = "";
  totpCode.value = "";
  try {
    detail.value = await api.entryGet(props.title);
    props.touch();
    if (detail.value.otp) {
      await refreshTotp();
      startTotpPolling();
    }
  } catch (e) {
    error.value = String(e);
  }
}

async function refreshTotp() {
  try {
    totpCode.value = await api.entryTotp(props.title);
    totpError.value = "";
    props.touch();
  } catch (e) {
    totpError.value = String(e);
  }
}

function startTotpPolling() {
  stopTotpPolling();
  totpTimer = window.setInterval(refreshTotp, 30_000);
}

function stopTotpPolling() {
  if (totpTimer !== undefined) {
    window.clearInterval(totpTimer);
    totpTimer = undefined;
  }
}

async function copy(text: string, label: string) {
  try {
    await api.clipCopy(text);
    copied.value = label;
    window.setTimeout(() => {
      if (copied.value === label) copied.value = "";
    }, 2000);
  } catch (e) {
    error.value = String(e);
  }
}

onUnmounted(stopTotpPolling);
watch(() => props.title, load);
load();
</script>

<template>
  <div class="detail-panel">
    <h2>{{ props.title }}</h2>
    <div class="error" v-if="error">{{ error }}</div>
    <template v-if="detail">
      <div class="field">
        <span class="label">用户名</span>
        <span class="value">{{ detail.username || "—" }}</span>
      </div>
      <div class="field">
        <span class="label">密码</span>
        <span class="value masked">●●●●●●●●</span>
        <button class="copy" @click="copy(detail.password || '', '密码')">复制</button>
      </div>
      <div class="field">
        <span class="label">URL</span>
        <span class="value">{{ detail.url || "—" }}</span>
      </div>
      <div class="field notes" v-if="detail.notes">
        <span class="label">备注</span>
        <pre class="value notes-text">{{ detail.notes }}</pre>
      </div>
      <div class="field" v-if="detail.otp">
        <span class="label">TOTP</span>
        <span class="value totp" :class="{ error: totpError }">
          {{ totpError || totpCode || "…" }}
        </span>
        <button class="copy" :disabled="!totpCode" @click="copy(totpCode, 'TOTP')">复制</button>
      </div>
      <div class="copied" v-if="copied">已复制 {{ copied }}，30 秒后自动清空</div>
    </template>
  </div>
</template>

<style scoped>
.detail-panel {
  max-width: 720px;
}

h2 {
  font-size: 18px;
  margin: 0 0 16px;
}

.error {
  font-size: 12px;
  color: var(--danger);
  margin-bottom: 12px;
}

.field {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}

.field.notes {
  align-items: flex-start;
}

.label {
  width: 56px;
  font-size: 12px;
  color: var(--muted);
  flex-shrink: 0;
}

.value {
  flex: 1;
  font-size: 13px;
  word-break: break-all;
  user-select: text;
}

.masked {
  letter-spacing: 2px;
}

.totp {
  font-family: "Consolas", monospace;
  font-size: 18px;
  letter-spacing: 4px;
}

.totp.error {
  color: var(--danger);
  font-size: 12px;
}

.notes-text {
  margin: 0;
  white-space: pre-wrap;
  font-family: inherit;
}

.copy {
  border: 1px solid var(--border);
  background: var(--panel);
  border-radius: 6px;
  padding: 3px 12px;
  font-size: 12px;
  cursor: pointer;
  flex-shrink: 0;
}

.copy:hover {
  border-color: var(--teal);
}

.copy:disabled {
  opacity: 0.5;
  cursor: default;
}

.copied {
  margin-top: 12px;
  font-size: 12px;
  color: var(--teal-dark);
}
</style>
