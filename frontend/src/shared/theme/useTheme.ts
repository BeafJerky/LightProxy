import { computed, ref } from 'vue'
import {
  isThemeId,
  paletteFor,
  resolveTheme,
  THEME_META,
  type ResolvedTheme,
  type ThemeId
} from './themes'

const STORAGE_KEY = 'lp.theme'
const query = typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)') : null

const choice = ref<ThemeId>(readChoice())
const systemDark = ref(query?.matches ?? false)
const resolved = ref<ResolvedTheme>(resolveTheme(choice.value, systemDark.value))
const palette = computed(() => paletteFor(resolved.value))

let booted = false

function readChoice(): ThemeId {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    return isThemeId(stored) ? stored : 'system'
  } catch {
    return 'system'
  }
}

function writeDom() {
  if (typeof document === 'undefined') return
  const root = document.documentElement
  resolved.value = resolveTheme(choice.value, systemDark.value)
  root.dataset.theme = resolved.value.theme
  root.dataset.themePack = resolved.value.pack
}

export function applyTheme(next: ThemeId, persist = true) {
  choice.value = isThemeId(next) ? next : 'system'
  if (persist) {
    try {
      localStorage.setItem(STORAGE_KEY, choice.value)
    } catch {
      // 存储不可用时只影响下次启动，不阻断切换
    }
  }
  writeDom()
}

export function initTheme() {
  if (booted) return
  booted = true
  writeDom()
  query?.addEventListener('change', (event) => {
    systemDark.value = event.matches
    if (choice.value === 'system') writeDom()
  })
}

export function useTheme() {
  const meta = computed(() => THEME_META[choice.value])
  return {
    choice,
    resolved,
    palette,
    meta,
    systemDark,
    applyTheme
  }
}
