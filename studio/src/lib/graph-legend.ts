/**
 * Where the graph Graph3D is drawing came from, as far as its legend needs to
 * say (#414).
 *
 * `replay` is the offline demonstration, built from ies-core. `live` is
 * whatever the engine holds: `file` is the name the desktop toolbar shows
 * (LiveChrome's project name, `studio-live.ttl` by default) and `triples` is
 * the `triples` key of the engine's `GET /api/stats` (src/graph.rs,
 * `get_stats`), read through `getStats()` in mcp-client.ts. Either may be
 * unknown.
 */
export type GraphOrigin =
  | { kind: 'replay' }
  | { kind: 'live'; file?: string; triples?: number }

export interface LegendText {
  /** Shown beside the legend heading. Null when nothing about the source is known. */
  caption: string | null
  /** What the ASSERTED row says an asserted edge is. */
  asserted: string
}

/** The replay bundle is built from ies-core.ttl, and these are its facts. */
const REPLAY: LegendText = {
  caption: 'ies-core.ttl · 1,083 triples',
  asserted: 'read from ies-core.ttl. claimed by a person',
}

/**
 * The text of Graph3D's legend for the graph on screen.
 *
 * In live mode it names only what is known. A count of zero is treated as
 * unknown: `getStats()` reports zero when `/api/stats` fails, and the legend
 * is drawn only over a graph that has nodes, so zero is never the store's
 * real size there.
 */
export function legendText(origin: GraphOrigin): LegendText {
  if (origin.kind === 'replay') return REPLAY
  const file = origin.file?.trim() || undefined
  const triples =
    typeof origin.triples === 'number' && Number.isFinite(origin.triples) && origin.triples > 0
      ? origin.triples
      : undefined
  const parts: string[] = []
  if (file) parts.push(file)
  if (triples !== undefined) {
    parts.push(`${triples.toLocaleString('en-GB')} ${triples === 1 ? 'triple' : 'triples'}`)
  }
  return {
    caption: parts.length > 0 ? parts.join(' · ') : null,
    asserted: file ? `read from ${file}. claimed by a person` : 'claimed by a person',
  }
}
