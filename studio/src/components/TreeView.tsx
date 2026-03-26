import { useRef, useEffect, useCallback, useState } from 'react';
import * as d3 from 'd3';
import * as mcp from '../lib/mcp-client';
import { useEngine } from '../hooks/useEngine';

interface TreeViewProps {
  onNodeSelect: (node: { id: string; label: string; uri: string } | null) => void;
}

interface SparqlBinding {
  [key: string]: { type: string; value: string };
}

type NodeType = 'Class' | 'ObjectProperty' | 'DatatypeProperty' | 'AnnotationProperty' | 'Individual' | 'Restriction' | 'Ontology';

interface OntologyNode {
  id: string;
  label: string;
  uri: string;
  nodeType: NodeType;
  children: OntologyNode[];
  _childCount?: number;
  propertyCount?: number;    // how many properties reference this class
  connectionCount?: number;  // total edges (for sizing)
}

interface CrossLink {
  sourceId: string;
  targetId: string;
  label: string;
  linkType: 'domain' | 'range' | 'equivalentClass' | 'disjointWith' | 'restriction';
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

// Node type colors (Catppuccin Mocha)
const NODE_TYPE_COLORS: Record<NodeType, string> = {
  Ontology:            '#f38ba8', // pink
  Class:               '#89b4fa', // blue
  ObjectProperty:      '#a6e3a1', // green
  DatatypeProperty:    '#f9e2af', // yellow
  AnnotationProperty:  '#cba6f7', // mauve
  Individual:          '#fab387', // peach
  Restriction:         '#94e2d5', // teal
};

const NODE_TYPE_SIZES: Record<NodeType, number> = {
  Ontology: 14,
  Class: 8,
  ObjectProperty: 7,
  DatatypeProperty: 6,
  AnnotationProperty: 5,
  Individual: 6,
  Restriction: 5,
};

function getColor(type: NodeType): string {
  return NODE_TYPE_COLORS[type] ?? '#a6adc8';
}

function getSize(type: NodeType, connectionCount: number): number {
  const base = NODE_TYPE_SIZES[type] ?? 6;
  // Scale up for highly connected nodes
  return base + Math.min(connectionCount * 0.3, 6);
}

// ── SPARQL Queries ──────────────────────────────────────────────

const CLASSES_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?c ?label WHERE {
  { ?c a owl:Class } UNION { ?c a rdfs:Class }
  OPTIONAL { ?c rdfs:label ?label }
  FILTER(!isBlank(?c))
}`;

const SUBCLASS_QUERY = `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?sub ?parent WHERE {
  ?sub rdfs:subClassOf ?parent .
  { ?sub a owl:Class } UNION { ?sub a rdfs:Class }
  FILTER(!isBlank(?sub) && !isBlank(?parent))
}`;

const OBJ_PROPS_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT ?p ?label ?domain ?range ?parent WHERE {
  { ?p a owl:ObjectProperty } UNION { ?p a rdf:Property }
  OPTIONAL { ?p rdfs:label ?label }
  OPTIONAL { ?p rdfs:domain ?domain . FILTER(!isBlank(?domain)) }
  OPTIONAL { ?p rdfs:range ?range . FILTER(!isBlank(?range)) }
  OPTIONAL { ?p rdfs:subPropertyOf ?parent . FILTER(!isBlank(?parent)) }
  FILTER(!isBlank(?p))
}`;

const DATA_PROPS_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?p ?label ?domain ?parent WHERE {
  ?p a owl:DatatypeProperty .
  OPTIONAL { ?p rdfs:label ?label }
  OPTIONAL { ?p rdfs:domain ?domain . FILTER(!isBlank(?domain)) }
  OPTIONAL { ?p rdfs:subPropertyOf ?parent . FILTER(!isBlank(?parent)) }
  FILTER(!isBlank(?p))
}`;

const ANNOTATION_PROPS_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?p ?label WHERE {
  ?p a owl:AnnotationProperty .
  OPTIONAL { ?p rdfs:label ?label }
  FILTER(!isBlank(?p))
}`;

const INDIVIDUALS_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT ?ind ?label ?type WHERE {
  ?ind a ?type .
  ?type a owl:Class .
  OPTIONAL { ?ind rdfs:label ?label }
  FILTER(!isBlank(?ind) && ?type != owl:Class && ?type != rdfs:Class && ?type != owl:ObjectProperty && ?type != owl:DatatypeProperty && ?type != owl:AnnotationProperty && ?type != owl:NamedIndividual)
} LIMIT 200`;

const EQUIV_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?a ?b WHERE {
  ?a owl:equivalentClass ?b .
  FILTER(!isBlank(?a) && !isBlank(?b) && ?a != ?b)
}`;

const DISJOINT_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?a ?b WHERE {
  ?a owl:disjointWith ?b .
  FILTER(!isBlank(?a) && !isBlank(?b))
}`;

function countDescendants(node: OntologyNode): number {
  if (!node.children || node.children.length === 0) return 0;
  let count = node.children.length;
  for (const c of node.children) count += countDescendants(c);
  return count;
}

export function TreeView({ onNodeSelect }: TreeViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const gRef = useRef<d3.Selection<SVGGElement, unknown, null, undefined> | null>(null);
  const zoomRef = useRef<d3.ZoomBehavior<SVGSVGElement, unknown> | null>(null);
  const treeDataRef = useRef<OntologyNode | null>(null);
  const crossLinksRef = useRef<CrossLink[]>([]);
  const collapsedRef = useRef<Set<string>>(new Set());
  const hiddenTypesRef = useRef<Set<NodeType>>(new Set());
  const { status, refreshStats } = useEngine();
  const [stats, setStats] = useState({ classes: 0, properties: 0, individuals: 0, depth: 0, crossLinks: 0 });
  const [hiddenTypes, setHiddenTypes] = useState<Set<NodeType>>(new Set());
  const [typeCounts, setTypeCounts] = useState<Map<NodeType, number>>(new Map());

  const filterTree = useCallback((node: OntologyNode): OntologyNode => {
    const copy = { ...node };
    if (collapsedRef.current.has(node.id)) {
      copy._childCount = countDescendants(node);
      copy.children = [];
    } else if (node.children) {
      copy.children = node.children
        .filter(c => !hiddenTypesRef.current.has(c.nodeType))
        .map(c => filterTree(c));
    }
    return copy;
  }, []);

  const renderTree = useCallback(() => {
    const svg = svgRef.current;
    const g = gRef.current;
    const treeData = treeDataRef.current;
    if (!svg || !g || !treeData) return;

    g.selectAll('*').remove();

    const filteredData = filterTree(treeData);
    const hierarchy = d3.hierarchy(filteredData, d => d.children);

    const leafCount = hierarchy.leaves().length;
    const rect = containerRef.current?.getBoundingClientRect();
    const viewW = rect?.width ?? 800;
    const viewH = rect?.height ?? 600;

    const treeHeight = Math.max(viewH - 80, leafCount * 24);
    const treeWidth = Math.max(viewW - 300, hierarchy.height * 200);

    const treeLayout = d3.tree<OntologyNode>()
      .size([treeHeight, treeWidth])
      .separation((a, b) => (a.parent === b.parent ? 1 : 1.4));

    treeLayout(hierarchy);

    const xOff = 180;
    const yOff = 40;

    // Build node position map for cross-links
    const nodePositions = new Map<string, { x: number; y: number }>();
    hierarchy.descendants().forEach(d => {
      nodePositions.set(d.data.id, { x: (d.y ?? 0) + xOff, y: (d.x ?? 0) + yOff });
    });

    // Draw tree links
    g.selectAll('.tree-link')
      .data(hierarchy.links())
      .enter().append('path')
      .attr('class', 'tree-link')
      .attr('fill', 'none')
      .attr('stroke', d => getColor(d.target.data.nodeType))
      .attr('stroke-opacity', 0.25)
      .attr('stroke-width', 1.5)
      .attr('d', d3.linkHorizontal<d3.HierarchyPointLink<OntologyNode>, d3.HierarchyPointNode<OntologyNode>>()
        .x(d => d.y + xOff)
        .y(d => d.x + yOff) as unknown as string);

    // Draw cross-links (domain→range, equivalentClass, disjointWith)
    const visibleCrossLinks = crossLinksRef.current.filter(cl => {
      const s = nodePositions.get(cl.sourceId);
      const t = nodePositions.get(cl.targetId);
      return s && t;
    });

    const crossLinkColor: Record<string, string> = {
      domain: '#a6e3a199',
      range: '#f9e2af99',
      equivalentClass: '#cba6f799',
      disjointWith: '#f38ba866',
      restriction: '#94e2d566',
    };

    g.selectAll('.cross-link')
      .data(visibleCrossLinks)
      .enter().append('path')
      .attr('class', 'cross-link')
      .attr('fill', 'none')
      .attr('stroke', d => crossLinkColor[d.linkType] ?? '#585b7044')
      .attr('stroke-opacity', 0.2)
      .attr('stroke-width', 1)
      .attr('stroke-dasharray', d => d.linkType === 'disjointWith' ? '2,4' : '4,4')
      .attr('d', d => {
        const s = nodePositions.get(d.sourceId)!;
        const t = nodePositions.get(d.targetId)!;
        const mx = (s.x + t.x) / 2;
        return `M${s.x},${s.y}C${mx},${s.y},${mx},${t.y},${t.x},${t.y}`;
      });

    // Draw nodes
    const nodes = g.selectAll('.tree-node')
      .data(hierarchy.descendants())
      .enter().append('g')
      .attr('class', d => {
        let cls = 'tree-node';
        if (collapsedRef.current.has(d.data.id)) cls += ' collapsed';
        return cls;
      })
      .attr('transform', d => `translate(${(d.y ?? 0) + xOff},${(d.x ?? 0) + yOff})`)
      .style('cursor', 'pointer')
      .on('click', (event: MouseEvent, d) => {
        event.stopPropagation();
        if (event.shiftKey || event.metaKey) {
          toggleCollapse(d.data.id);
        } else {
          if (d.data.id === '__ontology__') return;
          onNodeSelect({ id: d.data.id, label: d.data.label, uri: d.data.uri });
        }
      })
      .on('dblclick', (event: MouseEvent, d) => {
        event.stopPropagation();
        toggleCollapse(d.data.id);
      })
      .on('mouseover', function(_event: MouseEvent, d) {
        d3.select(this).select('circle')
          .attr('stroke', '#cdd6f4').attr('stroke-width', 3);

        // Highlight path to root
        const ancestors = new Set<string>();
        let current: d3.HierarchyNode<OntologyNode> | null = d;
        while (current) { ancestors.add(current.data.id); current = current.parent; }

        g.selectAll<SVGPathElement, d3.HierarchyPointLink<OntologyNode>>('.tree-link')
          .attr('stroke-opacity', l =>
            ancestors.has(l.source.data.id) && ancestors.has(l.target.data.id) ? 0.8 : 0.08
          )
          .attr('stroke-width', l =>
            ancestors.has(l.source.data.id) && ancestors.has(l.target.data.id) ? 2.5 : 1
          );

        // Highlight connected cross-links
        g.selectAll<SVGPathElement, CrossLink>('.cross-link')
          .attr('stroke-opacity', cl =>
            cl.sourceId === d.data.id || cl.targetId === d.data.id ? 0.6 : 0.05
          )
          .attr('stroke-width', cl =>
            cl.sourceId === d.data.id || cl.targetId === d.data.id ? 2 : 1
          );
      })
      .on('mouseout', function() {
        d3.select(this).select('circle')
          .attr('stroke', '#1e1e2e').attr('stroke-width', 2);
        g.selectAll('.tree-link').attr('stroke-opacity', 0.25).attr('stroke-width', 1.5);
        g.selectAll('.cross-link').attr('stroke-opacity', 0.2).attr('stroke-width', 1);
      });

    // Node circles — different shapes per type
    nodes.append('circle')
      .attr('r', d => getSize(d.data.nodeType, d.data.connectionCount ?? 0))
      .attr('fill', d => getColor(d.data.nodeType))
      .attr('stroke', '#1e1e2e')
      .attr('stroke-width', 2)
      .style('filter', 'drop-shadow(0 0 4px rgba(0,0,0,0.4))');

    // Property nodes get a diamond overlay
    nodes.filter(d => d.data.nodeType === 'ObjectProperty' || d.data.nodeType === 'DatatypeProperty')
      .append('rect')
      .attr('x', d => -getSize(d.data.nodeType, 0) * 0.5)
      .attr('y', d => -getSize(d.data.nodeType, 0) * 0.5)
      .attr('width', d => getSize(d.data.nodeType, 0))
      .attr('height', d => getSize(d.data.nodeType, 0))
      .attr('transform', 'rotate(45)')
      .attr('fill', 'none')
      .attr('stroke', d => getColor(d.data.nodeType))
      .attr('stroke-width', 1)
      .attr('stroke-opacity', 0.4)
      .style('pointer-events', 'none');

    // Collapse indicator
    nodes.filter(d => (d.data.children && d.data.children.length > 0) || (d.data._childCount != null && d.data._childCount > 0))
      .append('text')
      .attr('x', 0).attr('y', 1)
      .attr('text-anchor', 'middle')
      .attr('dominant-baseline', 'central')
      .style('font-size', d => `${getSize(d.data.nodeType, 0) * 1.1}px`)
      .style('fill', '#1e1e2e')
      .style('font-weight', '700')
      .style('pointer-events', 'none')
      .text(d => collapsedRef.current.has(d.data.id) ? '+' : '');

    // Collapsed child count
    nodes.filter(d => d.data._childCount != null && d.data._childCount > 0)
      .append('text')
      .attr('x', d => getSize(d.data.nodeType, 0) + 4)
      .attr('y', -8)
      .attr('text-anchor', 'start')
      .style('font-size', '9px')
      .style('fill', '#6c7086')
      .style('pointer-events', 'none')
      .text(d => `(${d.data._childCount})`);

    // Labels
    nodes.append('text')
      .attr('x', d => getSize(d.data.nodeType, d.data.connectionCount ?? 0) + 8)
      .attr('y', 4)
      .attr('text-anchor', 'start')
      .style('font-size', d => d.data.nodeType === 'Ontology' ? '14px' : d.depth <= 2 ? '12px' : '11px')
      .style('font-weight', d => d.data.children.length > 0 || d.data.nodeType === 'Ontology' ? '600' : '400')
      .style('fill', d => d.data.nodeType === 'Ontology' ? '#cdd6f4' : '#bac2de')
      .style('font-family', '-apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif')
      .style('pointer-events', 'none')
      .text(d => {
        const lbl = d.data.label || d.data.id;
        return lbl.length > 45 ? lbl.substring(0, 45) + '...' : lbl;
      });

    // Property count badge on classes
    nodes.filter(d => d.data.nodeType === 'Class' && d.data.propertyCount != null && d.data.propertyCount > 0)
      .append('text')
      .attr('x', d => {
        const lbl = d.data.label || d.data.id;
        const truncated = lbl.length > 45 ? lbl.substring(0, 45) + '...' : lbl;
        return getSize(d.data.nodeType, d.data.connectionCount ?? 0) + 8 + truncated.length * 6.5;
      })
      .attr('y', 4)
      .attr('text-anchor', 'start')
      .style('font-size', '9px')
      .style('fill', '#585b70')
      .style('pointer-events', 'none')
      .text(d => `[${d.data.propertyCount}p]`);

  }, [filterTree, onNodeSelect]);

  const toggleCollapse = useCallback((id: string) => {
    if (collapsedRef.current.has(id)) collapsedRef.current.delete(id);
    else collapsedRef.current.add(id);
    renderTree();
  }, [renderTree]);

  const fitGraph = useCallback(() => {
    const svg = svgRef.current;
    const g = gRef.current;
    if (!svg || !g) return;
    const gNode = g.node();
    if (!gNode) return;
    const bbox = gNode.getBBox();
    if (bbox.width === 0 || bbox.height === 0) return;

    const rect = containerRef.current?.getBoundingClientRect();
    const w = rect?.width ?? 800;
    const h = rect?.height ?? 600;

    const pad = 60;
    const scale = Math.min(w / (bbox.width + pad * 2), h / (bbox.height + pad * 2), 1.5);
    const transform = d3.zoomIdentity
      .translate(w / 2, h / 2)
      .scale(scale)
      .translate(-(bbox.x + bbox.width / 2), -(bbox.y + bbox.height / 2));

    if (zoomRef.current) {
      d3.select(svg).transition().duration(500).call(zoomRef.current.transform, transform);
    }
  }, []);

  // Load full ontology from SPARQL
  const loadTree = useCallback(async () => {
    try {
      const [classesText, subclassText, objPropsText, dataPropsText, annotPropsText, individualsText, equivText, disjointText] = await Promise.all([
        mcp.sparqlQuery(CLASSES_QUERY),
        mcp.sparqlQuery(SUBCLASS_QUERY),
        mcp.sparqlQuery(OBJ_PROPS_QUERY),
        mcp.sparqlQuery(DATA_PROPS_QUERY),
        mcp.sparqlQuery(ANNOTATION_PROPS_QUERY),
        mcp.sparqlQuery(INDIVIDUALS_QUERY),
        mcp.sparqlQuery(EQUIV_QUERY),
        mcp.sparqlQuery(DISJOINT_QUERY),
      ]);

      const classBindings = parseSparqlResults(classesText);
      const subclassBindings = parseSparqlResults(subclassText);
      const objPropBindings = parseSparqlResults(objPropsText);
      const dataPropBindings = parseSparqlResults(dataPropsText);
      const annotPropBindings = parseSparqlResults(annotPropsText);
      const individualBindings = parseSparqlResults(individualsText);
      const equivBindings = parseSparqlResults(equivText);
      const disjointBindings = parseSparqlResults(disjointText);

      // ── Build node registry ────────────────────────────────────
      const allNodes = new Map<string, { label: string; uri: string; nodeType: NodeType; connections: number; propertyCount: number }>();
      const parentToChildren = new Map<string, Set<string>>();
      const hasParent = new Set<string>();
      const crossLinks: CrossLink[] = [];

      // Track property counts per class
      const classPropertyCount = new Map<string, number>();

      // Classes
      for (const b of classBindings) {
        const uri = b.c?.value;
        if (!uri) continue;
        const id = shortUri(uri);
        if (!allNodes.has(id)) {
          allNodes.set(id, { label: b.label?.value || id, uri, nodeType: 'Class', connections: 0, propertyCount: 0 });
        }
      }

      // SubClass edges (tree structure)
      for (const b of subclassBindings) {
        const subUri = b.sub?.value;
        const parentUri = b.parent?.value;
        if (!subUri || !parentUri) continue;
        const sid = shortUri(subUri);
        const pid = shortUri(parentUri);

        if (!allNodes.has(pid)) {
          allNodes.set(pid, { label: pid, uri: parentUri, nodeType: 'Class', connections: 0, propertyCount: 0 });
        }

        if (!parentToChildren.has(pid)) parentToChildren.set(pid, new Set());
        parentToChildren.get(pid)!.add(sid);
        hasParent.add(sid);

        const pn = allNodes.get(pid)!; pn.connections++;
        const sn = allNodes.get(sid)!; sn.connections++;
      }

      // Object properties — add to tree under a "Properties" branch, create cross-links for domain/range
      const propParentToChildren = new Map<string, Set<string>>();
      const propHasParent = new Set<string>();
      for (const b of objPropBindings) {
        const uri = b.p?.value;
        if (!uri) continue;
        const id = shortUri(uri);
        if (!allNodes.has(id)) {
          allNodes.set(id, { label: b.label?.value || id, uri, nodeType: 'ObjectProperty', connections: 0, propertyCount: 0 });
        }
        // Property hierarchy
        if (b.parent?.value) {
          const pid = shortUri(b.parent.value);
          if (!allNodes.has(pid)) {
            allNodes.set(pid, { label: pid, uri: b.parent.value, nodeType: 'ObjectProperty', connections: 0, propertyCount: 0 });
          }
          if (!propParentToChildren.has(pid)) propParentToChildren.set(pid, new Set());
          propParentToChildren.get(pid)!.add(id);
          propHasParent.add(id);
        }
        // Domain cross-link
        if (b.domain?.value) {
          const did = shortUri(b.domain.value);
          crossLinks.push({ sourceId: id, targetId: did, label: 'domain', linkType: 'domain' });
          const dn = allNodes.get(did);
          if (dn) { dn.connections++; dn.propertyCount++; }
          const pn = allNodes.get(id)!; pn.connections++;
          classPropertyCount.set(did, (classPropertyCount.get(did) ?? 0) + 1);
        }
        // Range cross-link
        if (b.range?.value) {
          const rid = shortUri(b.range.value);
          crossLinks.push({ sourceId: id, targetId: rid, label: 'range', linkType: 'range' });
          const rn = allNodes.get(rid);
          if (rn) rn.connections++;
          const pn = allNodes.get(id)!; pn.connections++;
        }
      }

      // Datatype properties
      for (const b of dataPropBindings) {
        const uri = b.p?.value;
        if (!uri) continue;
        const id = shortUri(uri);
        if (!allNodes.has(id)) {
          allNodes.set(id, { label: b.label?.value || id, uri, nodeType: 'DatatypeProperty', connections: 0, propertyCount: 0 });
        }
        if (b.parent?.value) {
          const pid = shortUri(b.parent.value);
          if (!propParentToChildren.has(pid)) propParentToChildren.set(pid, new Set());
          propParentToChildren.get(pid)!.add(id);
          propHasParent.add(id);
        }
        if (b.domain?.value) {
          const did = shortUri(b.domain.value);
          crossLinks.push({ sourceId: id, targetId: did, label: 'domain', linkType: 'domain' });
          classPropertyCount.set(did, (classPropertyCount.get(did) ?? 0) + 1);
        }
      }

      // Annotation properties
      for (const b of annotPropBindings) {
        const uri = b.p?.value;
        if (!uri) continue;
        const id = shortUri(uri);
        if (!allNodes.has(id)) {
          allNodes.set(id, { label: b.label?.value || id, uri, nodeType: 'AnnotationProperty', connections: 0, propertyCount: 0 });
        }
      }

      // Individuals
      const individualsByClass = new Map<string, string[]>();
      for (const b of individualBindings) {
        const uri = b.ind?.value;
        const typeUri = b.type?.value;
        if (!uri || !typeUri) continue;
        const id = shortUri(uri);
        const typeId = shortUri(typeUri);
        if (!allNodes.has(id)) {
          allNodes.set(id, { label: b.label?.value || id, uri, nodeType: 'Individual', connections: 0, propertyCount: 0 });
        }
        if (!individualsByClass.has(typeId)) individualsByClass.set(typeId, []);
        individualsByClass.get(typeId)!.push(id);
      }

      // Equivalent class cross-links
      for (const b of equivBindings) {
        const a = b.a?.value;
        const bv = b.b?.value;
        if (!a || !bv) continue;
        crossLinks.push({ sourceId: shortUri(a), targetId: shortUri(bv), label: 'equivalentClass', linkType: 'equivalentClass' });
      }

      // Disjoint cross-links
      for (const b of disjointBindings) {
        const a = b.a?.value;
        const bv = b.b?.value;
        if (!a || !bv) continue;
        crossLinks.push({ sourceId: shortUri(a), targetId: shortUri(bv), label: 'disjointWith', linkType: 'disjointWith' });
      }

      // Set property counts on nodes
      for (const [id, count] of classPropertyCount) {
        const node = allNodes.get(id);
        if (node) node.propertyCount = count;
      }

      // ── Build tree hierarchy ───────────────────────────────────
      const visited = new Set<string>();

      function buildClassNode(id: string): OntologyNode {
        visited.add(id);
        const data = allNodes.get(id)!;
        const childIds = parentToChildren.get(id) ?? new Set();
        const children: OntologyNode[] = [];
        for (const cid of childIds) {
          if (!visited.has(cid) && allNodes.has(cid)) {
            children.push(buildClassNode(cid));
          }
        }
        // Add individuals as leaf children
        const inds = individualsByClass.get(id) ?? [];
        for (const iid of inds) {
          if (!visited.has(iid) && allNodes.has(iid)) {
            visited.add(iid);
            const idata = allNodes.get(iid)!;
            children.push({ id: iid, label: idata.label, uri: idata.uri, nodeType: 'Individual', children: [], connectionCount: idata.connections, propertyCount: 0 });
          }
        }
        children.sort((a, b) => a.label.localeCompare(b.label));
        return { id, label: data.label, uri: data.uri, nodeType: data.nodeType, children, connectionCount: data.connections, propertyCount: data.propertyCount };
      }

      function buildPropNode(id: string): OntologyNode {
        visited.add(id);
        const data = allNodes.get(id)!;
        const childIds = propParentToChildren.get(id) ?? new Set();
        const children: OntologyNode[] = [];
        for (const cid of childIds) {
          if (!visited.has(cid) && allNodes.has(cid)) {
            children.push(buildPropNode(cid));
          }
        }
        children.sort((a, b) => a.label.localeCompare(b.label));
        return { id, label: data.label, uri: data.uri, nodeType: data.nodeType, children, connectionCount: data.connections, propertyCount: 0 };
      }

      // Class roots
      const classRoots: string[] = [];
      for (const [id, data] of allNodes) {
        if (data.nodeType === 'Class' && !hasParent.has(id)) classRoots.push(id);
      }
      classRoots.sort((a, b) => (allNodes.get(a)!.label).localeCompare(allNodes.get(b)!.label));
      const classBranches = classRoots.map(id => buildClassNode(id));

      // Orphan classes
      for (const [id, data] of allNodes) {
        if (data.nodeType === 'Class' && !visited.has(id)) {
          classBranches.push({ id, label: data.label, uri: data.uri, nodeType: 'Class', children: [], connectionCount: data.connections, propertyCount: data.propertyCount });
          visited.add(id);
        }
      }

      // Property roots
      const propRoots: string[] = [];
      for (const [id, data] of allNodes) {
        if ((data.nodeType === 'ObjectProperty' || data.nodeType === 'DatatypeProperty') && !propHasParent.has(id) && !visited.has(id)) {
          propRoots.push(id);
        }
      }
      propRoots.sort((a, b) => (allNodes.get(a)!.label).localeCompare(allNodes.get(b)!.label));
      const propBranches = propRoots.map(id => buildPropNode(id));

      // Orphan properties
      for (const [id, data] of allNodes) {
        if ((data.nodeType === 'ObjectProperty' || data.nodeType === 'DatatypeProperty') && !visited.has(id)) {
          propBranches.push({ id, label: data.label, uri: data.uri, nodeType: data.nodeType, children: [], connectionCount: data.connections, propertyCount: 0 });
          visited.add(id);
        }
      }

      // Annotation properties
      const annotBranches: OntologyNode[] = [];
      for (const [id, data] of allNodes) {
        if (data.nodeType === 'AnnotationProperty' && !visited.has(id)) {
          visited.add(id);
          annotBranches.push({ id, label: data.label, uri: data.uri, nodeType: 'AnnotationProperty', children: [], connectionCount: 0, propertyCount: 0 });
        }
      }
      annotBranches.sort((a, b) => a.label.localeCompare(b.label));

      // Orphan individuals
      const orphanInds: OntologyNode[] = [];
      for (const [id, data] of allNodes) {
        if (data.nodeType === 'Individual' && !visited.has(id)) {
          visited.add(id);
          orphanInds.push({ id, label: data.label, uri: data.uri, nodeType: 'Individual', children: [], connectionCount: 0, propertyCount: 0 });
        }
      }

      // ── Assemble root ──────────────────────────────────────────
      const rootChildren: OntologyNode[] = [];

      if (classBranches.length > 0) {
        rootChildren.push({
          id: '__classes__', label: `Classes (${classBranches.length})`, uri: '',
          nodeType: 'Class', children: classBranches, connectionCount: 0, propertyCount: 0,
        });
      }
      if (propBranches.length > 0) {
        rootChildren.push({
          id: '__properties__', label: `Properties (${propBranches.length})`, uri: '',
          nodeType: 'ObjectProperty', children: propBranches, connectionCount: 0, propertyCount: 0,
        });
      }
      if (annotBranches.length > 0) {
        rootChildren.push({
          id: '__annotations__', label: `Annotations (${annotBranches.length})`, uri: '',
          nodeType: 'AnnotationProperty', children: annotBranches, connectionCount: 0, propertyCount: 0,
        });
      }
      if (orphanInds.length > 0) {
        rootChildren.push({
          id: '__individuals__', label: `Individuals (${orphanInds.length})`, uri: '',
          nodeType: 'Individual', children: orphanInds, connectionCount: 0, propertyCount: 0,
        });
      }

      const root: OntologyNode = {
        id: '__ontology__', label: 'Ontology', uri: '',
        nodeType: 'Ontology', children: rootChildren, connectionCount: 0, propertyCount: 0,
      };

      // Compute stats
      let md = 0;
      function findMaxDepth(node: OntologyNode, d: number) {
        if (d > md) md = d;
        node.children.forEach(c => findMaxDepth(c, d + 1));
      }
      findMaxDepth(root, 0);
      countDescendants(root);

      // Count by type
      const tc = new Map<NodeType, number>();
      for (const data of allNodes.values()) {
        tc.set(data.nodeType, (tc.get(data.nodeType) ?? 0) + 1);
      }
      setTypeCounts(tc);

      const totalClasses = tc.get('Class') ?? 0;
      const totalProps = (tc.get('ObjectProperty') ?? 0) + (tc.get('DatatypeProperty') ?? 0) + (tc.get('AnnotationProperty') ?? 0);
      const totalInds = tc.get('Individual') ?? 0;

      setStats({ classes: totalClasses, properties: totalProps, individuals: totalInds, depth: md, crossLinks: crossLinks.length });
      treeDataRef.current = root;
      crossLinksRef.current = crossLinks;
      collapsedRef.current = new Set();
      renderTree();
      setTimeout(fitGraph, 300);
      refreshStats();
    } catch (e) {
      console.error('Failed to load tree:', e);
    }
  }, [renderTree, fitGraph, refreshStats]);

  const toggleTypeFilter = useCallback((type: NodeType) => {
    const next = new Set(hiddenTypesRef.current);
    if (next.has(type)) next.delete(type);
    else next.add(type);
    hiddenTypesRef.current = next;
    setHiddenTypes(new Set(next));
    renderTree();
  }, [renderTree]);

  // Initialize SVG
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();

    const svg = d3.select(el)
      .append('svg')
      .attr('width', rect.width)
      .attr('height', rect.height)
      .style('background', '#1e1e2e')
      .style('display', 'block');

    const g = svg.append('g');

    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.02, 8])
      .on('zoom', (event) => g.attr('transform', event.transform.toString()));
    svg.call(zoom);
    svg.on('click', () => onNodeSelect(null));

    svgRef.current = svg.node();
    gRef.current = g;
    zoomRef.current = zoom;

    const ro = new ResizeObserver(() => {
      const r = el.getBoundingClientRect();
      svg.attr('width', r.width).attr('height', r.height);
    });
    ro.observe(el);

    (window as unknown as Record<string, unknown>).__refreshGraph = loadTree;

    return () => {
      ro.disconnect(); svg.remove();
      svgRef.current = null; gRef.current = null; zoomRef.current = null;
      delete (window as unknown as Record<string, unknown>).__refreshGraph;
    };
  }, [onNodeSelect, loadTree]);

  // Keyboard
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
      if (e.key === 'Escape') onNodeSelect(null);
      else if (e.key === 'f' || e.key === 'F') fitGraph();
      else if (e.key === 'r' || e.key === 'R') {
        if (svgRef.current && zoomRef.current) {
          d3.select(svgRef.current).transition().duration(500).call(zoomRef.current.transform, d3.zoomIdentity);
        }
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [onNodeSelect, fitGraph]);

  useEffect(() => { if (status === 'connected') loadTree(); }, [status, loadTree]);

  const typeOrder: NodeType[] = ['Class', 'ObjectProperty', 'DatatypeProperty', 'AnnotationProperty', 'Individual'];

  return (
    <div className="absolute inset-0" style={{ background: '#1e1e2e' }}>
      <div ref={containerRef} className="w-full h-full" />

      {/* Stats bar */}
      <div className="absolute top-3 left-3 flex items-center gap-2" style={{ zIndex: 10 }}>
        {[
          { label: 'classes', value: stats.classes, color: NODE_TYPE_COLORS.Class },
          { label: 'properties', value: stats.properties, color: NODE_TYPE_COLORS.ObjectProperty },
          { label: 'individuals', value: stats.individuals, color: NODE_TYPE_COLORS.Individual },
          { label: 'cross-links', value: stats.crossLinks, color: '#585b70' },
          { label: 'depth', value: stats.depth, color: '#cdd6f4' },
        ].map(s => (
          <span key={s.label} style={{
            background: '#181825', border: '1px solid #313244', borderRadius: 6,
            padding: '3px 10px', fontSize: 10, color: '#6c7086',
            display: 'flex', alignItems: 'center', gap: 4,
          }}>
            <span style={{ color: s.color, fontWeight: 600 }}>{s.value}</span> {s.label}
          </span>
        ))}
      </div>

      {/* Node type legend (clickable filters) */}
      <div className="absolute top-12 left-3 flex flex-col gap-1" style={{ zIndex: 10 }}>
        {typeOrder.filter(t => (typeCounts.get(t) ?? 0) > 0).map(type => (
          <button
            key={type}
            onClick={() => toggleTypeFilter(type)}
            style={{
              background: '#181825',
              border: '1px solid #313244',
              borderRadius: 6,
              padding: '4px 10px',
              fontSize: 11,
              color: hiddenTypes.has(type) ? '#45475a' : '#bac2de',
              display: 'flex', alignItems: 'center', gap: 8,
              cursor: 'pointer',
              textDecoration: hiddenTypes.has(type) ? 'line-through' : 'none',
              opacity: hiddenTypes.has(type) ? 0.5 : 1,
              textAlign: 'left',
              fontFamily: '-apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif',
            }}
          >
            <span style={{ width: 8, height: 8, borderRadius: '50%', background: getColor(type), display: 'inline-block', flexShrink: 0 }} />
            <span style={{ fontWeight: 600 }}>{type}</span>
            <span style={{ color: '#585b70', fontSize: 10 }}>({typeCounts.get(type) ?? 0})</span>
          </button>
        ))}
      </div>

      {/* Shortcuts */}
      <div className="absolute bottom-3 right-3" style={{ zIndex: 10 }}>
        <span style={{ background: '#181825', border: '1px solid #313244', borderRadius: 6, padding: '4px 10px', fontSize: 10, color: '#45475a' }}>
          Scroll zoom · Drag pan · Click inspect · Shift+click collapse · <b>F</b> fit · <b>R</b> reset · <b>Esc</b> deselect
        </span>
      </div>

      {status !== 'connected' && (
        <div className="absolute inset-0 flex items-center justify-center" style={{ color: '#585b70', zIndex: 5 }}>
          {status === 'connecting' ? 'Connecting to engine...' : 'Engine not connected'}
        </div>
      )}
    </div>
  );
}
