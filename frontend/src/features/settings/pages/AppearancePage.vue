<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { applyTheme } from '@/shared/theme/useTheme'
import { isThemeId, THEME_META, THEME_ORDER, type ThemeId } from '@/shared/theme/themes'
import SectionCard from '@/shared/ui/SectionCard.vue'
import Icon from '@/shared/ui/Icon.vue'

const store = useAppStore()
const current = computed<ThemeId>(() =>
  isThemeId(store.settings.theme) ? store.settings.theme : 'system'
)

async function choose(id: ThemeId) {
  const previous = current.value
  applyTheme(id)
  const ok = await store.updateSettings({ theme: id })
  if (!ok) applyTheme(previous)
}
</script>

<template>
  <div class="settings-page">
    <div class="page-head">
      <div>
        <h1>外观</h1>
        <p>8 种固定主题，面板、设置窗口与托盘菜单保持同一套外观。</p>
      </div>
    </div>

    <SectionCard title="主题" hint="即时生效">
      <div class="theme-grid">
        <button
          v-for="id in THEME_ORDER"
          :key="id"
          type="button"
          class="theme-card"
          :class="{ 'is-active': current === id }"
          role="radio"
          :aria-checked="current === id"
          @click="choose(id)"
        >
          <span v-if="THEME_META[id].theme !== 'auto'" class="theme-swatch" aria-hidden="true">
            <span
              class="swatch-surface"
              :data-theme="THEME_META[id].theme"
              :data-theme-pack="THEME_META[id].pack"
            >
              <i class="swatch-dot" />
              <i class="swatch-line" />
              <i class="swatch-bar" />
            </span>
          </span>
          <span v-else class="theme-swatch theme-swatch--split" aria-hidden="true">
            <span class="swatch-half">
              <span class="swatch-surface" data-theme="light" data-theme-pack="default">
                <i class="swatch-dot" />
                <i class="swatch-line" />
                <i class="swatch-bar" />
              </span>
            </span>
            <span class="swatch-half">
              <span class="swatch-surface" data-theme="dark" data-theme-pack="default">
                <i class="swatch-dot" />
                <i class="swatch-line" />
                <i class="swatch-bar" />
              </span>
            </span>
          </span>

          <span class="theme-copy">
            <strong>{{ THEME_META[id].label }}</strong>
            <small>{{ THEME_META[id].description }}</small>
          </span>
          <span v-if="current === id" class="theme-check"><Icon name="check" :size="12" /></span>
        </button>
      </div>
    </SectionCard>

    <SectionCard title="说明">
      <div class="note-row">
        <strong>跟随系统</strong>
        <p>只在系统深浅模式变化时切换默认浅色 / 深色，不解析其他风格包。</p>
      </div>
      <div class="note-row">
        <strong>高对比</strong>
        <p>纯黑背景、白色边框与高亮黄强调，适合弱视或强光环境。</p>
      </div>
      <div class="note-row">
        <strong>切换方式</strong>
        <p>主题在首帧前写入根节点，切换时不会重建窗口，也不会闪烁。</p>
      </div>
    </SectionCard>
  </div>
</template>

<style scoped>
.theme-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
  padding: 4px 0 12px;
}

.theme-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  padding: 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-2);
  text-align: left;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.theme-card:hover {
  border-color: var(--border-strong);
}

.theme-card.is-active {
  border-color: var(--accent);
  background: var(--accent-soft);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 14%, transparent);
}

.theme-swatch {
  flex: none;
  width: 62px;
  height: 42px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: var(--surface);
}

.theme-swatch--split {
  display: flex;
}

.swatch-half {
  width: 50%;
  height: 100%;
}

.swatch-surface {
  display: block;
  position: relative;
  width: 100%;
  height: 100%;
  padding: 7px 6px;
  background: var(--bg);
}

.swatch-dot {
  display: block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--accent);
}

.swatch-line {
  display: block;
  width: 60%;
  height: 2px;
  margin-top: 4px;
  border-radius: 2px;
  background: color-mix(in srgb, var(--text) 55%, transparent);
}

.swatch-bar {
  display: block;
  position: absolute;
  left: 6px;
  right: 6px;
  bottom: 6px;
  height: 9px;
  border-radius: 3px;
  background: var(--surface);
  border: 1px solid var(--border);
}

.theme-copy {
  min-width: 0;
  flex: 1;
}

.theme-copy strong {
  display: block;
  font-size: 12.5px;
}

.theme-copy small {
  display: block;
  margin-top: 2px;
  font-size: 10.5px;
  line-height: 1.4;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
}

.theme-check {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 18px;
  height: 18px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  background: var(--accent);
  color: var(--on-accent);
}

.note-row {
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}

.note-row:last-child {
  border-bottom: 0;
}

.note-row strong {
  display: block;
  font-size: 12.5px;
}

.note-row p {
  margin: 3px 0 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--muted);
}
</style>
