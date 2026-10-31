<script setup lang="ts">
// 条目列表：标题/用户名/URL/TOTP 标记（摘要不含密码/TOTP，SP-F-706 语义）。
import type { EntrySummary } from "../api";

const props = defineProps<{
  entries: EntrySummary[];
  selected: string | null;
  loading: boolean;
}>();

const emit = defineEmits<{ select: [title: string]; tag: [tag: string] }>();

function displayUrl(url: string | null): string {
  if (!url) return "";
  try {
    const u = new URL(url);
    return u.hostname;
  } catch {
    return url;
  }
}
</script>

<template>
  <div class="entry-list">
    <div class="loading" v-if="props.loading">加载中…</div>
    <div class="empty" v-else-if="props.entries.length === 0">
      无匹配条目
    </div>
    <button
      v-for="e in props.entries"
      :key="e.title"
      class="entry-item"
      :class="{ selected: e.title === props.selected }"
      @click="emit('select', e.title)"
    >
      <div class="entry-title">
        {{ e.title }}
        <span class="otp-badge" v-if="e.has_otp">TOTP</span>
      </div>
      <div class="entry-sub">{{ e.username || displayUrl(e.url) || "—" }}</div>
      <div class="entry-tags" v-if="e.tags.length">
        <button
          v-for="t in e.tags.slice(0, 3)"
          :key="t"
          class="tag-badge"
          :title="`按标签「${t}」过滤`"
          @click.stop="emit('tag', t)"
        >
          {{ t }}
        </button>
        <span class="tag-more" v-if="e.tags.length > 3">+{{ e.tags.length - 3 }}</span>
      </div>
    </button>
  </div>
</template>

<style scoped>
.entry-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.loading,
.empty {
  padding: 16px;
  font-size: 12px;
  color: var(--muted);
  text-align: center;
}

.entry-item {
  display: block;
  width: 100%;
  text-align: left;
  background: transparent;
  border: none;
  border-left: 3px solid transparent;
  padding: 8px 12px;
  cursor: pointer;
  font-family: inherit;
}

.entry-item:hover {
  background: #f5f5f5;
}

.entry-item.selected {
  background: #e0f2f1;
  border-left-color: var(--teal);
}

.entry-title {
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.entry-sub {
  font-size: 11px;
  color: var(--muted);
  margin-top: 2px;
}

.otp-badge {
  font-size: 9px;
  background: var(--teal);
  color: #fff;
  border-radius: 3px;
  padding: 1px 4px;
}

.entry-tags {
  margin-top: 4px;
  display: flex;
  gap: 4px;
  align-items: center;
  flex-wrap: wrap;
}

.tag-badge {
  font-size: 10px;
  background: #e0f2f1;
  color: var(--teal-dark);
  border: none;
  border-radius: 3px;
  padding: 1px 6px;
  cursor: pointer;
}

.tag-badge:hover {
  background: #b2dfdb;
}

.tag-more {
  font-size: 10px;
  color: var(--muted);
}
</style>
