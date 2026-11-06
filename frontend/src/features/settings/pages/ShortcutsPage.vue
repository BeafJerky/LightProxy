<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import { useAppStore } from '@/stores/app'
import { DEFAULT_SETTINGS } from '@/shared/api/defaults'
import type { Settings } from '@/shared/api/types'
import SectionCard from '@/shared/ui/SectionCard.vue'
import HotkeyInput from '@/shared/ui/HotkeyInput.vue'
import Icon from '@/shared/ui/Icon.vue'

type HotkeyKey = keyof Settings['hotkeys']

interface HotkeyRow {
  key: HotkeyKey
  title: string
  desc: string
}

const rows: HotkeyRow[] = [
  { key: 'toggle', title: '开关代理', desc: '快路径，目标 p95 < 50ms。' },
  { key: 'panel', title: '打开 / 聚焦面板', desc: '冷启动约 300–500ms，热启动 < 150ms。' },
  { key: 'nextProfile', title: '切换到下一套配置', desc: '已启用时直接切换，不经历关闭再开启。' }
]

const store = useAppStore()
const message = useMessage()

const fieldRefs: Partial<Record<HotkeyKey, InstanceType<typeof HotkeyInput>>> = {}

function bindField(key: HotkeyKey, el: unknown) {
  if (el) fieldRefs[key] = el as InstanceType<typeof HotkeyInput>
  else delete fieldRefs[key]
}

const failure = computed(() => store.diagnostics.find((item) => item.id === 'R07') ?? null)
const failedSpecs = computed(() => {
  if (!failure.value) return new Set<string>()
  return new Set(
    failure.value.detail
      .split('；')
      .map((item) => item.split('：')[0]?.trim())
      .filter(Boolean)
  )
})
const firstFailed = computed(
  () => rows.find((row) => failedSpecs.value.has(store.settings.hotkeys[row.key])) ?? null
)

onMounted(() => {
  void store.loadDiagnostics()
})

async function setHotkey(key: HotkeyKey, value: string) {
  if (!value || value === store.settings.hotkeys[key]) return
  const ok = await store.updateHotkeys({ [key]: value })
  if (!ok) return
  await store.loadDiagnostics()
  if (failedSpecs.value.has(value)) message.warning('快捷键已保存，但注册失败，请换一个组合')
  else message.success('快捷键已更新')
}

async function resetAll() {
  const ok = await store.updateHotkeys({ ...DEFAULT_SETTINGS.hotkeys })
  if (!ok) return
  await store.loadDiagnostics()
  message.success('已恢复默认快捷键')
}

function retryFailed() {
  if (firstFailed.value) fieldRefs[firstFailed.value.key]?.start()
}
</script>

<template>
  <div class="settings-page">
    <div class="page-head">
      <div>
        <h1>快捷键</h1>
        <p>全局生效，窗口未打开时也可用。</p>
      </div>
      <div class="page-head-actions">
        <button class="mini-btn" type="button" @click="resetAll">恢复默认</button>
      </div>
    </div>

    <SectionCard>
      <div v-for="row in rows" :key="row.key" class="hotkey-row">
        <div class="setting-copy">
          <strong>{{ row.title }}</strong>
          <p>{{ row.desc }}</p>
        </div>
        <HotkeyInput
          :ref="(el) => bindField(row.key, el)"
          :model-value="store.settings.hotkeys[row.key]"
          :failed="failedSpecs.has(store.settings.hotkeys[row.key])"
          @update:model-value="setHotkey(row.key, $event)"
        />
      </div>
    </SectionCard>

    <div v-if="failure" class="notice warn" role="alert">
      <Icon name="alert" :size="16" />
      <div>
        <strong>快捷键未生效</strong>
        <p>{{ failure.detail }}。请换一个组合重新录入，不会静默失效。</p>
      </div>
      <button v-if="firstFailed" class="mini-btn" type="button" @click="retryFailed">
        重新录入
      </button>
    </div>
  </div>
</template>

<style scoped>
.hotkey-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 13px 0;
  border-bottom: 1px solid var(--border);
}

.hotkey-row:last-child {
  border-bottom: 0;
}

.notice {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin-top: 12px;
  padding: 12px 13px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 11.5px;
}

.notice :deep(svg) {
  width: 16px;
  height: 16px;
  flex: none;
  margin-top: 1px;
}

.notice strong {
  display: block;
  font-size: 12px;
}

.notice p {
  margin: 2px 0 0;
  color: var(--muted);
  font-size: 11px;
  line-height: 1.5;
}

.notice.warn {
  background: var(--warn-soft);
  border-color: color-mix(in srgb, var(--warn) 26%, var(--border));
  color: var(--warn);
}

.notice.warn p {
  color: var(--text-2);
}

.notice .mini-btn {
  margin-left: auto;
  flex: none;
}
</style>
