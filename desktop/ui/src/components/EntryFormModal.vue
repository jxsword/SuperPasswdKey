<script setup lang="ts">
// 条目创建/编辑表单（模态）。校验对齐 CLI 语义：标题必填；otp 必须 otpauth:// 开头
// （对应 CLI 退出码 4 的参数错误，前端预校验 + 后端兜底）；同名允许（靠列表/搜索区分）。
// 编辑时 custom_fields 原样回传保真（S1b-1 不渲染编辑控件，S1b-2 提供 UI）。
import { computed, ref } from "vue";
import * as api from "../api";
import type { EntryDetail, EntryDraft } from "../api";

const props = defineProps<{
  mode: "create" | "edit";
  /** 编辑模式：详情预填（含 custom_fields 保真回传） */
  initial?: EntryDetail | null;
}>();

const emit = defineEmits<{
  saved: [oldTitle: string | null, newTitle: string];
  close: [];
}>();

const title = ref(props.initial?.title ?? "");
const username = ref(props.initial?.username ?? "");
const password = ref(props.initial?.password ?? "");
const url = ref(props.initial?.url ?? "");
const notes = ref(props.initial?.notes ?? "");
const otp = ref(props.initial?.otp ?? "");
const tagsText = ref((props.initial?.tags ?? []).join(", "));
const showPassword = ref(props.mode === "edit");
const busy = ref(false);
const error = ref("");

const heading = computed(() => (props.mode === "create" ? "新的项目" : `编辑：${props.initial?.title}`));

function buildDraft(): EntryDraft {
  return {
    title: title.value.trim(),
    username: username.value.trim() || null,
    password: password.value || null,
    url: url.value.trim() || null,
    notes: notes.value || null,
    otp: otp.value.trim() || null,
    custom_fields: props.initial?.custom_fields ?? [],
    tags: tagsText.value
      .split(/[，,]/)
      .map((t) => t.trim())
      .filter((t) => t.length > 0),
  };
}

function validate(draft: EntryDraft): string {
  if (!draft.title) return "标题必填";
  if (draft.otp && !draft.otp.startsWith("otpauth://")) {
    return "TOTP 链接必须以 otpauth:// 开头";
  }
  return "";
}

async function submit() {
  const draft = buildDraft();
  const invalid = validate(draft);
  if (invalid) {
    error.value = invalid;
    return;
  }
  busy.value = true;
  error.value = "";
  try {
    if (props.mode === "create") {
      await api.entryCreate(draft);
      emit("saved", null, draft.title);
    } else {
      await api.entryEdit(props.initial!.title, draft);
      emit("saved", props.initial!.title, draft.title);
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="form-overlay" @click.self="emit('close')">
    <div class="form-panel">
      <div class="form-header">
        <h2>{{ heading }}</h2>
        <button class="close" @click="emit('close')">✕</button>
      </div>
      <label for="f-title">标题 *</label>
      <input id="f-title" v-model="title" autocomplete="off" />
      <label for="f-username">用户名</label>
      <input id="f-username" v-model="username" autocomplete="off" />
      <label for="f-password">密码</label>
      <div class="pw-row">
        <input
          id="f-password"
          v-model="password"
          :type="showPassword ? 'text' : 'password'"
          autocomplete="off"
        />
        <button class="mini" type="button" @click="showPassword = !showPassword">
          {{ showPassword ? "隐藏" : "显示" }}
        </button>
      </div>
      <label for="f-url">URL</label>
      <input id="f-url" v-model="url" autocomplete="off" />
      <label for="f-notes">备注</label>
      <textarea id="f-notes" v-model="notes" rows="3"></textarea>
      <label for="f-otp">TOTP（otpauth:// 链接）</label>
      <input id="f-otp" v-model="otp" autocomplete="off" />
      <label for="f-tags">标签（逗号分隔）</label>
      <input id="f-tags" v-model="tagsText" autocomplete="off" placeholder="例如：work, infra" />
      <div class="error" v-if="error">{{ error }}</div>
      <div class="actions">
        <button class="primary" :disabled="busy" @click="submit">
          {{ busy ? "保存中…" : "保存" }}
        </button>
        <button class="secondary" :disabled="busy" @click="emit('close')">取消</button>
      </div>
      <p class="hint" v-if="mode === 'edit'">保存后旧版本自动进入条目历史（与 CLI/KeePassXC 互认）</p>
    </div>
  </div>
</template>

<style scoped>
.form-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
}

.form-panel {
  background: var(--panel);
  border-radius: 8px;
  padding: 24px;
  width: 440px;
  max-height: 86vh;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

h2 {
  font-size: 16px;
  margin: 0;
}

.close {
  border: none;
  background: transparent;
  font-size: 16px;
  cursor: pointer;
  color: var(--muted);
}

label {
  font-size: 12px;
  color: var(--muted);
}

input,
textarea {
  padding: 7px 10px;
  font-size: 13px;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-family: inherit;
  background: var(--panel);
}

input:focus,
textarea:focus {
  outline: 2px solid rgba(0, 150, 136, 0.2);
  border-color: var(--teal);
}

.pw-row {
  display: flex;
  gap: 6px;
}

.pw-row input {
  flex: 1;
}

.mini {
  border: 1px solid var(--border);
  background: var(--panel);
  border-radius: 6px;
  padding: 0 12px;
  font-size: 12px;
  cursor: pointer;
}

.error {
  font-size: 12px;
  color: var(--danger);
  margin-top: 6px;
}

.actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

button.primary,
button.secondary {
  flex: 1;
  padding: 9px;
  font-size: 14px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
}

.primary {
  background: var(--teal);
  color: #fff;
}

.secondary {
  background: #eceff1;
  color: var(--text);
}

button:disabled {
  opacity: 0.6;
  cursor: default;
}

.hint {
  font-size: 11px;
  color: var(--muted);
  margin: 8px 0 0;
}
</style>
