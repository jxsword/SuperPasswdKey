<script setup lang="ts">
// 搜索框：即时过滤（防抖由父组件 computed 承担；输入即过滤，条目量为个人库量级）。
const model = defineModel<string>({ default: "" });
</script>

<template>
  <div class="search-box">
    <input
      type="search"
      v-model="model"
      placeholder="搜索标题 / 用户名 / URL"
      autocomplete="off"
    />
  </div>
</template>

<style scoped>
.search-box {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
}

input {
  width: 100%;
  padding: 7px 10px;
  font-size: 13px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
}

input:focus {
  outline: 2px solid rgba(0, 150, 136, 0.2);
  border-color: var(--teal);
}
</style>
