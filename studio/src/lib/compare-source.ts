import { chooseSourceKind } from './source-factory'

// Deliberately a separate interface from DemoSource (see demo-source.ts).
// Tasks 9 through 11 stay untouched, and the comparison is an optional demo
// surface layered on top rather than a fifth core method.
//
// `status` is an explicit discriminant, the same fix Chunk got for its
// 'unscripted' variant: 'ok' is a real grounded-vs-baseline answer pair from
// compare.json, 'unscripted' is a question outside the scripted set (the
// replay fallback below), and 'error' is a comparison that failed to run at
// all (AppShell.tsx's handleAsk catch). Before this field existed, a failed
// comparison built a result with the exception message in grounded.answer
// and an empty baseline -- structurally identical to a real answer, so
// ComparePanel rendered it under the "Grounded" heading with the baseline
// showing no answer, which reads as a grounded win. Callers must render on
// `status`, not by inspecting whether the strings happen to be empty.
export interface CompareResult {
  question: string
  status: 'ok' | 'unscripted' | 'error'
  grounded: { answer: string; citations: string[] }
  baseline: { answer: string; citations: string[] }
  divergence: string | null
}

export interface CompareSource {
  compare(question: string): Promise<CompareResult>
}

// The single place a failed comparison becomes a CompareResult, used by
// AppShell.tsx's handleAsk catch block. Before this existed, that catch
// block built the failure result inline: the exception message went into
// grounded.answer with baseline left empty, a shape structurally identical
// to a real answer. ComparePanel then rendered it under the "Grounded"
// heading with the baseline column showing "(no answer)" -- a comparison
// failure reading as a grounded win. Extracted as a pure function so the
// discriminant is unit-testable without a DOM/component test harness (this
// repo has neither jsdom nor React Testing Library configured).
export function compareError(question: string, error: unknown): CompareResult {
  return {
    question,
    status: 'error',
    grounded: { answer: error instanceof Error ? error.message : String(error), citations: [] },
    baseline: { answer: '', citations: [] },
    divergence: null,
  }
}

// Fixtures on disk (compare.json, via bundle.json) predate the `status`
// field and carry no such key; every fixture is a real answer pair, so it
// is always 'ok'.
export type CompareFixtures = Record<string, Omit<CompareResult, 'status'>>

export function createReplayCompareSource(fixtures: CompareFixtures): CompareSource {
  return {
    async compare(question: string): Promise<CompareResult> {
      const hit = fixtures[question] ?? fixtures[question.trim().toLowerCase()]
      if (hit) return { ...hit, status: 'ok' }
      return {
        question,
        status: 'unscripted',
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
