import { useEffect, useRef, useState } from 'react';
import ForceGraph3D from 'react-force-graph-3d';
import * as mcp from '../lib/mcp-client';

/**
 * 3D view of the knowledge graph: classes and individuals as nodes, subclass
 * and property edges as links. Same store, same queries, different geometry.
 * The 2D tree stays the analytical view; this one is spatial: clusters and
 * hubs are visible at a glance, and the partition sits apart from the
 * document plumbing.
 */

interface GNode { id: string; name: string; group: string; val: number; }
interface GLink { source: string; target: string; kind: string; }

const COLORS: Record<string, string> = {
  Document: '#89b4fa',
  Individual: '#fab387',
  conflict: '#f38ba8',
};

function short(iri: string): string {
  const h = iri.lastIndexOf('#');
  return h >= 0 ? iri.slice(h + 1) : iri.slice(iri.lastIndexOf('/') + 1);
}

const strip = (v: string) =>
  v.startsWith('<') ? v.slice(1, -1) : v.replace(/^"(.*)"(@[\w-]+)?(\^\^.*)?$/, '$1');

async function rows(q: string): Promise<Array<Record<string, string>>> {
  try {
    const raw = await mcp.sparqlQuery(q);
    return (JSON.parse(raw).results ?? []).map((r: Record<string, string>) => {
      const o: Record<string, string> = {};
      for (const [k, v] of Object.entries(r)) o[k] = strip(String(v));
      return o;
    });
  } catch { return []; }
}

export function Graph3D({ onNodeSelect }: {
  onNodeSelect: (n: { id: string; label: string; uri: string } | null) => void;
}) {
  const [data, setData] = useState<{ nodes: GNode[]; links: GLink[] }>({ nodes: [], links: [] });
  const fgRef = useRef<{ d3Force: (name: string) => { distance?: (d: (l: GLink) => number) => void; strength?: (v: number) => void } | undefined; zoomToFit: (ms: number, px: number) => void } | null>(null);
  const framed = useRef(false);

  useEffect(() => {
    // Pull the constellation together: short leashes on provenance edges,
    // stronger mutual repulsion only at close range.
    const fg = fgRef.current;
    if (!fg || data.nodes.length === 0) return;
    framed.current = false;
    fg.d3Force('link')?.distance?.((l: GLink) => (l.kind === 'cooc' ? 55 : l.kind === 'rel' ? 18 : 26));
    fg.d3Force('charge')?.strength?.(-35);
  }, [data]);
  const holder = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 800, h: 600 });

  useEffect(() => {
    const el = holder.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setSize({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const load = async () => {
    // The projection that answers "how does information interrelate between
    // documents": documents as hubs, entities as the bridges between them.
    // An entity shared by many documents pulls them together spatially; the
    // contradiction sits visibly between the documents that disagree.
    const P = 'PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> PREFIX owl: <http://www.w3.org/2002/07/owl#> PREFIX prov: <http://www.w3.org/ns/prov#>';
    const prov = await rows(`${P} SELECT ?e ?d ?dl ?el WHERE { ?e prov:wasDerivedFrom ?d . ?d rdfs:label ?dl . OPTIONAL { ?e rdfs:label ?el } } LIMIT 2000`);
    const conflicts = await rows(`${P} SELECT DISTINCT ?s WHERE { ?s a ?a, ?b . FILTER(STR(?a) < STR(?b)) { ?a owl:disjointWith ?b } UNION { ?b owl:disjointWith ?a } }`);
    const rel = await rows(`${P} SELECT ?s ?o WHERE { ?s ?p ?o . ?o prov:wasDerivedFrom ?x . ?s prov:wasDerivedFrom ?y . FILTER(?p != prov:wasDerivedFrom && isIRI(?o)) } LIMIT 1500`);

    const conflicted = new Set(conflicts.map(r => r.s));
    const nodes = new Map<string, GNode>();
    const links: GLink[] = [];
    const seen = new Set<string>();
    const docsOf = new Map<string, number>();

    // No provenance in the store: fall back to an ontology projection so the
    // view works on any loaded graph. Documents-and-entities is the richer
    // picture, but it needs `?entity prov:wasDerivedFrom ?document` triples
    // (see examples/document-privacy/provenance.py).
    if (prov.length === 0) {
      const classes = await rows(`${P} SELECT ?c ?l WHERE { { ?c a owl:Class } UNION { ?c rdfs:subClassOf ?x } OPTIONAL { ?c rdfs:label ?l } FILTER(!isBlank(?c)) } LIMIT 300`);
      const subs = await rows(`${P} SELECT ?a ?b WHERE { ?a rdfs:subClassOf ?b . FILTER(!isBlank(?a) && !isBlank(?b)) }`);
      const inds = await rows(`${P} SELECT ?i ?t ?l WHERE { ?i a ?t . ?t a owl:Class . OPTIONAL { ?i rdfs:label ?l } FILTER(!isBlank(?i)) } LIMIT 300`);
      for (const r of classes) {
        if (r.c) nodes.set(r.c, { id: r.c, name: r.l || short(r.c), group: 'Document', val: 3 });
      }
      for (const r of subs) {
        if (nodes.has(r.a) && nodes.has(r.b)) links.push({ source: r.a, target: r.b, kind: 'rel' });
      }
      for (const r of inds) {
        if (!r.i || !nodes.has(r.t)) continue;
        if (!nodes.has(r.i)) {
          nodes.set(r.i, {
            id: r.i, name: r.l || short(r.i),
            group: conflicted.has(r.i) ? 'conflict' : 'Individual', val: 1,
          });
        }
        links.push({ source: r.i, target: r.t, kind: conflicted.has(r.i) ? 'conflictProv' : 'prov' });
      }
      setData({ nodes: [...nodes.values()], links });
      return;
    }

    for (const r of prov) {
      if (!r.e || !r.d) continue;
      if (!nodes.has(r.d)) nodes.set(r.d, { id: r.d, name: r.dl || short(r.d), group: 'Document', val: 6 });
      if (!nodes.has(r.e)) {
        nodes.set(r.e, {
          id: r.e, name: r.el || short(r.e),
          group: conflicted.has(r.e) ? 'conflict' : 'Individual', val: 1,
        });
      }
      const k = r.e + '|' + r.d;
      if (!seen.has(k)) {
        seen.add(k);
        links.push({ source: r.e, target: r.d, kind: conflicted.has(r.e) ? 'conflictProv' : 'prov' });
        docsOf.set(r.e, (docsOf.get(r.e) ?? 0) + 1);
      }
    }
    // entity-to-entity relationships, so structure inside a document shows too
    for (const r of rel) {
      if (nodes.has(r.s) && nodes.has(r.o) && r.s !== r.o) {
        const k = 'rel|' + r.s + '|' + r.o;
        if (!seen.has(k)) { seen.add(k); links.push({ source: r.s, target: r.o, kind: 'rel' }); }
      }
    }
    for (const n of nodes.values()) {
      if (n.group !== 'Document') n.val = Math.max(1, Math.min(9, (docsOf.get(n.id) ?? 1) * 1.6));
    }

    // Prune for shape: sections and hundreds of one-document leaves turn the
    // layout into spray. Shared entities, conflicted entities and anything in
    // a relationship stay; singletons are sampled.
    const inRel = new Set<string>();
    for (const l of links) if (l.kind === 'rel') { inRel.add(l.source); inRel.add(l.target); }
    let singletons = 0;
    const keep = new Set<string>();
    for (const n of nodes.values()) {
      const sid = short(n.id);
      if (n.group === 'Document') { keep.add(n.id); continue; }
      if (sid.startsWith('SEC_') || sid.startsWith('CLAIM_')) continue;
      const shared = (docsOf.get(n.id) ?? 0) >= 2;
      if (n.group === 'conflict' || shared || inRel.has(n.id)) { keep.add(n.id); continue; }
      if (singletons < 40) { keep.add(n.id); singletons += 1; }
    }
    let keptLinks = links.filter(l => keep.has(l.source) && keep.has(l.target));

    // One organism, not islands: documents that assert the same entities get
    // a co-occurrence edge weighted by how much they share. This is the
    // GraphRAG shape, and the edge is real: "these documents talk about the
    // same things".
    const docEnts = new Map<string, Set<string>>();
    for (const l of keptLinks) {
      if (l.kind === 'prov' || l.kind === 'conflictProv') {
        if (!docEnts.has(l.target)) docEnts.set(l.target, new Set());
        docEnts.get(l.target)!.add(l.source);
      }
    }
    const docIds = [...docEnts.keys()];
    for (let a = 0; a < docIds.length; a++) {
      for (let b = a + 1; b < docIds.length; b++) {
        const ea = docEnts.get(docIds[a])!, eb = docEnts.get(docIds[b])!;
        let sharedN = 0;
        for (const e of ea) if (eb.has(e)) sharedN += 1;
        if (sharedN > 0) keptLinks.push({ source: docIds[a], target: docIds[b], kind: 'cooc' });
      }
    }
    setData({ nodes: [...nodes.values()].filter(n => keep.has(n.id)), links: keptLinks });
  };

  useEffect(() => {
    load();
    const w = window as unknown as { __refreshGraph?: () => void };
    const prev = w.__refreshGraph;
    w.__refreshGraph = () => { prev?.(); load(); };
    const onRefresh = () => load();
    window.addEventListener('lineage-refresh', onRefresh);
    return () => {
      window.removeEventListener('lineage-refresh', onRefresh);
      w.__refreshGraph = prev;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
          nodeColor={(n) => COLORS[(n as GNode).group] ?? '#a6adc8'}
          nodeVal={(n) => (n as GNode).val}
          nodeOpacity={0.9}
          linkColor={(l) => (l as GLink).kind === 'conflictProv' ? '#f38ba8' : (l as GLink).kind === 'rel' ? '#a6e3a1' : (l as GLink).kind === 'cooc' ? '#313244' : '#45475a'}
          linkOpacity={0.4}
          linkWidth={(l) => (l as GLink).kind === 'conflictProv' ? 2 : (l as GLink).kind === 'cooc' ? 0.4 : (l as GLink).kind === 'rel' ? 0.6 : 0.9}
          onNodeClick={(n) => {
            const g = n as GNode;
            onNodeSelect({ id: short(g.id), label: g.name, uri: g.id });
          }}
        />
      )}
    </div>
  );
}
