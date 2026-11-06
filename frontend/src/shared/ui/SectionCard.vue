<script setup lang="ts">
withDefaults(defineProps<{ title?: string; hint?: string }>(), { title: '', hint: '' })
</script>

<template>
  <section class="setting-card">
    <header v-if="title || hint" class="card-title">
      <span>{{ title }}</span>
      <span v-if="hint" class="hint-inline">{{ hint }}</span>
      <div class="card-actions"><slot name="actions" /></div>
    </header>
    <slot />
  </section>
</template>

<style scoped>
.setting-card {
  margin-bottom: 11px;
  padding: 0 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface);
  background-image: var(--surface-glow);
  background-repeat: no-repeat;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 10px 0 8px;
  font-size: 12px;
  font-weight: 600;
}

.hint-inline {
  font-size: 10.5px;
  font-weight: 400;
  color: var(--muted);
}

.card-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 6px;
}
</style>
