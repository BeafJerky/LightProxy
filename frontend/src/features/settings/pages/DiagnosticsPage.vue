<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { profileServer, useAppStore } from '@/stores/app'
import { relativeTime } from '@/shared/time'
import Icon from '@/shared/ui/Icon.vue'
import SectionCard from '@/shared/ui/SectionCard.vue'
import type { Diagnostic } from '@/shared/api/types'

const store = useAppStore()
const message = useMessage()
const exporting = ref(false)

const autoFixable = computed(() => store.diagnostics.filter((item) => item.autoFixable))
const summary = computed(() => {
  const state = store.state
  const snapshot = state?.snapshot
  return [
    { label: '当前状态', value: store.statusText, mono: false },
    { label: '期望状态', value: state?.desired === 'on' ? '开启' : '关闭', mono: false },
    {
      label: '使用中配置',
      value: store.activeProfile ? profileServer(store.activeProfile) : '未选择',
      mono: true
    },
    {
      label: '系统代理',
      value: store.isActive
        ? store.activeProfile
          ? profileServer(store.activeProfile)
          : '已接管'
        : snapshot?.proxyEnable
          ? `${snapshot.proxyServer}（原始）`
          : '未接管',
      mono: Boolean(store.isActive || snapshot?.proxyEnable)
    },
    { label: 'PAC 脚本', value: snapshot?.autoConfigUrl ?? '无', mono: true },
    {
      label: '最近检查',
      value: state?.lastHealth ? relativeTime(state.lastHealth.checkedAtMs) : '尚未检查',
      mono: false
    }
  ]
})

async function refreshAll() {
  await Promise.all([store.loadDiagnostics(), store.loadRepairs(), store.refresh()])
  message.success('诊断信息已刷新')
}

async function repair(ids: string[]) {
  const results = await store.repair(ids)
  if (!results) return
  const done = results.filter((item) => item.ok).length
  message[done === results.length ? 'success' : 'warning'](`完成 ${done}/${results.length} 项修复`)
}

async function exportAll() {
  exporting.value = true
  const result = await store.exportDiagnostics()
  exporting.value = false
  if (result) message.success(`已导出到 ${result.path}`)
}

async function openLogs() {
  const path = await store.openLogDir()
  if (path) message.success(`日志目录：${path}`)
}

function severityLabel(item: Diagnostic): string {
  if (item.severity === 'error') return '异常'
  if (item.severity === 'warn') return '注意'
  return '信息'
}

onMounted(() => {
  void store.loadDiagnostics()
  void store.loadRepairs()
})
</script>

<template>
  <div class="settings-page">
    <div class="page-head">
      <div>
        <h1>诊断与修复</h1>
        <p>检查系统代理是否被外部修改，并对可自动修复的项一键处理。</p>
      </div>
      <div class="page-head-actions">
        <button class="mini-btn" type="button" :disabled="Boolean(store.busy)" @click="refreshAll">
          <Icon name="refresh" :size="14" />
          刷新
        </button>
        <button
          class="mini-btn primary"
          type="button"
          :disabled="autoFixable.length === 0 || Boolean(store.busy)"
          @click="repair(autoFixable.map((item) => item.id))"
        >
          <Icon name="wrench" :size="14" />
          修复可自动项
        </button>
      </div>
    </div>

    <SectionCard title="状态概览">
      <div v-for="row in summary" :key="row.label" class="summary-row">
        <span>{{ row.label }}</span>
        <strong :class="{ mono: row.mono }">{{ row.value }}</strong>
      </div>
    </SectionCard>

    <SectionCard title="发现的问题" :hint="store.diagnostics.length === 0 ? '全部正常' : `${store.diagnostics.length} 项`">
      <div v-if="store.diagnostics.length === 0" class="all-clear">
        <span class="clear-icon"><Icon name="shield" :size="20" /></span>
        <div>
          <strong>没有发现问题</strong>
          <p>系统代理与持久化状态一致，快捷键注册正常。</p>
        </div>
      </div>
      <div v-for="item in store.diagnostics" :key="item.id + item.title" class="diag-row" :class="item.severity">
        <span class="diag-icon"><Icon name="alert" :size="15" /></span>
        <div class="diag-copy">
          <div class="diag-title">
            <strong>{{ item.title }}</strong>
            <span class="diag-tag">{{ severityLabel(item) }}</span>
          </div>
          <p>{{ item.detail }}</p>
        </div>
        <button
          v-if="item.autoFixable"
          class="mini-btn"
          type="button"
          :disabled="Boolean(store.busy)"
          @click="repair([item.id])"
        >
          修复
        </button>
        <span v-else class="manual">手动处理</span>
      </div>
    </SectionCard>

    <SectionCard title="最近修复" :hint="`${store.repairs.length} 条`">
      <div v-if="store.repairs.length === 0" class="empty-state">还没有修复记录。</div>
      <div v-for="record in store.repairs" :key="record.atMs + record.id" class="repair-row">
        <span class="repair-dot" :class="{ ok: record.ok }" />
        <div class="repair-copy">
          <strong>{{ record.title }}</strong>
          <p>{{ record.message }}</p>
        </div>
        <span class="repair-time">{{ relativeTime(record.atMs) }}</span>
      </div>
    </SectionCard>

    <SectionCard title="工具">
      <div class="tool-row">
        <div class="setting-copy">
          <strong>导出诊断</strong>
          <p>把当前状态、诊断项与修复记录写入 JSON，便于排查。</p>
        </div>
        <button class="mini-btn" type="button" :disabled="exporting" @click="exportAll">
          <Icon name="download" :size="14" />
          导出
        </button>
      </div>
      <div class="tool-row">
        <div class="setting-copy">
          <strong>打开日志目录</strong>
          <p>日志只保留运行信息，不包含代理凭据。</p>
        </div>
        <button class="mini-btn" type="button" @click="openLogs">
          <Icon name="folder" :size="14" />
          打开
        </button>
      </div>
    </SectionCard>
  </div>
</template>

<style scoped>
.summary-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
  font-size: 12px;
}

.summary-row:last-child {
  border-bottom: 0;
}

.summary-row > span {
  color: var(--muted);
}

.summary-row strong {
  font-size: 12px;
  font-weight: 600;
  text-align: right;
  word-break: break-all;
}

.summary-row strong.mono {
  font-family: var(--mono);
}

.all-clear {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 12px 0;
}

.clear-icon {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 9px;
  background: var(--success-soft);
  color: var(--success);
}

.all-clear strong {
  font-size: 12.5px;
}

.all-clear p {
  margin: 2px 0 0;
  font-size: 11.5px;
  color: var(--muted);
}

.diag-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}

.diag-row:last-child {
  border-bottom: 0;
}

.diag-icon {
  width: 28px;
  height: 28px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: var(--warn-soft);
  color: var(--warn);
}

.diag-row.error .diag-icon {
  background: var(--danger-soft);
  color: var(--danger);
}

.diag-row.info .diag-icon {
  background: var(--accent-soft);
  color: var(--accent-strong);
}

.diag-copy {
  flex: 1;
  min-width: 0;
}

.diag-title {
  display: flex;
  align-items: center;
  gap: 6px;
}

.diag-title strong {
  font-size: 12.5px;
}

.diag-tag {
  padding: 1px 5px;
  border-radius: 999px;
  background: var(--warn-soft);
  color: var(--warn);
  font-size: 9.5px;
  font-weight: 600;
}

.diag-row.error .diag-tag {
  background: var(--danger-soft);
  color: var(--danger);
}

.diag-row.info .diag-tag {
  background: var(--accent-soft);
  color: var(--accent-strong);
}

.diag-copy p {
  margin: 3px 0 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--muted);
}

.manual {
  flex: none;
  padding-top: 4px;
  font-size: 10.5px;
  color: var(--muted);
  white-space: nowrap;
}

.repair-row {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  padding: 9px 0;
  border-bottom: 1px solid var(--border);
}

.repair-row:last-child {
  border-bottom: 0;
}

.repair-dot {
  width: 7px;
  height: 7px;
  margin-top: 5px;
  flex: none;
  border-radius: 50%;
  background: var(--danger);
}

.repair-dot.ok {
  background: var(--success);
}

.repair-copy {
  flex: 1;
  min-width: 0;
}

.repair-copy strong {
  font-size: 12px;
}

.repair-copy p {
  margin: 2px 0 0;
  font-size: 11px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.repair-time {
  flex: none;
  font-size: 10px;
  color: var(--muted);
}

.tool-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 9px 0;
  border-bottom: 1px solid var(--border);
}

.tool-row:last-child {
  border-bottom: 0;
}
</style>
