<script setup lang="ts">
// 注入启动器（S1c）：从 GUI 以 sp run 语义启动应用——选条目与注入字段，
// 进程生成经 sp-cli::spawner 单源（argv 直 exec 无 shell），env 值从会话条目
// 解析、秘密不落盘；子进程输出不遮蔽（对齐 sp run --no-masking 文档口径）。
import { onMounted, ref } from "vue";
import * as api from "../api";

const emit = defineEmits<{ close: [] }>();

const props = defineProps<{ initialTitle?: string | null }>();

const title = ref(props.initialTitle ?? "");
const detail = ref<api.EntryDetail | null>(null);
const program = ref("");
const argsText = ref("");
const selected = ref<Record<string, boolean>>({});
const busy = ref(false);
const error = ref("");
const done = ref("");

const STANDARD = ["UserName", "Password", "URL", "totp"] as const;

function fieldNames(): string[] {
  const names: string[] = [...STANDARD];
  for (const f of detail.value?.custom_fields ?? []) {
    if (!names.includes(f.name)) names.push(f.name);
  }
  return names;
}

async function loadEntry() {
  error.value = "";
  detail.value = null;
  selected.value = {};
  if (!title.value.trim()) {
    error.value = "请输入条目标题";
    return;
  }
  try {
    detail.value = await api.entryGet(title.value.trim());
    // 默认勾选 Password（最常用）
    for (const n of fieldNames()) selected.value[n] = n === "Password";
  } catch (e) {
    error.value = String(e);
  }
}

async function browse() {
  try {
    const p = await api.pickFile();
    if (p) program.value = p;
  } catch (e) {
    error.value = String(e);
  }
}

function chosenFields(): string[] {
  return fieldNames().filter((n) => selected.value[n]);
}

async function doLaunch() {
  busy.value = true;
  error.value = "";
  done.value = "";
  try {
    const args = argsText.value.trim() ? argsText.value.trim().split(/\s+/) : [];
    const pid = await api.launchWithEnv(
      program.value.trim(),
      args,
      title.value.trim(),
      chosenFields(),
    );
    done.value = `已启动（PID ${pid}）。注入变量仅存在于子进程环境；子进程输出不做遮蔽（对齐 sp run --no-masking）。`;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

// CLI 深度用户的等价 .env 工作流提示（SP-F-301：值不落盘，引用不落值）
const envHint =
  `CLI 等价写法（.env + sp run）：GH_TOKEN=sp://<条目>/Password 写入 .env 后 ` +
  `\`sp run --env-file .env -- <命令>\`；GUI 注入启动与本工作流同语义。`;
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="panel">
      <div class="header">
        <h2>注入启动</h2>
        <button class="close" @click="emit('close')">✕</button>
      </div>

      <label for="ln-title">条目标题</label>
      <div class="row">
        <input id="ln-title" v-model="title" autocomplete="off" @keydown.enter="loadEntry" />
        <button class="mini" type="button" @click="loadEntry">加载字段</button>
      </div>

      <template v-if="detail">
        <label>注入字段（作为环境变量注入子进程，键=字段名）</label>
        <div class="fields">
          <label class="chk" v-for="n in fieldNames()" :key="n">
            <input type="checkbox" v-model="selected[n]" />
            {{ n }}
          </label>
        </div>

        <label for="ln-program">程序路径</label>
        <div class="row">
          <input id="ln-program" v-model="program" autocomplete="off" />
          <button class="mini" type="button" @click="browse">浏览…</button>
        </div>

        <label for="ln-args">命令行参数（空格分隔）</label>
        <input id="ln-args" v-model="argsText" autocomplete="off" placeholder="例如：--port 8080" />

        <div class="actions">
          <button class="primary" :disabled="busy" @click="doLaunch">
            {{ busy ? "启动中…" : "启动" }}
          </button>
        </div>
      </template>

      <div class="error" v-if="error">{{ error }}</div>
      <div class="done" v-if="done">{{ done }}</div>
      <p class="hint">{{ envHint }}</p>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.3); display: flex; align-items: center; justify-content: center; z-index: 40; }
.panel { background: var(--panel); border-radius: 8px; padding: 20px 24px; width: 520px; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
h2 { font-size: 16px; margin: 0; }
.close { border: none; background: transparent; cursor: pointer; font-size: 14px; }
label { display: block; font-size: 12px; color: var(--muted); margin: 8px 0 4px; }
input { width: 100%; box-sizing: border-box; padding: 6px 8px; border: 1px solid var(--border); border-radius: 6px; font-size: 13px; background: var(--panel); color: var(--text); }
.row { display: flex; gap: 6px; }
.row input { flex: 1; }
.mini { border: 1px solid var(--border); background: var(--panel); border-radius: 6px; padding: 4px 10px; font-size: 12px; cursor: pointer; color: var(--text); white-space: nowrap; }
.fields { display: flex; flex-wrap: wrap; gap: 8px 14px; }
.chk { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--text); margin: 0; cursor: pointer; }
.chk input { width: auto; }
.actions { display: flex; gap: 8px; margin-top: 14px; }
.primary { background: var(--teal); color: #fff; border: none; border-radius: 6px; padding: 7px 16px; font-size: 13px; cursor: pointer; }
.primary:disabled { opacity: .5; cursor: default; }
.error { color: var(--danger); font-size: 12px; margin-top: 8px; white-space: pre-wrap; }
.done { color: var(--teal-dark); font-size: 12px; margin-top: 8px; line-height: 1.6; }
.hint { font-size: 11px; color: var(--muted); margin-top: 10px; line-height: 1.6; word-break: break-all; }
</style>
