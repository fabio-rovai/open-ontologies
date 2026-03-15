import { useState, useEffect, useCallback } from 'react';
import * as mcp from '../lib/mcp-client';

const TYPE_COLORS: Record<string, string> = {
  P:  '#89b4fa', // plan
  A:  '#a6e3a1', // apply
  E:  '#f9e2af', // enforce
  D:  '#cba6f7', // drift
  M:  '#fab387', // monitor
  AL: '#89dceb', // align
  AF: '#89dceb',
  LF: '#a6adc8',
  EF: '#a6adc8',
};

const OP_ICONS: Record<string, string> = {
  plan:             '📋',
  apply:            '✅',
  enforce:          '🛡',
  drift:            '📊',
  monitor:          '👁',
  align:            '🔗',
  align_feedback:   '🗳',
  lint_feedback:    '🗳',
  enforce_feedback: '🗳',
  load:             '📥',
  validate:         '✔',
  save:             '💾',
  reason:           '🧠',
  clear:            '🗑',
};

function formatTs(ts: string): string {
  const n = parseInt(ts, 10);
  if (isNaN(n)) return ts;
  return new Date(n * 1000).toLocaleTimeString();
}

function shortSession(s: string): string {
  return s.slice(0, 6);
}

export function LineagePanel() {
  const [events, setEvents] = useState<mcp.LineageEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [groupBySession, setGroupBySession] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const evts = await mcp.getLineage();
    setEvents(evts);
    setLoading(false);
  }, []);

  useEffect(() => { load(); }, [load]);

  // Group by session
  const sessions = groupBySession
    ? [...new Map(events.map(e => [e.session, true])).keys()]
    : [];

  return (
    <div className="flex flex-col h-full text-xs" style={{ color: 'var(--text-primary)' }}>
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2 border-b shrink-0"
           style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
        <span className="font-medium">Lineage</span>
        <button onClick={load} className="ml-auto px-2 py-0.5 rounded"
                style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)' }}
                title="Refresh">↺</button>
        <button
          onClick={() => setGroupBySession(g => !g)}
          className="px-2 py-0.5 rounded"
          style={{
            background: groupBySession ? 'var(--accent)' : 'var(--bg-panel)',
            color: groupBySession ? 'var(--bg-primary)' : 'var(--text-secondary)',
          }}>Sessions</button>
      </div>

      <div className="flex-1 overflow-y-auto px-2 py-2 space-y-1">
        {loading && (
          <div style={{ color: 'var(--text-secondary)' }} className="px-1">Loading…</div>
        )}

        {!loading && events.length === 0 && (
          <div style={{ color: 'var(--text-secondary)' }} className="px-1">
            No lineage events yet. Build or modify an ontology to see the trail.
          </div>
        )}

        {groupBySession
          ? sessions.map(sid => {
              const sessionEvents = events.filter(e => e.session === sid);
              return (
                <div key={sid} className="mb-3">
                  <div className="px-1 py-0.5 mb-1 rounded font-mono font-medium"
                       style={{ background: 'var(--bg-primary)', color: 'var(--accent)' }}>
                    Session {shortSession(sid)}
                    <span className="ml-2 font-normal" style={{ color: 'var(--text-secondary)' }}>
                      {sessionEvents.length} events
                    </span>
                  </div>
                  {sessionEvents.map((e, i) => (
                    <EventRow key={i} event={e} />
                  ))}
                </div>
              );
            })
          : events.map((e, i) => <EventRow key={i} event={e} />)
        }
      </div>
    </div>
  );
}

function EventRow({ event: e }: { event: mcp.LineageEvent }) {
  const color = TYPE_COLORS[e.type] ?? '#a6adc8';
  const icon = OP_ICONS[e.op] ?? '•';

  return (
    <div className="flex items-start gap-2 px-2 py-1 rounded"
         style={{ background: 'var(--bg-primary)' }}>
      <span className="shrink-0 w-4 text-center">{icon}</span>
      <span className="shrink-0 font-mono px-1 rounded text-xs"
            style={{ background: color + '22', color }}>
        {e.op}
      </span>
      {e.details && (
        <span className="flex-1 truncate" style={{ color: 'var(--text-secondary)' }}
              title={e.details}>
          {e.details}
        </span>
      )}
      <span className="shrink-0 ml-auto" style={{ color: 'var(--text-secondary)' }}>
        {formatTs(e.ts)}
      </span>
    </div>
  );
}
