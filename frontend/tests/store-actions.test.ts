import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

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

const health = { ok: true, stage: 'tcp', latencyMs: 5, checkedAtMs: 1, error: null }

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
    expect(mocks.checkHealth).toHaveBeenLastCalledWith(false)

    await store.checkHealth(true)
    expect(mocks.checkHealth).toHaveBeenLastCalledWith(true)
  })
})
