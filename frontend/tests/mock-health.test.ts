import { describe, expect, it } from 'vitest'
import { mockCall } from '@/shared/api/mock'
import type { CheckReport } from '@/shared/api/types'

describe('mock health commands', () => {
  it('returns only the tcp stage for the default mode', () => {
    const report = mockCall('check_health', { mode: 'tcp', force: true }) as CheckReport

    expect(report.tcp.stage).toBe('tcp')
    expect(report.tcp.ok).toBe(true)
    expect(report.proxy).toBeNull()
  })

  it('adds a proxy stage for deep checks', () => {
    const report = mockCall('check_health', { mode: 'deep', force: true }) as CheckReport

    expect(report.tcp.stage).toBe('tcp')
    expect(report.proxy?.stage).toBe('proxy')
    expect(report.proxy?.ok).toBe(true)
  })
})
