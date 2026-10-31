<script setup lang="ts">
// 解锁态主界面：搜索框 + 条目列表 + 详情侧栏。
// S1b-1：+ 新的项目（创建表单）；编辑/删除经 DetailPanel 上抛事件后刷新列表与选中态。
import { computed, onMounted, ref } from "vue";
import * as api from "../api";
import type { EntrySummary } from "../api";
import SearchBox from "./SearchBox.vue";
import EntryList from "./EntryList.vue";
import DetailPanel from "./DetailPanel.vue";
import EntryFormModal from "./EntryFormModal.vue";

const props = defineProps<{ touch: () => void }>();

const entries = ref<EntrySummary[]>([]);
const listError = ref("");
const loading = ref(false);
const query = ref("");
const selectedTitle = ref<string | null>(null);
const showCreate = ref(false);

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

/** 创建成功 → 刷新列表并选中新条目 */
function onCreated(_oldTitle: string | null, newTitle: string) {
  showCreate.value = false;
  loadEntries();
  selectedTitle.value = newTitle;
}

/** 编辑保存（可能改名）→ 刷新列表并把选中态跟到新标题 */
function onSaved(oldTitle: string | null, newTitle: string) {
  loadEntries();
  selectedTitle.value = newTitle;
  void oldTitle;
}

/** 删除 → 刷新列表并清空详情 */
function onDeleted() {
  loadEntries();
  selectedTitle.value = null;
}

onMounted(loadEntries);
</script>

<template>
  <div class="main-view">
    <aside class="left">
      <div class="list-header">
        <button class="new-entry" @click="showCreate = true">＋ 新的项目</button>
      </div>
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
      <DetailPanel
        v-if="selectedTitle"
        :key="selectedTitle"
        :title="selectedTitle"
        :touch="props.touch"
        @saved="onSaved"
        @deleted="onDeleted"
      />
      <div class="empty-hint" v-else>从左侧选择一个条目查看详情</div>
    </section>
    <EntryFormModal
      v-if="showCreate"
      mode="create"
      @saved="onCreated"
      @close="showCreate = false"
    />
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

.list-header {
  padding: 10px 12px 0;
}

.new-entry {
  width: 100%;
  padding: 7px;
  font-size: 13px;
  border: 1px dashed var(--teal);
  border-radius: 6px;
  background: transparent;
  color: var(--teal-dark);
  cursor: pointer;
}

.new-entry:hover {
  background: #e0f2f1;
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
