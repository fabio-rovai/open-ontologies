import { chooseSourceKind } from './source-factory'

// Deliberately a separate interface from DemoSource, the same choice
// compare-source.ts made and explains in its own header comment: this is an
// optional demo surface layered on top of the four core methods, not a
// fifth one bolted onto DemoSource itself.
//
// This is the moment the PIVOT in .superpowers/sdd/progress.md could not put
// in the web replay, because it needed the live engine: a SHACL validation
// that returns conforms=true over zero focus nodes, meaning nothing was
// actually checked. src/shacl.rs (around lines 552-567) already refuses to
// call that a pass -- it reports `conforms: null` with a warning that the
// shapes selected nothing -- and `verdict` here is the client-side mirror of
// that refusal: 'undetermined' for a run with zero focus nodes, never
// 'passes'. There is no 'passes' verdict in this type on purpose. A run
// either failed outright ('fails') or checked nothing ('undetermined');
// nothing here can render as a green tick.
//
// No violation count anywhere in this shape, deliberately. Three legitimate
// measurement methods over identical inputs give 178, 272 and 147
// violations, and a fourth figure (287) is already public
// (case-studies/dcat-us-binding/README.md); demo/README.md's own stated
// position is that none of the four is defensible as "the" figure. Focus
// node counts are stable across all methods and are the only counts this
// type carries.
export interface ValidationRun {
  id: string
  label: string
  corpusDescription: string
  dataTriples: number
  focusNodes: number
  matchedClassCount: number
  targetClassCount: number
  // The raw boolean the SHACL run itself returned (true for the
  // as-published run, since `violations.is_empty()` is vacuously true over
  // zero focus nodes). Kept alongside `verdict` rather than replaced by it,
  // so a caller can see exactly what a naive reading of `conforms` would
  // have said, next to why that reading is not the verdict.
  conformsRaw: boolean
  verdict: 'undetermined' | 'fails'
  reason: string | null
}

export interface ValidationFixtures {
  shapesFile: string
  commit: string
  measured: string
  runs: ValidationRun[]
}

export interface ValidationSource {
  runs(): Promise<ValidationFixtures>
}

// The single place a bundle's "validation" key becomes ValidationFixtures,
// mirroring compareFixturesFromBundle in compare-source.ts. bundle.json
// carries this as a sixth top-level key alongside corpus, graph, findings,
// chat and compare; ReplayFixtures does not name it, the same tolerance
// compare-source.ts relies on for "compare".
export function validationFixturesFromBundle(bundle: Record<string, unknown>): ValidationFixtures {
  const raw = bundle.validation
  if (!raw || typeof raw !== 'object') {
    throw new Error(
      'The precomputed bundle has no "validation" key (or it is not an object); the ' +
        'SHACL validation panel cannot be shown.',
    )
  }
  return raw as ValidationFixtures
}

export function createReplayValidationSource(fixtures: ValidationFixtures): ValidationSource {
  return {
    async runs(): Promise<ValidationFixtures> {
      return fixtures
    },
  }
}

// The single construction site for ValidationSource, mirroring
// getCompareSource() and getDemoSource(). In replay mode it reads the same
// bundle.json DemoSource does and pulls the "validation" key out of it via
// validationFixturesFromBundle. In live mode there is no honest
// implementation: showing this panel means running the SAME unchanged
// shapes file against TWO different corpus states (as published, and with
// the schema-derived binding applied), which is a build-time measurement
// pipeline (demo/dcat_conformance.py), not a single onto_shacl call a live
// session can make on demand against whatever happens to be loaded.
export async function getValidationSource(): Promise<ValidationSource> {
  if (chooseSourceKind(import.meta.env as unknown as Record<string, string | undefined>) === 'replay') {
    const response = await fetch('./precomputed/bundle.json')
    if (!response.ok) {
      throw new Error(`Could not load the precomputed demonstration: ${response.status}`)
    }
    const bundle = (await response.json()) as Record<string, unknown>
    return createReplayValidationSource(validationFixturesFromBundle(bundle))
  }
  return {
    async runs(): Promise<ValidationFixtures> {
      throw new Error(
        'No live validation comparison is implemented. Showing the as-published run next to ' +
          'the schema-derived-binding run means validating the same unchanged shapes file ' +
          'against two different corpus states, a measurement pipeline (see ' +
          'demo/dcat_conformance.py) rather than a single onto_shacl call against whatever ' +
          'is currently loaded. Run the offline replay (npm run build:web) to see the ' +
          'precomputed comparison.',
      )
    },
  }
}
