<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { useMessage } from 'naive-ui'
import { onNavigate, onStateChanged } from '@/shared/api'
import { useAppStore } from '@/stores/app'
import QuickPanel from '@/features/panel/QuickPanel.vue'
import SettingsView from '@/features/settings/SettingsView.vue'

const store = useAppStore()
const message = useMessage()
const listeners: Array<() => void> = []

onMounted(async () => {
  await store.initialize()
  listeners.push(
    await onStateChanged((state) => {
      store.setStateFromEvent(state)
    })
  )
  listeners.push(
    await onNavigate((view) => {
      store.view = view
    })
  )
})

onUnmounted(() => {
  listeners.forEach((unlisten) => unlisten())
})

watch(
  () => store.error,
  (error) => {
    if (!error || store.canForce) return
    const text = error.hint ? `${error.message} · ${error.hint}` : error.message
    message.error(text, { duration: 6000 })
  }
)
</script>

<template>
  <QuickPanel v-if="store.view === 'panel'" />
  <SettingsView v-else />
</template>
