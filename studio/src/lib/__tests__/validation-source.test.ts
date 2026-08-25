import { describe, it, expect } from 'vitest'
import {
  createReplayValidationSource,
  validationFixturesFromBundle,
  type ValidationFixtures,
} from '../validation-source'

const fixtures: ValidationFixtures = {
  shapesFile: 'recovered-shapes.ttl',
  commit: '7a6e803fb94ee9903e7e7405ec4afcc8da13383f',
  measured: '2026-08-25, this repository, no network, no model call',
  runs: [
    {
      id: 'as-published',
      label: 'Corpus as published',
      corpusDescription: '115 good examples, exactly as published',
      dataTriples: 73,
      focusNodes: 0,
      matchedClassCount: 0,
      targetClassCount: 34,
      conformsRaw: true,
      verdict: 'undetermined',
      reason: '0 of 34 target classes in the shapes file matched anything in the data.',
    },
    {
      id: 'schema-derived-binding',
      label: 'Corpus with schema-derived binding applied',
      corpusDescription: 'the same 115 examples, with the schema-derived binding applied',
      dataTriples: 1383,
      focusNodes: 228,
      matchedClassCount: 24,
      targetClassCount: 34,
      conformsRaw: false,
      verdict: 'fails',
      reason: null,
    },
  ],
}

describe('createReplayValidationSource', () => {
  it('returns the committed validation runs', async () => {
    const src = createReplayValidationSource(fixtures)
    const out = await src.runs()
    expect(out.runs).toHaveLength(2)
    expect(out.shapesFile).toBe('recovered-shapes.ttl')
  })

  // This is the regression the whole panel exists to guard against: a SHACL
  // run with zero focus nodes checked nothing, and must never be
  // indistinguishable from -- or worse, presented as -- a genuine pass.
  // ValidationRun's `verdict` field has exactly two values, 'undetermined'
  // and 'fails' (validation-source.ts); there is no 'passes' member of the
  // type at all, so a run cannot be typed as a pass even by mistake. This
  // test pins the runtime data alongside that static guarantee: the run
  // with focusNodes === 0 must carry verdict 'undetermined', never 'fails'
  // read as a clean pass and never a bare `conformsRaw: true` left to speak
  // for itself.
  it('marks the zero-focus-node run as undetermined, never as a pass', async () => {
    const src = createReplayValidationSource(fixtures)
    const out = await src.runs()
    const asPublished = out.runs.find((r) => r.focusNodes === 0)
    expect(asPublished).toBeDefined()
    expect(asPublished!.verdict).toBe('undetermined')
    expect(asPublished!.verdict).not.toBe('fails')
    // conformsRaw is allowed to be true (that is the honest raw reading),
    // but nothing in this type or fixture calls that reading a pass.
    expect(asPublished!.conformsRaw).toBe(true)
    expect(asPublished!.reason).toMatch(/nothing|matched/i)
  })

  it('marks the real failure as fails, distinct from the vacuous run', async () => {
    const src = createReplayValidationSource(fixtures)
    const out = await src.runs()
    const bound = out.runs.find((r) => r.focusNodes > 0)
    expect(bound).toBeDefined()
    expect(bound!.verdict).toBe('fails')
    expect(bound!.conformsRaw).toBe(false)
    expect(bound!.focusNodes).toBe(228)
  })

  it('never exposes a violation count field on any run', async () => {
    const src = createReplayValidationSource(fixtures)
    const out = await src.runs()
    for (const run of out.runs) {
      expect(Object.keys(run)).not.toContain('violations')
      expect(Object.keys(run)).not.toContain('violationCount')
      expect(Object.keys(run)).not.toContain('violation_count')
    }
  })
})

describe('validationFixturesFromBundle', () => {
  // bundle.json carries "validation" as a sixth top-level key alongside the
  // four ReplayFixtures declares plus "compare". This is the one place that
  // reaches into it, mirroring compareFixturesFromBundle in
  // compare-source.ts.
  it('extracts the validation key from a bundle carrying the other keys too', () => {
    const bundle = {
      corpus: [],
      graph: { classes: [], properties: [], edges: [] },
      findings: [],
      chat: {},
      compare: {},
      validation: fixtures,
    }
    expect(validationFixturesFromBundle(bundle)).toBe(fixtures)
  })

  it('throws rather than silently returning an empty validation set when the key is missing', () => {
    const bundle = { corpus: [], graph: { classes: [], properties: [], edges: [] }, findings: [], chat: {} }
    expect(() => validationFixturesFromBundle(bundle)).toThrow(/validation/)
  })
})
