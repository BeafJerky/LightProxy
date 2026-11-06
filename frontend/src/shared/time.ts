export function nowMs(): number {
  return Date.now()
}

export function relativeTime(atMs: number): string {
  const diff = Date.now() - atMs
  if (diff < 5_000) return '刚刚'
  if (diff < 60_000) return `${Math.round(diff / 1_000)} 秒前`
  if (diff < 3_600_000) return `${Math.round(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.round(diff / 3_600_000)} 小时前`
  return `${Math.round(diff / 86_400_000)} 天前`
}

export function clockTime(atMs: number): string {
  const date = new Date(atMs)
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${pad(date.getHours())}:${pad(date.getMinutes())}`
}
