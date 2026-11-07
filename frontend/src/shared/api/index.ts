import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { mockCall, setMockProgressHandler } from './mock'
import type {
  AppState,
  CheckReport,
  CoreError,
  Diagnostic,
  Profile,
  ProfileInput,
  RepairRecord,
  RepairResult,
  Settings,
  UpdateCheck,
  UpdatePackage,
  UpdateProgress,
  View
} from './types'

export const isMock = import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isMock) {
    await new Promise((resolve) => setTimeout(resolve, 90))
    return (await mockCall(command, args)) as T
  }
  return invoke<T>(command, args)
}

export function toCoreError(error: unknown): CoreError {
  if (error && typeof error === 'object' && 'message' in error) {
    const candidate = error as Partial<CoreError>
    return {
      code: candidate.code ?? 'E_UNKNOWN',
      stage: candidate.stage ?? 'frontend',
      message: String(candidate.message ?? '未知错误'),
      hint: candidate.hint ?? null
    }
  }
  return { code: 'E_UNKNOWN', stage: 'frontend', message: String(error), hint: null }
}

export const api = {
  getState: () => call<AppState>('get_state'),
  toggleProxy: (force = false) => call<AppState>('toggle_proxy', { force }),
  enableProxy: (profileId: string, force = false) => call<AppState>('enable_proxy', { profileId, force }),
  disableProxy: () => call<AppState>('disable_proxy'),
  switchProfile: (profileId: string, force = false) =>
    call<AppState>('switch_profile', { profileId, force }),
  checkHealth: (force = false, mode: 'tcp' | 'deep' = 'tcp') =>
    call<CheckReport>('check_health', { mode, force }),
  listProfiles: () => call<Profile[]>('list_profiles'),
  saveProfile: (input: ProfileInput) => call<AppState>('save_profile', { input }),
  deleteProfile: (id: string) => call<AppState>('delete_profile', { id }),
  setDefaultProfile: (id: string) => call<AppState>('set_default_profile', { id }),
  getSettings: () => call<Settings>('get_settings'),
  updateSettings: (patch: Record<string, unknown>) => call<AppState>('update_settings', { patch }),
  getDiagnostics: () => call<Diagnostic[]>('get_diagnostics'),
  runRepair: (ids: string[]) => call<RepairResult[]>('run_repair', { ids }),
  getRecentRepairs: () => call<RepairRecord[]>('get_recent_repairs'),
  openLogDir: () => call<string>('open_log_dir'),
  exportDiagnostics: () => call<{ path: string }>('export_diagnostics'),
  quitApp: () => call<void>('quit_app'),
  openSettings: () => call<void>('open_settings'),
  openPanel: () => call<void>('open_panel'),
  setView: (view: View) => call<void>('set_view', { view }),
  hideWindow: () => call<void>('hide_window'),
  getInitialView: () => call<View>('get_initial_view'),
  checkUpdate: (force = false) => call<UpdateCheck>('check_update', { force }),
  downloadUpdate: (version: string) => call<UpdatePackage>('download_update', { version }),
  installUpdate: () => call<void>('install_update'),
  openReleasePage: (version: string) => call<void>('open_release_page', { version })
}

export async function onStateChanged(handler: (state: AppState) => void): Promise<UnlistenFn> {
  if (isMock) return () => {}
  return listen<AppState>('state-changed', (event) => handler(event.payload))
}

export async function onNavigate(handler: (view: View) => void): Promise<UnlistenFn> {
  if (isMock) return () => {}
  return listen<View>('navigate', (event) => handler(event.payload))
}

export async function onUpdateProgress(
  handler: (progress: UpdateProgress) => void
): Promise<UnlistenFn> {
  if (isMock) {
    setMockProgressHandler(handler)
    return () => setMockProgressHandler(null)
  }
  return listen<UpdateProgress>('update-progress', (event) => handler(event.payload))
}
