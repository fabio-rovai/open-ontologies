import type { Contradiction, Decision } from '../lib/demo-source'

export interface ResolutionPanelProps {
  finding: Contradiction | null
  ledger: { id: string; decision: Decision }[]
  onResolve: (id: string, decision: Decision) => void
}

/**
 * The selected finding's claims, with accept/reject controls, and the
 * ledger of decisions made so far. Takes data and a callback only, and
 * imports no source: whether a decision actually persists anywhere (it
 * does not in live mode today, see live-source.ts's resolve()) is the
 * store's problem, surfaced through `error`, not this panel's.
 */
export function ResolutionPanel({ finding, ledger, onResolve }: ResolutionPanelProps) {
  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="p-3">
        {!finding ? (
          <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            Select a finding to review its claims.
          </p>
        ) : (
          <>
            <div className="font-mono text-xs break-all mb-2" style={{ color: 'var(--text-primary)' }}>
              {finding.subject}
            </div>
            <ul className="space-y-2 text-xs">
              {finding.claims.map((c, i) => (
                <li key={i} className="p-2 rounded" style={{ background: 'var(--bg-panel)' }}>
                  <div style={{ color: 'var(--accent)' }}>{c.document}</div>
                  <div style={{ color: 'var(--text-secondary)' }}>
                    {c.predicate}: <span style={{ color: 'var(--text-primary)' }}>{c.object}</span>
                  </div>
                </li>
              ))}
            </ul>
            <div className="flex gap-2 mt-3">
              <button
                onClick={() => onResolve(finding.id, { kind: 'accept' })}
                className="text-xs px-3 py-1.5 rounded font-medium"
                style={{ background: 'var(--success)', color: 'var(--bg-primary)' }}
              >
                Accept
              </button>
              <button
                onClick={() => onResolve(finding.id, { kind: 'reject' })}
                className="text-xs px-3 py-1.5 rounded font-medium"
                style={{ background: 'var(--error)', color: 'var(--bg-primary)' }}
              >
                Reject
              </button>
            </div>
          </>
        )}
      </div>

      <div className="mt-auto border-t p-3" style={{ borderColor: 'var(--border)' }}>
        <div className="text-xs font-semibold uppercase tracking-wide mb-2" style={{ color: 'var(--text-secondary)' }}>
          Ledger {ledger.length > 0 ? `(${ledger.length})` : ''}
        </div>
        {ledger.length === 0 ? (
          <p className="text-xs" style={{ color: 'var(--text-secondary)' }}>
            No decisions recorded yet.
          </p>
        ) : (
          <ol className="space-y-1 text-xs">
            {ledger.map((entry, i) => (
              <li key={i} style={{ color: 'var(--text-secondary)' }}>
                <span className="font-mono">{entry.id}</span>{' '}
                <span
                  style={{ color: entry.decision.kind === 'accept' ? 'var(--success)' : 'var(--error)' }}
                >
                  {entry.decision.kind}
                </span>
              </li>
            ))}
          </ol>
        )}
      </div>
    </div>
  )
}
