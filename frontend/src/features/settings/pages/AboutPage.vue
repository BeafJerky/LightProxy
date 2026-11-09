<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import { useAppStore } from '@/stores/app'
import { onUpdateProgress } from '@/shared/api'
import { APP_STAGE, APP_VERSION, APP_VERSION_LABEL } from '@/shared/version'
import Icon from '@/shared/ui/Icon.vue'
import SectionCard from '@/shared/ui/SectionCard.vue'

const store = useAppStore()
const message = useMessage()
const dialog = useDialog()

let unlisten: (() => void) | null = null
let disposed = false

onMounted(async () => {
  try {
    const off = await onUpdateProgress((progress) => store.applyUpdateProgress(progress))
    if (disposed) {
      off()
      return
    }
    unlisten = off
  } catch {
    // 订阅失败不该连带跳过自动检查：页面仍可手动检查更新
  } finally {
    // 卸载后不再触发检查；成功路径提前 return 时同样被 disposed 挡住
    if (!disposed) void store.checkUpdate(false)
  }
})

onUnmounted(() => {
  disposed = true
  unlisten?.()
  unlisten = null
})

async function openLogs() {
  const path = await store.openLogDir()
  if (path) message.success(`日志目录：${path}`)
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

const updateStatusText = computed(() => {
  switch (store.updatePhase) {
    case 'checking':
      return '正在检查更新…'
    case 'latest':
      return '已是最新版本'
    case 'available':
      return `发现新版本 v${store.updateCheck?.latestVersion}`
    case 'downloading':
      return `正在下载 v${store.updateCheck?.latestVersion}`
    case 'ready':
      return '安装包已就绪'
    case 'installing':
      return '正在启动安装程序…'
    case 'error':
      return '更新遇到问题'
    default:
      return '点击「检查更新」查询新版本'
  }
})

const updateHint = computed(() => {
  switch (store.updatePhase) {
    case 'idle':
      return `当前版本 v${APP_VERSION} · ${APP_STAGE}`
    case 'latest':
      return `当前版本 v${APP_VERSION} 已是最新`
    case 'available':
      return `当前 v${APP_VERSION} → v${store.updateCheck?.latestVersion}`
    case 'ready':
      return '安装包已下载并通过 SHA256 与文件头校验'
    default:
      return ''
  }
})

const showDownload = computed(
  () =>
    Boolean(store.updateCheck?.available) &&
    !store.updatePackage &&
    store.updatePhase !== 'downloading' &&
    store.updatePhase !== 'checking' &&
    store.updatePhase !== 'installing'
)

const progressText = computed(() => {
  const progress = store.updateProgress
  if (!progress) return '准备下载…'
  if (progress.percent === null || !progress.total) {
    return `已下载 ${formatSize(progress.received)}`
  }
  return `${progress.percent}% · ${formatSize(progress.received)} / ${formatSize(progress.total)}`
})

function confirmInstall() {
  dialog.warning({
    title: '安装更新',
    content: '应用将退出并启动安装程序，代理会按退出设置恢复。',
    positiveText: '立即安装',
    negativeText: '取消',
    onPositiveClick: () => {
      void store.installUpdate()
    }
  })
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
      <div class="repo-links">
        <button class="mini-btn repo-link" type="button" @click="store.openRepoPage()">
          <Icon name="github" :size="15" />
          项目仓库
        </button>
        <button class="mini-btn repo-link" type="button" @click="store.openReleasesPage()">
          <Icon name="github" :size="15" />
          发布说明
        </button>
      </div>
    </SectionCard>

    <SectionCard title="运行信息">
      <div class="info-row"><span>运行平台</span><strong>Windows 10 / 11（x64）</strong></div>
      <div class="info-row"><span>外壳</span><strong>Tauri 2 · Rust</strong></div>
      <div class="info-row"><span>界面</span><strong>Vue 3 · Naive UI</strong></div>
      <div class="info-row"><span>版本</span><strong class="mono">{{ APP_VERSION }}</strong></div>
      <div class="info-row"><span>发布阶段</span><strong>{{ APP_STAGE }}（0.x）</strong></div>
    </SectionCard>

    <SectionCard title="更新">
      <div class="update-body">
        <div class="update-status">
          <div class="update-copy">
            <strong>{{ updateStatusText }}</strong>
            <p v-if="store.updateError" class="update-error" role="alert">
              {{ store.updateError.message }}
              <span v-if="store.updateError.hint">· {{ store.updateError.hint }}</span>
            </p>
            <p v-else-if="updateHint" class="update-hint">{{ updateHint }}</p>
          </div>
          <div class="update-actions">
            <button
              class="mini-btn"
              type="button"
              :disabled="
                store.updatePhase === 'checking' ||
                store.updatePhase === 'downloading' ||
                store.updatePhase === 'installing'
              "
              @click="store.checkUpdate(true)"
            >
              <Icon name="refresh" :size="14" />
              {{ store.updatePhase === 'checking' ? '正在检查…' : '检查更新' }}
            </button>
            <button
              v-if="showDownload"
              class="mini-btn primary"
              type="button"
              @click="store.downloadUpdate()"
            >
              {{ store.updatePhase === 'error' ? '重试下载' : '下载更新' }}
            </button>
            <button
              v-if="store.updatePhase === 'ready'"
              class="mini-btn primary"
              type="button"
              @click="confirmInstall"
            >
              立即安装
            </button>
          </div>
        </div>

        <div v-if="store.updatePhase === 'downloading'" class="update-progress">
          <div
            class="bar"
            role="progressbar"
            aria-label="更新下载进度"
            aria-valuemin="0"
            aria-valuemax="100"
            :aria-valuenow="store.updateProgress?.percent ?? undefined"
            :aria-valuetext="progressText"
          >
            <span :style="{ width: `${store.updateProgress?.percent ?? 0}%` }" />
          </div>
          <div class="update-meta">
            <span>{{ progressText }}</span>
            <span v-if="store.updateProgress">来源：{{ store.updateProgress.source }}</span>
          </div>
        </div>

        <div v-else-if="store.updatePhase === 'ready' && store.updatePackage" class="update-meta ready">
          <span>{{ formatSize(store.updatePackage.size) }}</span>
          <span>来源：{{ store.updatePackage.source }}</span>
        </div>

        <button
          v-if="store.updateCheck?.releaseUrl"
          class="link-btn"
          type="button"
          @click="store.openReleasePage()"
        >
          查看发布说明
        </button>
        <p class="update-foot">更新需手动确认，不会静默安装；安装时应用将退出。</p>
      </div>
    </SectionCard>

    <SectionCard title="测试版说明">
      <div class="note-row">
        <strong>当前为 0.x 测试版</strong>
        <p>功能、界面与数据格式可能继续调整，暂不承诺向后兼容；更新需手动检查并确认，不会静默安装。</p>
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

.repo-links {
  display: flex;
  gap: 8px;
  margin-top: -2px;
  padding: 12px 0 6px;
  border-top: 1px solid var(--border);
}

.repo-link {
  height: 30px;
  padding: 0 12px;
  gap: 7px;
  font-size: 12px;
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

.update-body {
  padding: 4px 0 10px;
}

.update-status {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
  padding: 8px 0 2px;
}

.update-copy {
  min-width: 0;
}

.update-copy strong {
  display: block;
  font-size: 12.5px;
}

.update-hint,
.update-foot {
  margin: 4px 0 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--muted);
}

.update-error {
  margin: 4px 0 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--danger);
}

.update-actions {
  display: flex;
  flex: none;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}

.update-progress {
  padding: 8px 0 2px;
}

.bar {
  height: 6px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--surface-3);
}

.bar span {
  display: block;
  height: 100%;
  border-radius: 999px;
  background: var(--accent);
  transition: width 0.2s ease;
}

.update-meta {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  margin-top: 6px;
  font-family: var(--mono);
  font-size: 10.5px;
  color: var(--muted);
}

.update-meta.ready {
  margin-top: 4px;
}

.link-btn {
  margin-top: 6px;
  padding: 0;
  border: 0;
  background: none;
  color: var(--accent);
  font-size: 11.5px;
  cursor: pointer;
}

.link-btn:hover {
  text-decoration: underline;
}

.update-foot {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}
</style>
