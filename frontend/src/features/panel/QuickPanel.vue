<script setup lang="ts">
import { computed, ref } from 'vue'
import { useMessage } from 'naive-ui'
import Icon from '@/shared/ui/Icon.vue'
import StatusCard from '@/features/status/StatusCard.vue'
import { canActivate, profileServer, useAppStore } from '@/stores/app'
import { APP_VERSION } from '@/shared/version'
import type { Profile } from '@/shared/api/types'

const store = useAppStore()
const message = useMessage()
const pendingTarget = ref<string | null>(null)

const tone = computed(() => store.stateTone)
const heroTitle = computed(() => (store.isActive ? '关闭代理' : '开启代理'))
const heroSub = computed(() => {
  if (store.isBusyState) return '正在同步系统设置…'
  if (store.isActive) {
    return store.state?.state === 'degraded'
      ? '连接异常 · 查看诊断或重新开启'
      : `当前使用 · ${store.activeProfile?.name ?? '未命名'}`
  }
  return `将使用 · ${store.defaultProfile?.name ?? '未配置'}`
})

const hotkeyLabel = computed(() => store.settings.hotkeys.toggle.replaceAll('+', ' + '))

async function heroAction() {
  if (store.isBusyState || store.busy) return
  pendingTarget.value = null
  await store.toggle()
}

async function selectProfile(profile: Profile) {
  if (!canActivate(profile)) {
    message.warning('socks5 配置当前仅保存，暂不支持启用')
    return
  }
  if (profile.id === store.state?.activeProfileId) return
  pendingTarget.value = profile.id
  const ok = await store.switchProfile(profile.id)
  if (ok) pendingTarget.value = null
}

async function forceEnable() {
  const id = pendingTarget.value ?? store.activeProfile?.id
  if (!id) return
  const ok = await store.enable(id, true)
  if (ok) pendingTarget.value = null
}

async function checkHealth() {
  const result = await store.checkHealth()
  if (!result) return
  if (result.ok) {
    message.success(`连接正常，延迟 ${result.latencyMs}ms`)
  } else {
    message.error(result.error ?? '代理端口不可达')
  }
}

async function copyAddress() {
  const target = store.activeProfile
  if (!target) {
    message.warning('还没有可复制的代理地址')
    return
  }
  try {
    await navigator.clipboard.writeText(profileServer(target))
    message.success('已复制代理地址')
  } catch {
    message.error('复制失败，请手动选择地址')
  }
}

function openRepair() {
  store.page = 'diagnostics'
  store.openSettings()
  void store.loadDiagnostics()
  void store.loadRepairs()
}
</script>

<template>
  <div class="window tray-panel" :class="`state-${tone}`">
    <header class="panel-head" data-tauri-drag-region>
      <div class="app-id">
        <span class="app-mark"><Icon name="chain" :size="16" /></span>
        <div>
          <strong>LightProxy</strong>
          <span class="panel-sub">代理切换器</span>
        </div>
      </div>
      <div class="panel-head-actions">
        <button class="icon-btn" type="button" aria-label="打开设置" title="设置" @click="store.openSettings()">
          <Icon name="settings" :size="16" />
        </button>
        <button class="icon-btn" type="button" aria-label="隐藏面板" title="隐藏" @click="store.closeWindow()">
          <Icon name="close" :size="16" />
        </button>
      </div>
    </header>

    <StatusCard />

    <button class="hero-toggle" type="button" :disabled="store.isBusyState || Boolean(store.busy)" @click="heroAction">
      <span class="hero-icon">
        <Icon :name="store.isBusyState ? 'refresh' : 'power'" :size="18" :class="{ spin: store.isBusyState }" />
      </span>
      <span class="hero-copy">
        <strong>{{ heroTitle }}</strong>
        <span>{{ heroSub }}</span>
      </span>
      <span class="hero-switch" aria-hidden="true"><i /></span>
    </button>

    <div v-if="store.error && store.canForce" class="force-banner">
      <div class="force-copy">
        <strong>需要你确认</strong>
        <span>{{ store.error.message }}</span>
      </div>
      <button type="button" class="force-btn" :disabled="Boolean(store.busy)" @click="forceEnable">仍然开启</button>
    </div>

    <div class="section-head">
      <span>代理配置</span>
      <button class="link-btn" type="button" @click="store.page = 'profiles'; store.openSettings()">
        管理
        <Icon name="chevron" :size="13" />
      </button>
    </div>

    <div class="profile-list">
      <button
        v-for="profile in store.profiles"
        :key="profile.id"
        class="profile-row"
        :class="{ 'is-active': profile.id === store.state?.activeProfileId, 'is-disabled': !canActivate(profile) }"
        type="button"
        :title="canActivate(profile) ? profileServer(profile) : 'socks5 配置仅保存'"
        @click="selectProfile(profile)"
      >
        <span class="radio" />
        <span class="profile-main">
          <strong>{{ profile.name }}</strong>
          <span>{{ profileServer(profile) }}</span>
        </span>
        <span v-if="profile.id === store.state?.activeProfileId" class="badge">使用中</span>
        <span v-else-if="!canActivate(profile)" class="badge subtle">仅保存</span>
        <span v-else-if="profile.id === store.state?.defaultProfileId" class="badge subtle">默认</span>
      </button>
      <p v-if="store.profiles.length === 0" class="empty-hint">还没有配置，去设置里添加一套。</p>
    </div>

    <div class="quick-actions">
      <button class="quick" type="button" :class="{ 'is-loading': store.busy === 'health' }" @click="checkHealth">
        <Icon name="activity" :size="14" />
        <span>检查</span>
      </button>
      <button class="quick" type="button" @click="openRepair">
        <Icon name="wrench" :size="14" />
        <span>修复</span>
      </button>
      <button class="quick" type="button" @click="copyAddress">
        <Icon name="copy" :size="14" />
        <span>复制</span>
      </button>
    </div>

    <div class="panel-foot">
      <span class="hotkey-hint"><kbd>{{ hotkeyLabel }}</kbd> 开关</span>
      <span>v{{ APP_VERSION }}</span>
    </div>
  </div>
</template>

<style scoped>
.tray-panel {
  height: 100%;
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 15px 14px 10px 16px;
}

.app-id {
  display: flex;
  align-items: center;
  gap: 9px;
}

.app-mark {
  width: 30px;
  height: 30px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: 9px;
  background: var(--accent);
  color: var(--on-accent);
}

.app-id strong {
  display: block;
  font-size: 13.5px;
  letter-spacing: 0.01em;
  font-family: var(--font-display);
}

.panel-sub {
  display: block;
  margin-top: 1px;
  font-size: 10.5px;
  color: var(--muted);
}

.panel-head-actions {
  display: flex;
  gap: 2px;
}

.hero-toggle {
  display: flex;
  align-items: center;
  gap: 11px;
  width: calc(100% - 28px);
  margin: 11px 14px 4px;
  padding: 11px 13px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--surface-2);
  color: var(--text);
  text-align: left;
  transition: transform 0.15s ease, background 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease;
}

.hero-toggle:hover {
  transform: translateY(-1px);
  border-color: var(--border-strong);
}

.hero-toggle:active {
  transform: translateY(0);
}

.hero-toggle:disabled {
  cursor: default;
  opacity: 0.78;
}

.hero-icon {
  width: 36px;
  height: 36px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--surface);
  color: var(--accent);
  border: 1px solid var(--border);
}

.hero-copy {
  min-width: 0;
}

.hero-copy strong {
  display: block;
  font-size: 13.5px;
}

.hero-copy span {
  display: block;
  margin-top: 2px;
  font-size: 11px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hero-switch {
  position: relative;
  margin-left: auto;
  width: 38px;
  height: 22px;
  flex: none;
  border-radius: 999px;
  background: var(--success);
  transition: background 0.18s ease;
}

.hero-switch i {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--switch-knob);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--text) 18%, transparent), 0 1px 2px rgba(0, 0, 0, 0.22);
  transition: transform 0.2s cubic-bezier(0.3, 0.9, 0.4, 1);
  transform: translateX(16px);
}

.state-off .hero-switch {
  background: var(--border-strong);
}

.state-off .hero-switch i {
  transform: translateX(0);
}

.state-warn .hero-switch {
  background: var(--warn);
}

.state-error .hero-switch {
  background: var(--danger);
}

.state-busy .hero-switch {
  background: var(--accent);
}

.state-off .hero-toggle {
  background: var(--accent);
  border-color: transparent;
  color: var(--on-accent);
  box-shadow: 0 12px 24px -14px color-mix(in srgb, var(--accent) 75%, transparent);
}

.state-off .hero-icon {
  background: color-mix(in srgb, var(--on-accent) 16%, transparent);
  border-color: color-mix(in srgb, var(--on-accent) 22%, transparent);
  color: var(--on-accent);
}

.state-off .hero-copy span {
  color: var(--on-accent);
}

.state-off .hero-switch {
  background: color-mix(in srgb, var(--on-accent) 28%, transparent);
}

.state-off .hero-switch i {
  background: var(--on-accent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--on-accent) 30%, transparent);
}

.force-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 8px 14px 0;
  padding: 8px 10px;
  border: 1px solid color-mix(in srgb, var(--warn) 36%, var(--border));
  border-radius: var(--radius-sm);
  background: var(--warn-soft);
  color: var(--warn);
  animation: lp-fade-in 0.18s ease;
}

.force-copy {
  min-width: 0;
  flex: 1;
}

.force-copy strong {
  display: block;
  font-size: 11px;
}

.force-copy span {
  display: block;
  margin-top: 2px;
  font-size: 10.5px;
  line-height: 1.4;
}

.force-btn {
  flex: none;
  height: 26px;
  padding: 0 9px;
  border: 1px solid currentColor;
  border-radius: 6px;
  background: transparent;
  color: inherit;
  font-size: 10.5px;
  font-weight: 600;
}

.force-btn:hover {
  background: color-mix(in srgb, var(--warn) 12%, transparent);
}

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 13px 16px 6px;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--muted);
}

.link-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--accent-strong);
  font-size: 11px;
  font-weight: 600;
}

.link-btn:hover {
  background: var(--accent-soft);
}

.profile-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 0 10px;
  max-height: 174px;
  overflow-y: auto;
}

.profile-row {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 8px 9px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: transparent;
  color: var(--text);
  text-align: left;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.profile-row:hover {
  background: var(--surface-2);
}

.profile-row.is-disabled {
  opacity: 0.62;
}

.profile-row.is-active {
  background: var(--accent-soft);
  border-color: color-mix(in srgb, var(--accent) 22%, transparent);
}

.radio {
  width: 14px;
  height: 14px;
  flex: none;
  display: grid;
  place-items: center;
  border: 1.5px solid var(--border-strong);
  border-radius: 50%;
  transition: border-color 0.15s ease;
}

.profile-row.is-active .radio {
  border-color: var(--accent);
}

.profile-row.is-active .radio::after {
  content: '';
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--accent);
}

.profile-main {
  min-width: 0;
  flex: 1;
}

.profile-main strong {
  display: block;
  font-size: 12.5px;
}

.profile-main span {
  display: block;
  margin-top: 1px;
  font-family: var(--mono);
  font-size: 10.5px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.badge {
  display: inline-flex;
  align-items: center;
  flex: none;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent-strong);
  font-size: 10.5px;
  font-weight: 600;
}

.badge.subtle {
  background: var(--surface-3);
  color: var(--muted);
}

.empty-hint {
  margin: 6px 4px 10px;
  font-size: 11.5px;
  color: var(--muted);
}

.quick-actions {
  margin-top: auto;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
  padding: 11px 14px 10px;
}

.quick {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  height: 34px;
  padding: 0 6px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 11px;
  white-space: nowrap;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
}

.quick:hover {
  background: var(--surface-3);
  border-color: var(--border-strong);
  color: var(--text);
}

.quick.is-loading :deep(svg) {
  animation: lp-spin 1s linear infinite;
}

.panel-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 16px 12px;
  border-top: 1px solid var(--border);
  font-size: 10px;
  color: var(--muted);
}

.hotkey-hint kbd {
  padding: 1px 4px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--surface-2);
  font-family: var(--mono);
  font-size: 9.5px;
}
</style>
