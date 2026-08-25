import { describe, it, expect } from 'vitest'
import { createReplayCompareSource, compareFixturesFromBundle } from '../compare-source'

const fixture = {
  'does this profile implement W3C DCAT?': {
    question: 'does this profile implement W3C DCAT?',
    grounded: {
      answer: 'No. The published examples expand to 76 triples and no DCAT predicates.',
      citations: ['examples.json', 'w3c-dcat-conformance.md'],
    },
    baseline: {
      answer: 'Yes. The README states it is an implementation of the W3C DCAT standard.',
      citations: ['profile-readme.md'],
    },
    divergence: 'The baseline repeats the claim. The grounded answer checks it against the artifacts.',
  },
}

describe('ReplayCompareSource', () => {
  it('returns both answers with their citations', async () => {
    const src = createReplayCompareSource(fixture)
    const r = await src.compare('does this profile implement W3C DCAT?')
    expect(r.grounded.citations).toContain('examples.json')
    expect(r.baseline.citations).toEqual(['profile-readme.md'])
    expect(r.divergence).not.toBeNull()
  })

  it('reports an unscripted question rather than fabricating a comparison', async () => {
    const src = createReplayCompareSource(fixture)
    const r = await src.compare('something nobody scripted')
    expect(r.divergence).toBeNull()
    expect(r.grounded.answer).toMatch(/not scripted|offline replay/i)
  })
})

describe('compareFixturesFromBundle', () => {
  // bundle.json carries a fifth top-level key, "compare", that
  // ReplayFixtures (demo-source.ts / replay-source.ts) only tolerates
  // through an index signature rather than naming. This is the one place
  // that reaches into it, so the reconciliation is regression-tested
  // directly rather than only exercised indirectly through a fetch.
  it('extracts the compare key from a bundle carrying the other four DemoSource keys too', () => {
    const bundle = {
      corpus: [{ id: 'doc-1', title: 'doc-1', text: 'irrelevant to this test' }],
      graph: { classes: [], properties: [], edges: [] },
      findings: [],
      chat: {},
      compare: fixture,
    }
    expect(compareFixturesFromBundle(bundle)).toBe(fixture)
  })

  it('throws rather than silently returning an empty comparison set when the key is missing', () => {
    const bundle = { corpus: [], graph: { classes: [], properties: [], edges: [] }, findings: [], chat: {} }
    expect(() => compareFixturesFromBundle(bundle)).toThrow(/compare/)
  })
})
