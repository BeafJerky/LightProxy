import type {
  DiagnosePathResult,
  DiagnoseRecommendation,
  DiagnoseReport,
  PathStage
} from '@/shared/api/types'

export type DiagnoseProtocol = 'http' | 'https' | 'socks5'

export interface DiagnoseRow {
  key: 'http' | 'socks5Domain' | 'systemSocks'
  label: string
  hint: string
  result: DiagnosePathResult
}

export interface DiagnoseView {
  rows: DiagnoseRow[]
  summary: string
  recommendedProtocol: DiagnoseProtocol | null
  recommendedLabel: string
  warning: string | null
  canApply: boolean
  alreadyApplied: boolean
}

const ROWS = [
  { key: 'http', label: 'HTTP 代理', hint: '系统代理使用 HTTP / HTTPS 配置' },
  { key: 'socks5Domain', label: 'SOCKS5 远程解析', hint: '环境变量同步与更新下载' },
  { key: 'systemSocks', label: '系统 SOCKS（本机解析）', hint: '系统代理使用 SOCKS5 配置' }
] as const

const STAGE_LABELS: Record<PathStage, string> = {
  proxy: '连接代理',
  tunnel: '建立隧道',
  tls: 'TLS 握手',
  http: '读取响应'
}

export function stageLabel(stage: PathStage): string {
  return STAGE_LABELS[stage]
}

export function recommendationProtocol(
  recommendation: DiagnoseRecommendation
): DiagnoseProtocol | null {
  switch (recommendation) {
    case 'http':
      return 'http'
    case 'socks5System':
    case 'socks5EnvOnly':
      return 'socks5'
    default:
      return null
  }
}

export function protocolLabel(protocol: DiagnoseProtocol): string {
  if (protocol === 'http') return 'HTTP'
  if (protocol === 'https') return 'HTTPS'
  return 'SOCKS5'
}

export function diagnoseView(report: DiagnoseReport, currentProtocol: string): DiagnoseView {
  const recommendedProtocol = recommendationProtocol(report.recommendation)
  return {
    rows: ROWS.map((row) => ({ ...row, result: report[row.key] })),
    summary: report.summary,
    recommendedProtocol,
    recommendedLabel: recommendedProtocol ? protocolLabel(recommendedProtocol) : '',
    warning:
      report.recommendation === 'socks5EnvOnly'
        ? '浏览器仍可能打不开网页；可只开启环境变量同步，供命令行与更新使用。'
        : null,
    canApply: recommendedProtocol !== null,
    alreadyApplied: recommendedProtocol !== null && recommendedProtocol === currentProtocol
  }
}
