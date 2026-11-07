import { afterEach, describe, expect, it } from 'vitest'
import { mockCall, setMockProgressHandler } from '@/shared/api/mock'
import type { CoreError, UpdateCheck, UpdatePackage, UpdateProgress } from '@/shared/api/types'
import { APP_VERSION } from '@/shared/version'

function nextPatch(version: string): string {
  const [major, minor, patch] = version.split('.').map(Number)
  return `${major}.${minor}.${patch + 1}`
}

const LATEST = nextPatch(APP_VERSION)

// Node 26 的全局 localStorage getter 在未传 --localstorage-file 时返回 undefined，
// 会盖住 jsdom 的实现，因此测试内补一个内存实现。
function ensureLocalStorage() {
  if (localStorage) return
  const store = new Map<string, string>()
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => {
        store.set(key, String(value))
      },
      removeItem: (key: string) => {
        store.delete(key)
      },
      clear: () => store.clear(),
      key: (index: number) => [...store.keys()][index] ?? null,
      get length() {
        return store.size
      }
    } satisfies Storage
  })
}

ensureLocalStorage()

afterEach(() => {
  localStorage.removeItem('lp_mock_update_fail')
  setMockProgressHandler(null)
})

describe('mock update commands', () => {
  it('reports a release newer than the running app', () => {
    const check = mockCall('check_update') as UpdateCheck
    expect(check.currentVersion).toBe(APP_VERSION)
    expect(check.available).toBe(true)
    expect(check.latestVersion).toBe(LATEST)
    expect(check.releaseUrl).toContain(`/v${LATEST}`)
    expect(check.assetName).toBe(`LightProxy_${LATEST}_x64-setup.exe`)
  })

  it('streams progress and returns a ready package', async () => {
    const events: UpdateProgress[] = []
    setMockProgressHandler((event) => events.push(event))
    const pkg = (await mockCall('download_update', { version: LATEST })) as UpdatePackage
    expect(events.length).toBeGreaterThan(1)
    expect(events.at(-1)?.received).toBe(pkg.size)
    expect(events.at(-1)?.percent).toBe(100)
    expect(pkg.fileName).toBe(`LightProxy_${LATEST}_x64-setup.exe`)
  })

  it('can simulate a check failure for screenshots', () => {
    localStorage.setItem('lp_mock_update_fail', '1')
    let code = ''
    try {
      mockCall('check_update')
    } catch (error) {
      code = (error as CoreError).code
    }
    expect(code).toBe('update_check_failed')
  })
})
