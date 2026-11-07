import { nowMs } from '@/shared/time'
import { APP_VERSION } from '@/shared/version'
import { DEFAULT_SETTINGS } from './defaults'
import type {
  AppState,
  CoreError,
  Diagnostic,
  HealthResult,
  Profile,
  ProfileInput,
  RepairRecord,
  RepairResult,
  Settings,
  UpdateCheck,
  UpdatePackage,
  UpdateProgress
} from './types'

function error(code: string, message: string, hint?: string): never {
  throw { code, stage: 'mock', message, hint } satisfies CoreError
}

function initialState(): AppState {
  const profiles: Profile[] = [
    { id: 'p_default', name: '公司代理', protocol: 'http', host: '192.168.5.4', port: 7890 },
    { id: 'p_backup', name: '备用代理', protocol: 'http', host: '192.168.5.9', port: 7890 },
    { id: 'p_lab', name: '实验环境', protocol: 'http', host: '10.0.0.2', port: 8080 }
  ]
  return {
    state: 'on',
    desired: 'on',
    activeProfileId: 'p_default',
    lastHealth: {
      ok: true,
      stage: 'tcp',
      latencyMs: 12,
      checkedAtMs: nowMs() - 30_000,
      error: null
    },
    message: null,
    profiles,
    defaultProfileId: 'p_default',
    settings: structuredClone(DEFAULT_SETTINGS),
    snapshot: {
      proxyEnable: false,
      proxyServer: '',
      proxyOverride: '',
      autoConfigUrl: null
    }
  }
}

let state = initialState()

let mockProgressHandler: ((progress: UpdateProgress) => void) | null = null

export function setMockProgressHandler(handler: ((progress: UpdateProgress) => void) | null) {
  mockProgressHandler = handler
}

function updateFailEnabled(): boolean {
  // Node/vitest 下 localStorage 可能不可用，mock 只是开发期辅助，读不到即视为未开启。
  try {
    return localStorage.getItem('lp_mock_update_fail') === '1'
  } catch {
    return false
  }
}

function nextPatch(version: string): string {
  const [major, minor, patch] = version.split('.').map(Number)
  return `${major}.${minor}.${patch + 1}`
}

function mockUpdateCheck(): UpdateCheck {
  if (updateFailEnabled()) {
    error('update_check_failed', '检查更新失败：模拟网络不可用', '检查网络或代理后重试')
  }
  const latestVersion = nextPatch(APP_VERSION)
  return {
    currentVersion: APP_VERSION,
    latestVersion,
    available: true,
    releaseUrl: `https://github.com/BeafJerky/LightProxy/releases/tag/v${latestVersion}`,
    assetName: `LightProxy_${latestVersion}_x64-setup.exe`,
    source: '直连',
    checkedAtMs: nowMs()
  }
}

async function mockDownload(version: string): Promise<UpdatePackage> {
  if (updateFailEnabled()) {
    error('update_download_failed', '下载安装包失败：模拟网络中断', '重试或切换网络')
  }
  const total = 3_400_000
  for (let step = 1; step <= 5; step += 1) {
    await new Promise((resolve) => setTimeout(resolve, 120))
    mockProgressHandler?.({
      version,
      received: Math.round((total * step) / 5),
      total,
      percent: step * 20,
      source: '直连'
    })
  }
  return {
    version,
    fileName: `LightProxy_${version}_x64-setup.exe`,
    size: total,
    sha256: 'mock',
    source: '直连'
  }
}

const repairs: RepairRecord[] = []

const MOCK_DIAGNOSTICS: Diagnostic[] = [
  {
    id: 'drift',
    title: '系统代理与 LightProxy 记录不一致',
    detail: '检测到外部程序修改了 ProxyEnable，可按当前配置重写。',
    severity: 'warn',
    autoFixable: true
  }
]

const isActive = () => state.state === 'on' || state.state === 'degraded'

function requireActivatable(profile: Profile | undefined): Profile {
  if (!profile) error('E_PROFILE_NOT_FOUND', '配置不存在')
  return profile
}

function profileOf(id: string | null): Profile | undefined {
  return state.profiles.find((profile) => profile.id === id)
}

function activeTarget(): Profile | undefined {
  return (
    profileOf(state.activeProfileId) ??
    state.profiles.find((profile) => profile.id === state.defaultProfileId) ??
    state.profiles[0]
  )
}

function deepMerge(target: Settings, patch: Record<string, unknown>): Settings {
  const next = { ...target } as unknown as Record<string, unknown>
  for (const [key, value] of Object.entries(patch)) {
    if (value && typeof value === 'object' && !Array.isArray(value)) {
      const current = next[key]
      if (current && typeof current === 'object') {
        next[key] = { ...(current as object), ...(value as object) }
        continue
      }
    }
    next[key] = value
  }
  return next as unknown as Settings
}

export function mockCall(
  command: string,
  args: Record<string, unknown> = {}
): unknown | Promise<unknown> {
  switch (command) {
    case 'get_state':
      return state
    case 'toggle_proxy':
      if (isActive()) {
        state = { ...state, state: 'off', desired: 'off', activeProfileId: null }
      } else {
        const target = requireActivatable(activeTarget())
        state = { ...state, state: 'on', desired: 'on', activeProfileId: target.id }
      }
      return state
    case 'enable_proxy': {
      const target = requireActivatable(profileOf(String(args.profileId)))
      state = { ...state, state: 'on', desired: 'on', activeProfileId: target.id }
      return state
    }
    case 'disable_proxy':
      state = { ...state, state: 'off', desired: 'off', activeProfileId: null }
      return state
    case 'switch_profile': {
      const target = requireActivatable(profileOf(String(args.profileId)))
      state = isActive()
        ? { ...state, activeProfileId: target.id }
        : { ...state, state: 'on', desired: 'on', activeProfileId: target.id }
      return state
    }
    case 'check_health': {
      const target = activeTarget()
      if (!target) error('E_PROFILE_NOT_FOUND', '没有可用的代理配置')
      const result: HealthResult = {
        ok: true,
        stage: 'tcp',
        latencyMs: 8 + Math.round(Math.random() * 18),
        checkedAtMs: nowMs(),
        error: null
      }
      state = { ...state, lastHealth: result }
      return result
    }
    case 'list_profiles':
      return state.profiles
    case 'save_profile': {
      const input = args.input as ProfileInput
      if (!input.name.trim()) error('E_INVALID_NAME', '名称不能为空')
      const existing = input.id ? profileOf(input.id) : undefined
      const profile: Profile = {
        id: existing?.id ?? `p_${nowMs()}`,
        name: input.name.trim(),
        protocol: (input.protocol as Profile['protocol']) ?? 'http',
        host: input.host.trim(),
        port: input.port
      }
      state = {
        ...state,
        profiles: existing
          ? state.profiles.map((item) => (item.id === existing.id ? profile : item))
          : [...state.profiles, profile]
      }
      return state
    }
    case 'delete_profile': {
      const id = String(args.id)
      if (id === state.defaultProfileId) error('E_DEFAULT_PROFILE', '默认配置不能删除', '请先把其他配置设为默认')
      state = {
        ...state,
        profiles: state.profiles.filter((profile) => profile.id !== id),
        activeProfileId: state.activeProfileId === id ? null : state.activeProfileId
      }
      return state
    }
    case 'set_default_profile': {
      const id = String(args.id)
      if (!profileOf(id)) error('E_PROFILE_NOT_FOUND', '配置不存在')
      state = { ...state, defaultProfileId: id }
      return state
    }
    case 'get_settings':
      return state.settings
    case 'update_settings': {
      state = { ...state, settings: deepMerge(state.settings, args.patch as Record<string, unknown>) }
      return state
    }
    case 'get_diagnostics':
      return MOCK_DIAGNOSTICS
    case 'run_repair': {
      const results = (args.ids as string[]).map<RepairResult>((id) => {
        const diagnostic = MOCK_DIAGNOSTICS.find((item) => item.id === id)
        return {
          id,
          title: diagnostic?.title ?? '重写代理设置',
          ok: true,
          message: '已按当前配置重写系统代理'
        }
      })
      for (const result of results) {
        repairs.unshift({ ...result, atMs: nowMs() })
      }
      return results
    }
    case 'get_recent_repairs':
      return repairs
    case 'open_log_dir':
      return '%LOCALAPPDATA%\\com.lightproxy.desktop\\logs'
    case 'export_diagnostics':
      return { path: '%LOCALAPPDATA%\\com.lightproxy.desktop\\logs\\diagnostics-mock.json' }
    case 'get_initial_view':
      return 'panel'
    case 'check_update':
      return mockUpdateCheck()
    case 'download_update':
      return mockDownload(String(args.version))
    case 'install_update':
      return null
    case 'open_release_page':
      return null
    case 'quit_app':
    case 'open_settings':
    case 'open_panel':
    case 'set_view':
    case 'hide_window':
      return null
    default:
      error('E_UNKNOWN_COMMAND', `未实现的开发命令：${command}`)
  }
}
