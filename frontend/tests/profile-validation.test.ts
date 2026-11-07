import { describe, expect, it } from 'vitest'
import { hasErrors, validateProfileDraft } from '@/features/profiles/validation'
import type { Profile } from '@/shared/api/types'

const existing: Profile[] = [
  { id: 'p_default', name: '公司代理', protocol: 'http', host: '192.168.5.4', port: 7890 }
]

function draft(overrides: Partial<Parameters<typeof validateProfileDraft>[0]> = {}) {
  return {
    id: null,
    name: '备用代理',
    protocol: 'http',
    host: '192.168.5.9',
    port: 7890,
    ...overrides
  }
}

describe('validateProfileDraft', () => {
  it('accepts a normal profile', () => {
    expect(hasErrors(validateProfileDraft(draft(), existing))).toBe(false)
  })

  it('rejects an empty name', () => {
    expect(validateProfileDraft(draft({ name: '   ' }), existing).name).toBe('名称不能为空')
  })

  it('rejects names longer than 40 characters', () => {
    expect(validateProfileDraft(draft({ name: 'a'.repeat(41) }), existing).name).toContain('40')
  })

  it('rejects duplicate names case-insensitively but allows editing itself', () => {
    expect(validateProfileDraft(draft({ name: '公司代理' }), existing).name).toBe('配置名称已存在')
    expect(
      hasErrors(validateProfileDraft(draft({ id: 'p_default', name: '公司代理' }), existing, 'p_default'))
    ).toBe(false)
  })

  it('rejects invalid hosts and ports', () => {
    expect(validateProfileDraft(draft({ host: 'proxy server' }), existing).host).toBeTruthy()
    expect(validateProfileDraft(draft({ port: 0 }), existing).port).toBeTruthy()
    expect(validateProfileDraft(draft({ port: 65536 }), existing).port).toBeTruthy()
    expect(validateProfileDraft(draft({ port: null }), existing).port).toBeTruthy()
  })

  it('accepts IPv6 hosts and socks5 profiles', () => {
    expect(hasErrors(validateProfileDraft(draft({ host: 'fd00::4', protocol: 'socks5' }), existing))).toBe(false)
  })

  it('rejects unknown protocols', () => {
    expect(validateProfileDraft(draft({ protocol: 'ftp' }), existing).protocol).toBeTruthy()
  })
})
