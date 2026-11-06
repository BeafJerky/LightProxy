<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: boolean
    label: string
    description?: string
    disabled?: boolean
  }>(),
  { description: '', disabled: false }
)

const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
  <label class="setting-row" :class="{ 'is-disabled': disabled }">
    <span class="setting-copy">
      <strong>{{ label }}</strong>
      <p v-if="description">{{ description }}</p>
    </span>
    <span class="switch">
      <input
        type="checkbox"
        :checked="modelValue"
        :disabled="disabled"
        @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
      />
      <i />
    </span>
  </label>
</template>

<style scoped>
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 9px 0;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
}

.setting-row:last-child {
  border-bottom: 0;
}

.setting-row.is-disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.setting-copy {
  min-width: 0;
}

.setting-copy strong {
  display: block;
  font-size: 12.5px;
}

.setting-copy p {
  margin: 3px 0 0;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--muted);
}

.switch {
  position: relative;
  display: inline-flex;
  width: 38px;
  height: 22px;
  flex: none;
}

.switch input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.switch i {
  position: relative;
  width: 100%;
  height: 100%;
  border-radius: 999px;
  background: var(--control-border);
  transition: background 0.18s ease;
}

.switch i::after {
  content: '';
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--switch-knob);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--text) 18%, transparent), 0 1px 2px rgba(0, 0, 0, 0.22);
  transition: transform 0.2s cubic-bezier(0.3, 0.9, 0.4, 1);
}

.switch input:checked + i {
  background: var(--accent);
}

.switch input:checked + i::after {
  transform: translateX(16px);
}

.switch input:focus-visible + i {
  outline: 2px solid var(--focus-ring);
  outline-offset: 2px;
}

.switch input:disabled + i {
  opacity: 0.45;
}
</style>
