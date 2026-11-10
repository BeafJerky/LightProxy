import { describe, expect, it } from 'vitest'
import { mockCall } from '@/shared/api/mock'
import type { DiagnoseReport } from '@/shared/api/types'

describe('mock diagnose_profile', () => {
  it('returns a camel-case report with an HTTP recommendation', () => {
    const report = mockCall('diagnose_profile', {
      input: { name: '', protocol: 'socks5', host: '127.0.0.1', port: 7890 }
    }) as DiagnoseReport

    expect(report.recommendation).toBe('http')
    expect(report.http.ok).toBe(true)
    expect(report.socks5Domain.latencyMs).toBeGreaterThan(0)
    expect(report.systemSocks.ok).toBe(false)
  })

  it('rejects invalid protocols without touching the network', () => {
    expect(() =>
      mockCall('diagnose_profile', {
        input: { name: '', protocol: 'ftp', host: '127.0.0.1', port: 7890 }
      })
    ).toThrow()
  })
})
