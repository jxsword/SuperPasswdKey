<script setup lang="ts">
// 详情面板（S1b-1）：只读字段展示 + 密码行操作（复制 / ▾ 显示、放大显示）+
// 自定义字段只读展示（编辑保真回传，S1b-2 提供编辑 UI）+ 编辑/删除入口。
// 密码值仅瞬态展示（掩码默认），复制走 Rust clip_copy（arboard + 30s 超时清空）。
import { onUnmounted, ref, watch } from "vue";
import * as api from "../api";
import type { EntryDetail } from "../api";
import LargeTypeModal from "./LargeTypeModal.vue";
import EntryFormModal from "./EntryFormModal.vue";

const props = defineProps<{ title: string; touch: () => void }>();

const emit = defineEmits<{
  saved: [oldTitle: string | null, newTitle: string];
  deleted: [];
}>();

const detail = ref<EntryDetail | null>(null);
const totpCode = ref("");
const totpError = ref("");
const error = ref("");
const copied = ref("");
const revealed = ref(false); // 显示/隐藏（切条目复位）
const menuOpen = ref(false); // 密码行 ▾ 下拉
const showLargeType = ref(false);
const showEdit = ref(false);
const confirmingDelete = ref(false);
let totpTimer: number | undefined;

async function load() {
  error.value = "";
  totpError.value = "";
  totpCode.value = "";
  revealed.value = false;
  menuOpen.value = false;
  confirmingDelete.value = false;
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
  if (!text) return;
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

function runMenuAction(action: "reveal" | "large" | "copy") {
  menuOpen.value = false;
  if (action === "reveal") revealed.value = !revealed.value;
  else if (action === "large") showLargeType.value = true;
  else copy(detail.value?.password || "", "密码");
}

async function doDelete() {
  try {
    await api.entryDelete(props.title);
    emit("deleted");
  } catch (e) {
    error.value = String(e);
    confirmingDelete.value = false;
  }
}

function onSaved(oldTitle: string | null, newTitle: string) {
  showEdit.value = false;
  emit("saved", oldTitle, newTitle);
}

onUnmounted(stopTotpPolling);
watch(() => props.title, load);
load();
</script>

<template>
  <div class="detail-panel">
    <div class="detail-header">
      <h2>{{ props.title }}</h2>
      <div class="header-actions" v-if="detail">
        <button class="action" @click="showEdit = true">编辑</button>
        <button class="action danger" @click="confirmingDelete = true">删除</button>
      </div>
    </div>
    <div class="error" v-if="error">{{ error }}</div>
    <template v-if="detail">
      <div class="field">
        <span class="label">用户名</span>
        <span class="value">{{ detail.username || "—" }}</span>
      </div>
      <div class="field">
        <span class="label">密码</span>
        <span class="value mono" v-if="revealed">{{ detail.password || "（无）" }}</span>
        <span class="value masked" v-else>●●●●●●●●</span>
        <button class="copy" @click="copy(detail.password || '', '密码')">复制</button>
        <span class="menu-anchor">
          <button class="copy chevron" @click="menuOpen = !menuOpen">▾</button>
          <div class="menu" v-if="menuOpen">
            <button class="menu-item" @click="runMenuAction('reveal')">
              {{ revealed ? "隐藏" : "显示" }}
            </button>
            <button class="menu-item" @click="runMenuAction('large')">放大显示</button>
            <button class="menu-item" @click="runMenuAction('copy')">复制</button>
          </div>
        </span>
      </div>
      <div class="field">
        <span class="label">URL</span>
        <span class="value">{{ detail.url || "—" }}</span>
      </div>
      <div class="field notes" v-if="detail.notes">
        <span class="label">备注</span>
        <pre class="value notes-text">{{ detail.notes }}</pre>
      </div>
      <div class="field" v-for="f in detail.custom_fields" :key="f.name">
        <span class="label">{{ f.name }}</span>
        <span class="value mono">{{ f.protected ? "●●●●●●●●" : f.value }}</span>
        <button class="copy" v-if="!f.protected" @click="copy(f.value, f.name)">复制</button>
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

    <LargeTypeModal
      v-if="showLargeType"
      :value="detail?.password || ''"
      @close="showLargeType = false"
    />
    <EntryFormModal
      v-if="showEdit && detail"
      mode="edit"
      :initial="detail"
      @saved="onSaved"
      @close="showEdit = false"
    />
    <div class="confirm-overlay" v-if="confirmingDelete" @click.self="confirmingDelete = false">
      <div class="confirm-box">
        <p>确定删除「{{ props.title }}」？此操作不可撤销（旧版本历史随条目一并删除）。</p>
        <div class="actions">
          <button class="primary danger" @click="doDelete">删除</button>
          <button class="secondary" @click="confirmingDelete = false">取消</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.detail-panel {
  max-width: 720px;
  position: relative;
}

.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

h2 {
  font-size: 18px;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.action {
  border: 1px solid var(--border);
  background: var(--panel);
  border-radius: 6px;
  padding: 4px 14px;
  font-size: 13px;
  cursor: pointer;
}

.action:hover {
  border-color: var(--teal);
}

.action.danger {
  color: var(--danger);
}

.action.danger:hover {
  border-color: var(--danger);
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
  overflow: hidden;
  text-overflow: ellipsis;
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

.mono {
  font-family: "Consolas", monospace;
}

.totp {
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

.menu-anchor {
  position: relative;
  flex-shrink: 0;
}

.chevron {
  padding: 3px 8px;
}

.menu {
  position: absolute;
  right: 0;
  top: calc(100% + 4px);
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 6px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
  z-index: 5;
  min-width: 120px;
  overflow: hidden;
}

.menu-item {
  display: block;
  width: 100%;
  text-align: left;
  border: none;
  background: transparent;
  padding: 8px 14px;
  font-size: 13px;
  cursor: pointer;
}

.menu-item:hover {
  background: #e0f2f1;
}

.copied {
  margin-top: 12px;
  font-size: 12px;
  color: var(--teal-dark);
}

.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 30;
}

.confirm-box {
  background: var(--panel);
  border-radius: 8px;
  padding: 20px 24px;
  width: 360px;
}

.confirm-box p {
  font-size: 13px;
  margin: 0 0 16px;
}

.actions {
  display: flex;
  gap: 8px;
}

button.primary,
button.secondary {
  flex: 1;
  padding: 8px;
  font-size: 13px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
}

.primary {
  background: var(--teal);
  color: #fff;
}

.primary.danger {
  background: var(--danger);
}

.secondary {
  background: #eceff1;
}
</style>
