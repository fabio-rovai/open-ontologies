import type { Chunk, Claim, Contradiction, Decision, DemoSource, Document, GraphView } from './demo-source'

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
  const ledger: { id: string; decision: Decision }[] = []

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
    async resolve(id, decision) {
      ledger.push({ id, decision })
    },
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
