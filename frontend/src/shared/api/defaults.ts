import type { Settings } from './types'

export const DEFAULT_PROXY_OVERRIDE =
  '<local>;localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*'

export const DEFAULT_HTTP_CHECK_URL = 'http://www.msftconnecttest.com/connecttest.txt'

export const DEFAULT_SETTINGS: Settings = {
  autostart: true,
  launchHidden: true,
  closeAction: 'hide',
  preflightEnabled: true,
  preflightTimeoutMs: 800,
  periodicCheck: false,
  periodicIntervalSec: 60,
  httpCheckEnabled: false,
  httpCheckUrl: DEFAULT_HTTP_CHECK_URL,
  theme: 'system',
  hotkeys: {
    toggle: 'Ctrl+Alt+P',
    panel: 'Ctrl+Alt+O',
    nextProfile: 'Ctrl+Alt+N'
  },
  trayClickAction: 'panel',
  proxyOverride: DEFAULT_PROXY_OVERRIDE,
  keepProxyOnExit: false
}
