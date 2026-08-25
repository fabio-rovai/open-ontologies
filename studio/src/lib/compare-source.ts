import { chooseSourceKind } from './source-factory'

// Deliberately a separate interface from DemoSource (see demo-source.ts).
// Tasks 9 through 11 stay untouched, and the comparison is an optional demo
// surface layered on top rather than a fifth core method.
export interface CompareResult {
  question: string
  grounded: { answer: string; citations: string[] }
  baseline: { answer: string; citations: string[] }
  divergence: string | null
}

export interface CompareSource {
  compare(question: string): Promise<CompareResult>
}

export type CompareFixtures = Record<string, CompareResult>

export function createReplayCompareSource(fixtures: CompareFixtures): CompareSource {
  return {
    async compare(question: string): Promise<CompareResult> {
      const hit = fixtures[question] ?? fixtures[question.trim().toLowerCase()]
      if (hit) return hit
      return {
        question,
        grounded: {
          answer: 'This question is not scripted in the offline replay.',
          citations: [],
        },
        baseline: { answer: '', citations: [] },
        divergence: null,
      }
    },
  }
}

// The precomputed bundle (demo/precomputed/bundle.json) carries a fifth
// top-level key, "compare", alongside the four ReplayFixtures declares
// (corpus, graph, findings, chat). ReplayFixtures tolerates that with an
// index signature rather than naming it, precisely so CompareSource stays
// out of DemoSource's shape. This function is the one place that reaches
// into the extra key, so every other module can go on treating
// ReplayFixtures as a four-key type.
export function compareFixturesFromBundle(bundle: Record<string, unknown>): CompareFixtures {
  const raw = bundle.compare
  if (!raw || typeof raw !== 'object') {
    throw new Error(
      'The precomputed bundle has no "compare" key (or it is not an object); the ' +
        'baseline comparison cannot be shown.',
    )
  }
  return raw as CompareFixtures
}

// The single construction site for CompareSource, mirroring
// getDemoSource() in source-factory.ts. In replay mode it reads the same
// bundle.json DemoSource does and pulls the "compare" key out of it via
// compareFixturesFromBundle. In live mode there is no honest
// implementation yet: the divergence judgment between the grounded and
// baseline answers is a human read over both transcripts (see
// demo/build_compare.py's module docstring), not something a running
// session can compute for an arbitrary question, so this says that plainly
// instead of inventing a verdict.
export async function getCompareSource(): Promise<CompareSource> {
  if (chooseSourceKind(import.meta.env as unknown as Record<string, string | undefined>) === 'replay') {
    const response = await fetch('./precomputed/bundle.json')
    if (!response.ok) {
      throw new Error(`Could not load the precomputed demonstration: ${response.status}`)
    }
    const bundle = (await response.json()) as Record<string, unknown>
    return createReplayCompareSource(compareFixturesFromBundle(bundle))
  }
  return {
    async compare(question: string): Promise<CompareResult> {
      throw new Error(
        `No live comparison is implemented for "${question}". The grounded-vs-baseline ` +
          'divergence is a human judgment made once over a fixed question set (see ' +
          'demo/build_compare.py), not something a live session computes on demand. Run ' +
          'the offline replay (npm run build:web) to see the precomputed comparison.',
      )
    },
  }
}
