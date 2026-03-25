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

      const elements: cytoscape.ElementDefinition[] = [];
      const nodeIds = new Set<string>();

      for (const b of classBindings) {
        const uri = b.c?.value;
        if (!uri) continue;
        const id = shortUri(uri);
        if (!nodeIds.has(id)) {
          nodeIds.add(id);
          elements.push({
            data: { id, label: b.label?.value || id, uri },
          });
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
          elements.push({ data: { id: pid, label: pid, uri: parentUri } });
        }

        if (nodeIds.has(sid) && nodeIds.has(pid)) {
          elements.push({
            data: { id: `${sid}->${pid}`, source: pid, target: sid },
          });
        }
      }

      cy.elements().remove();
      cy.add(elements);

      cy.layout({
        name: 'dagre',
        rankDir: 'TB',
        nodeSep: 30,
        rankSep: 60,
        animate: false,
      } as cytoscape.LayoutOptions).run();

      cy.fit(undefined, 40);
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
            'background-color': '#89b4fa',
            'label': 'data(label)',
            'color': '#cdd6f4',
            'font-size': '10px',
            'text-valign': 'bottom',
            'text-margin-y': 5,
            'width': 20,
            'height': 20,
            'border-width': 1,
            'border-color': '#585b70',
          },
        },
        {
          selector: 'node:selected',
          style: {
            'background-color': '#f9e2af',
            'border-color': '#f9e2af',
            'border-width': 2,
          },
        },
        {
          selector: 'edge',
          style: {
            'line-color': '#585b70',
            'target-arrow-color': '#585b70',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            'width': 1.5,
            'arrow-scale': 0.8,
          },
        },
      ],
      layout: { name: 'preset' },
      wheelSensitivity: 0.3,
    });

    cy.on('tap', 'node', (evt) => {
      const node = evt.target;
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
      <div className="absolute bottom-4 right-4 text-xs" style={{ color: 'var(--text-secondary)' }}>
        Scroll to zoom · Drag to pan · Click node to inspect
      </div>
    </div>
  );
}
