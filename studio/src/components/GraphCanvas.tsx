import { useRef, useEffect, useState, useCallback } from 'react';
import ForceGraph3D from '3d-force-graph';
import * as THREE from 'three';
import * as mcp from '../lib/mcp-client';
import { useEngine } from '../hooks/useEngine';
import { AddClassDialog } from './AddClassDialog';

interface SelectedNode {
  id: string;
  label: string;
  uri: string;
}

interface GraphCanvasProps {
  onNodeSelect: (node: SelectedNode | null) => void;
  dagMode?: boolean;
}

interface SparqlBinding {
  [key: string]: { type: string; value: string };
}

interface GraphNode {
  id: string;
  label: string;
  uri: string;
  x?: number;
  y?: number;
  z?: number;
}

interface GraphLink {
  source: string;
  target: string;
  type?: 'subclass' | 'property';
  label?: string;
}

// --- Helpers ---

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

const ONTO_NS = 'http://example.org/ontology#';

const CLASSES_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?c ?label WHERE {
  ?c a owl:Class .
  OPTIONAL { ?c rdfs:label ?label }
  FILTER(!isBlank(?c))
}`;

const EDGES_QUERY = `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?sub ?parent WHERE {
  ?sub rdfs:subClassOf ?parent .
  ?sub a owl:Class .
  FILTER(!isBlank(?sub) && !isBlank(?parent))
}`;

const PROPERTY_EDGES_QUERY = `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?prop ?propLabel ?domain ?range WHERE {
  ?prop a owl:ObjectProperty .
  ?prop rdfs:domain ?domain .
  ?prop rdfs:range ?range .
  OPTIONAL { ?prop rdfs:label ?propLabel }
  FILTER(!isBlank(?domain) && !isBlank(?range))
}`;

// --- Component ---

export function GraphCanvas({ onNodeSelect, dagMode }: GraphCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const graphRef = useRef<any>(null);
  const { status, refreshStats } = useEngine();

  const [addDialog, setAddDialog] = useState<{ x: number; y: number } | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    setTimeout(() => setToast(null), 4000);
  }, []);

  // --- Load graph data ---
  const loadGraph = useCallback(async () => {
    const g = graphRef.current;
    if (!g) return;

    try {
      const [classesText, edgesText, propEdgesText] = await Promise.all([
        mcp.sparqlQuery(CLASSES_QUERY),
        mcp.sparqlQuery(EDGES_QUERY),
        mcp.sparqlQuery(PROPERTY_EDGES_QUERY),
      ]);

      const classBindings = parseSparqlResults(classesText);
      const edgeBindings = parseSparqlResults(edgesText);
      const propEdgeBindings = parseSparqlResults(propEdgesText);

      const nodeMap = new Map<string, GraphNode>();

      for (const b of classBindings) {
        const uri = b.c?.value;
        if (!uri) continue;
        const id = shortUri(uri);
        if (!nodeMap.has(id)) {
          nodeMap.set(id, { id, label: b.label?.value || id, uri });
        }
      }

      // Add parent nodes missing from classes query
      for (const b of edgeBindings) {
        const parentUri = b.parent?.value;
        if (!parentUri) continue;
        const pid = shortUri(parentUri);
        if (!nodeMap.has(pid)) {
          nodeMap.set(pid, { id: pid, label: pid, uri: parentUri });
        }
      }

      const edgeSet = new Set<string>();
      const links: GraphLink[] = [];
      for (const b of edgeBindings) {
        const subUri = b.sub?.value;
        const parentUri = b.parent?.value;
        if (!subUri || !parentUri) continue;
        const sid = shortUri(subUri);
        const pid = shortUri(parentUri);
        const eid = `${sid}→${pid}`;
        if (nodeMap.has(sid) && nodeMap.has(pid) && !edgeSet.has(eid)) {
          edgeSet.add(eid);
          links.push({ source: sid, target: pid, type: 'subclass' });
        }
      }

      // Object property edges (domain → range)
      for (const b of propEdgeBindings) {
        const domainUri = b.domain?.value;
        const rangeUri = b.range?.value;
        if (!domainUri || !rangeUri) continue;
        const did = shortUri(domainUri);
        const rid = shortUri(rangeUri);
        const propLabel = b.propLabel?.value || shortUri(b.prop?.value || '');
        const eid = `${did}→${rid}:${propLabel}`;
        if (nodeMap.has(did) && nodeMap.has(rid) && !edgeSet.has(eid)) {
          edgeSet.add(eid);
          links.push({ source: did, target: rid, type: 'property', label: propLabel });
        }
      }

      g.graphData({ nodes: Array.from(nodeMap.values()), links });
      refreshStats();
    } catch (e) {
      console.error('Failed to load graph:', e);
      showToast('Failed to load graph from engine');
    }
  }, [showToast, refreshStats]);

  // --- Initialize 3D graph ---
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const graph = ForceGraph3D()(el)
      .backgroundColor('#1e1e2e')
      .nodeLabel('label')
      .nodeColor((node: object) => {
        const n = node as GraphNode;
        return n.id === selectedId ? '#f9e2af' : '#89b4fa';
      })
      .nodeOpacity(0.95)
      .nodeResolution(16)
      .linkColor((link: object) => {
        const l = link as GraphLink;
        return l.type === 'property' ? '#f9e2af' : '#585b70';
      })
      .linkOpacity(0.6)
      .linkWidth((link: object) => {
        const l = link as GraphLink;
        return l.type === 'property' ? 2 : 1.5;
      })
      .linkDirectionalArrowLength(6)
      .linkDirectionalArrowRelPos(1)
      .linkDirectionalArrowColor((link: object) => {
        const l = link as GraphLink;
        return l.type === 'property' ? '#f9e2af' : '#89b4fa';
      })
      .linkLabel((link: object) => {
        const l = link as GraphLink;
        return l.type === 'property' ? (l.label || '') : '';
      })
      .nodeThreeObject((node: object) => {
        const n = node as GraphNode;
        const group = new THREE.Group();

        // Sphere
        const sphere = new THREE.Mesh(
          new THREE.SphereGeometry(5, 16, 16),
          new THREE.MeshLambertMaterial({ color: n.id === selectedId ? 0xf9e2af : 0x89b4fa })
        );
        group.add(sphere);

        // Label sprite
        const canvas = document.createElement('canvas');
        canvas.width = 256;
        canvas.height = 64;
        const ctx = canvas.getContext('2d')!;
        ctx.font = 'bold 24px sans-serif';
        ctx.fillStyle = '#cdd6f4';
        ctx.textAlign = 'center';
        ctx.fillText(n.label.slice(0, 20), 128, 40);
        const tex = new THREE.CanvasTexture(canvas);
        const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, depthWrite: false }));
        sprite.scale.set(40, 12, 1);
        sprite.position.set(0, 12, 0);
        group.add(sprite);

        return group;
      })
      .onNodeClick((node: object) => {
        const n = node as GraphNode;
        setSelectedId(n.id);
        onNodeSelect({ id: n.id, label: n.label, uri: n.uri });
        // Fly camera toward node
        const dist = 80;
        const distRatio = 1 + dist / Math.hypot(n.x ?? 0, n.y ?? 0, n.z ?? 0);
        graph.cameraPosition(
          { x: (n.x ?? 0) * distRatio, y: (n.y ?? 0) * distRatio, z: (n.z ?? 0) * distRatio },
          { x: n.x ?? 0, y: n.y ?? 0, z: n.z ?? 0 },
          800
        );
      })
      .onBackgroundClick(() => {
        setSelectedId(null);
        onNodeSelect(null);
      });

    // Spring-based force tuning
    graph.d3Force('link')?.distance(30).strength(0.7);
    graph.d3Force('charge')?.strength(-120);

    // DAG mode: top-down tree layout in 3D
    if (dagMode) {
      graph.dagMode('td');
      graph.dagLevelDistance(40);
    }

    // Warm lighting
    const scene = graph.scene() as THREE.Scene;
    scene.add(new THREE.AmbientLight(0xffffff, 0.6));
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
    dirLight.position.set(100, 200, 100);
    scene.add(dirLight);

    graphRef.current = graph;

    // Size to container
    const ro = new ResizeObserver(() => {
      graph.width(el.offsetWidth).height(el.offsetHeight);
    });
    ro.observe(el);
    graph.width(el.offsetWidth).height(el.offsetHeight);

    (window as unknown as Record<string, unknown>).__refreshGraph = loadGraph;

    return () => {
      ro.disconnect();
      graph._destructor?.();
      graphRef.current = null;
      delete (window as unknown as Record<string, unknown>).__refreshGraph;
    };
  }, [onNodeSelect, loadGraph]);

  // Refresh node colors when selection changes
  useEffect(() => {
    graphRef.current?.nodeThreeObjectExtend(false);
    graphRef.current?.refresh?.();
  }, [selectedId]);

  // Auto-load when connected
  useEffect(() => {
    if (status === 'connected') loadGraph();
  }, [status, loadGraph]);

  // Delete key handler
  useEffect(() => {
    async function handleKeyDown(e: KeyboardEvent) {
      if (e.key !== 'Delete' && e.key !== 'Backspace') return;
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
      if (!selectedId) return;

      // Find the selected node's URI from current graph data
      const data = graphRef.current?.graphData();
      const node = data?.nodes?.find((n: GraphNode) => n.id === selectedId) as GraphNode | undefined;
      if (!node?.uri) return;

      try {
        await mcp.sparqlUpdate(`DELETE WHERE { <${node.uri}> ?p ?o }`);
        await mcp.sparqlUpdate(`DELETE WHERE { ?s ?p <${node.uri}> }`);
        await mcp.saveGraphToFile();
        setSelectedId(null);
        onNodeSelect(null);
        await loadGraph();
        showToast(`Deleted: ${node.label}`);
      } catch (ex) {
        showToast(`Delete failed: ${ex instanceof Error ? ex.message : String(ex)}`);
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [selectedId, loadGraph, onNodeSelect, showToast]);

  // Right-click on canvas = add class
  function handleContextMenu(e: React.MouseEvent) {
    e.preventDefault();
    setAddDialog({ x: e.clientX, y: e.clientY });
  }

  async function handleAddClass(className: string) {
    setAddDialog(null);
    const localName = className.split(/\s+/).map(w => w.charAt(0).toUpperCase() + w.slice(1)).join('');
    const turtle = `@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n<${ONTO_NS}${localName}> a owl:Class ; rdfs:label "${className}" .`;
    try {
      const v = await mcp.validate(turtle);
      if (v.toLowerCase().includes('error')) { showToast(`Validation error: ${v}`); return; }
      await mcp.loadTurtle(turtle);
      await mcp.saveGraphToFile();
      await loadGraph();
      showToast(`Created: ${className}`);
    } catch (ex) {
      showToast(`Failed: ${ex instanceof Error ? ex.message : String(ex)}`);
    }
  }

  return (
    <div className="absolute inset-0" onContextMenu={handleContextMenu}>
      <div ref={containerRef} className="w-full h-full" />

      {addDialog && (
        <AddClassDialog
          position={addDialog}
          onSubmit={handleAddClass}
          onCancel={() => setAddDialog(null)}
        />
      )}

      {toast && (
        <div className="absolute bottom-4 left-1/2 -translate-x-1/2 px-4 py-2 rounded text-sm max-w-md text-center"
          style={{
            background: toast.toLowerCase().includes('error') || toast.toLowerCase().includes('fail')
              ? 'var(--error)' : 'var(--bg-panel)',
            color: toast.toLowerCase().includes('error') || toast.toLowerCase().includes('fail')
              ? '#1e1e2e' : 'var(--text-primary)',
            border: '1px solid var(--border)',
          }}>
          {toast}
        </div>
      )}

      {status !== 'connected' && (
        <div className="absolute inset-0 flex items-center justify-center"
          style={{ color: 'var(--text-secondary)' }}>
          {status === 'connecting' ? 'Connecting to engine...' : 'Engine not connected'}
        </div>
      )}

      <div className="absolute bottom-4 right-4 text-xs" style={{ color: 'var(--text-secondary)' }}>
        Drag to orbit · Scroll to zoom · Click node to inspect · Right-click to add
      </div>
    </div>
  );
}
