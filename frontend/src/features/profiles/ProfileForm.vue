<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { NButton, NInput, NInputNumber, NModal, NSelect, useMessage } from 'naive-ui'
import { useAppStore } from '@/stores/app'
import type { Profile } from '@/shared/api/types'
import { hasErrors, validateProfileDraft, type ProfileErrors } from './validation'

const props = defineProps<{ profile?: Profile | null }>()
const show = defineModel<boolean>('show', { required: true })

const store = useAppStore()
const message = useMessage()
const errors = ref<ProfileErrors>({})
const saving = ref(false)

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
  { label: 'SOCKS5（仅保存）', value: 'socks5' }
]

function reset() {
  const source = props.profile
  draft.id = source?.id ?? null
  draft.name = source?.name ?? ''
  draft.protocol = source?.protocol ?? 'http'
  draft.host = source?.host ?? ''
  draft.port = source?.port ?? 7890
  errors.value = {}
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
          <n-select v-model:value="draft.protocol" :options="protocolOptions" :consistent-menu-width="false" />
          <em v-if="errors.protocol">{{ errors.protocol }}</em>
        </label>

        <label class="field field--host">
          <span>主机</span>
          <n-input
            v-model:value="draft.host"
            placeholder="192.168.5.4 或 proxy.local"
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

      <p v-if="draft.protocol === 'socks5'" class="socks-hint">
        SOCKS5 配置当前只保存，不写入系统代理；后续版本会支持。
      </p>

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

.socks-hint {
  margin: 12px 0 0;
  padding: 7px 9px;
  border-radius: var(--radius-sm);
  background: var(--warn-soft);
  color: var(--warn);
  font-size: 11px;
  line-height: 1.5;
}

.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 18px;
}
</style>
