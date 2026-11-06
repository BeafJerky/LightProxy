<script setup lang="ts">
import { useMessage } from 'naive-ui'
import { useAppStore } from '@/stores/app'
import { APP_STAGE, APP_VERSION, APP_VERSION_LABEL } from '@/shared/version'
import Icon from '@/shared/ui/Icon.vue'
import SectionCard from '@/shared/ui/SectionCard.vue'

const store = useAppStore()
const message = useMessage()

async function openLogs() {
  const path = await store.openLogDir()
  if (path) message.success(`日志目录：${path}`)
}
</script>

<template>
  <div class="settings-page">
    <div class="page-head">
      <div>
        <h1>关于</h1>
        <p>一个常驻托盘的 Windows 系统代理切换器。</p>
      </div>
    </div>

    <SectionCard>
      <div class="brand">
        <span class="brand-mark"><Icon name="chain" :size="22" /></span>
        <div class="brand-copy">
          <div class="brand-name">
            <strong>LightProxy</strong>
            <span class="version">{{ APP_VERSION_LABEL }}</span>
          </div>
          <p>多套代理快速切换、状态检查与异常修复。只接管 Windows 当前用户的系统代理设置。</p>
        </div>
      </div>
    </SectionCard>

    <SectionCard title="运行信息">
      <div class="info-row"><span>运行平台</span><strong>Windows 10 / 11（x64）</strong></div>
      <div class="info-row"><span>外壳</span><strong>Tauri 2 · Rust</strong></div>
      <div class="info-row"><span>界面</span><strong>Vue 3 · Naive UI</strong></div>
      <div class="info-row"><span>版本</span><strong class="mono">{{ APP_VERSION }}</strong></div>
      <div class="info-row"><span>发布阶段</span><strong>{{ APP_STAGE }}（0.x）</strong></div>
    </SectionCard>

    <SectionCard title="测试版说明">
      <div class="note-row">
        <strong>当前为 0.x 测试版</strong>
        <p>功能、界面与数据格式可能继续调整，暂不承诺向后兼容，也不提供自动更新。</p>
      </div>
      <div class="note-row">
        <strong>反馈问题</strong>
        <p>请附上运行日志或诊断导出，便于定位问题。</p>
      </div>
    </SectionCard>

    <SectionCard title="接管范围">
      <div class="note-row">
        <strong>只写四项设置</strong>
        <p>ProxyEnable、ProxyServer、ProxyOverride、AutoConfigURL。首次启用时保存接管前快照，关闭时恢复。</p>
      </div>
      <div class="note-row">
        <strong>不覆盖未知代理</strong>
        <p>发现外部程序写入的代理时只提示，不会静默改写；需要确认后才接管。</p>
      </div>
      <div class="note-row">
        <strong>PAC 冲突</strong>
        <p>PAC 会优先于固定代理，检测到 AutoConfigURL 时默认拒绝启用，可在面板中确认后强制开启。</p>
      </div>
    </SectionCard>

    <SectionCard title="快捷键">
      <div class="info-row">
        <span>开关代理</span>
        <strong class="mono">{{ store.settings.hotkeys.toggle }}</strong>
      </div>
      <div class="info-row">
        <span>打开面板</span>
        <strong class="mono">{{ store.settings.hotkeys.panel }}</strong>
      </div>
      <div class="info-row">
        <span>下一套配置</span>
        <strong class="mono">{{ store.settings.hotkeys.nextProfile }}</strong>
      </div>
    </SectionCard>

    <SectionCard title="日志">
      <div class="tool-row">
        <div class="setting-copy">
          <strong>运行日志</strong>
          <p>按天滚动保存，只记录启动、代理切换与错误信息。</p>
        </div>
        <button class="mini-btn" type="button" @click="openLogs">
          <Icon name="folder" :size="14" />
          打开目录
        </button>
      </div>
    </SectionCard>
  </div>
</template>

<style scoped>
.brand {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 14px 0;
}

.brand-mark {
  width: 46px;
  height: 46px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: 13px;
  background: var(--accent);
  color: var(--on-accent);
  box-shadow: 0 10px 22px -12px color-mix(in srgb, var(--accent) 80%, transparent);
}

.brand-name {
  display: flex;
  align-items: center;
  gap: 8px;
}

.brand-name strong {
  font-size: 17px;
  font-family: var(--font-display);
}

.version {
  padding: 1px 7px;
  border-radius: 999px;
  background: var(--surface-3);
  color: var(--muted);
  font-family: var(--mono);
  font-size: 10px;
}

.brand-copy p {
  margin: 5px 0 0;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--muted);
}

.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
  font-size: 12px;
}

.info-row:last-child {
  border-bottom: 0;
}

.info-row span {
  color: var(--muted);
}

.info-row strong {
  font-weight: 600;
}

.mono {
  font-family: var(--mono);
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

.tool-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 0 10px;
}
</style>
