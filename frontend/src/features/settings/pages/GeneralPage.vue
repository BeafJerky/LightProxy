<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NInput } from 'naive-ui'
import { useAppStore } from '@/stores/app'
import SectionCard from '@/shared/ui/SectionCard.vue'
import ToggleRow from '@/shared/ui/ToggleRow.vue'

const store = useAppStore()
const overrideDraft = ref(store.settings.proxyOverride)
const preflightDisabled = computed(() => !store.settings.preflightEnabled)

watch(
  () => store.settings.proxyOverride,
  (value) => {
    overrideDraft.value = value
  }
)

function patch(key: string, value: unknown) {
  void store.updateSettings({ [key]: value })
}

function onSelect(event: Event, key: string) {
  patch(key, (event.target as HTMLSelectElement).value)
}

function commitOverride() {
  const value = overrideDraft.value.trim()
  if (value && value !== store.settings.proxyOverride) patch('proxyOverride', value)
}
</script>

<template>
  <div class="settings-page">
    <div class="page-head">
      <div>
        <h1>通用</h1>
        <p>启动方式、窗口行为与代理检查策略。</p>
      </div>
    </div>

    <SectionCard title="启动">
      <ToggleRow
        :model-value="store.settings.autostart"
        label="开机自启"
        description="登录 Windows 后自动启动 LightProxy 托盘程序。"
        @update:model-value="patch('autostart', $event)"
      />
    </SectionCard>

    <SectionCard title="窗口与托盘">
      <div class="setting-row">
        <span class="setting-copy">
          <strong>关闭设置窗口时</strong>
          <p>隐藏后仍可继续用快捷键和托盘操作。</p>
        </span>
        <span class="inline-control">
          <select class="select" :value="store.settings.closeAction" @change="onSelect($event, 'closeAction')">
            <option value="hide">隐藏到托盘</option>
            <option value="exit">退出应用</option>
          </select>
        </span>
      </div>
      <div class="setting-row">
        <span class="setting-copy">
          <strong>单击托盘图标</strong>
          <p>双击始终打开快捷面板。</p>
        </span>
        <span class="inline-control">
          <select
            class="select"
            :value="store.settings.trayClickAction"
            @change="onSelect($event, 'trayClickAction')"
          >
            <option value="panel">打开面板</option>
            <option value="toggle">切换代理</option>
          </select>
        </span>
      </div>
    </SectionCard>

    <SectionCard title="代理行为">
      <ToggleRow
        :model-value="store.settings.preflightEnabled"
        label="启用前检查端口"
        description="用 TCP 连接确认代理端口可达；失败时不写入系统代理。"
        @update:model-value="patch('preflightEnabled', $event)"
      />
      <div class="setting-row">
        <span class="setting-copy">
          <strong>检查超时</strong>
          <p>内网代理建议保持 800ms，避免局域网抖动导致误判。</p>
        </span>
        <span class="inline-control">
          <select
            class="select"
            :value="String(store.settings.preflightTimeoutMs)"
            :disabled="preflightDisabled"
            @change="patch('preflightTimeoutMs', Number(($event.target as HTMLSelectElement).value))"
          >
            <option value="300">300ms</option>
            <option value="500">500ms</option>
            <option value="800">800ms</option>
            <option value="1200">1.2s</option>
            <option value="2000">2s</option>
            <option value="3000">3s</option>
          </select>
        </span>
      </div>
      <ToggleRow
        :model-value="store.settings.keepProxyOnExit"
        label="退出时保持代理开启"
        description="默认会在退出时恢复接管前的系统代理；开启后则保持当前代理。"
        @update:model-value="patch('keepProxyOnExit', $event)"
      />
    </SectionCard>

    <SectionCard title="绕过代理" hint="ProxyOverride">
      <div class="override-row">
        <n-input
          v-model:value="overrideDraft"
          type="textarea"
          size="small"
          :autosize="{ minRows: 3, maxRows: 6 }"
          spellcheck="false"
          @blur="commitOverride"
        />
        <p class="hint">以分号分隔。保存后会立即同步写入系统代理，内网地址默认全部直连。</p>
      </div>
    </SectionCard>
  </div>
</template>

<style scoped>
.override-row {
  padding: 4px 0 12px;
}

.override-row :deep(textarea) {
  font-family: var(--mono);
  font-size: 11px;
  line-height: 1.55;
}
</style>
