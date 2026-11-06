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
  launchHidden: boolean
  closeAction: 'hide' | 'exit'
  preflightEnabled: boolean
  preflightTimeoutMs: number
  periodicCheck: boolean
  periodicIntervalSec: number
  httpCheckEnabled: boolean
  httpCheckUrl: string
  theme: string
  hotkeys: Hotkeys
  trayClickAction: 'panel' | 'toggle'
  proxyOverride: string
  keepProxyOnExit: boolean
}

export interface HealthResult {
  ok: boolean
  stage: string
  latencyMs: number
  checkedAtMs: number
  error: string | null
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
