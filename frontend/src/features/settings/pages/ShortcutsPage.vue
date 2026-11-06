<script setup lang="ts">
import { useMessage } from 'naive-ui'
import { useAppStore } from '@/stores/app'
import { DEFAULT_SETTINGS } from '@/shared/api/defaults'
import SectionCard from '@/shared/ui/SectionCard.vue'
import HotkeyInput from '@/shared/ui/HotkeyInput.vue'

const store = useAppStore()
const message = useMessage()

async function setHotkey(key: 'toggle' | 'panel' | 'nextProfile', value: string) {
  if (!value || value === store.settings.hotkeys[key]) return
  const ok = await store.updateHotkeys({ [key]: value })
  if (ok) message.success('快捷键已更新')
}

async function resetAll() {
  const ok = await store.updateHotkeys({ ...DEFAULT_SETTINGS.hotkeys })
  if (ok) message.success('已恢复默认快捷键')
}
</script>

<template>
  <div class="settings-page">
    <div class="page-head">
      <div>
        <h1>快捷键</h1>
        <p>快捷键全局生效，窗口隐藏时也能操作。</p>
      </div>
      <div class="page-head-actions">
        <button class="mini-btn" type="button" @click="resetAll">恢复默认</button>
      </div>
    </div>

    <SectionCard title="全局快捷键" hint="点击后按下组合键">
      <div class="setting-row">
        <span class="setting-copy">
          <strong>开关代理</strong>
          <p>有使用中配置时直接关闭，否则启用默认配置。</p>
        </span>
        <HotkeyInput
          :model-value="store.settings.hotkeys.toggle"
          @update:model-value="setHotkey('toggle', $event)"
        />
      </div>
      <div class="setting-row">
        <span class="setting-copy">
          <strong>打开快捷面板</strong>
          <p>在屏幕右下角打开面板并聚焦。</p>
        </span>
        <HotkeyInput
          :model-value="store.settings.hotkeys.panel"
          @update:model-value="setHotkey('panel', $event)"
        />
      </div>
      <div class="setting-row">
        <span class="setting-copy">
          <strong>切换下一套配置</strong>
          <p>按配置列表顺序循环切换，开启状态下直接换地址。</p>
        </span>
        <HotkeyInput
          :model-value="store.settings.hotkeys.nextProfile"
          @update:model-value="setHotkey('nextProfile', $event)"
        />
      </div>
    </SectionCard>

    <SectionCard title="使用提示">
      <div class="tip-row">
        <strong>组合键至少包含一个修饰键</strong>
        <p>支持 Ctrl、Alt、Shift、Win 与字母、数字、F1–F12、方向键等。</p>
      </div>
      <div class="tip-row">
        <strong>冲突处理</strong>
        <p>若组合被其他程序占用，注册会失败；在诊断页可以看到具体冲突项。</p>
      </div>
      <div class="tip-row">
        <strong>保存方式</strong>
        <p>录制完成即保存并重新注册，不需要额外确认。</p>
      </div>
    </SectionCard>
  </div>
</template>

<style scoped>
.tip-row {
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}

.tip-row:last-child {
  border-bottom: 0;
}

.tip-row strong {
  display: block;
  font-size: 12.5px;
}

.tip-row p {
  margin: 3px 0 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--muted);
}
</style>
