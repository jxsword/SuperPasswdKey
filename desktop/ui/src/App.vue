<script setup lang="ts">
// 根组件：按会话状态切换视图（锁定=解锁面板，解锁=主界面），顶栏常驻。
// 视图切换不引入路由库（MVP 口径，design_docs/16 §2）。
import { ref } from "vue";
import { useSession } from "./composables/useSession";
import UnlockPanel from "./components/UnlockPanel.vue";
import MainView from "./components/MainView.vue";
import SettingsPanel from "./components/SettingsPanel.vue";

const session = useSession();
session.startPolling();

const showSettings = ref(false);

function fmtRemaining(ms: number): string {
  if (ms <= 0) return "0s";
  const s = Math.ceil(ms / 1000);
  return s >= 60 ? `${Math.floor(s / 60)}m${s % 60}s` : `${s}s`;
}
</script>

<template>
  <div class="app-root">
    <header class="topbar">
      <div class="brand">SuperPasswdKey</div>
      <div class="session-info" v-if="session.status.value">
        <template v-if="session.status.value.unlocked">
          已解锁 · 空闲 {{ fmtRemaining(session.status.value.idle_remaining_ms) }} ·
          绝对 {{ fmtRemaining(session.status.value.absolute_remaining_ms) }}
        </template>
        <template v-else>已锁定</template>
      </div>
      <div class="topbar-actions">
        <button class="ghost" @click="showSettings = !showSettings">设置</button>
        <button class="danger-ghost" v-if="session.status.value?.unlocked" @click="session.lock()">
          锁定
        </button>
      </div>
    </header>

    <main class="content">
      <UnlockPanel
        v-if="!session.status.value?.unlocked"
        :error="session.error.value"
        @unlocked="session.refresh()"
      />
      <MainView v-else :touch="session.touch" />
    </main>

    <SettingsPanel v-if="showSettings" @close="showSettings = false" />
  </div>
</template>

<style scoped>
.app-root {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.topbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  background: var(--panel);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.brand {
  font-weight: 600;
  font-size: 14px;
}

.session-info {
  font-size: 12px;
  color: var(--muted);
  flex: 1;
}

.topbar-actions {
  display: flex;
  gap: 8px;
}

.ghost {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 12px;
  font-size: 13px;
  cursor: pointer;
  color: var(--text);
}

.ghost:hover {
  border-color: var(--teal);
}

.danger-ghost {
  background: transparent;
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 4px 12px;
  font-size: 13px;
  cursor: pointer;
  color: var(--danger);
}

.content {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
</style>
