import { describe, it, expect } from 'vitest'
import { runnerIsWired } from '../runner-check'

describe('test runner', () => {
  it('is wired up', () => {
    expect(runnerIsWired()).toBe(true)
  })
})
