import { describe, it, expect } from 'vitest'
import { chooseSourceKind } from '../source-factory'

describe('source selection', () => {
  it('replays when the build target says web', () => {
    expect(chooseSourceKind({ VITE_DEMO_MODE: 'replay' })).toBe('replay')
  })
  it('goes live by default', () => {
    expect(chooseSourceKind({})).toBe('live')
  })
})
