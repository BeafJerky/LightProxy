<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { NButton, NInput, NInputNumber, NModal, NSelect, useMessage } from 'naive-ui'
import { api, toCoreError } from '@/shared/api'
import { useAppStore } from '@/stores/app'
import type { DiagnoseReport, Profile } from '@/shared/api/types'
import { diagnoseView, protocolLabel, stageLabel } from './diagnosis'
import { hasErrors, validateProfileDraft, type ProfileErrors } from './validation'

const props = defineProps<{ profile?: Profile | null }>()
const show = defineModel<boolean>('show', { required: true })

const store = useAppStore()
const message = useMessage()
const errors = ref<ProfileErrors>({})
const saving = ref(false)
const diagnosing = ref(false)
const diagnosisError = ref<string | null>(null)
const report = ref<DiagnoseReport | null>(null)

const draft = reactive({
  id: null as string | null,
  name: '',
  protocol: 'http',
  host: '',
  port: 7890 as number | null
})

const protocolOptions = [
  { label: 'HTTP', value: 'http' },
  { label: 'HTTPS', value: 'https' },
  { label: 'SOCKS5', value: 'socks5' }
]

const view = computed(() => (report.value ? diagnoseView(report.value, draft.protocol) : null))

function reset() {
  const source = props.profile
  draft.id = source?.id ?? null
  draft.name = source?.name ?? ''
  draft.protocol = source?.protocol ?? 'http'
  draft.host = source?.host ?? ''
  draft.port = source?.port ?? 7890
  errors.value = {}
  report.value = null
  diagnosisError.value = null
  diagnosing.value = false
}

watch(
  show,
  (open) => {
    if (open) reset()
  }
)

watch(
  () => props.profile,
  () => {
    if (show.value) reset()
  }
)

watch(
  () => [draft.host, draft.port],
  () => {
    report.value = null
    diagnosisError.value = null
  }
)

function invalidateDiagnosis() {
  report.value = null
  diagnosisError.value = null
}

async function runDiagnosis() {
  if (diagnosing.value) return
  diagnosing.value = true
  diagnosisError.value = null
  report.value = null
  try {
    report.value = await api.diagnoseProfile({
      id: draft.id,
      name: draft.name.trim(),
      protocol: draft.protocol,
      host: draft.host.trim(),
      port: draft.port ?? 0
    })
  } catch (caught) {
    diagnosisError.value = toCoreError(caught).message
  } finally {
    diagnosing.value = false
  }
}

async function applyRecommendation() {
  const target = view.value?.recommendedProtocol
  if (!target) return
  draft.protocol = target

  if (!draft.id) {
    message.success(`已填入建议协议：${protocolLabel(target)}，保存后生效`)
    return
  }

  errors.value = validateProfileDraft(draft, store.profiles, draft.id)
  if (hasErrors(errors.value)) return

  saving.value = true
  const ok = await store.saveProfile({
    id: draft.id,
    name: draft.name.trim(),
    protocol: target,
    host: draft.host.trim(),
    port: draft.port ?? 0
  })
  saving.value = false

  if (ok) {
    message.success(`已应用建议：${protocolLabel(target)}`)
    show.value = false
  }
}

async function submit() {
  errors.value = validateProfileDraft(draft, store.profiles, draft.id)
  if (hasErrors(errors.value)) return

  saving.value = true
  const ok = await store.saveProfile({
    id: draft.id,
    name: draft.name.trim(),
    protocol: draft.protocol,
    host: draft.host.trim(),
    port: draft.port ?? 0
  })
  saving.value = false

  if (ok) {
    message.success(draft.id ? '配置已更新' : '配置已添加')
    show.value = false
  }
}
</script>

<template>
  <n-modal v-model:show="show" :mask-closable="false">
    <div class="profile-modal">
      <header class="modal-head">
        <div>
          <h2>{{ draft.id ? '编辑代理配置' : '新增代理配置' }}</h2>
          <p>{{ draft.id ? '修改后会立即写入系统代理设置。' : '保存后可在面板中一键切换。' }}</p>
        </div>
      </header>

      <div class="form-grid">
        <label class="field">
          <span>名称</span>
          <n-input
            v-model:value="draft.name"
            placeholder="例如：公司代理"
            :maxlength="40"
            :status="errors.name ? 'error' : undefined"
          />
          <em v-if="errors.name">{{ errors.name }}</em>
        </label>

        <label class="field">
          <span>协议</span>
          <n-select
            v-model:value="draft.protocol"
            :options="protocolOptions"
            :consistent-menu-width="false"
            @update:value="invalidateDiagnosis"
          />
          <em v-if="errors.protocol">{{ errors.protocol }}</em>
        </label>

        <label class="field field--host">
          <span>主机</span>
          <n-input
            v-model:value="draft.host"
            placeholder="127.0.0.1 或 proxy.local"
            :status="errors.host ? 'error' : undefined"
          />
          <em v-if="errors.host">{{ errors.host }}</em>
        </label>

        <label class="field field--port">
          <span>端口</span>
          <n-input-number
            v-model:value="draft.port"
            :min="1"
            :max="65535"
            :show-button="false"
            :status="errors.port ? 'error' : undefined"
          />
          <em v-if="errors.port">{{ errors.port }}</em>
        </label>
      </div>

      <section class="diagnose" aria-live="polite">
        <div class="diagnose-head">
          <div class="diagnose-copy">
            <span class="diagnose-title">检测可用方式</span>
            <p>通过代理访问 github.com:443，验证 HTTP、SOCKS5 远程解析与系统 SOCKS 三条通道。</p>
          </div>
          <n-button size="small" secondary :loading="diagnosing" :disabled="diagnosing" @click="runDiagnosis">
            检测可用方式
          </n-button>
        </div>

        <p v-if="diagnosisError" class="diagnose-error">{{ diagnosisError }}</p>

        <div v-if="diagnosing" class="diagnose-running">正在检测三条通道，最长约 9 秒…</div>

        <div v-else-if="view" class="diagnose-result">
          <ul class="diagnose-rows">
            <li
              v-for="row in view.rows"
              :key="row.key"
              :class="['diagnose-row', row.result.ok ? 'is-ok' : 'is-fail']"
            >
              <span class="diagnose-dot" aria-hidden="true"></span>
              <div class="diagnose-row-main">
                <span class="diagnose-label">{{ row.label }}</span>
                <span class="diagnose-hint">{{ row.hint }}</span>
                <em v-if="!row.result.ok && row.result.error" class="diagnose-row-error">
                  {{ row.result.error }}
                </em>
              </div>
              <span class="diagnose-state">
                {{ row.result.ok ? `可用 · ${row.result.latencyMs} ms` : `${stageLabel(row.result.stage)}失败` }}
              </span>
            </li>
          </ul>

          <div class="diagnose-summary">
            <p>{{ view.summary }}</p>
            <p v-if="view.warning" class="diagnose-warning">{{ view.warning }}</p>
          </div>

          <div class="diagnose-actions">
            <n-button v-if="view.alreadyApplied" size="small" disabled>已是建议协议</n-button>
            <n-button
              v-else-if="view.canApply"
              size="small"
              type="primary"
              ghost
              :loading="saving"
              @click="applyRecommendation"
            >
              应用建议（{{ view.recommendedLabel }}）
            </n-button>
          </div>
        </div>
      </section>

      <footer class="modal-foot">
        <n-button quaternary @click="show = false">取消</n-button>
        <n-button type="primary" :loading="saving" @click="submit">保存</n-button>
      </footer>
    </div>
  </n-modal>
</template>

<style scoped>
.profile-modal {
  width: 460px;
  max-width: calc(100vw - 40px);
  max-height: calc(100vh - 40px);
  overflow-y: auto;
  padding: 20px 20px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}

.modal-head h2 {
  margin: 0;
  font-size: 17px;
  font-family: var(--font-display);
}

.modal-head p {
  margin: 4px 0 0;
  font-size: 11.5px;
  color: var(--muted);
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-top: 16px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}

.field > span {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--text-2);
}

.field--host {
  grid-column: span 1;
}

.field em {
  font-style: normal;
  font-size: 10.5px;
  color: var(--danger);
}

.diagnose {
  margin-top: 18px;
  padding-top: 16px;
  border-top: 1px dashed var(--border);
}

.diagnose-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.diagnose-copy {
  min-width: 0;
}

.diagnose-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
}

.diagnose-copy p {
  margin: 3px 0 0;
  font-size: 10.5px;
  line-height: 1.5;
  color: var(--muted);
}

.diagnose-error {
  margin: 10px 0 0;
  font-size: 10.5px;
  color: var(--danger);
}

.diagnose-running {
  margin-top: 12px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-2);
  font-size: 10.5px;
  color: var(--muted);
}

.diagnose-result {
  margin-top: 12px;
}

.diagnose-rows {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.diagnose-row {
  display: grid;
  grid-template-columns: 8px minmax(0, 1fr) auto;
  gap: 8px;
  align-items: start;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-2);
}

.diagnose-dot {
  width: 8px;
  height: 8px;
  margin-top: 4px;
  border-radius: 50%;
  background: var(--muted);
}

.diagnose-row.is-ok .diagnose-dot {
  background: var(--success);
}

.diagnose-row.is-fail .diagnose-dot {
  background: var(--danger);
}

.diagnose-row-main {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.diagnose-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text);
}

.diagnose-hint {
  font-size: 10px;
  color: var(--muted);
}

.diagnose-row-error {
  margin-top: 3px;
  font-style: normal;
  font-size: 10px;
  line-height: 1.45;
  color: var(--danger);
  word-break: break-word;
}

.diagnose-state {
  font-size: 10.5px;
  color: var(--muted);
  white-space: nowrap;
}

.diagnose-row.is-ok .diagnose-state {
  color: var(--success);
}

.diagnose-summary {
  margin-top: 10px;
  padding: 9px 11px;
  border-radius: var(--radius-md);
  background: var(--surface-2);
  font-size: 11px;
  line-height: 1.5;
  color: var(--text-2);
}

.diagnose-summary p {
  margin: 0;
}

.diagnose-warning {
  margin-top: 5px;
  color: var(--warn);
}

.diagnose-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 10px;
}

.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 18px;
}
</style>
