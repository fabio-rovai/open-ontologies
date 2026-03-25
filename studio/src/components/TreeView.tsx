import { useRef, useEffect, useCallback } from 'react';
import cytoscape from 'cytoscape';
// @ts-expect-error dagre layout has no types
import dagre from 'cytoscape-dagre';
import * as mcp from '../lib/mcp-client';
import { useEngine } from '../hooks/useEngine';

cytoscape.use(dagre);

interface SparqlBinding {
  [key: string]: { type: string; value: string };
}

interface TreeViewProps {
  onNodeSelect: (node: { id: string; label: string; uri: string } | null) => void;
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

// Color palette for depth levels — matches brain-in-the-fish style
const DEPTH_COLORS = [
  '#f38ba8', // pink — root
  '#fab387', // peach — level 1
  '#f9e2af', // yellow — level 2
  '#a6e3a1', // green — level 3
  '#89b4fa', // blue — level 4
  '#cba6f7', // mauve — level 5
  '#94e2d5', // teal — level 6+
];

function colorForDepth(depth: number): string {
  return DEPTH_COLORS[Math.min(depth, DEPTH_COLORS.length - 1)];
}

const CLASSES_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?c ?label WHERE {
  { ?c a owl:Class } UNION { ?c a rdfs:Class }
  OPTIONAL { ?c rdfs:label ?label }
  FILTER(!isBlank(?c))
}`;

const EDGES_QUERY = `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?sub ?parent WHERE {
  ?sub rdfs:subClassOf ?parent .
  { ?sub a owl:Class } UNION { ?sub a rdfs:Class }
  FILTER(!isBlank(?sub) && !isBlank(?parent))
}`;

export function TreeView({ onNodeSelect }: TreeViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<cytoscape.Core | null>(null);
  const { status, refreshStats } = useEngine();

  const loadTree = useCallback(async () => {
    const cy = cyRef.current;
    if (!cy) return;

    try {
      const [classesText, edgesText] = await Promise.all([
        mcp.sparqlQuery(CLASSES_QUERY),
        mcp.sparqlQuery(EDGES_QUERY),
      ]);

      const classBindings = parseSparqlResults(classesText);
      const edgeBindings = parseSparqlResults(edgesText);

      const nodeIds = new Set<string>();
      const nodeData = new Map<string, { label: string; uri: string }>();
      const childToParents = new Map<string, Set<string>>();
      const hasParent = new Set<string>();

      for (const b of classBindings) {
        const uri = b.c?.value;
        if (!uri) continue;
        const id = shortUri(uri);
        if (!nodeIds.has(id)) {
          nodeIds.add(id);
          nodeData.set(id, { label: b.label?.value || id, uri });
        }
      }

      for (const b of edgeBindings) {
        const subUri = b.sub?.value;
        const parentUri = b.parent?.value;
        if (!subUri || !parentUri) continue;
        const sid = shortUri(subUri);
        const pid = shortUri(parentUri);

        if (!nodeIds.has(pid)) {
          nodeIds.add(pid);
          nodeData.set(pid, { label: pid, uri: parentUri });
        }

        if (!childToParents.has(sid)) childToParents.set(sid, new Set());
        childToParents.get(sid)!.add(pid);
        hasParent.add(sid);
      }

      // Find root nodes (no parent) and connect orphans to a virtual root
      const roots = new Set<string>();
      for (const id of nodeIds) {
        if (!hasParent.has(id)) roots.add(id);
      }

      // Compute depth for each node (BFS from roots)
      const depth = new Map<string, number>();
      const parentToChildren = new Map<string, Set<string>>();
      for (const [child, parents] of childToParents) {
        for (const parent of parents) {
          if (!parentToChildren.has(parent)) parentToChildren.set(parent, new Set());
          parentToChildren.get(parent)!.add(child);
        }
      }

      // BFS
      const queue: string[] = [];
      for (const r of roots) {
        depth.set(r, 0);
        queue.push(r);
      }
      while (queue.length > 0) {
        const node = queue.shift()!;
        const d = depth.get(node) ?? 0;
        const children = parentToChildren.get(node);
        if (children) {
          for (const child of children) {
            if (!depth.has(child)) {
              depth.set(child, d + 1);
              queue.push(child);
            }
          }
        }
      }

      // Build elements
      const elements: cytoscape.ElementDefinition[] = [];

      // Add virtual root if multiple roots exist
      if (roots.size > 1) {
        elements.push({
          data: { id: '__root__', label: 'Ontology', uri: '', depth: 0 },
        });
        for (const r of roots) {
          elements.push({
            data: { id: `__root__->${r}`, source: '__root__', target: r },
          });
          depth.set(r, 1);
        }
        // Recompute depths from virtual root
        const q2: string[] = [...roots];
        while (q2.length > 0) {
          const node = q2.shift()!;
          const d = depth.get(node) ?? 1;
          const children = parentToChildren.get(node);
          if (children) {
            for (const child of children) {
              depth.set(child, d + 1);
              q2.push(child);
            }
          }
        }
      }

      for (const id of nodeIds) {
        const data = nodeData.get(id);
        if (!data) continue;
        const d = depth.get(id) ?? 0;
        elements.push({
          data: { id, label: data.label, uri: data.uri, depth: d, color: colorForDepth(d) },
        });
      }

      for (const [child, parents] of childToParents) {
        for (const parent of parents) {
          if (nodeIds.has(child) && nodeIds.has(parent)) {
            elements.push({
              data: { id: `${parent}->${child}`, source: parent, target: child },
            });
          }
        }
      }

      cy.elements().remove();
      cy.add(elements);

      cy.layout({
        name: 'dagre',
        rankDir: 'LR',
        nodeSep: 8,
        rankSep: 120,
        edgeSep: 4,
        animate: false,
      } as cytoscape.LayoutOptions).run();

      cy.fit(undefined, 30);
      refreshStats();
    } catch (e) {
      console.error('Failed to load tree:', e);
    }
  }, [refreshStats]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const cy = cytoscape({
      container: el,
      style: [
        {
          selector: 'node',
          style: {
            'background-color': 'data(color)' as unknown as string,
            'label': 'data(label)',
            'color': '#bac2de',
            'font-size': '9px',
            'font-family': '-apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif',
            'text-valign': 'center',
            'text-halign': 'right',
            'text-margin-x': 8,
            'width': 10,
            'height': 10,
            'border-width': 0,
            'text-max-width': '200px',
            'text-wrap': 'ellipsis',
          },
        },
        {
          selector: 'node[id = "__root__"]',
          style: {
            'background-color': '#f38ba8',
            'width': 14,
            'height': 14,
            'font-size': '11px',
            'font-weight': 'bold' as unknown as number,
            'color': '#cdd6f4',
          },
        },
        {
          selector: 'node:selected',
          style: {
            'background-color': '#f9e2af',
            'border-color': '#f9e2af',
            'border-width': 2,
            'width': 14,
            'height': 14,
            'color': '#cdd6f4',
            'font-weight': 'bold' as unknown as number,
          },
        },
        {
          selector: 'edge',
          style: {
            'line-color': '#45475a',
            'target-arrow-color': '#45475a',
            'target-arrow-shape': 'triangle',
            'curve-style': 'unbundled-bezier',
            'control-point-distances': [20],
            'control-point-weights': [0.5],
            'width': 1,
            'arrow-scale': 0.6,
            'opacity': 0.5,
          },
        },
        {
          selector: 'edge[source = "__root__"]',
          style: {
            'line-color': '#585b70',
            'target-arrow-color': '#585b70',
            'width': 1.5,
            'opacity': 0.7,
          },
        },
      ],
      layout: { name: 'preset' },
      wheelSensitivity: 0.2,
      minZoom: 0.1,
      maxZoom: 4,
      pixelRatio: 2,
    });

    cy.on('tap', 'node', (evt) => {
      const node = evt.target;
      if (node.id() === '__root__') return;
      onNodeSelect({
        id: node.id(),
        label: node.data('label'),
        uri: node.data('uri'),
      });
    });

    cy.on('tap', (evt) => {
      if (evt.target === cy) {
        onNodeSelect(null);
      }
    });

    // Highlight path on hover
    cy.on('mouseover', 'node', (evt) => {
      const node = evt.target;
      const ancestors = node.predecessors();
      const descendants = node.successors();
      cy.elements().style('opacity', 0.15);
      node.style('opacity', 1);
      ancestors.style('opacity', 0.8);
      descendants.style('opacity', 0.8);
    });

    cy.on('mouseout', 'node', () => {
      cy.elements().removeStyle('opacity');
    });

    cyRef.current = cy;
    (window as unknown as Record<string, unknown>).__refreshGraph = loadTree;

    return () => {
      cy.destroy();
      cyRef.current = null;
      delete (window as unknown as Record<string, unknown>).__refreshGraph;
    };
  }, [onNodeSelect, loadTree]);

  useEffect(() => {
    if (status === 'connected') loadTree();
  }, [status, loadTree]);

  return (
    <div className="absolute inset-0">
      <div ref={containerRef} className="w-full h-full" style={{ background: '#1e1e2e' }} />
      <div className="absolute top-3 left-3 flex gap-2 text-xs" style={{ color: 'var(--text-secondary)' }}>
        {DEPTH_COLORS.map((color, i) => (
          <span key={i} className="flex items-center gap-1">
            <span className="inline-block w-2 h-2 rounded-full" style={{ background: color }} />
            {i === 0 ? 'root' : `L${i}`}
          </span>
        ))}
      </div>
      <div className="absolute bottom-4 right-4 text-xs" style={{ color: 'var(--text-secondary)' }}>
        Scroll to zoom · Drag to pan · Hover to trace path · Click to inspect
      </div>
    </div>
  );
}
