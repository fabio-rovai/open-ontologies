import { useEffect, useState } from 'react'
import type { Contradiction, DemoSource, Document, GraphView } from './lib/demo-source'
import { getDemoSource } from './lib/source-factory'
import './App.css'

/**
 * The static web build's entry surface. Task 12 wires the full desktop
 * Layout (TreeView, Graph3D, ChatPanel, ...) to DemoSource; until then this
 * is the minimal proof that the construction site in source-factory.ts
 * actually works end to end: no engine, no Tauri, no network, and the
 * corpus, graph and findings still render from the committed artifacts.
 */
export function PublicDemo() {
  const [source, setSource] = useState<DemoSource | null>(null)
  const [corpus, setCorpus] = useState<Document[] | null>(null)
  const [graph, setGraph] = useState<GraphView | null>(null)
  const [findings, setFindings] = useState<Contradiction[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    getDemoSource()
      .then(async (src) => {
        if (cancelled) return
        setSource(src)
        const [c, g, f] = await Promise.all([src.corpus(), src.graph(), src.findings()])
        if (cancelled) return
        setCorpus(c)
        setGraph(g)
        setFindings(f)
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e))
      })
    return () => {
      cancelled = true
    }
  }, [])

  if (error) {
    return (
      <div className="p-8" style={{ color: 'var(--error)' }}>
        <h1 className="text-lg font-semibold mb-2">Could not load the demonstration</h1>
        <p>{error}</p>
      </div>
    )
  }

  return (
    <div className="min-h-screen p-8 mx-auto" style={{ maxWidth: 960, color: 'var(--text-primary)' }}>
      <header className="mb-8">
        <h1 className="text-xl font-semibold" style={{ color: 'var(--accent)' }}>
          Open Ontologies &mdash; offline demonstration
        </h1>
        <p className="text-sm mt-1" style={{ color: 'var(--text-secondary)' }}>
          Replayed entirely from committed artifacts. No engine, no model, no network call.
        </p>
      </header>

      <section className="mb-8">
        <h2 className="text-sm font-semibold uppercase tracking-wide mb-3" style={{ color: 'var(--text-secondary)' }}>
          Corpus {corpus ? `(${corpus.length} documents)` : ''}
        </h2>
        {!corpus && <p style={{ color: 'var(--text-secondary)' }}>Loading&hellip;</p>}
        <ul className="space-y-1 text-sm">
          {corpus?.map((doc) => (
            <li key={doc.id} className="px-3 py-1.5 rounded" style={{ background: 'var(--bg-panel)' }}>
              {doc.title}
            </li>
          ))}
        </ul>
      </section>

      <section className="mb-8">
        <h2 className="text-sm font-semibold uppercase tracking-wide mb-3" style={{ color: 'var(--text-secondary)' }}>
          Graph
        </h2>
        {!graph && <p style={{ color: 'var(--text-secondary)' }}>Loading&hellip;</p>}
        {graph && (
          <p className="text-sm" style={{ color: 'var(--text-primary)' }}>
            {graph.classes.length} classes, {graph.properties.length} properties, {graph.edges.length} edges
          </p>
        )}
      </section>

      <section>
        <h2 className="text-sm font-semibold uppercase tracking-wide mb-3" style={{ color: 'var(--text-secondary)' }}>
          Findings {findings ? `(${findings.length})` : ''}
        </h2>
        {!findings && <p style={{ color: 'var(--text-secondary)' }}>Loading&hellip;</p>}
        <ul className="space-y-3 text-sm">
          {findings?.map((f) => (
            <li key={f.id} className="px-3 py-2 rounded" style={{ background: 'var(--bg-panel)' }}>
              <div className="font-medium" style={{ color: 'var(--accent)' }}>
                {f.subject} <span style={{ color: 'var(--text-secondary)' }}>[{f.kind}]</span>
              </div>
              <ul className="mt-1 space-y-0.5">
                {f.claims.map((c, i) => (
                  <li key={i} style={{ color: 'var(--text-secondary)' }}>
                    <span style={{ color: 'var(--text-primary)' }}>{c.document}</span> {c.predicate}: {c.object}
                  </li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      </section>

      {source && (
        <p className="mt-8 text-xs" style={{ color: 'var(--text-secondary)' }}>
          Source ready. The full desktop application (open-ontologies studio) adds live graph
          exploration and chat over a running engine.
        </p>
      )}
    </div>
  )
}
