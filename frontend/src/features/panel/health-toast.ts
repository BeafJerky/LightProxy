import type { CheckReport } from '@/shared/api/types'

export interface HealthToast {
  type: 'success' | 'error'
  text: string
}

export function healthToast(report: CheckReport): HealthToast {
  if (!report.tcp.ok) {
    return { type: 'error', text: report.tcp.error ?? '代理端口不可达' }
  }
  if (!report.proxy) {
    return { type: 'success', text: `连接正常 · TCP ${report.tcp.latencyMs}ms` }
  }
  if (!report.proxy.ok) {
    return {
      type: 'error',
      text: `端口可达，但代理无法转发：${report.proxy.error ?? '未知原因'}`
    }
  }
  return {
    type: 'success',
    text: `连接正常 · TCP ${report.tcp.latencyMs}ms · 协议 ${report.proxy.latencyMs}ms`
  }
}
