<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'

const props = withDefaults(
  defineProps<{ modelValue: string; placeholder?: string; failed?: boolean }>(),
  { placeholder: '未设置', failed: false }
)

const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const recording = ref(false)
const draft = ref('')
const invalid = ref(false)
const editBtn = ref<HTMLButtonElement | null>(null)

const keys = computed(() => (props.modelValue ? props.modelValue.split('+') : []))
const display = computed(() => draft.value || '按下组合键…')

const MODIFIERS = new Set(['Control', 'Alt', 'Shift', 'Meta'])

const KEY_NAMES: Record<string, string> = {
  ' ': 'Space',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Escape: 'Esc',
  Enter: 'Enter',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown'
}

async function start() {
  recording.value = true
  draft.value = ''
  invalid.value = false
  await nextTick()
  editBtn.value?.focus()
}

function stop() {
  recording.value = false
  draft.value = ''
  invalid.value = false
}

function toggle() {
  if (recording.value) stop()
  else void start()
}

function onFocusOut(event: FocusEvent) {
  const next = event.relatedTarget as Node | null
  const root = event.currentTarget as HTMLElement
  if (recording.value && next && root.contains(next)) return
  if (recording.value) stop()
}

function onKeydown(event: KeyboardEvent) {
  if (!recording.value) return
  event.preventDefault()
  event.stopPropagation()

  if (event.key === 'Escape') {
    stop()
    return
  }

  const modifiers: string[] = []
  if (event.ctrlKey) modifiers.push('Ctrl')
  if (event.altKey) modifiers.push('Alt')
  if (event.shiftKey) modifiers.push('Shift')
  if (event.metaKey) modifiers.push('Win')

  if (MODIFIERS.has(event.key)) {
    draft.value = modifiers.join('+')
    invalid.value = false
    return
  }

  let key = KEY_NAMES[event.key]
  if (!key && event.key.length === 1) key = event.key.toUpperCase()
  if (!key && /^F\d{1,2}$/.test(event.key)) key = event.key

  if (!key) {
    invalid.value = true
    draft.value = modifiers.join('+')
    return
  }

  const combo = [...modifiers, key].join('+')
  if (modifiers.length === 0) {
    invalid.value = true
    draft.value = combo
    return
  }

  invalid.value = false
  emit('update:modelValue', combo)
  stop()
}

defineExpose({ start })
</script>

<template>
  <div
    class="key-capture"
    :class="{ 'is-recording': recording, 'has-failure': failed }"
    @keydown="onKeydown"
    @focusout="onFocusOut"
  >
    <template v-if="recording">
      <span class="capture-hint" :class="{ 'is-invalid': invalid }">
        {{ invalid ? '需要包含 Ctrl / Alt / Shift / Win' : display }}
      </span>
    </template>
    <template v-else>
      <template v-if="keys.length">
        <template v-for="(part, index) in keys" :key="`${part}-${index}`">
          <span v-if="index > 0" class="key-plus">+</span>
          <kbd>{{ part }}</kbd>
        </template>
      </template>
      <span v-else class="key-empty">{{ placeholder }}</span>
      <span v-if="failed" class="key-state">未生效</span>
    </template>
    <button ref="editBtn" class="mini-btn" type="button" @click.stop="toggle">
      {{ recording ? '取消' : '修改' }}
    </button>
  </div>
</template>

<style scoped>
.key-capture {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  flex: none;
  min-width: 168px;
}

kbd {
  font-family: var(--font);
  font-size: 10.5px;
  font-weight: 600;
  color: var(--text-2);
  background: var(--surface-2);
  border: 1px solid var(--border-strong);
  border-bottom-width: 2px;
  border-radius: 6px;
  padding: 3px 7px;
}

.key-plus {
  font-size: 9px;
  color: var(--muted);
}

.key-empty {
  font-size: 11px;
  color: var(--muted);
}

.key-state {
  font-size: 9.5px;
  color: var(--danger);
}

.capture-hint {
  font-size: 10.5px;
  color: var(--muted);
  white-space: nowrap;
}

.capture-hint.is-invalid {
  color: var(--danger);
}

.key-capture.is-recording {
  border-radius: 7px;
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.key-capture.has-failure kbd {
  border-color: var(--danger);
}

.mini-btn {
  height: 26px;
  margin-left: 6px;
  padding: 0 8px;
  font-size: 11px;
}
</style>
