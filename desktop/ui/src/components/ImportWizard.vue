<script setup lang="ts">
// 导入向导（S1c）：两阶段——扫描（复用 sp-cli importer 单源，不触库不解锁）
// → 确认提交（经会话 actor 写入 + 单次保存）。候选值不回传前端（SP-F-707）：
// 表格仅显示标题/来源/密码长度。
import { ref } from "vue";
import * as api from "../api";

const emit = defineEmits<{ imported: []; close: [] }>();

const KINDS = [
  { value: "dotenv", label: ".env 文件" },
  { value: "csv", label: "CSV" },
  { value: "bw-json", label: "Bitwarden JSON" },
  { value: "files", label: "目录扫描（gitleaks 规则）" },
  { value: "env", label: "当前进程环境变量" },
] as const;

const kind = ref<string>("dotenv");
const path = ref("");
const busy = ref(false);
const error = ref("");
const scan = ref<api.ImportScanResult | null>(null);
const done = ref("");

function needPath(): boolean {
  return kind.value !== "env";
}

async function browse() {
  try {
    const p = kind.value === "files" ? await api.pickFolder() : await api.pickFile();
    if (p) path.value = p;
  } catch (e) {
    error.value = String(e);
  }
}

async function doScan() {
  busy.value = true;
  error.value = "";
  scan.value = null;
  try {
    scan.value = await api.importScan(kind.value, needPath() ? path.value || null : null);
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function doCommit() {
  busy.value = true;
  error.value = "";
  try {
    const n = await api.importCommit(kind.value, needPath() ? path.value || null : null);
    done.value = `已导入 ${n} 条条目`;
    emit("imported");
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
        <h2>导入</h2>
        <button class="close" @click="emit('close')">✕</button>
      </div>

      <label for="imp-kind">来源类型</label>
      <select id="imp-kind" v-model="kind">
        <option v-for="k in KINDS" :key="k.value" :value="k.value">{{ k.label }}</option>
      </select>

      <template v-if="needPath()">
        <label for="imp-path">路径</label>
        <div class="row">
          <input id="imp-path" v-model="path" autocomplete="off" />
          <button class="mini" type="button" @click="browse">浏览…</button>
        </div>
      </template>

      <div class="actions">
        <button class="primary" :disabled="busy || (needPath() && !path)" @click="doScan">
          {{ busy ? "扫描中…" : "扫描" }}
        </button>
      </div>

      <div class="error" v-if="error">{{ error }}</div>

      <template v-if="scan">
        <div class="summary">
          发现 {{ scan.total }} → 候选 {{ scan.unique }}（L1 去重 {{ scan.l1_duplicates }} /
          L2 合并 {{ scan.l2_merged }} / L3 冲突 {{ scan.l3_conflicts }}）
        </div>
        <ul class="review" v-if="scan.manual_review.length">
          <li v-for="m in scan.manual_review" :key="m">{{ m }}</li>
        </ul>
        <table class="cand" v-if="scan.candidates.length">
          <thead>
            <tr><th>标题</th><th>来源</th><th>密码长度</th></tr>
          </thead>
          <tbody>
            <tr v-for="c in scan.candidates" :key="c.title">
              <td>{{ c.title }}</td>
              <td class="src">{{ c.notes || "—" }}</td>
              <td>{{ c.password_len }}</td>
            </tr>
          </tbody>
        </table>
        <div class="actions" v-if="scan.candidates.length">
          <button class="primary" :disabled="busy" @click="doCommit">
            {{ busy ? "导入中…" : `确认导入 ${scan.candidates.length} 条` }}
          </button>
        </div>
        <div class="done" v-if="done">{{ done }}</div>
        <p class="hint">
          导入不与库内既有条目查重（同名允许，靠列表/搜索区分，DR-016 口径）；
          值不出本机、导入前请确认来源可信。
        </p>
      </template>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.3); display: flex; align-items: center; justify-content: center; z-index: 40; }
.panel { background: var(--panel); border-radius: 8px; padding: 20px 24px; width: 560px; max-height: 80vh; overflow: auto; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
h2 { font-size: 16px; margin: 0; }
.close { border: none; background: transparent; cursor: pointer; font-size: 14px; }
label { display: block; font-size: 12px; color: var(--muted); margin: 8px 0 4px; }
input, select { width: 100%; box-sizing: border-box; padding: 6px 8px; border: 1px solid var(--border); border-radius: 6px; font-size: 13px; background: var(--panel); color: var(--text); }
.row { display: flex; gap: 6px; }
.row input { flex: 1; }
.mini { border: 1px solid var(--border); background: var(--panel); border-radius: 6px; padding: 4px 10px; font-size: 12px; cursor: pointer; color: var(--text); }
.actions { display: flex; gap: 8px; margin-top: 12px; }
.primary { background: var(--teal); color: #fff; border: none; border-radius: 6px; padding: 7px 16px; font-size: 13px; cursor: pointer; }
.primary:disabled { opacity: .5; cursor: default; }
.error { color: var(--danger); font-size: 12px; margin-top: 8px; white-space: pre-wrap; }
.summary { font-size: 12px; color: var(--teal-dark); margin-top: 12px; }
.review { font-size: 11px; color: var(--muted); max-height: 80px; overflow: auto; margin: 6px 0; padding-left: 18px; }
.cand { width: 100%; border-collapse: collapse; font-size: 12px; margin-top: 8px; }
.cand th, .cand td { text-align: left; border-bottom: 1px solid var(--border); padding: 4px 6px; word-break: break-all; }
.src { color: var(--muted); }
.done { color: var(--teal-dark); font-size: 12px; margin-top: 8px; }
.hint { font-size: 11px; color: var(--muted); margin-top: 10px; }
</style>
