import { describe, it, expect } from 'vitest'
import { emptyGraphText, legendParts, legendText } from '../graph-legend'

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

// The proof-carrying parts of the legend (its heading, the CERTIFIED and
// REJECTED rows and the Lean 4 / Isabelle footer) describe warrants. Live mode
// drew them anyway, with zero counts, over every ontology it loaded (#424).
describe('legendParts', () => {
  const labels = (p: ReturnType<typeof legendParts>) => p.rows.map((r) => r.label)

  // The replay bundle's graph is a certified run over ies-core (#425), so its
  // edges carry all three warrants and the legend names all three.
  it('shows every part over a replay graph that carries warrants', () => {
    const p = legendParts({ kind: 'replay' }, ['asserted', 'certified', 'rejected'])
    expect(p.heading).toBe('PROOF-CARRYING INFERENCE')
    expect(p.caption).toBe('ies-core.ttl · 1,083 triples')
    expect(p.rows).toEqual([
      { w: 'asserted', label: 'ASSERTED', means: 'read from ies-core.ttl. claimed by a person' },
      { w: 'certified', label: 'CERTIFIED', means: 'derived, then PROVED. OOCert.certificate_sound' },
      { w: 'rejected', label: 'REJECTED', means: 'forged. the checker exited 1 and named the rule' },
    ])
    expect(p.proof).toBe(true)
  })

  // One rule for both origins: the proof parts describe warrants, so they are
  // drawn only over a graph that carries one. The replay used to keep them
  // whatever its edges held, and its edges held none (#425).
  it('drops the proof parts from a replay graph with no warrant on any edge', () => {
    const p = legendParts({ kind: 'replay' }, ['asserted'])
    expect(labels(p)).toEqual(['ASSERTED'])
    expect(p.proof).toBe(false)
    expect(p.heading).not.toMatch(/PROOF/)
  })

  it('shows only the ASSERTED row in live mode when no edge carries a warrant', () => {
    const p = legendParts({ kind: 'live', file: 'pizza.ttl', triples: 1940 }, ['asserted', 'asserted'])
    expect(labels(p)).toEqual(['ASSERTED'])
    expect(p.rows[0].means).toBe('read from pizza.ttl. claimed by a person')
    expect(p.proof).toBe(false)
    expect(p.heading).not.toMatch(/PROOF/)
    expect(p.caption).toBe('pizza.ttl · 1,940 triples')
  })

  it('shows the proof parts in live mode once an edge is certified or rejected', () => {
    for (const w of ['certified', 'rejected'] as const) {
      const p = legendParts({ kind: 'live', file: 'pizza.ttl' }, ['asserted', w])
      expect(labels(p)).toEqual(['ASSERTED', 'CERTIFIED', 'REJECTED'])
      expect(p.proof).toBe(true)
      expect(p.heading).toBe('PROOF-CARRYING INFERENCE')
    }
  })
})

describe('emptyGraphText', () => {
  // The replay's tabs have no Documents panel either (#425).
  it('does not send the replay to a Documents panel', () => {
    const text = emptyGraphText({ kind: 'replay' })
    expect(text).not.toMatch(/Documents panel/)
    expect(text).toBe('The replay bundle carries no graph.')
  })

  // Live mode has no Documents panel.
  it('does not send live mode to a Documents panel', () => {
    const text = emptyGraphText({ kind: 'live', file: 'pizza.ttl' })
    expect(text).not.toMatch(/Documents panel/)
    expect(text).toMatch(/Chat/)
  })
})
