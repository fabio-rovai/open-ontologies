import { useEffect, useState, useCallback, lazy, Suspense } from 'react'
import { useDemoStore } from './state/demo-store'
import { CorpusPanel } from './components/CorpusPanel'
import { FindingsPanel } from './components/FindingsPanel'
import { ResolutionPanel } from './components/ResolutionPanel'
import { ComparePanel } from './components/ComparePanel'
import { Graph3D } from './components/Graph3D'
import { chooseSourceKind } from './lib/source-factory'
import { getCompareSource, type CompareResult } from './lib/compare-source'
import './App.css'

// Desktop-only chrome (chat over the agent sidecar, engine status, save/open,
// the 2D tree, inspector, lineage) is dynamically imported so none of the
// Tauri-coupled hooks it pulls in ever execute in the replay bundle: a
// static `import` at the top of this file would still bundle and evaluate
// that module's side effects (event listeners registered at module scope)
// even inside an `if (isLive)` branch, since ES module evaluation happens at
// import time, not at render time.
const LiveChrome = lazy(() => import('./components/LiveChrome').then((m) => ({ default: m.LiveChrome })))

const isLive =
  chooseSourceKind(import.meta.env as unknown as Record<string, string | undefined>) === 'live'

type Tab = 'findings' | 'corpus' | 'compare'

/**
 * The tabbed sidebar: findings + resolution, corpus, and the grounded-vs-
 * baseline comparison. This is the one demonstration surface both build
 * targets render identically, so it is composed once here and used both as
 * the whole sidebar in the replay build and as an additional panel inside
 * LiveChrome in the desktop build, never reimplemented twice.
 */
function DemoPanels() {
  const documents = useDemoStore((s) => s.documents)
  const documentsError = useDemoStore((s) => s.documentsError)
  const findings = useDemoStore((s) => s.findings)
  const selectedFinding = useDemoStore((s) => s.selectedFinding)
  const ledger = useDemoStore((s) => s.ledger)
  const select = useDemoStore((s) => s.select)
  const resolve = useDemoStore((s) => s.resolve)
  // Surfaced here too, not only in ReplayBody's header: DemoPanels is also
  // embedded inside LiveChrome, which has no header banner of its own, and
  // a rejected resolve() (see live-source.ts: no engine tool resolves a
  // finding by id yet) must not fail silently there.
  const error = useDemoStore((s) => s.error)

  const [tab, setTab] = useState<Tab>('findings')
  const [compareResult, setCompareResult] = useState<CompareResult | null>(null)
  const [comparePending, setComparePending] = useState(false)
  const [compareQuestions, setCompareQuestions] = useState<string[]>([])

  // The scripted question list for the compare picker. CompareSource has no
  // "list questions" method by design (see compare-source.ts: it answers
  // one question at a time), so this reads the same precomputed bundle
  // replay mode's CompareSource reads internally, purely to populate the
  // picker. Live mode has no scripted set (getCompareSource() throws
  // there), so the picker stays empty and the panel explains why on ask.
  useEffect(() => {
    let cancelled = false
    if (!isLive) {
      fetch('./precomputed/bundle.json')
        .then((r) => r.json())
        .then((bundle: { compare?: Record<string, unknown> }) => {
          if (cancelled) return
          if (bundle.compare && typeof bundle.compare === 'object') {
            setCompareQuestions(Object.keys(bundle.compare))
          }
        })
        .catch(() => {})
    }
    return () => {
      cancelled = true
    }
  }, [])

  const handleAsk = useCallback(async (q: string) => {
    setComparePending(true)
    try {
      const src = await getCompareSource()
      const result = await src.compare(q)
      setCompareResult(result)
    } catch (e) {
      setCompareResult({
        question: q,
        grounded: { answer: e instanceof Error ? e.message : String(e), citations: [] },
        baseline: { answer: '', citations: [] },
        divergence: null,
      })
    } finally {
      setComparePending(false)
    }
  }, [])

  const selected = findings.find((f) => f.id === selectedFinding) ?? null

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="flex border-b text-xs shrink-0" style={{ borderColor: 'var(--border)' }}>
        {(['findings', 'corpus', 'compare'] as Tab[]).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className="flex-1 px-2 py-2 capitalize"
            style={{
              background: tab === t ? 'var(--bg-panel)' : 'transparent',
              color: tab === t ? 'var(--accent)' : 'var(--text-secondary)',
            }}
          >
            {t}
          </button>
        ))}
      </div>

      {error && (
        <div className="text-xs p-2" style={{ color: 'var(--error)', background: 'var(--bg-panel)' }}>
          {error}
        </div>
      )}

      <div className="flex-1 overflow-hidden flex flex-col">
        {tab === 'findings' && (
          <div className="flex-1 flex flex-col overflow-hidden">
            <div className="flex-1 overflow-y-auto">
              <FindingsPanel findings={findings} selected={selectedFinding} onSelect={select} />
            </div>
            <div className="h-72 border-t overflow-hidden" style={{ borderColor: 'var(--border)' }}>
              <ResolutionPanel finding={selected} ledger={ledger} onResolve={resolve} />
            </div>
          </div>
        )}
        {tab === 'corpus' && (
          <CorpusPanel documents={documents} error={documentsError} onOpen={() => {}} />
        )}
        {tab === 'compare' && (
          <ComparePanel
            result={compareResult}
            onAsk={handleAsk}
            questions={compareQuestions}
            pending={comparePending}
          />
        )}
      </div>
    </div>
  )
}

/** The replay build's whole shell: the 3D graph plus the shared demo panels. */
function ReplayBody() {
  const graph = useDemoStore((s) => s.graph)
  const error = useDemoStore((s) => s.error)
  const loading = useDemoStore((s) => s.loading)

  return (
    <div className="h-screen flex flex-col" style={{ background: 'var(--bg-primary)' }}>
      <div
        className="h-10 flex items-center px-4 border-b gap-3"
        style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}
      >
        <span className="text-sm font-semibold" style={{ color: 'var(--accent)' }}>
          Open Ontologies
        </span>
        <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>
          Offline replay: no engine, no model, no network
        </span>
        {loading && (
          <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>
            Loading&hellip;
          </span>
        )}
        {error && (
          <span className="text-xs ml-auto" style={{ color: 'var(--error)' }}>
            {error}
          </span>
        )}
      </div>
      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 relative">
          <Graph3D graph={graph} onNodeSelect={() => {}} />
        </div>
        <div
          className="w-96 border-l flex flex-col"
          style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}
        >
          <DemoPanels />
        </div>
      </div>
    </div>
  )
}

/**
 * The single entry point for the whole studio interface. Both build targets
 * mount this and nothing else: it decides which chrome to add around the
 * shared demonstration body (Graph3D + DemoPanels), and every component
 * beneath it takes data and callbacks as props rather than reaching for a
 * source directly, so the same tree renders live against the engine and
 * offline from the precomputed artifacts.
 */
export function AppShell() {
  const load = useDemoStore((s) => s.load)

  useEffect(() => {
    load()
  }, [load])

  if (!isLive) return <ReplayBody />

  return (
    <Suspense fallback={<ReplayBody />}>
      <LiveChrome>
        <DemoPanels />
      </LiveChrome>
    </Suspense>
  )
}
