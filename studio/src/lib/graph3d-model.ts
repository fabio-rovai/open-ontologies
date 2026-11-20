/**
 * The data Graph3D draws, built from a GraphView: nodes, links, which nodes
 * belong to the verification layer and where that layer is pinned. Kept free
 * of React and the WebGL renderer so it can be tested in node.
 */
import type { GraphView, Warrant } from './demo-source';
import type { GraphOrigin } from './graph-legend';

/**
 * What a node is drawn as. `kernel` is the Lean checker, `tool` the rest of
 * the verification layer, and `class` everything in the ontology itself.
 * Only the replay demonstration has a verification layer (#424).
 */
export type NodeRole = 'class' | 'tool' | 'kernel';

export interface GNode {
  id: string;
  name: string;
  val: number;
  role: NodeRole;
  /** Pinned position. Set for the verification layer, absent for classes. */
  fx?: number;
  fy?: number;
  fz?: number;
  /** Which connected piece of the ontology this class belongs to. */
  comp?: number;
}

/** A node as the force simulation mutates it. */
export interface SimNode extends GNode {
  x?: number;
  y?: number;
  z?: number;
  vx?: number;
  vy?: number;
  vz?: number;
}
export interface GLink { source: string; target: string; warrant: Warrant; }

/**
 * The verification layer of the replay demonstration, drawn hotter than the
 * ontology it judges. Matched by display name, which is why it is consulted
 * only for the replay origin: a live ontology may name a class `E`, `Z3` or
 * `certificate`, and that class is not a prover (#424).
 */
const TOOL_NODES = new Set([
  'certificate', 'problem.tsv', 'ies-core.ttl', 'forged line',
  'Lean 4 · oo-cert', 'Isabelle/HOL', 'Vampire', 'E', 'Z3', 'Mace4', 'oo-fores',
]);

/** The kernel that decides. Brighter and larger than anything it judges. */
const LEAN_NODE = 'Lean 4 · oo-cert';

export function short(iri: string): string {
  const h = iri.lastIndexOf('#');
  return h >= 0 ? iri.slice(h + 1) : iri.slice(iri.lastIndexOf('/') + 1);
}

/**
 * Where each member of the verification layer is PINNED, in simulation units.
 *
 * The pipeline is a process, not a graph, and a force layout is the wrong tool
 * for it: run free, the six judges land wherever repulsion puts them and the
 * picture stops showing that anything flows. Three columns left to right --
 * what was read, what it became, who judged it -- with the two that read the
 * CERTIFICATE together and above, and the four that read `problem.tsv`, a
 * different artefact, together and below. That grouping is the argument.
 *
 * `docs/assets/knowledge-graph.svg` is this layout, and the README calls that
 * figure the Studio 3D view, so this is where the claim is kept true.
 */
const PIPELINE: Record<string, [number, number, number]> = {
  'ies-core.ttl': [-300, 30, 0],
  certificate: [-190, 70, 0],
  'problem.tsv': [-190, -70, 0],
  'Lean 4 · oo-cert': [-70, 105, 0],
  'Isabelle/HOL': [-70, 45, 0],
  Vampire: [-70, -25, 0],
  E: [-70, -65, 0],
  Z3: [-70, -105, 0],
  Mace4: [-70, -145, 0],
  // The checker of Fo certificates, one rank out from Vampire whose proof it reads.
  'oo-fores': [20, -25, 0],
  'forged line': [-300, -110, 0],
};

/**
 * Connected components of the ontology, and a centre for each.
 *
 * `ies-core`'s subclass graph is not connected. Laid out as one system the two
 * big pieces end up in opposite corners and the small ones, which feel nothing
 * but repulsion, sail off into the gap next to the pipeline. Each piece gets
 * its own centre instead, placed on a ring whose radius grows with the piece,
 * so the arrangement says something true about their sizes.
 */
function components(nodes: GNode[], links: GLink[]): Map<string, number> {
  const parent = new Map<string, string>();
  for (const n of nodes) parent.set(n.id, n.id);
  const find = (x: string): string => {
    let r = x;
    while (parent.get(r) !== r) r = parent.get(r) as string;
    while (parent.get(x) !== r) {
      const nx = parent.get(x) as string;
      parent.set(x, r);
      x = nx;
    }
    return r;
  };
  for (const l of links) {
    if (!parent.has(l.source) || !parent.has(l.target)) continue;
    const a = find(l.source);
    const b = find(l.target);
    if (a !== b) parent.set(a, b);
  }
  const size = new Map<string, number>();
  for (const n of nodes) {
    const r = find(n.id);
    size.set(r, (size.get(r) ?? 0) + 1);
  }
  const order = [...size.entries()].sort((a, b) => b[1] - a[1]).map(([r]) => r);
  const rank = new Map(order.map((r, i) => [r, i]));
  const out = new Map<string, number>();
  for (const n of nodes) out.set(n.id, rank.get(find(n.id)) as number);
  return out;
}

/** The centre each component is pulled towards, biggest nearest the middle. */
export function componentCentres(count: number): [number, number, number][] {
  const out: [number, number, number][] = [];
  for (let i = 0; i < count; i += 1) {
    if (i === 0) {
      out.push([190, 0, 0]);
      continue;
    }
    const a = (i - 1) * ((2 * Math.PI) / Math.max(1, count - 1));
    const r = 230 + i * 26;
    out.push([190 + Math.cos(a) * r, Math.sin(a) * r, Math.sin(a * 1.7) * 90]);
  }
  return out;
}

/** What `name` is drawn as in a graph from `origin`. */
function roleOf(name: string, origin: GraphOrigin): NodeRole {
  if (origin.kind !== 'replay') return 'class';
  if (name === LEAN_NODE) return 'kernel';
  return TOOL_NODES.has(name) ? 'tool' : 'class';
}

export function toGraphData(graph: GraphView, origin: GraphOrigin): { nodes: GNode[]; links: GLink[] } {
  const nodes: GNode[] = graph.classes.map((c) => {
    const name = c.label || short(c.iri);
    const role = roleOf(name, origin);
    const pin = role === 'class' ? undefined : PIPELINE[name];
    return pin
      ? { id: c.iri, name, val: 3, role, fx: pin[0], fy: pin[1], fz: pin[2] }
      : { id: c.iri, name, val: 3, role };
  });
  const known = new Set(nodes.map((n) => n.id));
  const links = graph.edges
    .filter((e) => known.has(e.source) && known.has(e.target))
    .map((e) => ({ source: e.source, target: e.target, warrant: (e.warrant ?? 'asserted') as Warrant }));
  // Components are computed over the ONTOLOGY only. The pipeline is pinned, so
  // letting it join a component would drag that component onto the pins.
  const ontology = nodes.filter((n) => n.role === 'class');
  const within = new Set(ontology.map((n) => n.id));
  const comp = components(ontology, links.filter((l) => within.has(l.source) && within.has(l.target)));
  for (const n of ontology) {
    const c = comp.get(n.id);
    if (c !== undefined) n.comp = c;
  }
  return { nodes, links };
}
