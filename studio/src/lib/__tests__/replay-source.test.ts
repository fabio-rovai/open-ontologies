import { describe, it, expect } from 'vitest'
import { createReplaySource } from '../replay-source'

const fixtures = {
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

  it('records a resolution in session state without mutating fixtures', async () => {
    const src = createReplaySource(fixtures)
    await src.resolve('f1', { kind: 'accept' })
    expect(await src.findings()).toHaveLength(1)
    expect(fixtures.findings).toHaveLength(1)
  })

  it('streams a scripted answer', async () => {
    const src = createReplaySource(fixtures)
    const out: string[] = []
    for await (const chunk of src.ask('what disagrees?')) out.push(chunk.value)
    expect(out.join('')).toContain('disagree')
  })

  it('answers unknown questions without throwing', async () => {
    const src = createReplaySource(fixtures)
    const out: string[] = []
    for await (const chunk of src.ask('unscripted question')) out.push(chunk.value)
    expect(out.join('')).not.toHaveLength(0)
  })
})
