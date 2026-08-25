import type { ValidationFixtures, ValidationRun } from '../lib/validation-source'

export interface ValidationPanelProps {
  fixtures: ValidationFixtures | null
  error: string | null
  pending?: boolean
}

/**
 * The moment the rest of the replay cannot show without an engine: the same
 * unchanged SHACL shapes file, run against the corpus twice. Once as
 * GSA/dcat-us actually publishes it -- 0 focus nodes, meaning the shapes
 * selected nothing to check -- and once with the schema-derived binding
 * applied, where 228 focus nodes are actually reached and the run fails.
 *
 * The zero-focus-node run is the whole point of this panel and is rendered
 * as the loudest thing on it, never as a pass with a footnote: `verdict` is
 * either 'undetermined' or 'fails', there is no 'passes' case in the type
 * (validation-source.ts), and this component does not invent one. No
 * violation count appears anywhere here -- ValidationRun does not carry
 * one, so there is nothing for this component to render even by accident.
 */
export function ValidationPanel({ fixtures, error, pending }: ValidationPanelProps) {
  if (pending) {
    return (
      <p className="p-4 text-sm" style={{ color: 'var(--text-secondary)' }}>
        Loading the validation runs&hellip;
      </p>
    )
  }
  if (error) {
    return (
      <p className="p-4 text-sm" style={{ color: 'var(--error)' }}>
        {error}
      </p>
    )
  }
  if (!fixtures) {
    return (
      <p className="p-4 text-sm" style={{ color: 'var(--text-secondary)' }}>
        No validation runs available.
      </p>
    )
  }

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="p-3 border-b" style={{ borderColor: 'var(--border)' }}>
        <div className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--text-secondary)' }}>
          Same shapes file, two corpus states
        </div>
        <div className="text-xs mt-1 font-mono break-all" style={{ color: 'var(--text-secondary)' }}>
          {fixtures.shapesFile}, unchanged in both runs &middot; measured {fixtures.measured}
        </div>
      </div>
      <div className="p-3 flex flex-col gap-3">
        {fixtures.runs.map((run) => (
          <ValidationRunCard key={run.id} run={run} />
        ))}
      </div>
    </div>
  )
}

function ValidationRunCard({ run }: { run: ValidationRun }) {
  const vacuous = run.verdict === 'undetermined'
  return (
    <div
      className="rounded p-3"
      style={{
        background: 'var(--bg-panel)',
        border: `2px solid ${vacuous ? 'var(--error)' : 'var(--border)'}`,
      }}
    >
      <div className="text-sm font-semibold" style={{ color: 'var(--text-primary)' }}>
        {run.label}
      </div>
      <p className="text-xs mt-1" style={{ color: 'var(--text-secondary)' }}>
        {run.corpusDescription}
      </p>

      {/* Focus nodes is the loudest thing on the card: a large numeral, not
          a line item next to a green tick. For the vacuous run this is 0,
          and 0 is rendered exactly as prominently as 228 -- neither number
          is hidden or shrunk to make the panel look tidier. */}
      <div className="mt-3 flex items-baseline gap-2">
        <span
          className="text-3xl font-bold tabular-nums"
          style={{ color: vacuous ? 'var(--error)' : 'var(--text-primary)' }}
        >
          {run.focusNodes}
        </span>
        <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>
          focus node{run.focusNodes === 1 ? '' : 's'} &middot; {run.matchedClassCount} of{' '}
          {run.targetClassCount} target classes matched &middot; {run.dataTriples} data triples
        </span>
      </div>

      {vacuous ? (
        <div
          className="mt-3 p-2 rounded text-xs font-semibold"
          style={{ background: 'var(--error)', color: 'var(--bg-primary)' }}
        >
          NOT A PASS. The validator checked nothing.
        </div>
      ) : (
        <div
          className="mt-3 p-2 rounded text-xs font-semibold"
          style={{ background: 'var(--error)', color: 'var(--bg-primary)' }}
        >
          FAILS. Conforms = false.
        </div>
      )}

      {run.reason && (
        <p className="mt-2 text-xs" style={{ color: 'var(--text-primary)' }}>
          {run.reason}
        </p>
      )}

      <p className="mt-2 text-xs" style={{ color: 'var(--text-secondary)' }}>
        conforms (as reported by the run, before this panel&apos;s reading of it): {String(run.conformsRaw)}
      </p>
    </div>
  )
}
