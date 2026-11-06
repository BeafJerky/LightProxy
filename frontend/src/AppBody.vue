<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { useMessage } from 'naive-ui'
import { onNavigate, onStateChanged } from '@/shared/api'
import type { View } from '@/shared/api/types'
import { useAppStore } from '@/stores/app'
import QuickPanel from '@/features/panel/QuickPanel.vue'
import SettingsView from '@/features/settings/SettingsView.vue'

const store = useAppStore()
const message = useMessage()
const listeners: Array<() => void> = []

async function maybePanelHealth(view: View) {
  if (view !== 'panel' || !store.ready || !store.isActive) return
  await store.checkHealth(false)
}

onMounted(async () => {
  await store.initialize()
  void maybePanelHealth(store.view)
  listeners.push(
    await onStateChanged((state) => {
      store.setStateFromEvent(state)
    })
  )
  listeners.push(
    await onNavigate((view) => {
      store.view = view
      void maybePanelHealth(view)
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
