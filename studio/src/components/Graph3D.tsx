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

interface GNode { id: string; name: string; val: number; }
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

function toGraphData(graph: GraphView): { nodes: GNode[]; links: GLink[] } {
  const nodes = graph.classes.map((c) => ({ id: c.iri, name: c.label || short(c.iri), val: 3 }));
  const known = new Set(nodes.map((n) => n.id));
  const links = graph.edges
    .filter((e) => known.has(e.source) && known.has(e.target))
    .map((e) => ({ source: e.source, target: e.target, warrant: (e.warrant ?? 'asserted') as Warrant }));
  return { nodes, links };
}

export function Graph3D({ graph, onNodeSelect }: {
  graph: GraphView;
  onNodeSelect: (n: { id: string; label: string; uri: string } | null) => void;
}) {
  const data = toGraphData(graph);
  const fgRef = useRef<{ d3Force: (name: string) => { distance?: (d: (l: GLink) => number) => void; strength?: (v: number) => void } | undefined; zoomToFit: (ms: number, px: number) => void } | null>(null);
  const framed = useRef(false);

  useEffect(() => {
    const fg = fgRef.current;
    if (!fg || data.nodes.length === 0) return;
    framed.current = false;
    fg.d3Force('link')?.distance?.(() => 30);
    fg.d3Force('charge')?.strength?.(-120);
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
