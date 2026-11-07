import { describe, expect, it } from 'vitest'
import { healthToast } from '@/features/panel/health-toast'
import type { HealthResult } from '@/shared/api/types'

const tcp: HealthResult = {
  ok: true,
  stage: 'tcp',
  latencyMs: 12,
  checkedAtMs: 1,
  error: null
}

function proxy(overrides: Partial<HealthResult> = {}): HealthResult {
  return {
    ok: true,
    stage: 'proxy',
    latencyMs: 35,
    checkedAtMs: 2,
    error: null,
    ...overrides
  }
}

describe('healthToast', () => {
  it('reports both layers when the deep check passes', () => {
    expect(healthToast({ tcp, proxy: proxy() })).toEqual({
      type: 'success',
      text: '连接正常 · TCP 12ms · 协议 35ms'
    })
  })

  it('explains that the port is open but forwarding failed', () => {
    expect(
      healthToast({
        tcp,
        proxy: proxy({ ok: false, error: '代理要求认证，当前版本不支持' })
      })
    ).toEqual({
      type: 'error',
      text: '端口可达，但代理无法转发：代理要求认证，当前版本不支持'
    })
  })

  it('keeps the tcp error when the port is unreachable', () => {
    expect(
      healthToast({
        tcp: { ...tcp, ok: false, error: '连接超时' },
        proxy: null
      })
    ).toEqual({ type: 'error', text: '连接超时' })
  })

  it('falls back to the tcp latency when no proxy stage was run', () => {
    expect(healthToast({ tcp, proxy: null })).toEqual({
      type: 'success',
      text: '连接正常 · TCP 12ms'
    })
  })
})
