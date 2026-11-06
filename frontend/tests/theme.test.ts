import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import {
  paletteFor,
  resolveTheme,
  THEME_IDS,
  THEME_META,
  type ResolvedTheme
} from '@/shared/theme/themes'

const cssPath = [
  resolve(process.cwd(), 'src/styles/tokens.css'),
  resolve(process.cwd(), 'frontend/src/styles/tokens.css')
].find((path) => existsSync(path))
if (!cssPath) throw new Error('tokens.css not found')
const css = readFileSync(cssPath, 'utf8')

function block(marker: string): string {
  const index = css.indexOf(marker)
  if (index < 0) throw new Error(`missing theme block: ${marker}`)
  const start = css.lastIndexOf('{', index)
  const end = css.indexOf('}', index)
  return css.slice(start, end)
}

function token(source: string, name: string): string {
  const match = source.match(new RegExp(`--${name}:\\s*([^;]+);`))
  if (!match) throw new Error(`missing token --${name}`)
  return match[1].trim()
}

const CASES: Array<{ resolved: ResolvedTheme; marker: string }> = [
  { resolved: { theme: 'light', pack: 'default' }, marker: '--bg: #eef1f5;' },
  { resolved: { theme: 'dark', pack: 'default' }, marker: '--bg: #111317;' },
  { resolved: { theme: 'light', pack: 'win11' }, marker: '--bg: #f3f3f3;' },
  { resolved: { theme: 'dark', pack: 'win11-dark' }, marker: '--bg: #202020;' },
  { resolved: { theme: 'dark', pack: 'terminal' }, marker: '--bg: #070b0c;' },
  { resolved: { theme: 'light', pack: 'paper' }, marker: '--bg: #f5f0e6;' },
  { resolved: { theme: 'dark', pack: 'contrast' }, marker: '--bg: #000000;' }
]

describe('resolveTheme', () => {
  it('resolves every theme id deterministically', () => {
    expect(resolveTheme('light', true)).toEqual({ theme: 'light', pack: 'default' })
    expect(resolveTheme('dark', false)).toEqual({ theme: 'dark', pack: 'default' })
    expect(resolveTheme('system', true)).toEqual({ theme: 'dark', pack: 'default' })
    expect(resolveTheme('system', false)).toEqual({ theme: 'light', pack: 'default' })
    expect(resolveTheme('win11', true)).toEqual({ theme: 'light', pack: 'win11' })
    expect(resolveTheme('win11-dark', false)).toEqual({ theme: 'dark', pack: 'win11-dark' })
    expect(resolveTheme('terminal', false)).toEqual({ theme: 'dark', pack: 'terminal' })
    expect(resolveTheme('paper', true)).toEqual({ theme: 'light', pack: 'paper' })
    expect(resolveTheme('contrast', false)).toEqual({ theme: 'dark', pack: 'contrast' })
  })

  it('has metadata for all eight ids', () => {
    expect(THEME_IDS).toHaveLength(8)
    for (const id of THEME_IDS) {
      expect(THEME_META[id].id).toBe(id)
      expect(THEME_META[id].label.length).toBeGreaterThan(0)
    }
  })
})

describe('palette matches tokens.css', () => {
  it.each(CASES)('keeps $resolved.pack($resolved.theme) in sync', ({ resolved, marker }) => {
    const source = block(marker)
    const palette = paletteFor(resolved)
    expect(palette.bg).toBe(token(source, 'bg'))
    expect(palette.surface).toBe(token(source, 'surface'))
    expect(palette.surface2).toBe(token(source, 'surface-2'))
    expect(palette.border).toBe(token(source, 'border'))
    expect(palette.controlBorder).toBe(token(source, 'control-border'))
    expect(palette.text).toBe(token(source, 'text'))
    expect(palette.text2).toBe(token(source, 'text-2'))
    expect(palette.muted).toBe(token(source, 'muted'))
    expect(palette.accent).toBe(token(source, 'accent'))
    expect(palette.accentStrong).toBe(token(source, 'accent-strong'))
    expect(palette.onAccent).toBe(token(source, 'on-accent'))
    expect(palette.success).toBe(token(source, 'success'))
    expect(palette.warn).toBe(token(source, 'warn'))
    expect(palette.danger).toBe(token(source, 'danger'))
    expect(palette.radiusLg).toBe(token(source, 'radius-lg'))
    expect(palette.radiusMd).toBe(token(source, 'radius-md'))
  })
})
