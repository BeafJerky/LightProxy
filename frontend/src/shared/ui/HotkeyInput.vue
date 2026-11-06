<script setup lang="ts">
import { computed, ref } from 'vue'

const props = withDefaults(defineProps<{ modelValue: string; placeholder?: string }>(), {
  placeholder: '未设置'
})

const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const recording = ref(false)
const draft = ref('')
const invalid = ref(false)

const display = computed(() => {
  if (!recording.value) return props.modelValue || props.placeholder
  return draft.value || '请按下组合键…'
})

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

function start() {
  recording.value = true
  draft.value = ''
  invalid.value = false
}

function stop() {
  recording.value = false
  draft.value = ''
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
</script>

<template>
  <button
    type="button"
    class="hotkey-field"
    :class="{ 'is-recording': recording, 'is-invalid': invalid }"
    @click="start"
    @keydown="onKeydown"
    @blur="stop"
  >
    <span class="hotkey-value">{{ display }}</span>
    <span class="hotkey-hint">{{ recording ? (invalid ? '需要包含 Ctrl / Alt / Shift / Win' : 'Esc 取消') : '点击录制' }}</span>
  </button>
</template>

<style scoped>
.hotkey-field {
  min-width: 164px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  height: 32px;
  padding: 0 10px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  color: var(--text);
  text-align: left;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.hotkey-field:hover {
  border-color: var(--accent);
}

.hotkey-field.is-recording {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.hotkey-field.is-invalid {
  border-color: var(--danger);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--danger) 16%, transparent);
}

.hotkey-value {
  font-family: var(--mono);
  font-size: 11.5px;
  font-weight: 600;
  white-space: nowrap;
}

.hotkey-hint {
  font-size: 10px;
  color: var(--muted);
  white-space: nowrap;
}
</style>
