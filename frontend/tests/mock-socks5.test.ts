import { describe, expect, it } from 'vitest'
import { mockCall } from '@/shared/api/mock'
import type { AppState } from '@/shared/api/types'

function saveSocksProfile(name: string): string {
  const state = mockCall('save_profile', {
    input: { id: null, name, protocol: 'socks5', host: '192.168.5.4', port: 1080 }
  }) as AppState
  const profile = state.profiles.find((item) => item.name === name)
  if (!profile) throw new Error('saved profile missing')
  return profile.id
}

describe('mock socks5 activation', () => {
  it('enables a socks5 profile', () => {
    const id = saveSocksProfile('SOCKS 启用')
    const state = mockCall('enable_proxy', { profileId: id }) as AppState
    expect(state.state).toBe('on')
    expect(state.activeProfileId).toBe(id)
  })

  it('switches to a socks5 profile while active', () => {
    const id = saveSocksProfile('SOCKS 切换')
    const state = mockCall('switch_profile', { profileId: id, force: true }) as AppState
    expect(state.state).toBe('on')
    expect(state.activeProfileId).toBe(id)
  })
})