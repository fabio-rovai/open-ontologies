import { useEffect, useRef, useState } from 'react';
import ForceGraph3D from 'react-force-graph-3d';
import type { GraphView, Warrant } from '../lib/demo-source';

/**
 * 3D view of the knowledge graph: classes as nodes, subclass edges as links,
 * coloured by what is known about each edge.
 *
 * The colours are the point, not decoration. Grey is what a person asserted.
 * Green is what the engine derived AND the Lean checker accepted. Red is a
 * derivation the checker refused: `oo-cert` exited 1 and named the rule. So
 * red here is a checkable fact rather than a heuristic warning or a confidence
 * score, which is why it is allowed to be this loud.
 *
 * The graph also carries the verification layer itself as nodes, because a
 * reader should be able to see who judged what. Those links are drawn only
 * where they are true of the run on screen: the certificate goes to Lean and
 * to Isabelle, which read the same bytes, while the first-order family reads a
 * different artefact entirely and never sees this graph.
 *
 * This used to issue its own SPARQL straight at the engine, which made it
 * unusable in the replay build. It takes `graph` as a prop instead, behind the
 * same DemoSource interface the replay build satisfies from committed
 * artifacts.
 */

interface GNode {
  id: string;
  name: string;
  val: number;
  /** Pinned position. Set for the verification layer, absent for classes. */
  fx?: number;
  fy?: number;
  fz?: number;
  /** Which connected piece of the ontology this class belongs to. */
  comp?: number;
}

/** A node as the force simulation mutates it. */
interface SimNode extends GNode {
  x?: number;
  y?: number;
  z?: number;
  vx?: number;
  vy?: number;
  vz?: number;
}
interface GLink { source: string; target: string; warrant: Warrant; }

const NODE_COLOR = '#7dd3fc';

/** The verification layer, drawn hotter than the ontology it judges. */
const TOOL_NODES = new Set([
  'certificate', 'problem.tsv', 'ies-core.ttl', 'forged line',
  'Lean 4 · oo-cert', 'Isabelle/HOL', 'Vampire', 'E', 'Z3', 'Mace4',
]);
const TOOL_COLOR = '#f0abfc';

/** The kernel that decides. Brighter and larger than anything it judges. */
const LEAN_NODE = 'Lean 4 · oo-cert';
const LEAN_COLOR = '#34d399';

const LINK_COLOR: Record<Warrant, string> = {
  asserted: '#475569',
  certified: '#34d399',
  rejected: '#fb3b53',
};
const LINK_WIDTH: Record<Warrant, number> = { asserted: 0.5, certified: 0.9, rejected: 2.4 };

function short(iri: string): string {
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

function toGraphData(graph: GraphView): { nodes: GNode[]; links: GLink[] } {
  const nodes: GNode[] = graph.classes.map((c) => {
    const name = c.label || short(c.iri);
    const pin = PIPELINE[name];
    return pin
      ? { id: c.iri, name, val: 3, fx: pin[0], fy: pin[1], fz: pin[2] }
      : { id: c.iri, name, val: 3 };
  });
  const known = new Set(nodes.map((n) => n.id));
  const links = graph.edges
    .filter((e) => known.has(e.source) && known.has(e.target))
    .map((e) => ({ source: e.source, target: e.target, warrant: (e.warrant ?? 'asserted') as Warrant }));
  // Components are computed over the ONTOLOGY only. The pipeline is pinned, so
  // letting it join a component would drag that component onto the pins.
  const ontology = nodes.filter((n) => !TOOL_NODES.has(n.name));
  const within = new Set(ontology.map((n) => n.id));
  const comp = components(ontology, links.filter((l) => within.has(l.source) && within.has(l.target)));
  for (const n of nodes) {
    const c = comp.get(n.id);
    if (c !== undefined && !TOOL_NODES.has(n.name)) n.comp = c;
  }
  return { nodes, links };
}

export function Graph3D({ graph, onNodeSelect }: {
  graph: GraphView;
  onNodeSelect: (n: { id: string; label: string; uri: string } | null) => void;
}) {
  const data = toGraphData(graph);
  const fgRef = useRef<{
    d3Force: (
      name: string,
      force?: (alpha: number) => void,
    ) => { distance?: (d: (l: GLink) => number) => void; strength?: (v: number) => void } | undefined;
    zoomToFit: (ms: number, px: number) => void;
  } | null>(null);
  const framed = useRef(false);

  useEffect(() => {
    const fg = fgRef.current;
    if (!fg || data.nodes.length === 0) return;
    framed.current = false;
    fg.d3Force('link')?.distance?.(() => 30);
    fg.d3Force('charge')?.strength?.(-120);

    // Each piece of the ontology is pulled towards its own centre.
    //
    // Without this the simulation has nothing to hold the pieces apart: a
    // disconnected node feels only repulsion, so the small ones drift until
    // they meet the pinned pipeline and sit on top of it. One force, one term
    // per node, and the pinned nodes are skipped because d3 ignores velocity
    // on a node that has fx/fy/fz anyway.
    const centres = componentCentres(
      data.nodes.reduce((m, n) => Math.max(m, (n.comp ?? -1) + 1), 0),
    );
    fg.d3Force('cluster', (alpha: number) => {
      for (const n of data.nodes as SimNode[]) {
        if (n.comp === undefined || n.fx !== undefined) continue;
        const c = centres[n.comp];
        if (!c) continue;
        n.vx = (n.vx ?? 0) + (c[0] - (n.x ?? 0)) * alpha * 0.22;
        n.vy = (n.vy ?? 0) + (c[1] - (n.y ?? 0)) * alpha * 0.22;
        n.vz = (n.vz ?? 0) + (c[2] - (n.z ?? 0)) * alpha * 0.22;
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graph]);

  const holder = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 800, h: 600 });

  useEffect(() => {
    const el = holder.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setSize({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  return (
    <div ref={holder} className="absolute inset-0" style={{ background: '#020617' }}>
      {data.nodes.length === 0 ? (
        <div className="h-full flex items-center justify-center text-sm" style={{ color: '#6c7086' }}>
          Build the knowledge graph first (Documents panel).
        </div>
      ) : (
        <>
          <ForceGraph3D
            ref={fgRef as never}
            onEngineStop={() => {
              // Frame AFTER the layout settles, and then once more a beat
              // later: the first call can land while nodes are still moving,
              // which leaves the camera inside the graph. Observed doing
              // exactly that at 152 nodes.
              if (!framed.current) {
                framed.current = true;
                fgRef.current?.zoomToFit(700, 60);
                setTimeout(() => fgRef.current?.zoomToFit(500, 60), 900);
              }
            }}
            cooldownTicks={260}
            width={size.w}
            height={size.h}
            graphData={data}
            backgroundColor="#020617"
            showNavInfo={false}
            nodeLabel={(n) => `<div style="font-family:ui-monospace,monospace;font-size:12px;color:#e0f2fe;background:rgba(2,6,23,.92);padding:3px 7px;border:1px solid #334155;border-radius:4px">${(n as GNode).name}</div>`}
            nodeColor={(n) => ((n as GNode).name === LEAN_NODE ? LEAN_COLOR
              : TOOL_NODES.has((n as GNode).name) ? TOOL_COLOR : NODE_COLOR)}
            nodeVal={(n) => ((n as GNode).name === LEAN_NODE ? 22
              : TOOL_NODES.has((n as GNode).name) ? 9 : 2.2)}
            nodeOpacity={0.95}
            nodeResolution={16}
            linkColor={(l) => LINK_COLOR[(l as GLink).warrant]}
            linkOpacity={0.95}
            linkWidth={(l) => LINK_WIDTH[(l as GLink).warrant]}
            linkDirectionalParticles={(l) => ((l as GLink).warrant === 'rejected' ? 6
              : (l as GLink).warrant === 'certified' ? 1 : 0)}
            linkDirectionalParticleWidth={1.6}
            linkDirectionalParticleSpeed={0.006}
            linkDirectionalParticleColor={(l) => LINK_COLOR[(l as GLink).warrant]}
            onNodeClick={(n) => {
              const g = n as GNode;
              onNodeSelect({ id: short(g.id), label: g.name, uri: g.id });
            }}
          />
          <Legend links={data.links} />
        </>
      )}
    </div>
  );
}

/**
 * A heads-up panel rather than a bare key. A reader arriving at a dense graph
 * needs three things before the colours mean anything: what was read, who
 * judged it, and what the judgement was worth. Every number is counted from
 * the data on screen, never typed.
 */
function Legend({ links }: { links: GLink[] }) {
  const n = (w: Warrant) => links.filter((l) => l.warrant === w).length;
  const rows: { w: Warrant; label: string; means: string }[] = [
    { w: 'asserted', label: 'ASSERTED', means: 'read from ies-core.ttl. claimed by a person' },
    { w: 'certified', label: 'CERTIFIED', means: 'derived, then PROVED. OOCert.certificate_sound' },
    { w: 'rejected', label: 'REJECTED', means: 'forged. the checker exited 1 and named the rule' },
  ];
  return (
    <div
      className="absolute left-4 bottom-4 rounded-xl px-4 py-3 text-xs"
      style={{
        maxWidth: 560,
        background: 'linear-gradient(180deg, rgba(2,6,23,.95), rgba(8,15,40,.95))',
        border: '1px solid #1e3a5f',
        boxShadow: '0 0 30px rgba(52,211,153,.12), inset 0 0 40px rgba(14,165,233,.05)',
      }}
    >
      <div className="flex items-baseline gap-2 pb-2" style={{ borderBottom: '1px solid #1e3a5f' }}>
        <span style={{ color: '#34d399', fontWeight: 800, letterSpacing: '.12em' }}>
          PROOF-CARRYING INFERENCE
        </span>
        <span style={{ color: '#64748b' }}>ies-core.ttl · 1,083 triples</span>
      </div>
      {rows.map((r) => (
        <div key={r.w} className="flex items-center gap-2 pt-1.5">
          <span
            style={{
              width: 22, height: 3, display: 'inline-block',
              background: LINK_COLOR[r.w], boxShadow: `0 0 8px ${LINK_COLOR[r.w]}`,
            }}
          />
          <span style={{ color: LINK_COLOR[r.w], fontWeight: 700, letterSpacing: '.06em', width: 78 }}>
            {r.label}
          </span>
          <span style={{ color: '#e2e8f0', fontWeight: 700, width: 34, textAlign: 'right' }}>{n(r.w)}</span>
          <span style={{ color: '#94a3b8' }}>{r.means}</span>
        </div>
      ))}
      <div className="pt-2 mt-2" style={{ borderTop: '1px solid #1e3a5f', color: '#64748b', lineHeight: 1.55 }}>
        <span style={{ color: '#34d399', fontWeight: 700 }}>● Lean 4</span> decides.{' '}
        <span style={{ color: '#f0abfc' }}>● Isabelle/HOL</span> checks the same bytes independently.{' '}
        <span style={{ color: '#f0abfc' }}>● Vampire, E, Z3, Mace4</span> read a different artefact and
        their verdicts are oracle opinions, never certificates.
      </div>
    </div>
  );
}
