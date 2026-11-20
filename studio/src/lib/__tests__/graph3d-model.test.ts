import { describe, it, expect } from 'vitest'
import { toGraphData, type GNode } from '../graph3d-model'
import type { GraphView } from '../demo-source'

// A live ontology whose classes happen to carry the names of the replay
// demo's verification layer: a class `E` by local name, and classes labelled
// `Z3`, `certificate` and `Lean 4 · oo-cert`. Each is joined to an ordinary
// class by a subclass edge.
const NAMED_LIKE_TOOLS: GraphView = {
  classes: [
    { iri: 'http://example.org/onto#E' },
    { iri: 'http://example.org/onto#Solver', label: 'Z3' },
    { iri: 'http://example.org/onto#Cert', label: 'certificate' },
    { iri: 'http://example.org/onto#Kernel', label: 'Lean 4 · oo-cert' },
    { iri: 'http://example.org/onto#Thing' },
  ],
  properties: [],
  edges: [
    { source: 'http://example.org/onto#E', target: 'http://example.org/onto#Thing' },
    { source: 'http://example.org/onto#Solver', target: 'http://example.org/onto#Thing' },
    { source: 'http://example.org/onto#Cert', target: 'http://example.org/onto#Thing' },
    { source: 'http://example.org/onto#Kernel', target: 'http://example.org/onto#Thing' },
  ],
}

const byName = (nodes: GNode[], name: string): GNode => {
  const n = nodes.find((x) => x.name === name)
  if (!n) throw new Error(`no node named ${name}`)
  return n
}

// Graph3D matched its verification-layer nodes by display name, so in live
// mode a class named like one of them was pinned to the pipeline, left out of
// its component and tinted as a tool (#424).
describe('toGraphData in live mode', () => {
  const { nodes } = toGraphData(NAMED_LIKE_TOOLS, { kind: 'live', file: 'tools.ttl' })

  it('draws every class as a class, whatever it is called', () => {
    expect(nodes.map((n) => n.role)).toEqual(['class', 'class', 'class', 'class', 'class'])
  })

  it('pins none of them to the pipeline', () => {
    for (const n of nodes) {
      expect([n.name, n.fx, n.fy, n.fz]).toEqual([n.name, undefined, undefined, undefined])
    }
  })

  it('keeps them in the component they are connected to', () => {
    const thing = byName(nodes, 'Thing').comp
    expect(thing).toBe(0)
    for (const name of ['E', 'Z3', 'certificate', 'Lean 4 · oo-cert']) {
      expect([name, byName(nodes, name).comp]).toEqual([name, thing])
    }
  })
})

describe('toGraphData in replay', () => {
  // The verification layer as the docs figure names it, beside one class.
  const PIPELINE_GRAPH: GraphView = {
    classes: [
      { iri: 'urn:tool:lean', label: 'Lean 4 · oo-cert' },
      { iri: 'urn:tool:certificate', label: 'certificate' },
      { iri: 'urn:tool:e', label: 'E' },
      { iri: 'http://example.org/onto#Thing' },
    ],
    properties: [],
    edges: [
      { source: 'urn:tool:certificate', target: 'urn:tool:lean', warrant: 'certified' },
      { source: 'urn:tool:e', target: 'urn:tool:certificate' },
      { source: 'http://example.org/onto#Thing', target: 'urn:nowhere' },
    ],
  }
  const { nodes, links } = toGraphData(PIPELINE_GRAPH, { kind: 'replay' })

  it('still pins and marks the verification layer exactly as before', () => {
    expect(byName(nodes, 'Lean 4 · oo-cert')).toMatchObject({ role: 'kernel', fx: -70, fy: 105, fz: 0 })
    expect(byName(nodes, 'certificate')).toMatchObject({ role: 'tool', fx: -190, fy: 70, fz: 0 })
    expect(byName(nodes, 'E')).toMatchObject({ role: 'tool', fx: -70, fy: -65, fz: 0 })
    for (const name of ['Lean 4 · oo-cert', 'certificate', 'E']) {
      expect([name, byName(nodes, name).comp]).toEqual([name, undefined])
    }
  })

  it('draws an ordinary class as a class in its own component', () => {
    expect(byName(nodes, 'Thing')).toMatchObject({ role: 'class', comp: 0 })
    expect(byName(nodes, 'Thing').fx).toBeUndefined()
  })

  it('keeps only links between drawn nodes, defaulting the warrant to asserted', () => {
    expect(links).toEqual([
      { source: 'urn:tool:certificate', target: 'urn:tool:lean', warrant: 'certified' },
      { source: 'urn:tool:e', target: 'urn:tool:certificate', warrant: 'asserted' },
    ])
  })
})
