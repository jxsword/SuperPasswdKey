<script setup lang="ts">
// 导出对话框（S1c）：加密归档（zip + age 口令加密）——复用 CLI 同源实现
// （sp-kdbx::export，经会话凭据零凭据入参）。口令仅存在于本次调用栈。
// 明文导出不提供（三重防护口径）：如需明文 JSON 请走 CLI `sp export --plaintext`。
import { ref } from "vue";
import * as api from "../api";

const emit = defineEmits<{ close: [] }>();

const pw1 = ref("");
const pw2 = ref("");
const busy = ref(false);
const error = ref("");
const result = ref<api.ExportArchiveOut | null>(null);

async function doExport() {
  if (!pw1.value) {
    error.value = "归档口令不能为空";
    return;
  }
  if (pw1.value !== pw2.value) {
    error.value = "两次口令不一致";
    return;
  }
  busy.value = true;
  error.value = "";
  try {
    result.value = await api.exportArchive(pw1.value, pw2.value);
    pw1.value = "";
    pw2.value = "";
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="panel">
      <div class="header">
        <h2>导出（加密归档）</h2>
        <button class="close" @click="emit('close')">✕</button>
      </div>

      <template v-if="!result">
        <label for="exp-pw1">归档口令</label>
        <input id="exp-pw1" type="password" v-model="pw1" autocomplete="off" />
        <label for="exp-pw2">确认口令</label>
        <input id="exp-pw2" type="password" v-model="pw2" autocomplete="off" @keydown.enter="doExport" />
        <div class="actions">
          <button class="primary" :disabled="busy" @click="doExport">
            {{ busy ? "导出中…" : "选择位置并导出" }}
          </button>
        </div>
        <div class="error" v-if="error">{{ error }}</div>
        <p class="hint">
          归档为 age 口令加密（zip 容器：条目 JSON + 附件），可用官方 age 工具独立解密；
          与主密码无关——请将归档口令与主密码分开保管。含全部条目/自定义字段/TOTP/附件；
          不含回收站对象。
        </p>
      </template>

      <template v-else>
        <div class="done">已导出：{{ result.path }}</div>
        <div class="summary">
          {{ result.summary.entries }} 条条目 · 附件 {{ result.summary.attachments }} 个
        </div>
        <ul class="warn" v-if="result.summary.warnings.length">
          <li v-for="w in result.summary.warnings" :key="w">{{ w }}</li>
        </ul>
        <div class="actions">
          <button class="secondary" @click="emit('close')">完成</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.3); display: flex; align-items: center; justify-content: center; z-index: 40; }
.panel { background: var(--panel); border-radius: 8px; padding: 20px 24px; width: 460px; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
h2 { font-size: 16px; margin: 0; }
.close { border: none; background: transparent; cursor: pointer; font-size: 14px; }
label { display: block; font-size: 12px; color: var(--muted); margin: 8px 0 4px; }
input { width: 100%; box-sizing: border-box; padding: 6px 8px; border: 1px solid var(--border); border-radius: 6px; font-size: 13px; background: var(--panel); color: var(--text); }
.actions { display: flex; gap: 8px; margin-top: 14px; }
.primary { background: var(--teal); color: #fff; border: none; border-radius: 6px; padding: 7px 16px; font-size: 13px; cursor: pointer; }
.primary:disabled { opacity: .5; cursor: default; }
.secondary { border: 1px solid var(--border); background: var(--panel); color: var(--text); border-radius: 6px; padding: 7px 16px; font-size: 13px; cursor: pointer; }
.error { color: var(--danger); font-size: 12px; margin-top: 8px; }
.done { color: var(--teal-dark); font-size: 12px; word-break: break-all; margin-top: 8px; }
.summary { font-size: 12px; color: var(--text); margin-top: 6px; }
.warn { font-size: 11px; color: var(--muted); padding-left: 18px; }
.hint { font-size: 11px; color: var(--muted); margin-top: 10px; line-height: 1.6; }
</style>
