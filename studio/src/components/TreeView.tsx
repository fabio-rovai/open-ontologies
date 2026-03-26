import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import * as mcp from '../lib/mcp-client';
import { useEngine } from '../hooks/useEngine';

interface TreeViewProps {
  onNodeSelect: (node: { id: string; label: string; uri: string } | null) => void;
}

interface SparqlBinding {
  [key: string]: { type: string; value: string };
}

type NodeType = 'Class' | 'ObjectProperty' | 'DatatypeProperty' | 'Individual';

interface TreeNode {
  id: string;
  label: string;
  uri: string;
  nodeType: NodeType;
  children: TreeNode[];
  childCount: number;
  depth: number;
}

function parseSparqlResults(text: string): SparqlBinding[] {
  try {
    const parsed = JSON.parse(text);
    const rows: Record<string, string>[] = parsed?.results ?? [];
    return rows.map(row => {
      const binding: SparqlBinding = {};
      for (const [key, val] of Object.entries(row)) {
        const s = String(val);
        if (s.startsWith('<') && s.endsWith('>')) {
          binding[key] = { type: 'uri', value: s.slice(1, -1) };
        } else {
          const unquoted = s.replace(/^"(.*)"(@\w+)?(\^\^.*)?$/, '$1').replace(/\\"/g, '"');
          binding[key] = { type: 'literal', value: unquoted };
        }
      }
      return binding;
    });
  } catch {
    return [];
  }
}

function shortUri(uri: string): string {
  const hash = uri.lastIndexOf('#');
  if (hash >= 0) return uri.slice(hash + 1);
  const slash = uri.lastIndexOf('/');
  if (slash >= 0) return uri.slice(slash + 1);
  return uri;
}

const TYPE_COLORS: Record<NodeType, string> = {
  Class: '#89b4fa',
  ObjectProperty: '#a6e3a1',
  DatatypeProperty: '#f9e2af',
  Individual: '#fab387',
};

const QUERIES = {
  classes: `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?c ?label WHERE {
  { ?c a owl:Class } UNION { ?c a rdfs:Class }
  OPTIONAL { ?c rdfs:label ?label }
  FILTER(!isBlank(?c))
}`,
  subclass: `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?sub ?parent WHERE {
  ?sub rdfs:subClassOf ?parent .
  { ?sub a owl:Class } UNION { ?sub a rdfs:Class }
  FILTER(!isBlank(?sub) && !isBlank(?parent))
}`,
  objProps: `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT ?p ?label ?parent WHERE {
  { ?p a owl:ObjectProperty } UNION { ?p a rdf:Property }
  OPTIONAL { ?p rdfs:label ?label }
  OPTIONAL { ?p rdfs:subPropertyOf ?parent . FILTER(!isBlank(?parent)) }
  FILTER(!isBlank(?p))
}`,
  dataProps: `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?p ?label WHERE {
  ?p a owl:DatatypeProperty .
  OPTIONAL { ?p rdfs:label ?label }
  FILTER(!isBlank(?p))
}`,
  individuals: `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?ind ?label ?type WHERE {
  ?ind a ?type . ?type a owl:Class .
  OPTIONAL { ?ind rdfs:label ?label }
  FILTER(!isBlank(?ind) && ?type != owl:Class && ?type != rdfs:Class && ?type != owl:ObjectProperty && ?type != owl:DatatypeProperty && ?type != owl:AnnotationProperty && ?type != owl:NamedIndividual)
} LIMIT 500`,
};

function countDescendants(node: TreeNode): number {
  let count = node.children.length;
  for (const c of node.children) count += countDescendants(c);
  node.childCount = count;
  return count;
}

function matchesSearch(node: TreeNode, term: string): boolean {
  if (node.label.toLowerCase().includes(term) || node.id.toLowerCase().includes(term)) return true;
  return node.children.some(c => matchesSearch(c, term));
}

// Flatten visible nodes for virtualized rendering
interface FlatNode {
  node: TreeNode;
  indent: number;
  isExpanded: boolean;
  hasChildren: boolean;
}

function flattenTree(nodes: TreeNode[], expanded: Set<string>, searchTerm: string, indent: number): FlatNode[] {
  const result: FlatNode[] = [];
  for (const node of nodes) {
    const visible = !searchTerm || matchesSearch(node, searchTerm);
    if (!visible) continue;
    const isExpanded = expanded.has(node.id);
    result.push({ node, indent, isExpanded, hasChildren: node.children.length > 0 });
    if (isExpanded) {
      const children = searchTerm
        ? node.children.filter(c => matchesSearch(c, searchTerm))
        : node.children;
      result.push(...flattenTree(children, expanded, searchTerm, indent + 1));
    }
  }
  return result;
}

const ROW_HEIGHT = 28;
const OVERSCAN = 20;

export function TreeView({ onNodeSelect }: TreeViewProps) {
  const { status, refreshStats } = useEngine();
  const [roots, setRoots] = useState<TreeNode[]>([]);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [stats, setStats] = useState({ classes: 0, properties: 0, individuals: 0, depth: 0 });
  const [typeCounts, setTypeCounts] = useState<Map<NodeType, number>>(new Map());
  const [hiddenTypes, setHiddenTypes] = useState<Set<NodeType>>(new Set());
  const scrollRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewHeight, setViewHeight] = useState(600);

  const loadTree = useCallback(async () => {
    try {
      const [classesText, subclassText, objPropsText, dataPropsText, individualsText] = await Promise.all([
        mcp.sparqlQuery(QUERIES.classes),
        mcp.sparqlQuery(QUERIES.subclass),
        mcp.sparqlQuery(QUERIES.objProps),
        mcp.sparqlQuery(QUERIES.dataProps),
        mcp.sparqlQuery(QUERIES.individuals),
      ]);

      const nodeMap = new Map<string, { label: string; uri: string; nodeType: NodeType }>();
      const parentToChildren = new Map<string, Set<string>>();
      const hasParent = new Set<string>();

      // Classes
      for (const b of parseSparqlResults(classesText)) {
        const uri = b.c?.value; if (!uri) continue;
        const id = shortUri(uri);
        if (!nodeMap.has(id)) nodeMap.set(id, { label: b.label?.value || id, uri, nodeType: 'Class' });
      }

      // SubClass hierarchy
      for (const b of parseSparqlResults(subclassText)) {
        const subUri = b.sub?.value, parentUri = b.parent?.value;
        if (!subUri || !parentUri) continue;
        const sid = shortUri(subUri), pid = shortUri(parentUri);
        if (!nodeMap.has(pid)) nodeMap.set(pid, { label: pid, uri: parentUri, nodeType: 'Class' });
        if (!parentToChildren.has(pid)) parentToChildren.set(pid, new Set());
        parentToChildren.get(pid)!.add(sid);
        hasParent.add(sid);
      }

      // Object properties
      const propParentToChildren = new Map<string, Set<string>>();
      const propHasParent = new Set<string>();
      for (const b of parseSparqlResults(objPropsText)) {
        const uri = b.p?.value; if (!uri) continue;
        const id = shortUri(uri);
        if (!nodeMap.has(id)) nodeMap.set(id, { label: b.label?.value || id, uri, nodeType: 'ObjectProperty' });
        if (b.parent?.value) {
          const pid = shortUri(b.parent.value);
          if (!propParentToChildren.has(pid)) propParentToChildren.set(pid, new Set());
          propParentToChildren.get(pid)!.add(id);
          propHasParent.add(id);
        }
      }

      // Data properties
      for (const b of parseSparqlResults(dataPropsText)) {
        const uri = b.p?.value; if (!uri) continue;
        const id = shortUri(uri);
        if (!nodeMap.has(id)) nodeMap.set(id, { label: b.label?.value || id, uri, nodeType: 'DatatypeProperty' });
      }

      // Individuals
      const indsByClass = new Map<string, string[]>();
      for (const b of parseSparqlResults(individualsText)) {
        const uri = b.ind?.value, typeUri = b.type?.value;
        if (!uri || !typeUri) continue;
        const id = shortUri(uri), tid = shortUri(typeUri);
        if (!nodeMap.has(id)) nodeMap.set(id, { label: b.label?.value || id, uri, nodeType: 'Individual' });
        if (!indsByClass.has(tid)) indsByClass.set(tid, []);
        indsByClass.get(tid)!.push(id);
      }

      // Build tree
      const visited = new Set<string>();

      function buildClassTree(id: string, depth: number): TreeNode {
        visited.add(id);
        const data = nodeMap.get(id)!;
        const children: TreeNode[] = [];
        for (const cid of parentToChildren.get(id) ?? new Set()) {
          if (!visited.has(cid) && nodeMap.has(cid)) children.push(buildClassTree(cid, depth + 1));
        }
        for (const iid of indsByClass.get(id) ?? []) {
          if (!visited.has(iid) && nodeMap.has(iid)) {
            visited.add(iid);
            const idata = nodeMap.get(iid)!;
            children.push({ id: iid, label: idata.label, uri: idata.uri, nodeType: 'Individual', children: [], childCount: 0, depth: depth + 1 });
          }
        }
        children.sort((a, b) => a.label.localeCompare(b.label));
        return { id, label: data.label, uri: data.uri, nodeType: data.nodeType, children, childCount: 0, depth };
      }

      function buildPropTree(id: string, depth: number): TreeNode {
        visited.add(id);
        const data = nodeMap.get(id)!;
        const children: TreeNode[] = [];
        for (const cid of propParentToChildren.get(id) ?? new Set()) {
          if (!visited.has(cid) && nodeMap.has(cid)) children.push(buildPropTree(cid, depth + 1));
        }
        children.sort((a, b) => a.label.localeCompare(b.label));
        return { id, label: data.label, uri: data.uri, nodeType: data.nodeType, children, childCount: 0, depth };
      }

      // Class roots
      const classRoots: TreeNode[] = [];
      for (const [id, data] of nodeMap) {
        if (data.nodeType === 'Class' && !hasParent.has(id) && !visited.has(id)) {
          classRoots.push(buildClassTree(id, 1));
        }
      }
      // Orphan classes
      for (const [id, data] of nodeMap) {
        if (data.nodeType === 'Class' && !visited.has(id)) {
          visited.add(id);
          classRoots.push({ id, label: data.label, uri: data.uri, nodeType: 'Class', children: [], childCount: 0, depth: 1 });
        }
      }
      classRoots.sort((a, b) => a.label.localeCompare(b.label));

      // Property roots
      const propRoots: TreeNode[] = [];
      for (const [id, data] of nodeMap) {
        if ((data.nodeType === 'ObjectProperty' || data.nodeType === 'DatatypeProperty') && !propHasParent.has(id) && !visited.has(id)) {
          propRoots.push(buildPropTree(id, 1));
        }
      }
      for (const [id, data] of nodeMap) {
        if ((data.nodeType === 'ObjectProperty' || data.nodeType === 'DatatypeProperty') && !visited.has(id)) {
          visited.add(id);
          propRoots.push({ id, label: data.label, uri: data.uri, nodeType: data.nodeType, children: [], childCount: 0, depth: 1 });
        }
      }
      propRoots.sort((a, b) => a.label.localeCompare(b.label));

      // Orphan individuals
      const orphanInds: TreeNode[] = [];
      for (const [id, data] of nodeMap) {
        if (data.nodeType === 'Individual' && !visited.has(id)) {
          visited.add(id);
          orphanInds.push({ id, label: data.label, uri: data.uri, nodeType: 'Individual', children: [], childCount: 0, depth: 1 });
        }
      }

      // Assemble
      const treeRoots: TreeNode[] = [];
      if (classRoots.length > 0) {
        const branch: TreeNode = { id: '__classes__', label: `Classes (${classRoots.length})`, uri: '', nodeType: 'Class', children: classRoots, childCount: 0, depth: 0 };
        countDescendants(branch);
        treeRoots.push(branch);
      }
      if (propRoots.length > 0) {
        const branch: TreeNode = { id: '__properties__', label: `Properties (${propRoots.length})`, uri: '', nodeType: 'ObjectProperty', children: propRoots, childCount: 0, depth: 0 };
        countDescendants(branch);
        treeRoots.push(branch);
      }
      if (orphanInds.length > 0) {
        const branch: TreeNode = { id: '__individuals__', label: `Individuals (${orphanInds.length})`, uri: '', nodeType: 'Individual', children: orphanInds, childCount: 0, depth: 0 };
        countDescendants(branch);
        treeRoots.push(branch);
      }

      // Stats
      const tc = new Map<NodeType, number>();
      for (const data of nodeMap.values()) tc.set(data.nodeType, (tc.get(data.nodeType) ?? 0) + 1);
      setTypeCounts(tc);

      let maxDepth = 0;
      function findDepth(n: TreeNode) { if (n.depth > maxDepth) maxDepth = n.depth; n.children.forEach(findDepth); }
      treeRoots.forEach(findDepth);

      setStats({
        classes: tc.get('Class') ?? 0,
        properties: (tc.get('ObjectProperty') ?? 0) + (tc.get('DatatypeProperty') ?? 0),
        individuals: tc.get('Individual') ?? 0,
        depth: maxDepth,
      });

      setRoots(treeRoots);
      // Auto-expand top-level branches
      setExpanded(new Set(treeRoots.map(r => r.id)));
      refreshStats();
    } catch (e) {
      console.error('Failed to load tree:', e);
    }
  }, [refreshStats]);

  useEffect(() => { if (status === 'connected') loadTree(); }, [status, loadTree]);
  useEffect(() => {
    (window as unknown as Record<string, unknown>).__refreshGraph = loadTree;
    return () => { delete (window as unknown as Record<string, unknown>).__refreshGraph; };
  }, [loadTree]);

  const toggleExpand = useCallback((id: string) => {
    setExpanded(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  }, []);

  const handleSelect = useCallback((node: TreeNode) => {
    if (node.id.startsWith('__')) return;
    setSelectedId(node.id);
    onNodeSelect({ id: node.id, label: node.label, uri: node.uri });
  }, [onNodeSelect]);

  const normalizedSearch = searchTerm.toLowerCase().trim();

  // Auto-expand matching paths during search
  const effectiveExpanded = useMemo(() => {
    if (!normalizedSearch) return expanded;
    const auto = new Set<string>();
    function walk(n: TreeNode): boolean {
      const selfMatch = n.label.toLowerCase().includes(normalizedSearch) || n.id.toLowerCase().includes(normalizedSearch);
      let childMatch = false;
      for (const c of n.children) { if (walk(c)) childMatch = true; }
      if (childMatch) auto.add(n.id);
      return selfMatch || childMatch;
    }
    roots.forEach(walk);
    return auto;
  }, [roots, normalizedSearch, expanded]);

  // Filter by hidden types
  const filteredRoots = useMemo(() => {
    if (hiddenTypes.size === 0) return roots;
    function filterNode(n: TreeNode): TreeNode | null {
      if (hiddenTypes.has(n.nodeType) && !n.id.startsWith('__')) return null;
      const children = n.children.map(filterNode).filter(Boolean) as TreeNode[];
      if (children.length === 0 && n.children.length > 0 && hiddenTypes.has(n.nodeType)) return null;
      return { ...n, children };
    }
    return roots.map(filterNode).filter(Boolean) as TreeNode[];
  }, [roots, hiddenTypes]);

  // Flatten for virtual scroll
  const flatNodes = useMemo(
    () => flattenTree(filteredRoots, effectiveExpanded, normalizedSearch, 0),
    [filteredRoots, effectiveExpanded, normalizedSearch]
  );

  // Virtual scroll
  const totalHeight = flatNodes.length * ROW_HEIGHT;
  const startIdx = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const endIdx = Math.min(flatNodes.length, Math.ceil((scrollTop + viewHeight) / ROW_HEIGHT) + OVERSCAN);
  const visibleNodes = flatNodes.slice(startIdx, endIdx);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const onScroll = () => setScrollTop(el.scrollTop);
    const ro = new ResizeObserver(() => setViewHeight(el.clientHeight));
    el.addEventListener('scroll', onScroll, { passive: true });
    ro.observe(el);
    setViewHeight(el.clientHeight);
    return () => { el.removeEventListener('scroll', onScroll); ro.disconnect(); };
  }, []);

  const expandAll = useCallback(() => {
    const all = new Set<string>();
    function walk(n: TreeNode) { if (n.children.length > 0) { all.add(n.id); n.children.forEach(walk); } }
    roots.forEach(walk);
    setExpanded(all);
  }, [roots]);

  const collapseAll = useCallback(() => setExpanded(new Set(roots.map(r => r.id))), [roots]);

  const toggleType = useCallback((type: NodeType) => {
    setHiddenTypes(prev => {
      const next = new Set(prev);
      if (next.has(type)) next.delete(type); else next.add(type);
      return next;
    });
  }, []);

  const typeOrder: NodeType[] = ['Class', 'ObjectProperty', 'DatatypeProperty', 'Individual'];

  return (
    <div className="absolute inset-0 flex flex-col" style={{ background: '#1e1e2e' }}>
      {/* Header */}
      <div style={{ padding: '8px 12px', borderBottom: '1px solid #313244', background: '#181825', flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 6 }}>
        {/* Stats */}
        <div style={{ display: 'flex', gap: 8, fontSize: 11, color: '#6c7086', flexWrap: 'wrap' }}>
          {[
            { label: 'classes', value: stats.classes, color: TYPE_COLORS.Class },
            { label: 'properties', value: stats.properties, color: TYPE_COLORS.ObjectProperty },
            { label: 'individuals', value: stats.individuals, color: TYPE_COLORS.Individual },
            { label: 'depth', value: stats.depth, color: '#cdd6f4' },
          ].map(s => (
            <span key={s.label} style={{ display: 'flex', alignItems: 'center', gap: 3 }}>
              <span style={{ color: s.color, fontWeight: 600 }}>{s.value}</span> {s.label}
            </span>
          ))}
        </div>

        {/* Search */}
        <input
          type="text"
          placeholder="Search..."
          value={searchTerm}
          onChange={e => setSearchTerm(e.target.value)}
          style={{
            width: '100%', padding: '5px 10px', borderRadius: 6, border: '1px solid #313244',
            background: '#1e1e2e', color: '#cdd6f4', fontSize: 12, outline: 'none',
            fontFamily: '-apple-system, BlinkMacSystemFont, sans-serif',
          }}
        />

        {/* Type filters + controls */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexWrap: 'wrap' }}>
          {typeOrder.filter(t => (typeCounts.get(t) ?? 0) > 0).map(type => (
            <button key={type} onClick={() => toggleType(type)} style={{
              background: 'none', border: '1px solid #313244', borderRadius: 4, padding: '1px 6px',
              fontSize: 10, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 3,
              color: hiddenTypes.has(type) ? '#45475a' : '#bac2de',
              opacity: hiddenTypes.has(type) ? 0.4 : 1,
              textDecoration: hiddenTypes.has(type) ? 'line-through' : 'none',
            }}>
              <span style={{ width: 6, height: 6, borderRadius: '50%', background: TYPE_COLORS[type] }} />
              {type} ({typeCounts.get(type) ?? 0})
            </button>
          ))}
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 3 }}>
            <button onClick={expandAll} style={{ background: 'none', border: '1px solid #313244', color: '#6c7086', borderRadius: 4, padding: '1px 6px', fontSize: 10, cursor: 'pointer' }}>
              Expand
            </button>
            <button onClick={collapseAll} style={{ background: 'none', border: '1px solid #313244', color: '#6c7086', borderRadius: 4, padding: '1px 6px', fontSize: 10, cursor: 'pointer' }}>
              Collapse
            </button>
          </div>
        </div>

        {normalizedSearch && (
          <div style={{ fontSize: 10, color: '#6c7086' }}>{flatNodes.length} matches</div>
        )}
      </div>

      {/* Virtualized tree */}
      <div ref={scrollRef} style={{ flex: 1, overflow: 'auto' }}>
        <div style={{ height: totalHeight, position: 'relative' }}>
          {visibleNodes.map((flat, i) => {
            const { node, indent, isExpanded, hasChildren } = flat;
            const isSelected = selectedId === node.id;
            const color = TYPE_COLORS[node.nodeType] ?? '#a6adc8';
            const isLeaf = !hasChildren;
            const isBranch = node.id.startsWith('__');
            const top = (startIdx + i) * ROW_HEIGHT;

            // Highlight search match
            let labelEl: React.ReactNode = node.label;
            if (normalizedSearch) {
              const idx = node.label.toLowerCase().indexOf(normalizedSearch);
              if (idx >= 0) {
                labelEl = <>
                  {node.label.slice(0, idx)}
                  <span style={{ background: '#f9e2af33', color: '#f9e2af', borderRadius: 2, padding: '0 1px' }}>
                    {node.label.slice(idx, idx + normalizedSearch.length)}
                  </span>
                  {node.label.slice(idx + normalizedSearch.length)}
                </>;
              }
            }

            return (
              <div
                key={node.id}
                onClick={() => handleSelect(node)}
                onDoubleClick={() => hasChildren && toggleExpand(node.id)}
                style={{
                  position: 'absolute',
                  top,
                  left: 0,
                  right: 0,
                  height: ROW_HEIGHT,
                  paddingLeft: indent * 18 + 10,
                  paddingRight: 10,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  cursor: 'pointer',
                  background: isSelected ? '#313244' : 'transparent',
                  borderLeft: isSelected ? `2px solid ${color}` : '2px solid transparent',
                  fontSize: isBranch ? 12 : 11,
                  fontWeight: isBranch ? 600 : hasChildren ? 500 : 400,
                  color: isSelected ? '#cdd6f4' : isBranch ? '#cdd6f4' : hasChildren ? '#bac2de' : '#a6adc8',
                  fontFamily: '-apple-system, BlinkMacSystemFont, sans-serif',
                  userSelect: 'none',
                }}
                onMouseEnter={e => { if (!isSelected) e.currentTarget.style.background = '#181825'; }}
                onMouseLeave={e => { if (!isSelected) e.currentTarget.style.background = 'transparent'; }}
              >
                {/* Arrow */}
                <span
                  onClick={e => { e.stopPropagation(); if (hasChildren) toggleExpand(node.id); }}
                  style={{
                    width: 14, fontSize: 8, color: '#585b70', flexShrink: 0, textAlign: 'center',
                    transition: 'transform 0.1s',
                    transform: hasChildren ? (isExpanded ? 'rotate(90deg)' : 'rotate(0deg)') : 'none',
                    visibility: hasChildren ? 'visible' : 'hidden',
                  }}
                >
                  {'\u25B6'}
                </span>

                {/* Type dot */}
                <span style={{
                  width: isLeaf ? 5 : 7, height: isLeaf ? 5 : 7,
                  borderRadius: isLeaf ? '50%' : 2,
                  background: color, flexShrink: 0, opacity: isLeaf ? 0.6 : 1,
                }} />

                {/* Label */}
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}>
                  {labelEl}
                </span>

                {/* Child count */}
                {hasChildren && node.childCount > 0 && (
                  <span style={{ fontSize: 9, color: '#45475a', flexShrink: 0 }}>
                    {node.childCount}
                  </span>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Footer */}
      <div style={{ padding: '4px 12px', borderTop: '1px solid #313244', background: '#181825', fontSize: 9, color: '#45475a', flexShrink: 0 }}>
        Click to inspect · Double-click to expand · Search to filter
      </div>
    </div>
  );
}
