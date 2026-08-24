import type { Chunk, Contradiction, Decision, DemoSource, Document, GraphView } from './demo-source'

export interface ReplayFixtures {
  corpus: Document[]
  graph: GraphView
  findings: Contradiction[]
  chat: Record<string, Chunk[]>
}

const UNSCRIPTED: Chunk[] = [
  {
    type: 'text',
    value:
      'This is the offline replay of the demonstration. Only the scripted questions are answered here. Run the desktop application against the engine for a live session.',
  },
]

export function createReplaySource(fixtures: ReplayFixtures): DemoSource {
  const findings = fixtures.findings.map((f) => ({ ...f, claims: [...f.claims] }))
  const ledger: { id: string; decision: Decision }[] = []

  return {
    async corpus() {
      return fixtures.corpus
    },
    async graph() {
      return fixtures.graph
    },
    async findings() {
      return findings
    },
    async resolve(id, decision) {
      ledger.push({ id, decision })
    },
    async *ask(question) {
      const scripted = fixtures.chat[question.trim().toLowerCase()] ?? fixtures.chat[question]
      for (const chunk of scripted ?? UNSCRIPTED) yield chunk
    },
  }
}
