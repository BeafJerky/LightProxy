export type Protocol = 'http' | 'https' | 'socks5'
export type ProxyState = 'off' | 'enabling' | 'on' | 'degraded' | 'repairing' | 'disabling' | 'error'
export type Desired = 'off' | 'on'

export interface Profile {
  id: string
  name: string
  protocol: Protocol
  host: string
  port: number
}

export interface ProfileInput {
  id?: string | null
  name: string
  protocol: string
  host: string
  port: number
}

export interface Hotkeys {
  toggle: string
  panel: string
  nextProfile: string
}

export interface Settings {
  autostart: boolean
  closeAction: 'hide' | 'exit'
  preflightEnabled: boolean
  preflightTimeoutMs: number
  theme: string
  hotkeys: Hotkeys
  trayClickAction: 'panel' | 'toggle'
  proxyOverride: string
  keepProxyOnExit: boolean
  syncEnvProxy: boolean
}

export interface HealthResult {
  ok: boolean
  stage: string
  latencyMs: number
  checkedAtMs: number
  error: string | null
}

export interface CheckReport {
  tcp: HealthResult
  proxy: HealthResult | null
}

export type PathStage = 'proxy' | 'tunnel' | 'tls' | 'http'
export type DiagnoseRecommendation = 'http' | 'socks5System' | 'socks5EnvOnly' | 'unavailable'

export interface DiagnosePathResult {
  ok: boolean
  stage: PathStage
  latencyMs: number
  error: string | null
}

export interface DiagnoseReport {
  http: DiagnosePathResult
  socks5Domain: DiagnosePathResult
  systemSocks: DiagnosePathResult
  recommendation: DiagnoseRecommendation
  summary: string
}

export interface ProxySnapshot {
  proxyEnable: boolean
  proxyServer: string
  proxyOverride: string
  autoConfigUrl: string | null
}

export interface AppState {
  state: ProxyState
  desired: Desired
  activeProfileId: string | null
  lastHealth: HealthResult | null
  message: string | null
  profiles: Profile[]
  defaultProfileId: string | null
  settings: Settings
  snapshot: ProxySnapshot | null
}

export interface Diagnostic {
  id: string
  title: string
  detail: string
  severity: 'info' | 'warn' | 'error'
  autoFixable: boolean
}

export interface RepairResult {
  id: string
  title: string
  ok: boolean
  message: string
}

export interface RepairRecord {
  id: string
  title: string
  ok: boolean
  message: string
  atMs: number
}

export interface CoreError {
  code: string
  stage: string
  message: string
  hint?: string | null
}

export type View = 'panel' | 'settings'
export type SettingsPage = 'general' | 'appearance' | 'profiles' | 'shortcuts' | 'diagnostics' | 'about'

export interface UpdateCheck {
  currentVersion: string
  latestVersion: string | null
  available: boolean
  releaseUrl: string | null
  assetName: string | null
  source: string | null
  checkedAtMs: number
}

export interface UpdateProgress {
  version: string
  received: number
  total: number | null
  percent: number | null
  source: string
}

export interface UpdatePackage {
  version: string
  fileName: string
  size: number
  sha256: string
  source: string
}
