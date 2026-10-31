<script setup lang="ts">
// 放大显示（对齐 1Password Large Type）：全屏遮罩 + 超大等宽字体；点击任意处/Esc 关闭，
// 不自动消失（值已在屏上，自动消失无安全增益）。纯前端组件，零 Rust 改动。
import { onMounted, onUnmounted } from "vue";

const props = defineProps<{ value: string }>();
const emit = defineEmits<{ close: [] }>();

function onKeydown(ev: KeyboardEvent) {
  if (ev.key === "Escape") emit("close");
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onUnmounted(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div class="large-type-overlay" @click="emit('close')">
    <div class="large-type-text" :class="{ empty: !props.value }">
      {{ props.value || "（无值）" }}
    </div>
    <div class="hint">点击任意处或按 Esc 关闭</div>
  </div>
</template>

<style scoped>
.large-type-overlay {
  position: fixed;
  inset: 0;
  background: rgba(255, 255, 255, 0.97);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 100;
  cursor: pointer;
  padding: 32px;
}

.large-type-text {
  font-family: "Consolas", "Cascadia Mono", monospace;
  font-size: clamp(32px, 9vw, 120px);
  letter-spacing: 4px;
  color: var(--text);
  word-break: break-all;
  text-align: center;
  max-width: 100%;
  user-select: text;
}

.large-type-text.empty {
  font-size: 24px;
  color: var(--muted);
}

.hint {
  margin-top: 32px;
  font-size: 12px;
  color: var(--muted);
}
</style>
