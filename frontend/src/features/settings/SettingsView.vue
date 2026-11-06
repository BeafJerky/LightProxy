<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { APP_VERSION_LABEL } from '@/shared/version'
import type { SettingsPage } from '@/shared/api/types'
import Icon from '@/shared/ui/Icon.vue'
import type { IconName } from '@/shared/ui/icons'
import GeneralPage from './pages/GeneralPage.vue'
import AppearancePage from './pages/AppearancePage.vue'
import ProfilesPage from './pages/ProfilesPage.vue'
import ShortcutsPage from './pages/ShortcutsPage.vue'
import DiagnosticsPage from './pages/DiagnosticsPage.vue'
import AboutPage from './pages/AboutPage.vue'

const store = useAppStore()

interface NavItem {
  id: SettingsPage
  label: string
  icon: IconName
  component: unknown
}

const nav: NavItem[] = [
  { id: 'general', label: '通用', icon: 'settings', component: GeneralPage },
  { id: 'appearance', label: '外观', icon: 'palette', component: AppearancePage },
  { id: 'profiles', label: '代理配置', icon: 'globe', component: ProfilesPage },
  { id: 'shortcuts', label: '快捷键', icon: 'keyboard', component: ShortcutsPage },
  { id: 'diagnostics', label: '诊断与修复', icon: 'shield', component: DiagnosticsPage },
  { id: 'about', label: '关于', icon: 'info', component: AboutPage }
]

const current = computed(() => nav.find((item) => item.id === store.page) ?? nav[0])

function minimize() {
  store.closeWindow()
}

function close() {
  if (store.settings.closeAction === 'exit') {
    store.quit()
  } else {
    store.closeWindow()
  }
}
</script>

<template>
  <div class="window settings-window">
    <header class="titlebar" data-tauri-drag-region>
      <div class="titlebar-title" data-tauri-drag-region>
        <button class="icon-btn back" type="button" title="返回快捷面板" @click="store.openPanel()">
          <Icon name="arrowLeft" :size="16" />
        </button>
        <span class="app-mark small"><Icon name="chain" :size="12" /></span>
        <span>LightProxy 设置</span>
      </div>
      <div class="window-controls">
        <button type="button" title="最小化" aria-label="最小化" @click="minimize">
          <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2 6h8" stroke="currentColor" stroke-width="1.2" />
          </svg>
        </button>
        <button class="close" type="button" title="关闭" aria-label="关闭" @click="close">
          <Icon name="close" :size="12" />
        </button>
      </div>
    </header>

    <div class="settings-body">
      <nav class="settings-nav">
        <button
          v-for="item in nav"
          :key="item.id"
          type="button"
          class="nav-item"
          :class="{ 'is-active': store.page === item.id }"
          @click="store.page = item.id"
        >
          <Icon :name="item.icon" :size="16" />
          <span>{{ item.label }}</span>
        </button>
        <div class="nav-foot">
          <span>{{ APP_VERSION_LABEL }}</span>
          <span>Windows 10 / 11</span>
        </div>
      </nav>

      <main class="settings-content">
        <component :is="current.component" />
      </main>
    </div>
  </div>
</template>

<style scoped>
.settings-window {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 44px;
  flex: none;
  padding-left: 10px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
}

.titlebar-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 600;
  min-width: 0;
}

.titlebar-title .back {
  margin-right: 2px;
}

.app-mark {
  display: grid;
  place-items: center;
  background: var(--accent);
  color: var(--on-accent);
}

.app-mark.small {
  width: 20px;
  height: 20px;
  border-radius: 6px;
}

.window-controls {
  display: flex;
  height: 100%;
}

.window-controls button {
  width: 46px;
  height: 100%;
  display: grid;
  place-items: center;
  border: 0;
  background: transparent;
  color: var(--muted);
}

.window-controls button:hover {
  background: var(--surface-2);
  color: var(--text);
}

.window-controls button.close:hover {
  background: var(--close-bg);
  color: var(--close-ink);
}

.settings-body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.settings-nav {
  width: 176px;
  flex: none;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 13px 10px 10px;
  background: var(--surface-2);
  border-right: 1px solid var(--border);
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 8px 10px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--text-2);
  font-size: 12.5px;
  text-align: left;
  transition: background 0.15s ease, color 0.15s ease;
}

.nav-item :deep(svg) {
  color: var(--muted);
}

.nav-item:hover {
  background: var(--surface-3);
  color: var(--text);
}

.nav-item.is-active {
  background: var(--accent-soft);
  color: var(--accent-strong);
  font-weight: 600;
}

.nav-item.is-active :deep(svg) {
  color: var(--accent-strong);
}

.nav-foot {
  margin-top: auto;
  padding: 8px 10px 2px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 10px;
  line-height: 1.5;
  color: var(--muted);
}

.settings-content {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 18px 24px 20px;
}
</style>
