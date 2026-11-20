/**
 * Data model for the 2D graph view: the schema (classes, the links object
 * properties draw between them, the data properties each class carries) and
 * an incrementally expanded neighbourhood of individuals.
 *
 * Kept free of React and cytoscape so it can be tested against the exact
 * rows the engine returns.
 */
import { localName, type Row } from './sparql-rows';

export type Label = { value: string; lang?: string };

export interface ClassInfo { uri: string; labels: Label[]; parents: string[]; root: string }
export interface PropInfo {
  uri: string;
  kind: 'object' | 'data';
  labels: Label[];
  codes: string[];
  domains: string[];
  ranges: string[];
}
export interface Schema {
  classes: Map<string, ClassInfo>;
  props: Map<string, PropInfo>;
  ancestors: Map<string, Set<string>>;
  roots: string[];
  langs: string[];
}

const CORE_NS = [
  'http://www.w3.org/2002/07/owl#',
  'http://www.w3.org/1999/02/22-rdf-syntax-ns#',
  'http://www.w3.org/2000/01/rdf-schema#',
  'http://www.w3.org/2001/XMLSchema#',
];
export const isCoreTerm = (uri: string) => CORE_NS.some((ns) => uri.startsWith(ns));

export const PALETTE = ['#89b4fa', '#a6e3a1', '#f9e2af', '#fab387', '#cba6f7', '#94e2d5', '#f38ba8', '#74c7ec', '#f5c2e7', '#b4befe'];

export function pickLabel(labels: Label[] | undefined, lang: string, uri: string): string {
  if (!labels || labels.length === 0) return localName(uri);
  const find = (tag: string) => labels.find((l) => (l.lang ?? '') === tag);
  return find(lang)?.value ?? find('en')?.value ?? find('')?.value ?? labels[0].value;
}

function addLabel(list: Label[], value: string, lang?: string) {
  if (value && !list.some((l) => l.value === value && l.lang === lang)) list.push({ value, lang });
}

/** Keep only the members of `set` that no other member specialises. */
function mostSpecific(set: Iterable<string>, ancestors: Map<string, Set<string>>): string[] {
  const xs = [...new Set(set)];
  return xs.filter((x) => !xs.some((y) => y !== x && ancestors.get(y)?.has(x))).sort();
}

/**
 * Build the schema from four query results: classes (?c ?label), subclass
 * pairs (?s ?o), properties (?p ?t ?label ?code) and domain/range rows
 * (?p ?k ?c, k = "d" | "r"). Reasoned stores carry the transitive closure of
 * rdfs:subClassOf and every superclass of each domain and range, so both are
 * reduced back to their most specific members here.
 */
export function buildSchema(classRows: Row[], subRows: Row[], propRows: Row[], drRows: Row[]): Schema {
  const classes = new Map<string, ClassInfo>();
  for (const r of classRows) {
    const uri = r.c?.value;
    if (!uri || r.c.type !== 'uri' || isCoreTerm(uri)) continue;
    if (!classes.has(uri)) classes.set(uri, { uri, labels: [], parents: [], root: uri });
    if (r.label?.type === 'literal') addLabel(classes.get(uri)!.labels, r.label.value, r.label.lang);
  }

  const raw = new Map<string, Set<string>>();
  for (const r of subRows) {
    const s = r.s?.value, o = r.o?.value;
    if (!s || !o || s === o || !classes.has(s) || !classes.has(o)) continue;
    if (!raw.has(s)) raw.set(s, new Set());
    raw.get(s)!.add(o);
  }

  const ancestors = new Map<string, Set<string>>();
  const ancestorsOf = (c: string): Set<string> => {
    const seen = new Set<string>();
    const stack = [...(raw.get(c) ?? [])];
    while (stack.length) {
      const x = stack.pop()!;
      if (x === c || seen.has(x)) continue;
      seen.add(x);
      for (const y of raw.get(x) ?? []) stack.push(y);
    }
    return seen;
  };
  for (const c of classes.keys()) ancestors.set(c, ancestorsOf(c));

  for (const info of classes.values()) {
    const ps = [...(raw.get(info.uri) ?? [])];
    // A parent is direct when no other parent already sits below it.
    info.parents = ps.filter((p) => !ps.some((q) => q !== p && ancestors.get(q)?.has(p))).sort();
  }
  for (const info of classes.values()) {
    let cur = info.uri;
    const seen = new Set([cur]);
    while (true) {
      const next = classes.get(cur)?.parents[0];
      if (!next || seen.has(next)) break;
      seen.add(next);
      cur = next;
    }
    info.root = cur;
  }

  const props = new Map<string, PropInfo>();
  for (const r of propRows) {
    const uri = r.p?.value;
    if (!uri || r.p.type !== 'uri' || isCoreTerm(uri)) continue;
    const kind = localName(r.t?.value ?? '') === 'DatatypeProperty' ? 'data' : 'object';
    if (!props.has(uri)) props.set(uri, { uri, kind, labels: [], codes: [], domains: [], ranges: [] });
    const p = props.get(uri)!;
    if (r.label?.type === 'literal') addLabel(p.labels, r.label.value, r.label.lang);
    if (r.code?.type === 'literal' && !p.codes.includes(r.code.value)) p.codes.push(r.code.value);
  }
  const dom = new Map<string, Set<string>>(), rng = new Map<string, Set<string>>();
  for (const r of drRows) {
    const p = r.p?.value, c = r.c?.value, k = r.k?.value;
    if (!p || !c || !props.has(p) || !classes.has(c)) continue;
    const m = k === 'r' ? rng : dom;
    if (!m.has(p)) m.set(p, new Set());
    m.get(p)!.add(c);
  }
  for (const p of props.values()) {
    p.domains = mostSpecific(dom.get(p.uri) ?? [], ancestors);
    p.ranges = mostSpecific(rng.get(p.uri) ?? [], ancestors);
  }

  const langs = new Set<string>();
  for (const x of [...classes.values(), ...props.values()]) for (const l of x.labels) if (l.lang) langs.add(l.lang);
  const roots = [...new Set([...classes.values()].map((c) => c.root))].sort();
  return { classes, props, ancestors, roots, langs: [...langs].sort() };
}

export const NEUTRAL = '#9399b2';

/** Roots that head a hierarchy (have at least one subclass). */
export function hierarchyRoots(s: Schema): string[] {
  const heads = new Set<string>();
  for (const c of s.classes.values()) if (c.root !== c.uri) heads.add(c.root);
  return [...heads].sort();
}

/** Colour a class by the root of its hierarchy; roots outside `palette` stay neutral. */
export function colorOf(s: Schema, cls: string | undefined, palette: string[]): string {
  if (!cls || !s.classes.has(cls)) return NEUTRAL;
  const i = palette.indexOf(s.classes.get(cls)!.root);
  return i < 0 ? NEUTRAL : PALETTE[i % PALETTE.length];
}

function groupOf(s: Schema, cls: string | undefined, lang: string): string {
  if (!cls || !s.classes.has(cls)) return '';
  const r = s.classes.get(cls)!.root;
  return pickLabel(s.classes.get(r)?.labels, lang, r);
}

export function dataPropsOf(s: Schema, cls: string): PropInfo[] {
  return [...s.props.values()].filter((p) => p.kind === 'data' && p.domains.includes(cls));
}

export interface ElementDef { group: 'nodes' | 'edges'; data: Record<string, unknown>; classes?: string }

export interface SchemaViewOpts {
  lang: string;
  expanded: Set<string>;
  showFields: boolean;
  focus: string | null;
  maxFields?: number;
}

/** Classes as cards; subclass and object-property links as edges. */
export function schemaElements(s: Schema, o: SchemaViewOpts): ElementDef[] {
  const maxFields = o.maxFields ?? 14;
  let visible = new Set(s.classes.keys());
  if (o.focus && s.classes.has(o.focus)) {
    const f = o.focus;
    visible = new Set([f, ...s.classes.get(f)!.parents]);
    for (const c of s.classes.values()) if (c.parents.includes(f)) visible.add(c.uri);
    for (const p of s.props.values()) {
      if (p.kind !== 'object') continue;
      if (p.domains.includes(f)) p.ranges.forEach((r) => visible.add(r));
      if (p.ranges.includes(f)) p.domains.forEach((d) => visible.add(d));
    }
  }
  const zh = o.lang.startsWith('zh');
  const palette = hierarchyRoots(s);
  const out: ElementDef[] = [];
  for (const uri of [...visible].sort()) {
    const c = s.classes.get(uri)!;
    const name = pickLabel(c.labels, o.lang, uri);
    const fields = dataPropsOf(s, uri)
      .map((p) => ({ label: pickLabel(p.labels, o.lang, p.uri), code: p.codes[0] }))
      .sort((a, b) => a.label.localeCompare(b.label, o.lang));
    let label = name;
    const open = fields.length > 0 && (o.showFields || o.expanded.has(uri));
    if (open) {
      const lines = fields.slice(0, maxFields).map((f) => (f.code ? `${f.label}   ${f.code}` : f.label));
      if (fields.length > maxFields) lines.push(zh ? `另有 ${fields.length - maxFields} 个` : `+${fields.length - maxFields} more`);
      label = `${name}\n\n${lines.join('\n')}`;
    } else if (fields.length > 0) {
      label = `${name}\n${zh ? `${fields.length} 个数据属性` : `${fields.length} data properties`}`;
    }
    out.push({
      group: 'nodes',
      data: { id: uri, uri, name, label, color: colorOf(s, uri, palette), group: groupOf(s, uri, o.lang), kind: 'class' },
      classes: open ? 'card open' : 'card',
    });
  }
  for (const uri of visible) {
    for (const p of s.classes.get(uri)!.parents) {
      // Drawn parent -> child so layered layouts put the parent above; the
      // hollow arrowhead sits on the parent end, as in UML.
      if (visible.has(p)) out.push({ group: 'edges', data: { id: `sub|${uri}|${p}`, source: p, target: uri, kind: 'sub' }, classes: 'sub' });
    }
  }
  for (const p of [...s.props.values()].sort((a, b) => a.uri.localeCompare(b.uri))) {
    if (p.kind !== 'object') continue;
    for (const d of p.domains) for (const r of p.ranges) {
      if (!visible.has(d) || !visible.has(r)) continue;
      out.push({
        group: 'edges',
        data: { id: `prop|${p.uri}|${d}|${r}`, source: d, target: r, uri: p.uri, label: pickLabel(p.labels, o.lang, p.uri), kind: 'prop' },
        classes: 'prop',
      });
    }
  }
  return out;
}

// ---------------------------------------------------------------- instances

export interface InstNode { uri: string; labels: Label[]; type?: string; depth: number }
export interface InstLink { s: string; o: string; preds: string[] }
export interface InstGroup { id: string; anchor: string; pred: string; dir: 'out' | 'in'; type?: string; members: string[] }
export interface InstGraph {
  root: string | null;
  nodes: Map<string, InstNode>;
  links: Map<string, InstLink>;
  groups: Map<string, InstGroup>;
  expanded: Set<string>;
}
export interface Neighbour { other: string; pred: string; dir: 'out' | 'in' }
export interface NodeInfo { labels: Label[]; types: string[] }

export const emptyInstGraph = (): InstGraph => ({ root: null, nodes: new Map(), links: new Map(), groups: new Map(), expanded: new Set() });

export function mostSpecificType(types: string[], s: Schema): string | undefined {
  return mostSpecific(types.filter((t) => s.classes.has(t)), s.ancestors)[0];
}

function addLink(links: Map<string, InstLink>, a: string, pred: string, b: string) {
  const key = a < b ? `${a}|${b}` : `${b}|${a}`;
  const l = links.get(key);
  if (!l) links.set(key, { s: a, o: b, preds: [pred] });
  else if (!l.preds.includes(pred)) l.preds.push(pred);
}

/** Start a neighbourhood graph at `root`. */
export function seedInstGraph(root: string, info: NodeInfo | undefined, s: Schema): InstGraph {
  const g = emptyInstGraph();
  g.root = root;
  g.nodes.set(root, { uri: root, labels: info?.labels ?? [], type: mostSpecificType(info?.types ?? [], s), depth: 0 });
  return g;
}

/**
 * Add `anchor`'s neighbours. Neighbours that share a predicate, a direction
 * and a type collapse into one group node once there are more than
 * `groupMax` of them (hundreds of readings stay one node until asked for).
 */
export function expandInstGraph(
  prev: InstGraph, anchor: string, nbrs: Neighbour[], info: Map<string, NodeInfo>, s: Schema, groupMax = 3,
): InstGraph {
  const g: InstGraph = {
    root: prev.root,
    nodes: new Map(prev.nodes),
    links: new Map([...prev.links].map(([k, v]) => [k, { ...v, preds: [...v.preds] }])),
    groups: new Map(prev.groups),
    expanded: new Set(prev.expanded).add(anchor),
  };
  const depth = (g.nodes.get(anchor)?.depth ?? 0) + 1;
  const buckets = new Map<string, { pred: string; dir: 'out' | 'in'; type?: string; members: Set<string> }>();
  for (const n of nbrs) {
    if (n.other === anchor || s.classes.has(n.other) || s.props.has(n.other) || isCoreTerm(n.other)) continue;
    const type = g.nodes.get(n.other)?.type ?? mostSpecificType(info.get(n.other)?.types ?? [], s);
    const key = `${n.pred}|${n.dir}|${type ?? ''}`;
    if (!buckets.has(key)) buckets.set(key, { pred: n.pred, dir: n.dir, type, members: new Set() });
    buckets.get(key)!.members.add(n.other);
  }
  for (const [key, b] of buckets) {
    const members = [...b.members].sort();
    const fresh = members.filter((m) => !g.nodes.has(m));
    const grouped = fresh.length > groupMax;
    for (const m of members) {
      if (grouped && !g.nodes.has(m)) continue;
      if (!g.nodes.has(m)) {
        const i = info.get(m);
        g.nodes.set(m, { uri: m, labels: i?.labels ?? [], type: b.type, depth });
      }
      if (b.dir === 'out') addLink(g.links, anchor, b.pred, m);
      else addLink(g.links, m, b.pred, anchor);
    }
    if (grouped) {
      const id = `group|${anchor}|${key}`;
      g.groups.set(id, { id, anchor, pred: b.pred, dir: b.dir, type: b.type, members: fresh });
    }
  }
  return g;
}

/** Replace a group node by its members. */
export function explodeGroup(prev: InstGraph, id: string, info: Map<string, NodeInfo>): InstGraph {
  const grp = prev.groups.get(id);
  if (!grp) return prev;
  const g: InstGraph = {
    ...prev,
    nodes: new Map(prev.nodes),
    links: new Map([...prev.links].map(([k, v]) => [k, { ...v, preds: [...v.preds] }])),
    groups: new Map(prev.groups),
  };
  g.groups.delete(id);
  const depth = (g.nodes.get(grp.anchor)?.depth ?? 0) + 1;
  for (const m of grp.members) {
    if (!g.nodes.has(m)) g.nodes.set(m, { uri: m, labels: info.get(m)?.labels ?? [], type: grp.type, depth });
    if (grp.dir === 'out') addLink(g.links, grp.anchor, grp.pred, m);
    else addLink(g.links, m, grp.pred, grp.anchor);
  }
  return g;
}

export function instElements(g: InstGraph, s: Schema, lang: string): ElementDef[] {
  const out: ElementDef[] = [];
  const predLabel = (p: string) => pickLabel(s.props.get(p)?.labels, lang, p);
  const typeLabel = (t?: string) => (t ? pickLabel(s.classes.get(t)?.labels, lang, t) : '');
  // Few hierarchies meet in one neighbourhood, so every root present gets a colour.
  const types = [...[...g.nodes.values()].map((n) => n.type), ...[...g.groups.values()].map((x) => x.type)];
  const palette = [...new Set(types.filter((t): t is string => !!t && s.classes.has(t)).map((t) => s.classes.get(t)!.root))].sort();
  for (const n of g.nodes.values()) {
    const name = pickLabel(n.labels, lang, n.uri);
    const t = typeLabel(n.type);
    out.push({
      group: 'nodes',
      data: {
        id: n.uri, uri: n.uri, name, label: t ? `${name}\n${t}` : name,
        color: colorOf(s, n.type, palette), group: groupOf(s, n.type, lang), kind: 'inst', depth: n.depth,
      },
      classes: [n.uri === g.root ? 'inst root' : 'inst', g.expanded.has(n.uri) ? 'done' : ''].join(' ').trim(),
    });
  }
  for (const grp of g.groups.values()) {
    out.push({
      group: 'nodes',
      data: {
        id: grp.id, name: typeLabel(grp.type), label: `${typeLabel(grp.type) || '…'} ×${grp.members.length}`,
        color: colorOf(s, grp.type, palette), group: groupOf(s, grp.type, lang), kind: 'group', depth: (g.nodes.get(grp.anchor)?.depth ?? 0) + 1,
      },
      classes: 'group',
    });
    const [src, tgt] = grp.dir === 'out' ? [grp.anchor, grp.id] : [grp.id, grp.anchor];
    out.push({ group: 'edges', data: { id: `e|${grp.id}`, source: src, target: tgt, label: predLabel(grp.pred), kind: 'link' }, classes: 'link' });
  }
  for (const [key, l] of g.links) {
    out.push({
      group: 'edges',
      data: { id: `e|${key}`, source: l.s, target: l.o, label: l.preds.map(predLabel).join(' · '), kind: 'link' },
      classes: 'link',
    });
  }
  return out;
}
