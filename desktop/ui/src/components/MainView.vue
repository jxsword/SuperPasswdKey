<script setup lang="ts">
// 解锁态主界面：搜索框 + 条目列表 + 详情侧栏（MVP 只读）。
// 列表/详情加载失败（含锁定语义）→ 提示重新解锁。
import { computed, onMounted, ref } from "vue";
import * as api from "../api";
import type { EntrySummary } from "../api";
import SearchBox from "./SearchBox.vue";
import EntryList from "./EntryList.vue";
import DetailPanel from "./DetailPanel.vue";

const props = defineProps<{ touch: () => void }>();

const entries = ref<EntrySummary[]>([]);
const listError = ref("");
const loading = ref(false);
const query = ref("");
const selectedTitle = ref<string | null>(null);

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return entries.value;
  return entries.value.filter((e) => {
    const hay = [e.title, e.username, e.url]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return hay.includes(q);
  });
});

async function loadEntries() {
  loading.value = true;
  listError.value = "";
  try {
    entries.value = await api.entryList();
    props.touch();
  } catch (e) {
    listError.value = String(e);
  } finally {
    loading.value = false;
  }
}

function onSelect(title: string) {
  selectedTitle.value = title;
  props.touch();
}

onMounted(loadEntries);
</script>

<template>
  <div class="main-view">
    <aside class="left">
      <SearchBox v-model="query" />
      <div class="list-error" v-if="listError">
        {{ listError }}
        <button class="retry" @click="loadEntries">重试</button>
      </div>
      <EntryList
        :entries="filtered"
        :selected="selectedTitle"
        :loading="loading"
        @select="onSelect"
      />
    </aside>
    <section class="right">
      <DetailPanel v-if="selectedTitle" :title="selectedTitle" :touch="props.touch" />
      <div class="empty-hint" v-else>从左侧选择一个条目查看详情</div>
    </section>
  </div>
</template>

<style scoped>
.main-view {
  display: flex;
  height: 100%;
}

.left {
  width: 340px;
  min-width: 280px;
  border-right: 1px solid var(--border);
  background: var(--panel);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.right {
  flex: 1;
  min-width: 0;
  overflow: auto;
  padding: 16px 20px;
}

.list-error {
  padding: 8px 12px;
  font-size: 12px;
  color: var(--danger);
  background: #ffebee;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.retry {
  border: none;
  background: transparent;
  color: var(--teal-dark);
  cursor: pointer;
  font-size: 12px;
  text-decoration: underline;
}

.empty-hint {
  color: var(--muted);
  font-size: 13px;
  margin-top: 40px;
  text-align: center;
}
</style>
