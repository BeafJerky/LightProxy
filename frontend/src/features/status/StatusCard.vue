<script setup lang="ts">
import { computed } from 'vue'
import { profileServer, useAppStore } from '@/stores/app'
import { relativeTime } from '@/shared/time'

const store = useAppStore()

const profile = computed(() => store.activeProfile)
const address = computed(() => (profile.value ? profileServer(profile.value) : '尚未配置代理'))
const health = computed(() => store.state?.lastHealth ?? null)
const checkText = computed(() => (health.value ? `${relativeTime(health.value.checkedAtMs)}检查` : '尚未检查'))

const meta = computed(() => {
  const current = store.state
  if (current?.state === 'degraded' && current.message) return current.message
  if (!health.value) return '启用后可检查连通性'
  if (!health.value.ok) return health.value.error ?? '代理不可达'
  return `延迟 ${health.value.latencyMs}ms · TCP 预检通过`
})
</script>

<template>
  <div class="status-card">
    <div class="status-top">
      <span class="status-pill"><i class="dot" />{{ store.statusText }}</span>
      <span class="status-check">{{ checkText }}</span>
    </div>
    <div class="status-address">{{ address }}</div>
    <div class="status-meta">
      <span v-if="profile" class="proto">{{ profile.protocol }}</span>
      <span>{{ meta }}</span>
    </div>
  </div>
</template>

<style scoped>
.status-card {
  margin: 2px 14px 0;
  padding: 13px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: linear-gradient(180deg, var(--surface-2), var(--surface));
}

.status-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.status-check {
  font-size: 10.5px;
  color: var(--muted);
  white-space: nowrap;
}

.status-address {
  margin-top: 9px;
  font-family: var(--mono);
  font-size: 16px;
  font-weight: 600;
  letter-spacing: -0.01em;
  word-break: break-all;
}

.status-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 3px;
  font-size: 11.5px;
  color: var(--muted);
  min-width: 0;
}

.status-meta span:last-child {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proto {
  padding: 1px 5px;
  border-radius: 5px;
  background: var(--surface-3);
  color: var(--text-2);
  font-family: var(--mono);
  font-size: 9.5px;
  text-transform: uppercase;
}
</style>
