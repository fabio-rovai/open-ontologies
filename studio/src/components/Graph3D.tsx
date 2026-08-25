import { useEffect, useRef, useState } from 'react';
import ForceGraph3D from 'react-force-graph-3d';
import type { GraphView } from '../lib/demo-source';

/**
 * 3D view of the knowledge graph: classes as nodes, subclass edges as
 * links. Same shape, same layout, different geometry from the 2D tree.
 *
 * This used to issue its own SPARQL (a document-and-entity provenance
 * projection) straight at the engine. That made it unusable in the replay
 * build, where there is no engine to query. It now takes `graph` as a prop
 * instead: the class/subclass queries this rendered live now live in
 * live-source.ts, behind the same DemoSource interface the replay build
 * satisfies from committed artifacts. Rendering, camera behaviour and
 * interaction handling are unchanged; only where the data comes from is.
 */

interface GNode { id: string; name: string; val: number; }
interface GLink { source: string; target: string; }

const NODE_COLOR = '#89b4fa';
const LINK_COLOR = '#45475a';

function short(iri: string): string {
  const h = iri.lastIndexOf('#');
  return h >= 0 ? iri.slice(h + 1) : iri.slice(iri.lastIndexOf('/') + 1);
}

function toGraphData(graph: GraphView): { nodes: GNode[]; links: GLink[] } {
  const nodes = graph.classes.map((c) => ({ id: c.iri, name: c.label || short(c.iri), val: 3 }));
  const known = new Set(nodes.map((n) => n.id));
  const links = graph.edges
    .filter((e) => known.has(e.source) && known.has(e.target))
    .map((e) => ({ source: e.source, target: e.target }));
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
    fg.d3Force('link')?.distance?.(() => 26);
    fg.d3Force('charge')?.strength?.(-35);
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
    <div ref={holder} className="absolute inset-0" style={{ background: '#11111b' }}>
      {data.nodes.length === 0 ? (
        <div className="h-full flex items-center justify-center text-sm" style={{ color: '#6c7086' }}>
          Build the knowledge graph first (Documents panel).
        </div>
      ) : (
        <ForceGraph3D
          ref={fgRef as never}
          onEngineStop={() => {
            if (!framed.current) { framed.current = true; fgRef.current?.zoomToFit(600, 40); }
          }}
          cooldownTicks={120}
          width={size.w}
          height={size.h}
          graphData={data}
          backgroundColor="#11111b"
          nodeLabel={(n) => `<div style="font-family:sans-serif;font-size:12px">${(n as GNode).name}</div>`}
          nodeColor={() => NODE_COLOR}
          nodeVal={(n) => (n as GNode).val}
          nodeOpacity={0.9}
          linkColor={() => LINK_COLOR}
          linkOpacity={0.4}
          linkWidth={0.9}
          onNodeClick={(n) => {
            const g = n as GNode;
            onNodeSelect({ id: short(g.id), label: g.name, uri: g.id });
          }}
        />
      )}
    </div>
  );
}
