import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import cytoscape, { type Core, type ElementDefinition, type NodeSingular, type StylesheetStyle } from 'cytoscape';
import dagre from 'cytoscape-dagre';
import * as mcp from '../lib/mcp-client';
import { onGraphChanged } from '../lib/graph-refresh';
import { parseRows } from '../lib/sparql-rows';
import type { Collection } from 'cytoscape';
import {
  buildSchema, emptyInstGraph, expandInstGraph, explodeGroup, instElements, schemaElements,
  seedInstGraph, NEUTRAL, PALETTE, type InstGraph, type Neighbour, type NodeInfo, type Schema,
} from '../lib/graph-model';
import QUERIES from '../lib/graph-queries.json';

cytoscape.use(dagre);

type Selected = { id: string; label: string; uri: string } | null;
interface GraphViewProps {
  onNodeSelect: (node: Selected) => void;
  /** The node selected elsewhere (tree, inspector); seeds the instance view. */
  selected?: Selected;
}

// The queries live in graph-queries.json so tests/studio_graph_queries_test.rs can run these
// exact strings against the engine. Classes are owl:Class or rdfs:Class, and an rdf:Property
// counts as a property when it has an IRI domain and range (#440).
const P = QUERIES.prefixes;
const Q = {
  classes: P + QUERIES.classes,
  sub: P + QUERIES.sub,
  props: P + QUERIES.props,
  domRange: P + QUERIES.domRange,
};
const DAGRE = { name: 'dagre', rankDir: 'TB', nodeSep: 36, rankSep: 90, edgeSep: 14, animate: false, fit: false };

/**
 * Lay out each connected component with dagre, then shelf-pack the
 * components towards the viewport's aspect ratio. Plain dagre puts
 * unconnected classes side by side in one very wide row.
 */
function packedDagre(cy: Core, aspect: number) {
  const gap = 48;
  const comps = cy.elements().components().map((c: Collection) => {
    c.layout(DAGRE as cytoscape.LayoutOptions).run();
    return { c, bb: c.boundingBox({}) };
  });
  comps.sort((a, b) => b.bb.h - a.bb.h || b.bb.w - a.bb.w);
  const area = comps.reduce((sum, x) => sum + (x.bb.w + gap) * (x.bb.h + gap), 0);
  const rowW = Math.max(Math.sqrt(area * aspect), ...comps.map((x) => x.bb.w));
  let x = 0, y = 0, rowH = 0;
  for (const { c, bb } of comps) {
    if (x > 0 && x + bb.w > rowW) { x = 0; y += rowH + gap; rowH = 0; }
    const dx = x - bb.x1, dy = y - bb.y1;
    c.nodes().positions((n: NodeSingular) => ({ x: n.position('x') + dx, y: n.position('y') + dy }));
    x += bb.w + gap;
    rowH = Math.max(rowH, bb.h);
  }
}

const iri = (u: string) => `<${u.replace(/[<>"{}|^`\\\s]/g, '')}>`;
const esc = (s: string) => s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');

const FONT = '-apple-system, BlinkMacSystemFont, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", "Segoe UI", sans-serif';
const STYLE: StylesheetStyle[] = [
  { selector: 'node', style: {
    label: 'data(label)', 'font-family': FONT, 'font-size': 12, color: '#cdd6f4',
    'text-wrap': 'wrap', 'text-max-width': '220px', 'text-valign': 'center', 'text-halign': 'center',
    shape: 'round-rectangle', width: 'label', height: 'label', padding: '10px',
    'background-color': '#24273a', 'border-width': 2, 'border-color': 'data(color)',
  } },
  // text-justify is supported by cytoscape 3.33 but missing from @types/cytoscape.
  { selector: 'node.card.open', style: { 'text-justify': 'left', 'font-size': 11, 'line-height': 1.35 } as StylesheetStyle['style'] },
  // text-overflow-wrap is supported by cytoscape 3.33 but missing from @types/cytoscape; it lets
  // identifier-like labels with no spaces (long local names) wrap instead of making very wide nodes.
  { selector: 'node.inst', style: { 'font-size': 14, 'text-max-width': '260px', 'text-overflow-wrap': 'anywhere', 'background-color': '#1e1e2e', padding: '7px' } as StylesheetStyle['style'] },
  { selector: 'node.group', style: { 'font-size': 14 } },
  { selector: 'edge.link', style: { 'font-size': 12 } },
  { selector: 'node.inst.root', style: { 'border-width': 4, 'font-weight': 'bold', 'background-color': '#313244' } },
  { selector: 'node.group', style: { 'border-style': 'dashed', 'background-color': '#181825' } },
  { selector: 'node:selected', style: { 'border-color': '#ffffff', 'border-width': 4 } },
  { selector: 'edge', style: {
    width: 1.4, 'curve-style': 'bezier', 'line-color': '#7f849c', 'target-arrow-color': '#7f849c',
    'target-arrow-shape': 'triangle', 'arrow-scale': 0.9, 'font-family': FONT, 'font-size': 10, color: '#bac2de',
    'text-background-color': '#1e1e2e', 'text-background-opacity': 0.9, 'text-background-padding': '2px',
  } },
  { selector: 'edge.sub', style: {
    'line-color': '#585b70', 'source-arrow-color': '#585b70', 'source-arrow-shape': 'triangle', 'source-arrow-fill': 'hollow',
    'target-arrow-shape': 'none', 'arrow-scale': 1.3, width: 1.2,
  } },
  { selector: 'edge.prop, edge.link', style: { label: 'data(label)' } },
  { selector: 'edge.prop', style: { 'line-color': '#a6e3a1', 'target-arrow-color': '#a6e3a1', color: '#a6e3a1' } },
  { selector: 'edge.prop, edge.link', style: { 'text-rotation': 'autorotate' } },
  { selector: 'edge:loop', style: { 'loop-direction': '90deg', 'loop-sweep': '-60deg', 'control-point-step-size': 56, 'text-rotation': 'none' } },
  { selector: '.faded', style: { opacity: 0.18 } },
];

function chip(active: boolean): React.CSSProperties {
  return {
    background: active ? '#89b4fa' : '#1e1e2e', color: active ? '#1e1e2e' : '#a6adc8',
    border: '1px solid #45475a', borderRadius: 4, padding: '2px 8px', fontSize: 11, fontWeight: active ? 600 : 400, cursor: 'pointer',
  };
}

export function GraphView({ onNodeSelect, selected }: GraphViewProps) {
  const box = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const [schema, setSchema] = useState<Schema | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<'schema' | 'instances'>('schema');
  const [lang, setLang] = useState('en');
  const [showFields, setShowFields] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [focus, setFocus] = useState<string | null>(null);
  const [focusOn, setFocusOn] = useState(false);
  const [inst, setInst] = useState<InstGraph>(emptyInstGraph());
  const infoRef = useRef<Map<string, NodeInfo>>(new Map());
  const [search, setSearch] = useState('');
  const [hits, setHits] = useState<{ uri: string; label: string }[]>([]);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setError(null);
    try {
      const [c, s, p, d] = await Promise.all([Q.classes, Q.sub, Q.props, Q.domRange].map((q) => mcp.sparqlQuery(q)));
      const sc = buildSchema(parseRows(c), parseRows(s), parseRows(p), parseRows(d));
      setSchema(sc);
      setLang((cur) => (sc.langs.includes(cur) || sc.langs.length === 0 ? cur : sc.langs.includes('en') ? 'en' : sc.langs[0]));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);
  useEffect(() => { load(); }, [load]);
  // Re-read the schema after a chat or inspector edit, like the 2D tree (#421).
  useEffect(() => onGraphChanged(load), [load]);

  /** Labels and asserted or inferred types for a batch of IRIs. */
  const fetchInfo = useCallback(async (uris: string[]) => {
    const want = uris.filter((u) => !infoRef.current.has(u));
    for (let i = 0; i < want.length; i += 150) {
      const vals = want.slice(i, i + 150).map(iri).join(' ');
      const [lt, tt] = await Promise.all([
        mcp.sparqlQuery(`${P}SELECT ?n ?label WHERE { VALUES ?n { ${vals} } ?n rdfs:label ?label }`),
        mcp.sparqlQuery(`${P}SELECT ?n ?t WHERE { VALUES ?n { ${vals} } ?n a ?t . FILTER(isIRI(?t)) }`),
      ]);
      for (const u of want.slice(i, i + 150)) infoRef.current.set(u, { labels: [], types: [] });
      for (const r of parseRows(lt)) {
        const x = infoRef.current.get(r.n?.value);
        if (x && r.label && !x.labels.some((l) => l.value === r.label.value && l.lang === r.label.lang)) x.labels.push({ value: r.label.value, lang: r.label.lang });
      }
      for (const r of parseRows(tt)) {
        const x = infoRef.current.get(r.n?.value);
        if (x && r.t && !x.types.includes(r.t.value)) x.types.push(r.t.value);
      }
    }
  }, []);

  const expand = useCallback(async (anchor: string, base?: InstGraph) => {
    if (!schema) return;
    setBusy(true);
    try {
      const [o, i] = await Promise.all([
        mcp.sparqlQuery(`${P}SELECT ?p ?o WHERE { ${iri(anchor)} ?p ?o . FILTER(isIRI(?o) && ?p != rdf:type) } LIMIT 500`),
        mcp.sparqlQuery(`${P}SELECT ?s ?p WHERE { ?s ?p ${iri(anchor)} . FILTER(isIRI(?s) && ?p != rdf:type) } LIMIT 500`),
      ]);
      const nbrs: Neighbour[] = [
        ...parseRows(o).map((r) => ({ other: r.o?.value, pred: r.p?.value, dir: 'out' as const })),
        ...parseRows(i).map((r) => ({ other: r.s?.value, pred: r.p?.value, dir: 'in' as const })),
      ].filter((n) => n.other && n.pred);
      await fetchInfo(nbrs.map((n) => n.other));
      setInst((prev) => expandInstGraph(base ?? prev, anchor, nbrs, infoRef.current, schema));
    } finally {
      setBusy(false);
    }
  }, [schema, fetchInfo]);

  const startAt = useCallback(async (uri: string) => {
    if (!schema) return;
    await fetchInfo([uri]);
    const seed = seedInstGraph(uri, infoRef.current.get(uri), schema);
    setInst(seed);
    setHits([]);
    await expand(uri, seed);
  }, [schema, fetchInfo, expand]);

  // An individual selected in the tree seeds the instance view, once.
  useEffect(() => {
    if (mode !== 'instances' || !schema || inst.root || !selected) return;
    if (schema.classes.has(selected.uri) || schema.props.has(selected.uri)) return;
    startAt(selected.uri);
  }, [mode, schema, selected, inst.root, startAt]);

  useEffect(() => {
    const term = search.trim();
    if (mode !== 'instances' || term.length < 2) { setHits([]); return; }
    const t = setTimeout(async () => {
      const text = await mcp.sparqlQuery(P + QUERIES.instanceSearch.split('$TERM').join(esc(term)));
      const seen = new Map<string, string>();
      for (const r of parseRows(text)) {
        const u = r.n?.value;
        if (!u || schema?.classes.has(u) || schema?.props.has(u)) continue;
        const cur = seen.get(u);
        if (!cur || r.label?.lang === lang) seen.set(u, r.label?.value ?? cur ?? u);
      }
      setHits([...seen].slice(0, 15).map(([uri, label]) => ({ uri, label })));
    }, 250);
    return () => clearTimeout(t);
  }, [search, mode, schema, lang]);

  const elements = useMemo(() => {
    if (!schema) return [];
    return mode === 'schema'
      ? schemaElements(schema, { lang, expanded, showFields, focus: focusOn ? focus : null })
      : instElements(inst, schema, lang);
  }, [schema, mode, lang, expanded, showFields, focus, focusOn, inst]);

  // Keep latest handlers reachable from cytoscape listeners bound once.
  const handlers = useRef({ onNodeSelect, expand, setExpanded, setFocus, setInst });
  handlers.current = { onNodeSelect, expand, setExpanded, setFocus, setInst };

  useEffect(() => {
    if (!box.current) return;
    const cy = cytoscape({ container: box.current, style: STYLE, minZoom: 0.08, maxZoom: 3, wheelSensitivity: 0.25 });
    cyRef.current = cy;
    const highlight = (id: string | null) => {
      cy.elements().removeClass('faded');
      if (!id) return;
      const n = cy.getElementById(id);
      if (n.nonempty()) cy.elements().difference(n.closedNeighborhood()).addClass('faded');
    };
    cy.on('tap', 'node', (e) => {
      const d = e.target.data();
      highlight(d.id);
      if (d.kind === 'class') handlers.current.setFocus(d.id);
      if (d.uri) handlers.current.onNodeSelect({ id: d.uri, label: d.name, uri: d.uri });
    });
    cy.on('tap', (e) => { if (e.target === cy) highlight(null); });
    cy.on('dbltap', 'node', (e) => {
      const d = e.target.data();
      if (d.kind === 'class') handlers.current.setExpanded((prev) => {
        const next = new Set(prev);
        if (next.has(d.id)) next.delete(d.id); else next.add(d.id);
        return next;
      });
      if (d.kind === 'inst') handlers.current.expand(d.id);
      if (d.kind === 'group') handlers.current.setInst((prev) => explodeGroup(prev, d.id, infoRef.current));
    });
    (window as unknown as { __oo_cy?: Core }).__oo_cy = cy;
    return () => { cy.destroy(); cyRef.current = null; };
  }, []);

  const lastMode = useRef(mode);
  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    const fresh = lastMode.current !== mode;
    lastMode.current = mode;
    const before = new Set(cy.nodes().map((n: NodeSingular) => n.id()));
    cy.batch(() => {
      cy.elements().remove();
      cy.add(elements as ElementDefinition[]);
    });
    const grew = fresh || cy.nodes().toArray().some((n) => !before.has(n.id()));
    if (mode === 'schema') {
      const c = cy.container();
      packedDagre(cy, c && c.clientHeight > 0 ? c.clientWidth / c.clientHeight : 16 / 9);
    } else if (inst.root) {
      // Left to right along the links: what points at the root (records,
      // results) sits on its left, what the root points at on its right.
      cy.layout({ ...DAGRE, rankDir: 'LR', nodeSep: 8, rankSep: 110, edgeSep: 6 } as cytoscape.LayoutOptions).run();
    }
    if (grew) cy.fit(undefined, 52);
  }, [elements, mode, inst.root]);

  const counts = useMemo(() => {
    const n = elements.filter((e) => e.group === 'nodes').length;
    const l = elements.filter((e) => e.group === 'edges' && e.data.kind !== 'sub').length;
    return mode === 'schema' ? `${n} classes · ${l} links` : `${n} nodes · ${l} links`;
  }, [elements, mode]);

  const legend = useMemo(() => {
    const seen = new Map<string, string>();
    for (const e of elements) {
      const color = String(e.data.color ?? NEUTRAL), group = String(e.data.group ?? '');
      if (e.group === 'nodes' && group && color !== NEUTRAL && !seen.has(group)) seen.set(group, color);
    }
    // Past the palette size colours repeat, and a legend would mislead;
    // instance cards name their type on the second line anyway.
    if (seen.size > PALETTE.length) return [];
    return [...seen].map(([label, color]) => ({ label, color })).sort((a, b) => a.label.localeCompare(b.label, lang));
  }, [elements, lang]);

  return (
    <div className="absolute inset-0" style={{ background: '#1e1e2e' }}>
      {/* Explicit size: cytoscape resets the container's position, so inset alone collapses it. */}
      <div ref={box} style={{ width: '100%', height: '100%' }} data-testid="graph-canvas" />

      <div className="absolute top-2 left-2 right-2 flex flex-wrap items-center gap-2" style={{ pointerEvents: 'none' }}>
        <div className="flex gap-1" style={{ pointerEvents: 'auto' }}>
          <button style={chip(mode === 'schema')} onClick={() => setMode('schema')}>Schema</button>
          <button style={chip(mode === 'instances')} onClick={() => setMode('instances')}>Instances</button>
        </div>
        {schema && schema.langs.length > 1 && (
          <div className="flex gap-1" style={{ pointerEvents: 'auto' }}>
            {schema.langs.map((l) => <button key={l} style={chip(lang === l)} onClick={() => setLang(l)}>{l}</button>)}
          </div>
        )}
        {mode === 'schema' && (
          <div className="flex gap-1" style={{ pointerEvents: 'auto' }}>
            <button style={chip(showFields)} onClick={() => setShowFields((v) => !v)} title="Show data properties and their codes (skos:notation) on every class; double-click one class to open just that card">Fields</button>
            <button style={chip(focusOn)} onClick={() => setFocusOn((v) => !v)} title="Show only the selected class and the classes linked to it">Focus</button>
          </div>
        )}
        {mode === 'instances' && (
          <div className="relative" style={{ pointerEvents: 'auto' }}>
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Find an individual..."
                   className="text-xs px-2 py-1 rounded outline-none w-56"
                   style={{ background: '#1e1e2e', color: '#cdd6f4', border: '1px solid #45475a' }} />
            {hits.length > 0 && (
              <div className="absolute mt-1 w-80 rounded shadow-lg z-10 max-h-72 overflow-auto"
                   style={{ background: '#181825', border: '1px solid #45475a' }}>
                {hits.map((h) => (
                  <div key={h.uri} onClick={() => { setSearch(''); startAt(h.uri); }}
                       className="px-2 py-1 text-xs cursor-pointer hover:bg-[#313244]" style={{ color: '#cdd6f4' }}>
                    {h.label}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
        <div className="flex gap-1" style={{ pointerEvents: 'auto' }}>
          <button style={chip(false)} onClick={() => cyRef.current?.fit(undefined, 52)}>Fit</button>
          <button style={chip(false)} onClick={() => { infoRef.current.clear(); setInst(emptyInstGraph()); load(); }}>Reload</button>
        </div>
        <span className="ml-auto text-xs" style={{ color: '#7f849c' }}>{busy ? 'Loading…' : counts}</span>
      </div>

      {legend.length > 0 && (
        <div className="absolute bottom-2 left-2 flex flex-wrap gap-x-3 gap-y-1 px-2 py-1 rounded text-xs"
             style={{ background: 'rgba(24,24,37,0.85)', border: '1px solid #313244', color: '#a6adc8', maxWidth: '70%' }}>
          {legend.map((l) => (
            <span key={l.label} className="flex items-center gap-1">
              <span style={{ width: 9, height: 9, borderRadius: 2, border: `2px solid ${l.color}`, display: 'inline-block' }} />{l.label}
            </span>
          ))}
        </div>
      )}

      {(error || (schema && elements.length === 0)) && (
        <div className="absolute inset-0 flex items-center justify-center text-sm" style={{ color: '#7f849c', pointerEvents: 'none' }}>
          {error ? `Query failed: ${error}`
            : mode === 'schema' ? 'No classes in the store.'
            : 'Find an individual above, or select one in the 2D tree, to see its neighbourhood.'}
        </div>
      )}
    </div>
  );
}
