import { describe, it, expect } from 'vitest'
import bundleJson from '../../../../demo/precomputed/bundle.json'
import { legendParts, legendText } from '../graph-legend'
import { toGraphData, warrantCounts } from '../graph3d-model'
import type { GraphView, Warrant } from '../demo-source'

// The replay legend is fixed text, and the graph it is drawn over is the
// committed bundle. Before #425 the text said "ies-core.ttl · 1,083 triples"
// and named three warrants over a graph of 28 DCAT-US classes with none. These
// read the bundle the replay build ships and hold the text to it.
type Drawn = { asserted: number; certified: number; rejected: number }
const graph = (bundleJson as unknown as {
  graph: GraphView & { source: { file: string; triples: number }; evidence: { drawn: Drawn } }
}).graph
const warrants = graph.edges.map((e) => (e.warrant ?? 'asserted') as Warrant)

describe('the replay legend over the replay bundle', () => {
  it('names the file and the triple count the bundle was built from', () => {
    const { file, triples } = graph.source
    expect(legendText({ kind: 'replay' }).caption).toBe(`${file} · ${triples.toLocaleString('en-GB')} triples`)
    expect(legendText({ kind: 'replay' }).asserted).toBe(`read from ${file}. claimed by a person`)
  })

  it('has an edge for every warrant its rows name', () => {
    const parts = legendParts({ kind: 'replay' }, warrants)
    expect(parts.proof).toBe(true)
    for (const row of parts.rows) expect(warrants).toContain(row.w)
  })

  // The verification layer's three edges are drawn in warrant colours, but
  // they are not triples of ies-core.ttl nor conclusions of its certificate.
  it('prints the counts of the ontology edges the generator recorded', () => {
    const { nodes, links } = toGraphData(graph, { kind: 'replay' })
    const { asserted, certified, rejected } = graph.evidence.drawn
    expect(warrantCounts(nodes, links)).toEqual({ asserted, certified, rejected })
  })

  it('draws every edge, with the Lean checker as the kernel', () => {
    const { nodes, links } = toGraphData(graph, { kind: 'replay' })
    expect(links.length).toBe(graph.edges.length)
    expect(nodes.filter((n) => n.role === 'kernel').map((n) => n.name)).toEqual(['Lean 4 · oo-cert'])
  })
})
