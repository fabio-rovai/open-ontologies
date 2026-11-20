import { create } from 'zustand'
import type { Chunk, Contradiction, Decision, DemoSource, Document, GraphView } from '../lib/demo-source'
import { getDemoSource } from '../lib/source-factory'

interface DemoState {
  documents: Document[]
  // corpus() has no honest implementation in live mode today (see
  // live-source.ts: no tool or Tauri command returns document text), and
  // that must not read the same as "this corpus has zero documents". Kept
  // separate from `error` so a dead corpus panel never masks a working
  // graph and findings panel underneath it.
  documentsError: string | null
  graph: GraphView
  findings: Contradiction[]
  // Set when findings() throws: an engine failure, or live-source.ts's
  // undetermined SHACL result (conforms: null, the usual answer for an
  // ontology the DCAT-US shapes select nothing in). Kept apart from `error`
  // so that it costs the findings panel alone and never the graph, and
  // shown by that panel so an undetermined run never reads as "no findings"
  // (#422).
  findingsError: string | null
  selectedFinding: string | null
  ledger: { id: string; decision: Decision }[]
  chat: Chunk[]
  chatPending: boolean
  error: string | null
  loading: boolean
  load: () => Promise<void>
  // Re-fetches only the graph. Chat-driven ontology mutations in the live
  // desktop app (onto_load, onto_apply, onto_reason, ...) happen outside
  // this store, over the agent sidecar; LiveChrome calls this afterwards so
  // Graph3D, which now reads graph state instead of querying the engine
  // itself, still updates.
  refreshGraph: () => Promise<void>
  select: (id: string | null) => void
  resolve: (id: string, decision: Decision) => Promise<void>
  ask: (question: string) => Promise<void>
}

const EMPTY_GRAPH: GraphView = { classes: [], properties: [], edges: [] }

const message = (e: unknown): string => (e instanceof Error ? e.message : String(e))

// The only module in the frontend that touches a DemoSource. Components
// take data and callbacks as props, so the same tree renders live against
// the engine and offline from the precomputed artifacts.
export const useDemoStore = create<DemoState>((set, get) => ({
  documents: [],
  documentsError: null,
  graph: EMPTY_GRAPH,
  findings: [],
  findingsError: null,
  selectedFinding: null,
  ledger: [],
  chat: [],
  chatPending: false,
  error: null,
  loading: false,

  async load() {
    set({ loading: true, error: null })
    let source: DemoSource
    try {
      source = await getDemoSource()
    } catch (e) {
      // Surfaced, never swallowed. An empty graph and a dead engine must not look alike.
      set({ error: message(e), loading: false })
      return
    }

    // graph() and findings() are fetched together but settle apart (#422).
    // A failed graph() is a real error (engine down, session lost) and is
    // surfaced as the shell's error. A failed findings() goes to its own
    // `findingsError` and leaves the graph in place: live-source.ts throws
    // there on purpose whenever SHACL is undetermined, the usual answer for
    // an ontology that is not a DCAT-US catalogue.
    const [graph, findings] = await Promise.allSettled([source.graph(), source.findings()])
    set({
      ...(graph.status === 'fulfilled' ? { graph: graph.value } : { error: message(graph.reason) }),
      ...(findings.status === 'fulfilled'
        ? { findings: findings.value, findingsError: null }
        : { findings: [], findingsError: message(findings.reason) }),
      loading: false,
    })

    // corpus() is fetched separately and after the above, on purpose: today
    // it throws in live mode (no tool reaches document text there), and
    // that must degrade the corpus panel alone, not take the graph and
    // findings panels down with it.
    try {
      const documents = await source.corpus()
      set({ documents, documentsError: null })
    } catch (e) {
      set({ documents: [], documentsError: message(e) })
    }
  },

  async refreshGraph() {
    try {
      const source = await getDemoSource()
      const graph = await source.graph()
      set({ graph })
    } catch (e) {
      set({ error: e instanceof Error ? e.message : String(e) })
    }
  },

  select(id) {
    set({ selectedFinding: id })
  },

  async resolve(id, decision) {
    try {
      const source = await getDemoSource()
      await source.resolve(id, decision)
      set({ ledger: [...get().ledger, { id, decision }] })
    } catch (e) {
      set({ error: e instanceof Error ? e.message : String(e) })
    }
  },

  async ask(question) {
    set({ chatPending: true })
    try {
      const source = await getDemoSource()
      const chunks: Chunk[] = []
      for await (const chunk of source.ask(question)) {
        chunks.push(chunk)
      }
      set({ chat: [...get().chat, ...chunks], chatPending: false })
    } catch (e) {
      set({ error: e instanceof Error ? e.message : String(e), chatPending: false })
    }
  },
}))
