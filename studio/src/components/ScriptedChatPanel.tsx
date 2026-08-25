import { useState } from 'react'
import type { Chunk } from '../lib/demo-source'

export interface ScriptedChatPanelProps {
  chat: Chunk[]
  pending?: boolean
  onAsk: (question: string) => void
  // The scripted question set, read from the same chat.json keys
  // replay-source.ts's ask() looks up. Empty in live mode: there is no
  // fixed set there, and DemoSource.ask() throws with an honest message
  // instead (see live-source.ts) that this panel just displays like any
  // other 'unscripted' chunk.
  questions?: string[]
}

/**
 * The scripted-Q&A chat surface, backed by demo/precomputed/chat.json (built
 * mechanically from compare.json's grounded half by build_precomputed.py --
 * see its module docstring). Distinct from ChatPanel.tsx, which drives a
 * live model conversation over the agent sidecar and is desktop-only: this
 * reads through the same DemoSource.ask() every other panel in this file
 * uses, so it works in the static web replay build with no engine and no
 * model.
 *
 * Renders on Chunk.type, never guesses from string content: 'tool_call'
 * chunks render as a small mono trace line, 'unscripted' chunks (the
 * fixture's own honest "not scripted" fallback, or live mode's thrown-and-
 * caught error) render as a status line, and only 'text' renders as an
 * answer.
 */
export function ScriptedChatPanel({ chat, pending, onAsk, questions = [] }: ScriptedChatPanelProps) {
  const [input, setInput] = useState('')

  const send = (q: string) => {
    const trimmed = q.trim()
    if (!trimmed) return
    onAsk(trimmed)
    setInput('')
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {questions.length > 0 && (
        <div className="p-3 border-b flex flex-col gap-1 shrink-0" style={{ borderColor: 'var(--border)' }}>
          <span className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--text-secondary)' }}>
            Scripted questions
          </span>
          {questions.map((q) => (
            <button
              key={q}
              onClick={() => send(q)}
              className="text-left text-xs px-2 py-1.5 rounded"
              style={{ background: 'transparent', color: 'var(--text-primary)' }}
            >
              {q}
            </button>
          ))}
        </div>
      )}

      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {chat.length === 0 && !pending && (
          <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            Ask a question, or pick one of the scripted questions above.
          </p>
        )}
        {chat.map((chunk, i) => (
          <ChunkView key={i} chunk={chunk} />
        ))}
        {pending && (
          <p className="text-xs" style={{ color: 'var(--text-secondary)' }}>
            Running&hellip;
          </p>
        )}
      </div>

      <div className="p-3 border-t flex gap-2 shrink-0" style={{ borderColor: 'var(--border)' }}>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') send(input)
          }}
          placeholder="Ask a scripted question..."
          className="flex-1 px-2 py-1.5 text-xs rounded outline-none"
          style={{ background: 'var(--bg-panel)', color: 'var(--text-primary)', border: '1px solid var(--border)' }}
        />
        <button
          onClick={() => send(input)}
          disabled={!input.trim() || pending}
          className="px-2 py-1.5 text-xs rounded font-medium"
          style={{
            background: input.trim() && !pending ? 'var(--accent)' : 'var(--bg-panel)',
            color: input.trim() && !pending ? 'var(--bg-primary)' : 'var(--text-secondary)',
          }}
        >
          Ask
        </button>
      </div>
    </div>
  )
}

function ChunkView({ chunk }: { chunk: Chunk }) {
  if (chunk.type === 'tool_call') {
    return (
      <div
        className="text-xs px-2 py-1 rounded font-mono"
        style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)' }}
      >
        {chunk.value}
      </div>
    )
  }
  if (chunk.type === 'unscripted') {
    return (
      <div className="text-xs p-2 rounded" style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)' }}>
        {chunk.value}
      </div>
    )
  }
  return (
    <div className="text-sm whitespace-pre-wrap" style={{ color: 'var(--text-primary)' }}>
      {chunk.value}
    </div>
  )
}
