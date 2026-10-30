<script setup lang="ts">
// 设置面板：空闲/绝对分钟 + 自启（复用 v0.3.0 静态页语义；超时配置重启生效，自启即时生效）。
import { onMounted, ref } from "vue";
import * as api from "../api";

const emit = defineEmits<{ close: [] }>();

const idleMin = ref(5);
const absMin = ref(30);
const autostart = ref(false);
const msg = ref("");
const busy = ref(false);

const msToMin = (ms: number) => Math.max(1, Math.round(ms / 60000));
const minToMs = (min: number) => Math.max(60000, Math.round(min * 60000));

onMounted(async () => {
  try {
    const s = await api.getSettings();
    idleMin.value = msToMin(s.idle_ms);
    absMin.value = msToMin(s.absolute_ms);
    autostart.value = s.autostart;
  } catch (e) {
    msg.value = `设置读取失败：${e}`;
  }
});

async function save() {
  busy.value = true;
  msg.value = "保存中…";
  try {
    await api.saveSettings({
      idle_ms: minToMs(Number(idleMin.value)),
      absolute_ms: minToMs(Number(absMin.value)),
      autostart: autostart.value,
    });
    msg.value = "已保存：自启即时生效；超时配置重启后生效。";
  } catch (e) {
    msg.value = `保存失败：${e}`;
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="settings-overlay" @click.self="emit('close')">
    <div class="settings-panel">
      <div class="settings-header">
        <h2>设置</h2>
        <button class="close" @click="emit('close')">✕</button>
      </div>
      <label for="idleMin">空闲锁定（分钟）</label>
      <input id="idleMin" type="number" min="1" step="1" v-model.number="idleMin" />
      <label for="absMin">绝对过期（分钟）</label>
      <input id="absMin" type="number" min="1" step="1" v-model.number="absMin" />
      <label class="check">
        <input type="checkbox" v-model="autostart" />
        开机自启（写入 HKCU Run，免管理员）
      </label>
      <button :disabled="busy" @click="save">保存设置</button>
      <div class="msg" role="status">{{ msg }}</div>
    </div>
  </div>
</template>

<style scoped>
.settings-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}

.settings-panel {
  background: var(--panel);
  border-radius: 8px;
  padding: 24px;
  width: 360px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
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

label.check {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text);
  margin-top: 8px;
}

input[type="number"] {
  padding: 7px 10px;
  font-size: 14px;
  border: 1px solid var(--border);
  border-radius: 6px;
}

button {
  margin-top: 12px;
  padding: 9px;
  font-size: 14px;
  border: none;
  border-radius: 6px;
  background: var(--teal);
  color: #fff;
  cursor: pointer;
}

button:disabled {
  opacity: 0.6;
}

.msg {
  font-size: 12px;
  color: var(--muted);
  min-height: 16px;
}
</style>
