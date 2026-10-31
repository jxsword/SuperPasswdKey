// 会话状态 composable：5s 轮询 session_status（展示不构成安全边界，机密操作以后端惰性检查为准）。
// 用户交互（列表/详情/搜索）经 touch 刷新空闲计时。
import { onUnmounted, ref } from "vue";
import * as api from "../api";
import type { SessionStatus } from "../api";

export function useSession() {
  const status = ref<SessionStatus | null>(null);
  const error = ref<string>("");
  let timer: number | undefined;

  async function refresh() {
    try {
      status.value = await api.sessionStatus();
      error.value = "";
    } catch (e) {
      error.value = String(e);
    }
  }

  async function unlock(password: string, keyfile: string) {
    status.value = await api.unlock(password, keyfile || null);
    await refresh();
  }

  async function lock() {
    status.value = await api.lockNow();
  }

  async function touch() {
    try {
      await api.touch();
    } catch {
      // touch 失败不阻断交互（下次轮询会收敛状态）
    }
  }

  function startPolling() {
    refresh();
    timer = window.setInterval(refresh, 5000);
  }

  onUnmounted(() => {
    if (timer !== undefined) window.clearInterval(timer);
  });

  return { status, error, refresh, unlock, lock, touch, startPolling };
}
