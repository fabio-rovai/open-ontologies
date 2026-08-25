import { describe, it, expect } from 'vitest'
import { createReplaySource, type ReplayFixtures } from '../replay-source'
import type { Chunk } from '../demo-source'

const fixtures: ReplayFixtures = {
  corpus: [{ id: 'README', title: 'README', text: 'an implementation of the W3C DCAT standard' }],
  graph: { classes: [{ iri: 'ex:Dataset' }], properties: [], edges: [] },
  findings: [
    {
      id: 'f1',
      subject: 'ex:conformance',
      kind: 'provenance-split' as const,
      claims: [
        { document: 'README', predicate: 'claims', object: 'dcat-conformant' },
        { document: 'examples', predicate: 'yields', object: 'zero-dcat-triples' },
      ],
    },
  ],
  chat: { 'what disagrees?': [{ type: 'text' as const, value: 'README and examples disagree.' }] },
}

describe('ReplaySource', () => {
  it('returns the committed findings', async () => {
    const src = createReplaySource(fixtures)
    const found = await src.findings()
    expect(found).toHaveLength(1)
    expect(found[0].claims.map((c) => c.document)).toEqual(['README', 'examples'])
  })

  // resolve() itself keeps no ledger (see replay-source.ts: the caller,
  // useDemoStore, is the actual record of a resolution). What this test
  // verifies is narrower and is exactly what it asserts: resolve() resolves
  // without throwing, and does not mutate the committed fixtures as a side
  // effect.
  it('resolves without mutating the committed fixtures', async () => {
    const src = createReplaySource(fixtures)
    await src.resolve('f1', { kind: 'accept' })
    expect(await src.findings()).toHaveLength(1)
    expect(fixtures.findings).toHaveLength(1)
  })

  it('does not hand out live references into the claim fixtures', async () => {
    const src = createReplaySource(fixtures)
    const found = await src.findings()
    found[0].claims[0].object = 'mutated-in-place'
    expect(fixtures.findings[0].claims[0].object).toBe('dcat-conformant')
  })

  it('does not hand out a live reference into the corpus fixtures', async () => {
    const src = createReplaySource(fixtures)
    const found = await src.corpus()
    found[0].title = 'mutated-in-place'
    expect(fixtures.corpus[0].title).toBe('README')
  })

  it('does not hand out a live reference into the graph fixtures', async () => {
    const src = createReplaySource(fixtures)
    const found = await src.graph()
    found.classes[0].label = 'mutated-in-place'
    expect(fixtures.graph.classes[0].label).toBeUndefined()
  })

  it('streams a scripted answer', async () => {
    const src = createReplaySource(fixtures)
    const out: string[] = []
    for await (const chunk of src.ask('what disagrees?')) out.push(chunk.value)
    expect(out.join('')).toContain('disagree')
  })

  it('marks an unscripted answer with a distinct chunk type instead of relying on wording', async () => {
    const src = createReplaySource(fixtures)
    const chunks: Chunk[] = []
    for await (const chunk of src.ask('unscripted question')) chunks.push(chunk)
    expect(chunks).toHaveLength(1)
    expect(chunks[0].type).toBe('unscripted')
    expect(chunks[0].value.length).toBeGreaterThan(0)
  })
})
