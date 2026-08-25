import type { Contradiction } from '../lib/demo-source'

export interface FindingsPanelProps {
  findings: Contradiction[]
  selected: string | null
  onSelect: (id: string) => void
}

const KIND_LABEL: Record<Contradiction['kind'], string> = {
  conformance: 'conformance',
  'provenance-split': 'provenance split',
  disjointness: 'disjointness',
  typing: 'typing',
}

/**
 * Lists findings and cites the documents behind each one. Naming the
 * documents on every finding is the whole point of the panel: a count
 * without citations is not evidence. Takes data and a callback only, and
 * imports no source, so it renders identically whether the findings came
 * from a live SHACL run or the precomputed replay bundle.
 */
export function FindingsPanel({ findings, selected, onSelect }: FindingsPanelProps) {
  if (findings.length === 0) {
    return (
      <p className="p-4 text-sm" style={{ color: 'var(--text-secondary)' }}>
        No findings in this corpus.
      </p>
    )
  }
  return (
    <ul className="divide-y overflow-y-auto" style={{ borderColor: 'var(--border)' }}>
      {findings.map((f) => (
        <li
          key={f.id}
          onClick={() => onSelect(f.id)}
          className="cursor-pointer p-3"
          style={{
            borderColor: 'var(--border)',
            background: selected === f.id ? 'var(--bg-panel)' : 'transparent',
          }}
        >
          <div className="font-mono text-xs break-all" style={{ color: 'var(--text-primary)' }}>
            {f.subject}
          </div>
          <div className="text-xs mt-0.5" style={{ color: 'var(--accent)' }}>
            {KIND_LABEL[f.kind] ?? f.kind}
          </div>
          <ul className="mt-2 space-y-1 text-xs">
            {f.claims.map((c, i) => (
              <li key={i} style={{ color: 'var(--text-secondary)' }}>
                <span className="font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {c.document}
                </span>{' '}
                {c.predicate}: {c.object}
              </li>
            ))}
          </ul>
        </li>
      ))}
    </ul>
  )
}
