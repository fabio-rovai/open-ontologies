import { describe, it, expect } from 'vitest'
import { legendText } from '../graph-legend'

// Graph3D's legend used to hard-code the replay demo's file and triple count,
// and rendered them over whatever ontology live mode had loaded (#414).
describe('legendText', () => {
  it('keeps the replay demo text exactly: that graph is built from ies-core', () => {
    expect(legendText({ kind: 'replay' })).toEqual({
      caption: 'ies-core.ttl · 1,083 triples',
      asserted: 'read from ies-core.ttl. claimed by a person',
    })
  })

  it('names the loaded file and the engine triple count in live mode', () => {
    expect(legendText({ kind: 'live', file: 'studio-live.ttl', triples: 15234 })).toEqual({
      caption: 'studio-live.ttl · 15,234 triples',
      asserted: 'read from studio-live.ttl. claimed by a person',
    })
  })

  it('never names ies-core in live mode', () => {
    const t = legendText({ kind: 'live', file: 'pizza.ttl', triples: 1940 })
    expect(JSON.stringify(t)).not.toContain('ies-core')
    expect(JSON.stringify(t)).not.toContain('1,083')
  })

  it('shows the file alone while the stats have not arrived', () => {
    expect(legendText({ kind: 'live', file: 'studio-live.ttl' })).toEqual({
      caption: 'studio-live.ttl',
      asserted: 'read from studio-live.ttl. claimed by a person',
    })
  })

  it('shows the count alone when no file is named', () => {
    expect(legendText({ kind: 'live', file: '  ', triples: 42 })).toEqual({
      caption: '42 triples',
      asserted: 'claimed by a person',
    })
  })

  it('says one triple, not one triples', () => {
    expect(legendText({ kind: 'live', triples: 1 }).caption).toBe('1 triple')
  })

  // getStats() in mcp-client.ts reports 0 triples when /api/stats fails, and
  // the legend is only drawn over a graph with nodes, so a zero there is never
  // the store's real size.
  it('omits a zero count rather than printing it beside a non-empty graph', () => {
    expect(legendText({ kind: 'live', file: 'studio-live.ttl', triples: 0 }).caption).toBe('studio-live.ttl')
  })

  it('shows no caption at all when neither the file nor the count is known', () => {
    expect(legendText({ kind: 'live' })).toEqual({ caption: null, asserted: 'claimed by a person' })
  })
})
