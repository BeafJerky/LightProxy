export const THEME_IDS = [
  'light',
  'dark',
  'system',
  'win11',
  'win11-dark',
  'terminal',
  'paper',
  'contrast'
] as const

export type ThemeId = (typeof THEME_IDS)[number]
export type ThemeScheme = 'light' | 'dark'
export type ThemePack = 'default' | 'win11' | 'win11-dark' | 'terminal' | 'paper' | 'contrast'

export interface ResolvedTheme {
  theme: ThemeScheme
  pack: ThemePack
}

export interface ThemeMeta {
  id: ThemeId
  label: string
  description: string
  theme: ThemeScheme | 'auto'
  pack: ThemePack
}

export const THEME_META: Record<ThemeId, ThemeMeta> = {
  light: { id: 'light', label: '浅色', description: '冷灰底 · 蓝色强调', theme: 'light', pack: 'default' },
  dark: { id: 'dark', label: '深色', description: '低亮度 · 蓝色强调', theme: 'dark', pack: 'default' },
  system: { id: 'system', label: '跟随系统', description: '只解析默认浅 / 深', theme: 'auto', pack: 'default' },
  win11: { id: 'win11', label: 'WIN11 云母', description: 'Fluent 浅色 · 原生质感', theme: 'light', pack: 'win11' },
  'win11-dark': {
    id: 'win11-dark',
    label: 'WIN11 云母（深）',
    description: 'Fluent 深色 · 青色强调', theme: 'dark', pack: 'win11-dark'
  },
  terminal: { id: 'terminal', label: '机房终端', description: '深空青 · 等宽数字', theme: 'dark', pack: 'terminal' },
  paper: { id: 'paper', label: '纸墨', description: '暖纸 · 赭石强调', theme: 'light', pack: 'paper' },
  contrast: { id: 'contrast', label: '高对比', description: '纯黑底 · 黄色强调', theme: 'dark', pack: 'contrast' }
}

export const THEME_ORDER: ThemeId[] = [...THEME_IDS]

export function isThemeId(value: unknown): value is ThemeId {
  return typeof value === 'string' && (THEME_IDS as readonly string[]).includes(value)
}

export function resolveTheme(choice: ThemeId, systemDark: boolean): ResolvedTheme {
  switch (choice) {
    case 'light':
      return { theme: 'light', pack: 'default' }
    case 'dark':
      return { theme: 'dark', pack: 'default' }
    case 'system':
      return { theme: systemDark ? 'dark' : 'light', pack: 'default' }
    case 'win11':
      return { theme: 'light', pack: 'win11' }
    case 'win11-dark':
      return { theme: 'dark', pack: 'win11-dark' }
    case 'terminal':
      return { theme: 'dark', pack: 'terminal' }
    case 'paper':
      return { theme: 'light', pack: 'paper' }
    case 'contrast':
      return { theme: 'dark', pack: 'contrast' }
    default:
      return { theme: systemDark ? 'dark' : 'light', pack: 'default' }
  }
}

export interface ThemePalette {
  bg: string
  surface: string
  surface2: string
  surface3: string
  border: string
  borderStrong: string
  controlBorder: string
  text: string
  text2: string
  muted: string
  accent: string
  accentStrong: string
  accentSoft: string
  onAccent: string
  success: string
  successSoft: string
  warn: string
  danger: string
  knob: string
  radiusLg: string
  radiusMd: string
  radiusSm: string
  font: string
}

const FONT = '"Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", system-ui, -apple-system, sans-serif'

const DEFAULT_LIGHT: ThemePalette = {
  bg: '#eef1f5',
  surface: '#ffffff',
  surface2: '#f6f7f9',
  surface3: '#eceff4',
  border: '#e2e6eb',
  borderStrong: '#cbd2db',
  controlBorder: '#8a94a3',
  text: '#1b1f26',
  text2: '#49515c',
  muted: '#626c7a',
  accent: '#2563eb',
  accentStrong: '#1d4ed8',
  accentSoft: '#e8efff',
  onAccent: '#ffffff',
  success: '#0b7a42',
  successSoft: '#e4f6ec',
  warn: '#9a5b00',
  danger: '#c62828',
  knob: '#ffffff',
  radiusLg: '18px',
  radiusMd: '12px',
  radiusSm: '8px',
  font: FONT
}

const DEFAULT_DARK: ThemePalette = {
  bg: '#111317',
  surface: '#1c2026',
  surface2: '#232830',
  surface3: '#2a3039',
  border: '#30363f',
  borderStrong: '#3d4550',
  controlBorder: '#6b7686',
  text: '#f0f3f7',
  text2: '#c2c9d3',
  muted: '#8e98a6',
  accent: '#5b8cff',
  accentStrong: '#7aa2ff',
  accentSoft: '#24345c',
  onAccent: '#07101f',
  success: '#40c57a',
  successSoft: '#163525',
  warn: '#e2a33a',
  danger: '#ff6b6b',
  knob: '#07101f',
  radiusLg: '18px',
  radiusMd: '12px',
  radiusSm: '8px',
  font: FONT
}

const WIN11: ThemePalette = {
  bg: '#f3f3f3',
  surface: '#fbfbfb',
  surface2: '#f5f5f5',
  surface3: '#ebebeb',
  border: '#e3e3e3',
  borderStrong: '#cfcfcf',
  controlBorder: '#8a8a8a',
  text: '#1b1b1b',
  text2: '#3d3d3d',
  muted: '#5e5e5e',
  accent: '#005fb8',
  accentStrong: '#004c94',
  accentSoft: '#e7f1fa',
  onAccent: '#ffffff',
  success: '#0f7b3f',
  successSoft: '#e6f4ec',
  warn: '#8a5300',
  danger: '#c42b1c',
  knob: '#ffffff',
  radiusLg: '12px',
  radiusMd: '8px',
  radiusSm: '4px',
  font: FONT
}

const WIN11_DARK: ThemePalette = {
  bg: '#202020',
  surface: '#2b2b2b',
  surface2: '#323232',
  surface3: '#3a3a3a',
  border: '#3d3d3d',
  borderStrong: '#4d4d4d',
  controlBorder: '#8a8a8a',
  text: '#ffffff',
  text2: '#d6d6d6',
  muted: '#a9a9a9',
  accent: '#60cdff',
  accentStrong: '#8bddff',
  accentSoft: '#123642',
  onAccent: '#001a24',
  success: '#4cc38a',
  successSoft: '#153126',
  warn: '#f0b429',
  danger: '#ff99a4',
  knob: '#ffffff',
  radiusLg: '12px',
  radiusMd: '8px',
  radiusSm: '4px',
  font: FONT
}

const TERMINAL: ThemePalette = {
  bg: '#070b0c',
  surface: '#0f1618',
  surface2: '#131c1f',
  surface3: '#1a2529',
  border: '#23353a',
  borderStrong: '#35505a',
  controlBorder: '#56747c',
  text: '#e6f4f1',
  text2: '#b8ccc8',
  muted: '#7f9995',
  accent: '#22d3ee',
  accentStrong: '#67e8f9',
  accentSoft: '#0b2e36',
  onAccent: '#001417',
  success: '#4ade80',
  successSoft: '#0e2e20',
  warn: '#f5b942',
  danger: '#ff7b72',
  knob: '#001417',
  radiusLg: '10px',
  radiusMd: '6px',
  radiusSm: '4px',
  font: FONT
}

const PAPER: ThemePalette = {
  bg: '#f5f0e6',
  surface: '#fffdf8',
  surface2: '#fbf6ec',
  surface3: '#f2ebdd',
  border: '#e4d9c6',
  borderStrong: '#cdbea6',
  controlBorder: '#8c7a63',
  text: '#2b241c',
  text2: '#5a4e40',
  muted: '#756757',
  accent: '#a94b24',
  accentStrong: '#8a3a19',
  accentSoft: '#f3e4d9',
  onAccent: '#ffffff',
  success: '#2f6b45',
  successSoft: '#e8f2ea',
  warn: '#86540a',
  danger: '#a32a1e',
  knob: '#ffffff',
  radiusLg: '16px',
  radiusMd: '10px',
  radiusSm: '6px',
  font: FONT
}

const CONTRAST: ThemePalette = {
  bg: '#000000',
  surface: '#0a0a0a',
  surface2: '#141414',
  surface3: '#1e1e1e',
  border: '#c8c8c8',
  borderStrong: '#ffffff',
  controlBorder: '#ffffff',
  text: '#ffffff',
  text2: '#f2f2f2',
  muted: '#d6d6d6',
  accent: '#ffd400',
  accentStrong: '#ffe45c',
  accentSoft: '#332b00',
  onAccent: '#000000',
  success: '#5cff9d',
  successSoft: '#062e1a',
  warn: '#ffd166',
  danger: '#ff8080',
  knob: '#000000',
  radiusLg: '6px',
  radiusMd: '4px',
  radiusSm: '2px',
  font: FONT
}

export function paletteFor(resolved: ResolvedTheme): ThemePalette {
  if (resolved.pack === 'win11') return WIN11
  if (resolved.pack === 'win11-dark') return WIN11_DARK
  if (resolved.pack === 'terminal') return TERMINAL
  if (resolved.pack === 'paper') return PAPER
  if (resolved.pack === 'contrast') return CONTRAST
  return resolved.theme === 'dark' ? DEFAULT_DARK : DEFAULT_LIGHT
}
