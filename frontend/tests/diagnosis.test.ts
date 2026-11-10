import { describe, expect, it } from 'vitest'
import {
  diagnoseView,
  protocolLabel,
  recommendationProtocol,
  stageLabel
} from '@/features/profiles/diagnosis'
import type { DiagnoseReport } from '@/shared/api/types'

function report(overrides: Partial<DiagnoseReport> = {}): DiagnoseReport {
  return {
    http: { ok: true, stage: 'http', latencyMs: 120, error: null },
    socks5Domain: { ok: true, stage: 'http', latencyMs: 160, error: null },
    systemSocks: { ok: false, stage: 'tls', latencyMs: 3000, error: 'TLS 握手失败：连接已关闭' },
    recommendation: 'http',
    summary: 'HTTP 通道可用，建议使用 HTTP。',
    ...overrides
  }
}

describe('diagnoseView', () => {
  it('renders the three transports in App order', () => {
    const view = diagnoseView(report(), 'socks5')

    expect(view.rows.map((row) => row.key)).toEqual(['http', 'socks5Domain', 'systemSocks'])
    expect(view.rows[2].result.ok).toBe(false)
    expect(view.canApply).toBe(true)
    expect(view.alreadyApplied).toBe(false)
  })

  it('marks the current protocol as already applied', () => {
    expect(diagnoseView(report(), 'http').alreadyApplied).toBe(true)
  })

  it('explains the environment-only recommendation', () => {
    const view = diagnoseView(
      report({
        recommendation: 'socks5EnvOnly',
        summary: '只有 SOCKS5 远程解析可用；系统代理会按本机解析，建议仅启用环境变量同步。'
      }),
      'http'
    )

    expect(view.recommendedProtocol).toBe('socks5')
    expect(view.warning).toContain('环境变量')
  })

  it('offers no apply button when every path fails', () => {
    const failed = { ok: false, stage: 'tunnel' as const, latencyMs: 3000, error: '代理拒绝转发' }
    const view = diagnoseView(
      report({
        http: failed,
        socks5Domain: failed,
        systemSocks: failed,
        recommendation: 'unavailable',
        summary: '代理端口可达，但转发或解析失败，请检查代理上游。'
      }),
      'http'
    )

    expect(view.canApply).toBe(false)
    expect(view.recommendedProtocol).toBeNull()
  })
})

describe('diagnosis labels', () => {
  it('maps stages and protocols to Chinese labels', () => {
    expect(stageLabel('proxy')).toBe('连接代理')
    expect(stageLabel('tls')).toBe('TLS 握手')
    expect(recommendationProtocol('socks5System')).toBe('socks5')
    expect(recommendationProtocol('unavailable')).toBeNull()
    expect(protocolLabel('http')).toBe('HTTP')
    expect(protocolLabel('socks5')).toBe('SOCKS5')
  })
})
