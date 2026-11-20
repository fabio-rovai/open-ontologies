import type { Warrant } from './demo-source'

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

export interface LegendRow {
  w: Warrant
  label: string
  /** What an edge drawn with this warrant is. */
  means: string
}

export interface LegendParts {
  /**
   * Whether the proof-carrying parts are drawn. When true the heading says so
   * and the legend ends with the Lean 4 / Isabelle footer naming who judged
   * the graph.
   */
  proof: boolean
  heading: string
  caption: string | null
  rows: LegendRow[]
}

/**
 * Everything Graph3D's legend draws for the graph on screen (#424).
 *
 * Its proof-carrying parts (the heading, the CERTIFIED and REJECTED rows and
 * the footer naming Lean 4 and Isabelle) describe warrants, and only the
 * replay demonstration is presented as carrying them: it keeps every part,
 * exactly as before, whatever its edges hold. The live source sets no warrant
 * on any edge, so in live mode those parts appear only if an edge is
 * certified or rejected, and otherwise the legend is the ASSERTED row alone.
 */
export function legendParts(origin: GraphOrigin, warrants: Iterable<Warrant>): LegendParts {
  const text = legendText(origin)
  const asserted: LegendRow = { w: 'asserted', label: 'ASSERTED', means: text.asserted }
  const proof =
    origin.kind === 'replay' || [...warrants].some((w) => w === 'certified' || w === 'rejected')
  if (!proof) {
    return { proof, heading: 'KNOWLEDGE GRAPH', caption: text.caption, rows: [asserted] }
  }
  return {
    proof,
    heading: 'PROOF-CARRYING INFERENCE',
    caption: text.caption,
    rows: [
      asserted,
      { w: 'certified', label: 'CERTIFIED', means: 'derived, then PROVED. OOCert.certificate_sound' },
      { w: 'rejected', label: 'REJECTED', means: 'forged. the checker exited 1 and named the rule' },
    ],
  }
}

/**
 * What Graph3D says over a graph with no classes. Replay keeps its text; live
 * mode has no Documents panel, and an ontology reaches its engine through
 * Chat (#424).
 */
export function emptyGraphText(origin: GraphOrigin): string {
  return origin.kind === 'replay'
    ? 'Build the knowledge graph first (Documents panel).'
    : 'No classes to draw yet. Load or build an ontology through Chat and it appears here.'
}
