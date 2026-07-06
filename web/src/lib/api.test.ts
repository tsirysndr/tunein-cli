import { describe, expect, it } from 'vitest'
import { isFolder, type Station } from './api'

const station = (overrides: Partial<Station>): Station => ({
  id: 's24939',
  name: 'BBC Radio 1',
  codec: 'MP3',
  bitrate: 128,
  streamUrl: '',
  playing: null,
  ...overrides,
})

describe('isFolder', () => {
  it('treats tunein "s…" guide ids as stations', () => {
    expect(isFolder(station({ id: 's24939' }), 'tunein')).toBe(false)
  })

  it('treats tunein "c…" guide ids as browsable folders', () => {
    expect(isFolder(station({ id: 'c57942' }), 'tunein')).toBe(true)
  })

  it('treats tunein "g…" guide ids as browsable folders', () => {
    expect(isFolder(station({ id: 'g123' }), 'tunein')).toBe(true)
  })

  it('never treats radiobrowser results as folders', () => {
    expect(isFolder(station({ id: 'ef52b56c-6830-4346-b4d3-e42e5ae5d928' }), 'radiobrowser')).toBe(
      false,
    )
  })
})
