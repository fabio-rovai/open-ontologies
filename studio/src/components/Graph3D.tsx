import { useEffect, useRef, useState } from 'react';
import ForceGraph3D from 'react-force-graph-3d';
import type { GraphView, Warrant } from '../lib/demo-source';
import { emptyGraphText, legendParts, type GraphOrigin } from '../lib/graph-legend';
import {
  componentCentres, short, toGraphData, warrantCounts,
  type GLink, type GNode, type NodeRole, type SimNode,
} from '../lib/graph3d-model';

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
 * Both the warrants and the verification layer belong to the replay
 * demonstration. In live mode every node is a class, whatever it is called,
 * and the legend drops its proof-carrying parts unless an edge carries a
 * warrant (graph3d-model.ts and graph-legend.ts, #424).
 *
 * This used to issue its own SPARQL straight at the engine, which made it
 * unusable in the replay build. It takes `graph` as a prop instead, behind the
 * same DemoSource interface the replay build satisfies from committed
 * artifacts.
 */

const NODE_COLOR = '#7dd3fc';
/** The verification layer, drawn hotter than the ontology it judges. */
const TOOL_COLOR = '#f0abfc';
/** The kernel that decides. Brighter and larger than anything it judges. */
const LEAN_COLOR = '#34d399';

/** Colour and size by what a node is (graph3d-model.ts), never by its name (#424). */
const ROLE_COLOR: Record<NodeRole, string> = { kernel: LEAN_COLOR, tool: TOOL_COLOR, class: NODE_COLOR };
const ROLE_VAL: Record<NodeRole, number> = { kernel: 22, tool: 9, class: 2.2 };

const LINK_COLOR: Record<Warrant, string> = {
  asserted: '#475569',
  certified: '#34d399',
  rejected: '#fb3b53',
};
const LINK_WIDTH: Record<Warrant, number> = { asserted: 0.5, certified: 0.9, rejected: 2.4 };

export function Graph3D({ graph, origin, onNodeSelect }: {
  graph: GraphView;
  /**
   * Where `graph` came from, so the legend names that file rather than the
   * replay demo's. Required, so neither build can fall back to the other's
   * text by omission.
   */
  origin: GraphOrigin;
  onNodeSelect: (n: { id: string; label: string; uri: string } | null) => void;
}) {
  const data = toGraphData(graph, origin);
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
          {emptyGraphText(origin)}
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
            nodeColor={(n) => ROLE_COLOR[(n as GNode).role]}
            nodeVal={(n) => ROLE_VAL[(n as GNode).role]}
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
          <Legend counts={warrantCounts(data.nodes, data.links)} links={data.links} origin={origin} />
        </>
      )}
    </div>
  );
}

/**
 * A heads-up panel rather than a bare key. A reader arriving at a dense graph
 * needs three things before the colours mean anything: what was read, who
 * judged it, and what the judgement was worth. Every number is counted from
 * the data on screen, never typed. The file and triple count come from
 * `origin`: the replay demo's own in replay, the loaded file's in live mode.
 */
function Legend({ counts, links, origin }: {
  counts: Record<Warrant, number>;
  links: GLink[];
  origin: GraphOrigin;
}) {
  // The ontology's own edges only: the verification layer's are drawn in the
  // same colours but are not what the rows describe (#425).
  const n = (w: Warrant) => counts[w];
  const parts = legendParts(origin, links.map((l) => l.warrant));
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
        <span style={{ color: parts.proof ? '#34d399' : NODE_COLOR, fontWeight: 800, letterSpacing: '.12em' }}>
          {parts.heading}
        </span>
        {parts.caption && <span style={{ color: '#64748b' }}>{parts.caption}</span>}
      </div>
      {parts.rows.map((r) => (
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
      {parts.proof && (
        <div className="pt-2 mt-2" style={{ borderTop: '1px solid #1e3a5f', color: '#64748b', lineHeight: 1.55 }}>
          <span style={{ color: '#34d399', fontWeight: 700 }}>● Lean 4</span> decides.{' '}
          <span style={{ color: '#f0abfc' }}>● Isabelle/HOL</span> checks the same bytes independently.{' '}
          <span style={{ color: '#f0abfc' }}>● Vampire, E, Z3, Mace4</span> read a different artefact and
          their verdicts are oracle opinions, never certificates.
        </div>
      )}
    </div>
  );
}
