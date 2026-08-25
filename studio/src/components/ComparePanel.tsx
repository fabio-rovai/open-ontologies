import type { CompareResult } from '../lib/compare-source'

export interface ComparePanelProps {
  result: CompareResult | null
  onAsk: (q: string) => void
  // The scripted question set, in the exact order the comparison data
  // carries them. Rendered as-is: no reordering, no filtering, and no
  // indication of which side "won" on any question. One of the five has the
  // grounded answer wrong and the plain baseline right, and it must render
  // exactly like every other row, in its own place in this order.
  questions?: string[]
  pending?: boolean
}

/**
 * The same question answered twice, side by side: once grounded in the
 * ontology with citations into the corpus, once by a plain keyword-chunk
 * baseline. The divergence note sits between the two columns. Imports no
 * source; whether a question is scripted, and which side is right, comes
 * entirely from `result`.
 */
export function ComparePanel({ result, onAsk, questions = [], pending }: ComparePanelProps) {
  return (
    <div className="flex flex-col h-full overflow-y-auto">
      {questions.length > 0 && (
        <div className="p-3 border-b flex flex-col gap-1" style={{ borderColor: 'var(--border)' }}>
          <span className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--text-secondary)' }}>
            Scripted questions
          </span>
          {questions.map((q) => (
            <button
              key={q}
              onClick={() => onAsk(q)}
              className="text-left text-xs px-2 py-1.5 rounded"
              style={{
                background: result?.question === q ? 'var(--bg-panel)' : 'transparent',
                color: 'var(--text-primary)',
              }}
            >
              {q}
            </button>
          ))}
        </div>
      )}

      <div className="p-3 flex-1">
        {pending && (
          <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            Running both answers&hellip;
          </p>
        )}
        {!pending && !result && (
          <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            Pick a question above to compare a grounded answer against a plain baseline.
          </p>
        )}
        {!pending && result && (
          <div className="space-y-3">
            <div className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>
              {result.question}
            </div>

            {/* status is an explicit discriminant (compare-source.ts): a
                failed or unscripted comparison renders as a status line, not
                under either column's heading, so it can never read as a
                grounded win with an empty baseline. */}
            {result.status !== 'ok' ? (
              <div
                className="text-sm p-3 rounded"
                style={{
                  background: 'var(--bg-panel)',
                  color: result.status === 'error' ? 'var(--error)' : 'var(--text-secondary)',
                }}
              >
                {result.status === 'error' ? `Comparison failed: ${result.grounded.answer}` : result.grounded.answer}
              </div>
            ) : (
              <>
                <div className="grid grid-cols-2 gap-3">
                  <AnswerColumn label="Grounded (ontology)" answer={result.grounded} />
                  <AnswerColumn label="Baseline (plain retrieval)" answer={result.baseline} />
                </div>

                <div className="text-xs p-2 rounded" style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)' }}>
                  {result.divergence ?? 'No divergence recorded: this question was not in the scripted set.'}
                </div>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  )
}

function AnswerColumn({ label, answer }: { label: string; answer: { answer: string; citations: string[] } }) {
  return (
    <div className="p-2 rounded" style={{ background: 'var(--bg-panel)' }}>
      <div className="text-xs font-semibold mb-1" style={{ color: 'var(--accent)' }}>
        {label}
      </div>
      <p className="text-xs whitespace-pre-wrap" style={{ color: 'var(--text-primary)' }}>
        {answer.answer || '(no answer)'}
      </p>
      {answer.citations.length > 0 && (
        <ul className="mt-2 space-y-0.5 text-xs">
          {answer.citations.map((c) => (
            <li key={c} style={{ color: 'var(--text-secondary)' }}>
              &middot; {c}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
