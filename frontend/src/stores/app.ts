import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { api, toCoreError } from '@/shared/api'
import { DEFAULT_SETTINGS } from '@/shared/api/defaults'
import type {
  AppState,
  CoreError,
  Diagnostic,
  Profile,
  ProfileInput,
  RepairRecord,
  RepairResult,
  Settings,
  SettingsPage,
  UpdateCheck,
  UpdatePackage,
  UpdateProgress,
  View
} from '@/shared/api/types'
import { isThemeId } from '@/shared/theme/themes'
import { applyTheme } from '@/shared/theme/useTheme'

const FORCEABLE = new Set(['pac_conflict', 'proxy_unreachable'])

export function profileServer(profile: Profile): string {
  return profile.host.includes(':') ? `[${profile.host}]:${profile.port}` : `${profile.host}:${profile.port}`
}

export function profileUrl(profile: Profile): string {
  return `${profile.protocol}://${profileServer(profile)}`
}

export const useAppStore = defineStore('app', () => {
  const state = ref<AppState | null>(null)
  const view = ref<View>('panel')
  const page = ref<SettingsPage>('general')
  const busy = ref<string | null>(null)
  const error = ref<CoreError | null>(null)
  const diagnostics = ref<Diagnostic[]>([])
  const repairs = ref<RepairRecord[]>([])
  const ready = ref(false)
  const updateCheck = ref<UpdateCheck | null>(null)
  const updatePackage = ref<UpdatePackage | null>(null)
  const updateProgress = ref<UpdateProgress | null>(null)
  const updateError = ref<CoreError | null>(null)
  const updateBusy = ref<'check' | 'download' | 'install' | null>(null)
  // 安装器启动成功后应用会退出；mock 下不会，用独立标记保持 installing 而不占住 updateBusy
  const installLaunched = ref(false)

  const settings = computed<Settings>(() => state.value?.settings ?? DEFAULT_SETTINGS)
  const profiles = computed(() => state.value?.profiles ?? [])
  const activeProfile = computed<Profile | null>(() => {
    const current = state.value
    if (!current) return null
    return (
      current.profiles.find((profile) => profile.id === current.activeProfileId) ??
      current.profiles.find((profile) => profile.id === current.defaultProfileId) ??
      current.profiles[0] ??
      null
    )
  })
  const defaultProfile = computed<Profile | null>(() => {
    const current = state.value
    if (!current) return null
    return (
      current.profiles.find((profile) => profile.id === current.defaultProfileId) ?? current.profiles[0] ?? null
    )
  })
  const isActive = computed(() => state.value?.state === 'on' || state.value?.state === 'degraded')
  const isBusyState = computed(() =>
    ['enabling', 'repairing', 'disabling'].includes(state.value?.state ?? '')
  )
  const canForce = computed(() => Boolean(error.value && FORCEABLE.has(error.value.code)))
  const statusText = computed(() => {
    switch (state.value?.state) {
      case 'on':
        return '已开启'
      case 'enabling':
        return '正在开启'
      case 'degraded':
        return '需要检查'
      case 'repairing':
        return '正在修复'
      case 'disabling':
        return '正在关闭'
      case 'error':
        return '操作失败'
      default:
        return '已关闭'
    }
  })
  const stateTone = computed(() => {
    switch (state.value?.state) {
      case 'on':
        return 'on'
      case 'degraded':
        return 'warn'
      case 'error':
        return 'error'
      case 'enabling':
      case 'repairing':
      case 'disabling':
        return 'busy'
      default:
        return 'off'
    }
  })
  const updatePhase = computed<
    'idle' | 'checking' | 'latest' | 'available' | 'downloading' | 'ready' | 'installing' | 'error'
  >(() => {
    if (updateError.value) return 'error'
    if (updateBusy.value === 'check') return 'checking'
    if (updateBusy.value === 'download') return 'downloading'
    if (updateBusy.value === 'install') return 'installing'
    if (installLaunched.value) return 'installing'
    if (updatePackage.value && updateCheck.value?.latestVersion === updatePackage.value.version) {
      return 'ready'
    }
    if (updateCheck.value) return updateCheck.value.available ? 'available' : 'latest'
    return 'idle'
  })

  async function run<T>(key: string, work: () => Promise<T>): Promise<T | null> {
    busy.value = key
    error.value = null
    try {
      return await work()
    } catch (caught) {
      error.value = toCoreError(caught)
      return null
    } finally {
      busy.value = null
    }
  }

  async function applyState(key: string, work: () => Promise<AppState>): Promise<boolean> {
    const result = await run(key, work)
    if (!result) return false
    state.value = result
    return true
  }

  function clearError() {
    error.value = null
  }

  async function initialize() {
    const initialView = await run('init', () => api.getInitialView())
    view.value = initialView ?? 'panel'
    await refresh()
    if (state.value && isThemeId(state.value.settings.theme)) {
      applyTheme(state.value.settings.theme)
    }
    ready.value = true
  }

  async function refresh() {
    const result = await run('refresh', () => api.getState())
    if (result) state.value = result
  }

  function setStateFromEvent(next: AppState) {
    state.value = next
  }

  function openSettings() {
    view.value = 'settings'
    void api.openSettings().catch((caught) => {
      error.value = toCoreError(caught)
    })
  }

  function openPanel() {
    view.value = 'panel'
    void api.openPanel().catch((caught) => {
      error.value = toCoreError(caught)
    })
  }

  function closeWindow() {
    void api.hideWindow().catch((caught) => {
      error.value = toCoreError(caught)
    })
  }

  function quit() {
    void api.quitApp().catch((caught) => {
      error.value = toCoreError(caught)
    })
  }

  const toggle = (force = false) => applyState('toggle', () => api.toggleProxy(force))
  const enable = (profileId: string, force = false) =>
    applyState('enable', () => api.enableProxy(profileId, force))
  const disable = () => applyState('disable', () => api.disableProxy())
  const switchProfile = (profileId: string, force = false) =>
    applyState('switch', () => api.switchProfile(profileId, force))

  async function checkHealth(force = false, mode: 'tcp' | 'deep' = 'tcp') {
    const report = await run('health', () => api.checkHealth(force, mode))
    if (report && state.value) {
      state.value = { ...state.value, lastHealth: report.tcp }
    }
    return report
  }

  const saveProfile = (input: ProfileInput) => applyState('save-profile', () => api.saveProfile(input))
  const deleteProfile = (id: string) => applyState('delete-profile', () => api.deleteProfile(id))
  const setDefault = (id: string) => applyState('default-profile', () => api.setDefaultProfile(id))

  async function updateSettings(patch: Record<string, unknown>) {
    return applyState('settings', () => api.updateSettings(patch))
  }

  async function updateHotkeys(patch: Partial<Settings['hotkeys']>) {
    return updateSettings({ hotkeys: { ...settings.value.hotkeys, ...patch } })
  }

  async function loadDiagnostics() {
    const result = await run('diagnostics', () => api.getDiagnostics())
    if (result) diagnostics.value = result
    return result
  }

  async function loadRepairs() {
    const result = await run('repairs', () => api.getRecentRepairs())
    if (result) repairs.value = result
    return result
  }

  async function repair(ids: string[]): Promise<RepairResult[] | null> {
    const result = await run('repair', () => api.runRepair(ids))
    if (result) await Promise.all([loadDiagnostics(), loadRepairs(), refresh()])
    return result
  }

  async function repairAll() {
    const ids = diagnostics.value.filter((item) => item.autoFixable).map((item) => item.id)
    if (ids.length === 0) return []
    return repair(ids)
  }

  async function exportDiagnostics() {
    return run('export', () => api.exportDiagnostics())
  }

  async function openLogDir() {
    return run('logs', () => api.openLogDir())
  }

  async function checkUpdate(force = false) {
    if (updateBusy.value) return
    updateError.value = null
    installLaunched.value = false
    updateBusy.value = 'check'
    try {
      const result = await api.checkUpdate(force)
      updateCheck.value = result
      if (!result.available) {
        updatePackage.value = null
        updateProgress.value = null
      } else if (updatePackage.value && updatePackage.value.version !== result.latestVersion) {
        updatePackage.value = null
        updateProgress.value = null
      }
    } catch (caught) {
      updateError.value = toCoreError(caught)
    } finally {
      updateBusy.value = null
    }
  }

  async function downloadUpdate() {
    const latest = updateCheck.value
    if (!latest?.available || !latest.latestVersion || updateBusy.value) return
    updateError.value = null
    updateProgress.value = null
    updateBusy.value = 'download'
    try {
      updatePackage.value = await api.downloadUpdate(latest.latestVersion)
    } catch (caught) {
      updatePackage.value = null
      updateError.value = toCoreError(caught)
    } finally {
      updateBusy.value = null
    }
  }

  async function installUpdate() {
    if (!updatePackage.value || updateBusy.value) return
    updateError.value = null
    updateBusy.value = 'install'
    try {
      await api.installUpdate()
      installLaunched.value = true
    } catch (caught) {
      updateError.value = toCoreError(caught)
    } finally {
      updateBusy.value = null
    }
  }

  async function openReleasePage() {
    const version = updateCheck.value?.latestVersion
    if (!version) return
    try {
      await api.openReleasePage(version)
    } catch (caught) {
      updateError.value = toCoreError(caught)
    }
  }

  function applyUpdateProgress(progress: UpdateProgress) {
    // 只有下载阶段接受进度：其它阶段的事件都是迟到的旧值
    if (updateBusy.value !== 'download') return
    // 事件可能晚于状态切换到达，只接受当前检查版本的进度
    if (updateCheck.value?.latestVersion !== progress.version) return
    updateProgress.value = progress
  }

  watch(
    () => settings.value.theme,
    (theme) => {
      if (isThemeId(theme)) applyTheme(theme)
    }
  )

  return {
    state,
    view,
    page,
    busy,
    error,
    diagnostics,
    repairs,
    ready,
    settings,
    profiles,
    activeProfile,
    defaultProfile,
    isActive,
    isBusyState,
    canForce,
    statusText,
    stateTone,
    initialize,
    refresh,
    setStateFromEvent,
    clearError,
    openSettings,
    openPanel,
    closeWindow,
    quit,
    toggle,
    enable,
    disable,
    switchProfile,
    checkHealth,
    saveProfile,
    deleteProfile,
    setDefault,
    updateSettings,
    updateHotkeys,
    loadDiagnostics,
    loadRepairs,
    repair,
    repairAll,
    exportDiagnostics,
    openLogDir,
    updateCheck,
    updatePackage,
    updateProgress,
    updateError,
    updatePhase,
    checkUpdate,
    downloadUpdate,
    installUpdate,
    openReleasePage,
    applyUpdateProgress
  }
})
