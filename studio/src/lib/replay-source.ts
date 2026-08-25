import type { Chunk, Claim, Contradiction, DemoSource, Document, GraphView } from './demo-source'

// A later artifact bundle carries a fifth key ('compare') alongside the
// four this replay reads. The index signature makes that tolerance
// explicit in the type rather than incidental to how a caller happens to
// construct the object, so a literal with the extra key compiles.
export interface ReplayFixtures {
  corpus: Document[]
  graph: GraphView
  findings: Contradiction[]
  chat: Record<string, Chunk[]>
  [key: string]: unknown
}

const UNSCRIPTED_CHUNK: Chunk = {
  type: 'unscripted',
  value:
    'This is the offline replay of the demonstration. Only the scripted questions are answered here. Run the desktop application against the engine for a live session.',
}

const cloneDocument = (d: Document): Document => ({ ...d })
const cloneClaim = (c: Claim): Claim => ({ ...c })
const cloneContradiction = (f: Contradiction): Contradiction => ({ ...f, claims: f.claims.map(cloneClaim) })
const cloneGraph = (g: GraphView): GraphView => ({
  classes: g.classes.map((c) => ({ ...c })),
  properties: g.properties.map((p) => ({ ...p })),
  edges: g.edges.map((e) => ({ ...e })),
})

export function createReplaySource(fixtures: ReplayFixtures): DemoSource {
  return {
    async corpus() {
      return fixtures.corpus.map(cloneDocument)
    },
    async graph() {
      return cloneGraph(fixtures.graph)
    },
    async findings() {
      return fixtures.findings.map(cloneContradiction)
    },
    // DemoSource.resolve() returns void by design (live-source.ts's
    // implementation throws instead, since no engine tool resolves a
    // finding by id yet), and the caller already keeps its own record: the
    // ledger a resolution shows up in is useDemoStore's `ledger` state
    // (demo-store.ts's resolve() appends to it after this call succeeds),
    // not anything this function needs to track. An earlier version of this
    // function wrote to an internal `ledger` array that nothing ever read
    // back out; it has been removed rather than left as write-only state.
    async resolve(_id, _decision) {},
    async *ask(question) {
      const scripted = fixtures.chat[question.trim().toLowerCase()] ?? fixtures.chat[question]
      if (scripted) {
        for (const chunk of scripted) yield chunk
      } else {
        yield UNSCRIPTED_CHUNK
      }
    },
  }
}
