import type { Profile } from '@/shared/api/types'

export interface ProfileDraft {
  id?: string | null
  name: string
  protocol: string
  host: string
  port: number | null
}

export interface ProfileErrors {
  name?: string
  host?: string
  port?: string
  protocol?: string
}

const HOST_PATTERN = /^[A-Za-z0-9._:-]+$/
const PROTOCOLS = new Set(['http', 'https', 'socks5'])

export function validateProfileDraft(
  draft: ProfileDraft,
  existing: Profile[],
  editingId: string | null = null
): ProfileErrors {
  const errors: ProfileErrors = {}
  const name = draft.name.trim()
  const host = draft.host.trim()

  if (!name) {
    errors.name = '名称不能为空'
  } else if ([...name].length > 40) {
    errors.name = '名称不能超过 40 个字符'
  } else if (
    existing.some(
      (profile) => profile.id !== editingId && profile.name.toLowerCase() === name.toLowerCase()
    )
  ) {
    errors.name = '配置名称已存在'
  }

  if (!PROTOCOLS.has(draft.protocol)) {
    errors.protocol = '协议必须是 http、https 或 socks5'
  }

  if (!host) {
    errors.host = '主机不能为空'
  } else if (host.length > 253) {
    errors.host = '主机名不能超过 253 个字符'
  } else if (!HOST_PATTERN.test(host)) {
    errors.host = '主机只能包含字母、数字、点、横线、下划线和冒号'
  }

  if (draft.port === null || !Number.isInteger(draft.port) || draft.port < 1 || draft.port > 65535) {
    errors.port = '端口范围是 1–65535'
  }

  return errors
}

export function hasErrors(errors: ProfileErrors): boolean {
  return Object.keys(errors).length > 0
}
