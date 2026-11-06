import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const mocks = vi.hoisted(() => ({
  checkUpdate: vi.fn(),
  downloadUpdate: vi.fn(),
  installUpdate: vi.fn(),
  openReleasePage: vi.fn()
}))

vi.mock('@/shared/api', () => ({
  api: mocks,
  toCoreError: (error: unknown) => error
}))

window.matchMedia = vi.fn().mockReturnValue({
  matches: false,
  addEventListener: () => {},
  removeEventListener: () => {}
}) as unknown as typeof window.matchMedia

const { useAppStore } = await import('@/stores/app')

const check = {
  currentVersion: '0.1.3',
  latestVersion: '0.1.4',
  available: true,
  releaseUrl: 'https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.4',
  assetName: 'LightProxy_0.1.4_x64-setup.exe',
  source: '直连',
  checkedAtMs: 1
}

const pkg = {
  version: '0.1.4',
  fileName: 'LightProxy_0.1.4_x64-setup.exe',
  size: 3_400_000,
  sha256: 'abc',
  source: '直连'
}

describe('store update actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.checkUpdate.mockReset()
    mocks.downloadUpdate.mockReset()
    mocks.installUpdate.mockReset()
    mocks.openReleasePage.mockReset()
  })

  it('uses the cached check by default and forces on demand', async () => {
    mocks.checkUpdate.mockResolvedValue(check)
    const store = useAppStore()

    await store.checkUpdate()
    expect(mocks.checkUpdate).toHaveBeenLastCalledWith(false)
    expect(store.updatePhase).toBe('available')

    await store.checkUpdate(true)
    expect(mocks.checkUpdate).toHaveBeenLastCalledWith(true)
  })

  it('moves to ready after downloading a package', async () => {
    mocks.checkUpdate.mockResolvedValue(check)
    let release!: (value: typeof pkg) => void
    mocks.downloadUpdate.mockReturnValue(
      new Promise<typeof pkg>((resolve) => {
        release = resolve
      })
    )
    const store = useAppStore()

    await store.checkUpdate()
    const downloading = store.downloadUpdate()
    store.applyUpdateProgress({
      version: '0.1.4',
      received: 1_700_000,
      total: 3_400_000,
      percent: 50,
      source: '直连'
    })
    expect(store.updateProgress?.percent).toBe(50)

    release(pkg)
    await downloading

    expect(mocks.downloadUpdate).toHaveBeenCalledWith('0.1.4')
    expect(store.updatePhase).toBe('ready')
    expect(store.updatePackage?.fileName).toBe(pkg.fileName)
  })

  it('ignores progress when no update task is running', async () => {
    mocks.checkUpdate.mockResolvedValue(check)
    const store = useAppStore()
    await store.checkUpdate()

    store.applyUpdateProgress({ version: '0.1.4', received: 1, total: 2, percent: 50, source: '直连' })

    expect(store.updateProgress).toBeNull()
  })

  it('ignores progress while a check is in flight', async () => {
    mocks.checkUpdate.mockResolvedValue(check)
    const store = useAppStore()
    await store.checkUpdate()

    let release!: (value: typeof check) => void
    mocks.checkUpdate.mockReturnValue(
      new Promise<typeof check>((resolve) => {
        release = resolve
      })
    )
    const inFlight = store.checkUpdate()
    expect(store.updatePhase).toBe('checking')

    store.applyUpdateProgress({ version: '0.1.4', received: 1, total: 2, percent: 50, source: '直连' })
    expect(store.updateProgress).toBeNull()

    release(check)
    await inFlight
  })

  it('ignores progress that does not belong to the checked version', async () => {
    mocks.checkUpdate.mockResolvedValue(check)
    let release!: (value: typeof pkg) => void
    mocks.downloadUpdate.mockReturnValue(
      new Promise<typeof pkg>((resolve) => {
        release = resolve
      })
    )
    const store = useAppStore()
    await store.checkUpdate()
    const downloading = store.downloadUpdate()

    store.applyUpdateProgress({ version: '0.1.3', received: 1, total: 2, percent: 50, source: '直连' })
    expect(store.updateProgress).toBeNull()

    store.applyUpdateProgress({ version: '0.1.4', received: 1, total: 2, percent: 50, source: '直连' })
    expect(store.updateProgress?.version).toBe('0.1.4')

    release(pkg)
    await downloading
  })

  it('releases the lock after install resolves without exiting', async () => {
    mocks.checkUpdate.mockResolvedValue(check)
    mocks.downloadUpdate.mockResolvedValue(pkg)
    mocks.installUpdate.mockResolvedValue(undefined)
    const store = useAppStore()

    await store.checkUpdate()
    await store.downloadUpdate()
    await store.installUpdate()

    expect(store.updatePhase).toBe('installing')

    await store.checkUpdate()
    expect(mocks.checkUpdate).toHaveBeenCalledTimes(2)
    expect(store.updatePhase).toBe('ready')

    await store.downloadUpdate()
    expect(mocks.downloadUpdate).toHaveBeenCalledTimes(2)
    expect(store.updatePhase).toBe('ready')
  })

  it('surfaces errors and clears them on the next check', async () => {
    const failure = { code: 'update_download_failed', stage: 'update', message: '下载失败', hint: null }
    mocks.checkUpdate.mockResolvedValue(check)
    mocks.downloadUpdate.mockRejectedValue(failure)
    const store = useAppStore()

    await store.checkUpdate()
    await store.downloadUpdate()
    expect(store.updatePhase).toBe('error')
    expect(store.updateError?.code).toBe('update_download_failed')

    mocks.checkUpdate.mockResolvedValue({ ...check, latestVersion: '0.1.4' })
    await store.checkUpdate()
    expect(store.updateError).toBeNull()
  })

  it('keeps the install error local to the update card', async () => {
    mocks.checkUpdate.mockResolvedValue(check)
    mocks.downloadUpdate.mockResolvedValue(pkg)
    mocks.installUpdate.mockRejectedValue({
      code: 'update_install_failed',
      stage: 'update',
      message: '启动失败',
      hint: null
    })
    const store = useAppStore()

    await store.checkUpdate()
    await store.downloadUpdate()
    await store.installUpdate()

    expect(store.updateError?.code).toBe('update_install_failed')
    expect(store.updatePhase).toBe('error')
    expect(store.error).toBeNull()
  })

  it('opens the release page for the checked version', async () => {
    mocks.checkUpdate.mockResolvedValue(check)
    const store = useAppStore()

    await store.checkUpdate()
    await store.openReleasePage()

    expect(mocks.openReleasePage).toHaveBeenCalledWith('0.1.4')
  })
})
