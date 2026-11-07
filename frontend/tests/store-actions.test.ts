import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { DEFAULT_SETTINGS } from '@/shared/api/defaults'

const mocks = vi.hoisted(() => ({
  switchProfile: vi.fn(),
  checkHealth: vi.fn()
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

const health = {
  tcp: { ok: true, stage: 'tcp', latencyMs: 5, checkedAtMs: 1, error: null },
  proxy: null
}

describe('store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.switchProfile.mockReset()
    mocks.checkHealth.mockReset()
  })

  it('passes the force flag when switching profiles', async () => {
    mocks.switchProfile.mockResolvedValue({ state: 'on' })
    const store = useAppStore()

    const ok = await store.switchProfile('p_backup', true)

    expect(ok).toBe(true)
    expect(mocks.switchProfile).toHaveBeenCalledWith('p_backup', true)
  })

  it('uses the cached health result by default and forces on demand', async () => {
    mocks.checkHealth.mockResolvedValue(health)
    const store = useAppStore()

    await store.checkHealth()
    expect(mocks.checkHealth).toHaveBeenLastCalledWith(false, 'tcp')

    await store.checkHealth(true)
    expect(mocks.checkHealth).toHaveBeenLastCalledWith(true, 'tcp')
  })

  it('passes deep mode and keeps only the tcp result in state', async () => {
    const tcp = { ok: true, stage: 'tcp', latencyMs: 9, checkedAtMs: 2, error: null }
    const report = {
      tcp,
      proxy: { ok: true, stage: 'proxy', latencyMs: 40, checkedAtMs: 3, error: null }
    }
    mocks.checkHealth.mockResolvedValue(report)
    const store = useAppStore()
    store.state = {
      state: 'on',
      desired: 'on',
      activeProfileId: 'p1',
      lastHealth: null,
      message: null,
      profiles: [],
      defaultProfileId: null,
      settings: structuredClone(DEFAULT_SETTINGS),
      snapshot: null
    }

    const returned = await store.checkHealth(true, 'deep')

    expect(mocks.checkHealth).toHaveBeenLastCalledWith(true, 'deep')
    expect(returned).toEqual(report)
    expect(store.state?.lastHealth).toEqual(tcp)
  })
})
