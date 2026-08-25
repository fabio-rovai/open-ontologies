import { useEffect, useState, useCallback, lazy, Suspense } from 'react'
import { useDemoStore } from './state/demo-store'
import { CorpusPanel } from './components/CorpusPanel'
import { FindingsPanel } from './components/FindingsPanel'
import { ResolutionPanel } from './components/ResolutionPanel'
import { ComparePanel } from './components/ComparePanel'
import { ScriptedChatPanel } from './components/ScriptedChatPanel'
import { ValidationPanel } from './components/ValidationPanel'
import { Graph3D } from './components/Graph3D'
import { chooseSourceKind } from './lib/source-factory'
import { getCompareSource, compareError, type CompareResult } from './lib/compare-source'
import { getValidationSource, type ValidationFixtures } from './lib/validation-source'
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

type Tab = 'findings' | 'corpus' | 'compare' | 'validation' | 'chat'

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
  const chat = useDemoStore((s) => s.chat)
  const chatPending = useDemoStore((s) => s.chatPending)
  const ask = useDemoStore((s) => s.ask)
  // Surfaced here too, not only in ReplayBody's header: DemoPanels is also
  // embedded inside LiveChrome, which has no header banner of its own, and
  // a rejected resolve() (see live-source.ts: no engine tool resolves a
  // finding by id yet) must not fail silently there.
  const error = useDemoStore((s) => s.error)

  const [tab, setTab] = useState<Tab>('findings')
  const [compareResult, setCompareResult] = useState<CompareResult | null>(null)
  const [comparePending, setComparePending] = useState(false)
  const [compareQuestions, setCompareQuestions] = useState<string[]>([])
  const [chatQuestions, setChatQuestions] = useState<string[]>([])
  const [validationFixtures, setValidationFixtures] = useState<ValidationFixtures | null>(null)
  const [validationError, setValidationError] = useState<string | null>(null)
  const [validationPending, setValidationPending] = useState(false)

  // The scripted question lists for the compare and chat pickers. Neither
  // CompareSource nor DemoSource has a "list questions" method by design
  // (compare-source.ts answers one question at a time; ask() in
  // demo-source.ts streams chunks for one question), so this reads the same
  // precomputed bundle replay mode's sources read internally, purely to
  // populate the pickers. Live mode has no scripted set for either (a live
  // compare throws, and live ask() throws -- see compare-source.ts and
  // live-source.ts), so both pickers stay empty there and each panel
  // explains why on ask.
  useEffect(() => {
    let cancelled = false
    if (!isLive) {
      fetch('./precomputed/bundle.json')
        .then((r) => r.json())
        .then((bundle: { compare?: Record<string, unknown>; chat?: Record<string, unknown> }) => {
          if (cancelled) return
          if (bundle.compare && typeof bundle.compare === 'object') {
            setCompareQuestions(Object.keys(bundle.compare))
          }
          if (bundle.chat && typeof bundle.chat === 'object') {
            setChatQuestions(Object.keys(bundle.chat))
          }
        })
        .catch(() => {})
    }
    return () => {
      cancelled = true
    }
  }, [])

  // The validation panel's two runs are static: there is no question to
  // ask, so they load once on mount rather than on a user action, the way
  // graph and findings load in demo-store.ts. In live mode getValidationSource()
  // throws immediately (see validation-source.ts), and that message is
  // shown as-is rather than swallowed, the same honesty compare-source.ts's
  // live branch already applies.
  useEffect(() => {
    let cancelled = false
    setValidationPending(true)
    getValidationSource()
      .then((src) => src.runs())
      .then((fixtures) => {
        if (!cancelled) setValidationFixtures(fixtures)
      })
      .catch((e) => {
        if (!cancelled) setValidationError(e instanceof Error ? e.message : String(e))
      })
      .finally(() => {
        if (!cancelled) setValidationPending(false)
      })
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
      // status: 'error' is an explicit discriminant (compare-source.ts's
      // compareError): ComparePanel renders it as a status line, never under
      // the "Grounded" column heading, so a failed comparison cannot read as
      // a grounded win with no baseline answer.
      setCompareResult(compareError(q, e))
    } finally {
      setComparePending(false)
    }
  }, [])

  const selected = findings.find((f) => f.id === selectedFinding) ?? null

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="flex border-b text-xs shrink-0" style={{ borderColor: 'var(--border)' }}>
        {(['findings', 'corpus', 'compare', 'validation', 'chat'] as Tab[]).map((t) => (
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
        {tab === 'validation' && (
          <ValidationPanel fixtures={validationFixtures} error={validationError} pending={validationPending} />
        )}
        {tab === 'chat' && (
          <ScriptedChatPanel chat={chat} pending={chatPending} onAsk={ask} questions={chatQuestions} />
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

// While LiveChrome's lazy chunk is still loading, the desktop app has an
// engine, a model and a network -- it just has not finished loading its own
// chrome yet. ReplayBody's header claims the opposite ("Offline replay: no
// engine, no model, no network"), which briefly lies to a live session on
// every load. This fallback is genuinely neutral instead.
function LoadingShell() {
  return (
    <div
      className="h-screen flex items-center justify-center"
      style={{ background: 'var(--bg-primary)' }}
    >
      <span className="text-sm" style={{ color: 'var(--text-secondary)' }}>
        Loading&hellip;
      </span>
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
    <Suspense fallback={<LoadingShell />}>
      <LiveChrome>
        <DemoPanels />
      </LiveChrome>
    </Suspense>
  )
}
