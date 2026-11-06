<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { applyTheme, useTheme } from '@/shared/theme/useTheme'
import { isThemeId, resolveTheme, THEME_META, type ThemeId } from '@/shared/theme/themes'

interface ThemeGroup {
  title: string
  hint: string
  ids: ThemeId[]
  two?: boolean
}

const groups: ThemeGroup[] = [
  {
    title: '基础',
    hint: '跟随系统只解析默认浅 / 深',
    ids: ['light', 'dark', 'system']
  },
  {
    title: '系统风格',
    hint: 'Fluent 云母 · 原生控件',
    ids: ['win11', 'win11-dark'],
    two: true
  },
  {
    title: '个性风格',
    hint: '完整预设 · 不改布局',
    ids: ['terminal', 'paper', 'contrast']
  }
]

const store = useAppStore()
const { systemDark } = useTheme()
const current = computed<ThemeId>(() =>
  isThemeId(store.settings.theme) ? store.settings.theme : 'system'
)
const currentMeta = computed(() => THEME_META[current.value])
const resolved = computed(() => resolveTheme(current.value, systemDark.value))
const schemeLabel = computed(() => (resolved.value.theme === 'dark' ? '深色' : '浅色'))
const summaryDesc = computed(() =>
  current.value === 'system'
    ? `当前系统为${schemeLabel.value} · 只解析默认浅 / 深`
    : currentMeta.value.description
)

function tabIndexFor(group: ThemeGroup, id: ThemeId, index: number) {
  const active = group.ids.indexOf(current.value)
  return (active === -1 ? index === 0 : group.ids[active] === id) ? 0 : -1
}

async function choose(id: ThemeId) {
  const previous = current.value
  applyTheme(id)
  const ok = await store.updateSettings({ theme: id })
  if (!ok) applyTheme(previous)
}

function onCardKey(event: KeyboardEvent, group: ThemeGroup, index: number) {
  const keys = ['ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp', 'Home', 'End']
  if (!keys.includes(event.key)) return
  event.preventDefault()
  let next = index
  if (event.key === 'Home') next = 0
  else if (event.key === 'End') next = group.ids.length - 1
  else if (event.key === 'ArrowRight' || event.key === 'ArrowDown')
    next = (index + 1) % group.ids.length
  else next = (index - 1 + group.ids.length) % group.ids.length
  void choose(group.ids[next])
  const grid = (event.currentTarget as HTMLElement).parentElement
  grid?.querySelectorAll<HTMLButtonElement>('[role="radio"]')[next]?.focus()
}
</script>

<template>
  <div class="settings-page">
    <div class="page-head">
      <div>
        <h1>外观</h1>
        <p>8 种固定外观，单一作用域；面板、菜单与窗口同步。正文 ≥ 4.5:1，高对比 ≥ 7:1。</p>
      </div>
    </div>

    <div class="theme-current" role="status" aria-live="polite">
      <span class="theme-dot" :data-theme-dot="current" aria-hidden="true" />
      <div>
        <strong>{{ currentMeta.label }}</strong>
        <span class="desc">{{ summaryDesc }}</span>
      </div>
      <span class="theme-badge">{{ schemeLabel }}</span>
    </div>

    <div v-for="group in groups" :key="group.title" class="theme-group">
      <div class="theme-group-head">
        <strong>{{ group.title }}</strong>
        <span>{{ group.hint }}</span>
      </div>
      <div
        class="theme-grid"
        :class="{ 'theme-grid--two': group.two }"
        role="radiogroup"
        :aria-label="`${group.title}外观`"
      >
        <button
          v-for="(id, index) in group.ids"
          :key="id"
          type="button"
          class="theme-card"
          :class="{ 'is-active': current === id }"
          role="radio"
          :aria-checked="current === id"
          :tabindex="tabIndexFor(group, id, index)"
          @click="choose(id)"
          @keydown="onCardKey($event, group, index)"
        >
          <span
            v-if="id === 'system'"
            class="theme-preview theme-preview--split"
            aria-hidden="true"
          >
            <span class="tp-half" data-theme="light" data-theme-pack="default">
              <span class="tp-head"><i class="tp-mark" /><i class="tp-name" /></span>
              <span class="tp-card">
                <span class="tp-lines"><i /><i /></span>
              </span>
            </span>
            <span class="tp-half" data-theme="dark" data-theme-pack="default">
              <span class="tp-head"><i class="tp-mark" /><i class="tp-name" /></span>
              <span class="tp-card">
                <span class="tp-lines"><i /><i /></span>
              </span>
            </span>
          </span>
          <span
            v-else
            class="theme-preview"
            :data-theme="THEME_META[id].theme"
            :data-theme-pack="THEME_META[id].pack"
            aria-hidden="true"
          >
            <span class="tp-head"><i class="tp-mark" /><i class="tp-name" /><i class="tp-dot" /></span>
            <span class="tp-card">
              <span class="tp-lines"><i /><i /></span>
              <i class="tp-toggle" />
            </span>
          </span>
          <strong>{{ THEME_META[id].label }}</strong>
          <p>{{ THEME_META[id].description }}</p>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.theme-current {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 13px;
  padding: 8px 11px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-2);
}

.theme-current > div {
  flex: 1;
  min-width: 0;
}

.theme-current strong {
  display: block;
  font-size: 12px;
}

.theme-current .desc {
  display: block;
  margin-top: 1px;
  font-size: 10.5px;
  color: var(--muted);
}

.theme-badge {
  flex: none;
  padding: 2px 7px;
  border-radius: 999px;
  background: var(--surface-3);
  color: var(--muted);
  font-size: 10px;
}

.theme-dot {
  --sw-bg: #fff;
  --sw-accent: #2563eb;
  --sw-dot: #0b7a42;
  position: relative;
  width: 22px;
  height: 22px;
  flex: none;
  border: 1px solid var(--border-strong);
  border-radius: 50%;
  background: linear-gradient(135deg, var(--sw-bg) 0 58%, var(--sw-accent) 58%);
}

.theme-dot::after {
  content: "";
  position: absolute;
  right: 1px;
  bottom: 1px;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  border: 1px solid var(--sw-bg);
  background: var(--sw-dot);
}

.theme-dot[data-theme-dot='dark'] {
  --sw-bg: #1c2026;
  --sw-accent: #5b8cff;
  --sw-dot: #40c57a;
}

.theme-dot[data-theme-dot='system'] {
  --sw-bg: #fff;
  --sw-accent: #2563eb;
  --sw-dot: #0b7a42;
  background: linear-gradient(90deg, #fff 0 47%, #2563eb 47% 53%, #1c2026 53%);
}

.theme-dot[data-theme-dot='win11'] {
  --sw-bg: #fbfbfb;
  --sw-accent: #005fb8;
  --sw-dot: #0f7b3f;
}

.theme-dot[data-theme-dot='win11-dark'] {
  --sw-bg: #2b2b2b;
  --sw-accent: #60cdff;
  --sw-dot: #4cc38a;
}

.theme-dot[data-theme-dot='terminal'] {
  --sw-bg: #0f1618;
  --sw-accent: #22d3ee;
  --sw-dot: #4ade80;
}

.theme-dot[data-theme-dot='paper'] {
  --sw-bg: #fffdf8;
  --sw-accent: #a94b24;
  --sw-dot: #2f6b45;
}

.theme-dot[data-theme-dot='contrast'] {
  --sw-bg: #000;
  --sw-accent: #ffd400;
  --sw-dot: #5cff9d;
}

.theme-group {
  margin-bottom: 10px;
}

.theme-group:last-child {
  margin-bottom: 0;
}

.theme-group-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
  margin: 2px 0 6px;
}

.theme-group-head strong {
  font-size: 11.5px;
  letter-spacing: 0.01em;
}

.theme-group-head span {
  font-size: 10.5px;
  color: var(--muted);
  text-align: right;
}

.theme-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 9px;
}

.theme-grid--two {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.theme-card {
  display: block;
  width: 100%;
  padding: 8px;
  border: 1.5px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface);
  color: var(--text);
  text-align: left;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.theme-card:hover {
  border-color: var(--border-strong);
}

.theme-card.is-active {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
}

.theme-card strong {
  display: block;
  margin-top: 6px;
  font-size: 11.5px;
}

.theme-card p {
  margin: 2px 0 0;
  font-size: 10px;
  line-height: 1.3;
  color: var(--muted);
}

.theme-preview {
  display: block;
  height: 54px;
  padding: 6px;
  border: 1px solid var(--border);
  border-radius: max(3px, calc(var(--radius-sm) - 2px));
  background: var(--bg);
  color: var(--text);
  font-family: var(--font);
  overflow: hidden;
}

.tp-head {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 11px;
}

.tp-mark {
  position: relative;
  width: 11px;
  height: 11px;
  flex: none;
  border-radius: 3px;
  background: var(--accent);
}

.tp-mark::after {
  content: "";
  position: absolute;
  inset: 3.5px;
  border-radius: 50%;
  background: var(--on-accent);
}

.tp-name {
  width: 36px;
  height: 5px;
  border-radius: 2px;
  background: var(--text);
  opacity: 0.85;
}

.tp-dot {
  width: 5px;
  height: 5px;
  margin-left: auto;
  border-radius: 50%;
  background: var(--success);
}

.tp-card {
  display: flex;
  align-items: center;
  gap: 5px;
  height: 24px;
  margin-top: 4px;
  padding: 0 6px;
  border: 1px solid var(--border);
  border-radius: calc(var(--radius-sm) * 0.8);
  background: var(--surface);
  background-image: var(--surface-glow);
}

.tp-lines {
  flex: 1;
  min-width: 0;
  display: grid;
  gap: 3px;
}

.tp-lines i:first-child {
  width: 68%;
  height: 4px;
  border-radius: 1px;
  background: var(--text);
  opacity: 0.8;
}

.tp-lines i:last-child {
  width: 44%;
  height: 3px;
  border-radius: 1px;
  background: var(--muted);
}

.tp-toggle {
  position: relative;
  width: 17px;
  height: 9px;
  flex: none;
  border-radius: 99px;
  background: var(--accent);
}

.tp-toggle::after {
  content: "";
  position: absolute;
  top: 2px;
  right: 2px;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--on-accent);
}

.theme-preview--split {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0;
  padding: 0;
}

.theme-preview--split .tp-half {
  min-width: 0;
  padding: 6px;
  background: var(--bg);
}

.theme-preview--split .tp-half + .tp-half {
  border-left: 1px solid var(--border);
}

.theme-preview--split .tp-name {
  width: 60%;
}
</style>
