import { useState, useEffect, useRef } from 'react';
import * as mcp from '../lib/mcp-client';

interface Props {
  node: { id: string; label: string; uri: string } | null;
  onGraphChanged: () => void;
}

interface NodeProperty {
  predicate: string;
  value: string;
  valueType: 'uri' | 'literal';
  predicateLabel: string;
}

const COMMON_PREDICATES = [
  { uri: 'http://www.w3.org/2000/01/rdf-schema#label', label: 'rdfs:label' },
  { uri: 'http://www.w3.org/2000/01/rdf-schema#comment', label: 'rdfs:comment' },
  { uri: 'http://www.w3.org/2000/01/rdf-schema#subClassOf', label: 'rdfs:subClassOf' },
  { uri: 'http://www.w3.org/2002/07/owl#equivalentClass', label: 'owl:equivalentClass' },
  { uri: 'http://www.w3.org/2004/02/skos/core#definition', label: 'skos:definition' },
  { uri: 'http://www.w3.org/2004/02/skos/core#example', label: 'skos:example' },
  { uri: 'http://www.w3.org/2002/07/owl#disjointWith', label: 'owl:disjointWith' },
];

export function PropertyInspector({ node, onGraphChanged }: Props) {
  const [properties, setProperties] = useState<NodeProperty[]>([]);
  const [validationStatus, setValidationStatus] = useState<'valid' | 'invalid' | 'checking' | null>(null);
  const [editingIdx, setEditingIdx] = useState<number | null>(null);
  const [editValue, setEditValue] = useState('');
  const [adding, setAdding] = useState(false);
  const [newPred, setNewPred] = useState('');
  const [newVal, setNewVal] = useState('');
  const [newValType, setNewValType] = useState<'uri' | 'literal'>('literal');
  const [saving, setSaving] = useState(false);
  const editRef = useRef<HTMLInputElement>(null);
  const newPredRef = useRef<HTMLInputElement>(null);

  async function loadProperties() {
    if (!node) { setProperties([]); return; }
    const text = await mcp.sparqlQuery(
      `SELECT ?p ?o WHERE { <${node.uri}> ?p ?o . FILTER(!isBlank(?o)) }`
    );
    try {
      const data = JSON.parse(text);
      const bindings = data.results?.bindings || data?.results || [];
      // Handle both engine-native format and standard SPARQL JSON
      let rows: NodeProperty[] = [];
      if (Array.isArray(bindings)) {
        rows = bindings.map((b: Record<string, { value: string; type?: string } | string>) => {
          let pVal: string, oVal: string, oType: 'uri' | 'literal';
          if (typeof b.p === 'string') {
            // Engine-native: { p: "<uri>", o: "\"lit\"" | "<uri>" }
            const ps = b.p as string;
            const os = b.o as string;
            pVal = ps.startsWith('<') ? ps.slice(1, -1) : ps;
            if ((os as string).startsWith('<')) {
              oVal = (os as string).slice(1, -1);
              oType = 'uri';
            } else {
              oVal = (os as string).replace(/^"(.*)"(@\w+)?(\^\^.*)?$/, '$1');
              oType = 'literal';
            }
          } else {
            // Standard SPARQL JSON
            const pb = b.p as { value: string };
            const ob = b.o as { value: string; type?: string };
            pVal = pb?.value || '';
            oVal = ob?.value || '';
            oType = ob?.type === 'uri' ? 'uri' : 'literal';
          }
          return { predicate: pVal, value: oVal, valueType: oType, predicateLabel: shortUri(pVal) };
        });
      }
      // Also parse engine-native flat results array
      if (!Array.isArray(bindings) && Array.isArray(data?.results)) {
        rows = (data.results as Record<string, string>[]).map(row => {
          const ps = row.p || '';
          const os = row.o || '';
          const pVal = ps.startsWith('<') ? ps.slice(1, -1) : ps;
          let oVal: string, oType: 'uri' | 'literal';
          if (os.startsWith('<')) {
            oVal = os.slice(1, -1);
            oType = 'uri';
          } else {
            oVal = os.replace(/^"(.*)"(@\w+)?(\^\^.*)?$/, '$1');
            oType = 'literal';
          }
          return { predicate: pVal, value: oVal, valueType: oType, predicateLabel: shortUri(pVal) };
        });
      }
      setProperties(rows);
    } catch { setProperties([]); }
  }

  useEffect(() => {
    if (!node) { setProperties([]); setValidationStatus(null); return; }
    setEditingIdx(null);
    setAdding(false);
    loadProperties();
    setValidationStatus('checking');
    mcp.callTool('onto_lint', { input: '', inline: false }).then(result => {
      try {
        const issues = JSON.parse(result);
        const nodeIssues = Array.isArray(issues)
          ? issues.filter((i: { entity?: string }) => i.entity === node?.uri)
          : [];
        setValidationStatus(nodeIssues.length === 0 ? 'valid' : 'invalid');
      } catch { setValidationStatus('valid'); }
    });
  }, [node?.uri]);

  useEffect(() => {
    if (editingIdx !== null && editRef.current) {
      editRef.current.focus();
      editRef.current.select();
    }
  }, [editingIdx]);

  useEffect(() => {
    if (adding && newPredRef.current) {
      newPredRef.current.focus();
    }
  }, [adding]);

  async function saveEdit(idx: number) {
    if (!node) return;
    const prop = properties[idx];
    if (editValue === prop.value) { setEditingIdx(null); return; }
    setSaving(true);
    try {
      const oldPart = prop.valueType === 'uri' ? `<${prop.value}>` : `"${prop.value.replace(/"/g, '\\"')}"`;
      const newPart = editValue.startsWith('http') ? `<${editValue}>` : `"${editValue.replace(/"/g, '\\"')}"`;
      await mcp.sparqlUpdate(
        `DELETE { <${node.uri}> <${prop.predicate}> ${oldPart} } INSERT { <${node.uri}> <${prop.predicate}> ${newPart} } WHERE {}`
      );
      await mcp.saveGraphToFile();
      await loadProperties();
      onGraphChanged();
    } catch (e) { console.error(e); }
    setSaving(false);
    setEditingIdx(null);
  }

  async function deleteProp(idx: number) {
    if (!node) return;
    const prop = properties[idx];
    setSaving(true);
    try {
      const valPart = prop.valueType === 'uri' ? `<${prop.value}>` : `"${prop.value.replace(/"/g, '\\"')}"`;
      await mcp.sparqlUpdate(`DELETE WHERE { <${node.uri}> <${prop.predicate}> ${valPart} }`);
      await mcp.saveGraphToFile();
      await loadProperties();
      onGraphChanged();
    } catch (e) { console.error(e); }
    setSaving(false);
  }

  async function addProperty() {
    if (!node || !newPred.trim() || !newVal.trim()) return;
    const predUri = newPred.includes(':') && !newPred.startsWith('http')
      ? expandPrefix(newPred)
      : newPred.startsWith('<') ? newPred.slice(1, -1) : newPred;
    const valPart = newValType === 'uri'
      ? `<${newVal.startsWith('<') ? newVal.slice(1, -1) : newVal}>`
      : `"${newVal.replace(/"/g, '\\"')}"`;
    setSaving(true);
    try {
      await mcp.sparqlUpdate(`INSERT DATA { <${node.uri}> <${predUri}> ${valPart} }`);
      await mcp.saveGraphToFile();
      await loadProperties();
      onGraphChanged();
    } catch (e) { console.error(e); }
    setSaving(false);
    setAdding(false);
    setNewPred('');
    setNewVal('');
    setNewValType('literal');
  }

  if (!node) {
    return (
      <div className="p-3 text-sm" style={{ color: 'var(--text-secondary)' }}>
        Select a node to inspect
      </div>
    );
  }

  return (
    <div className="p-3 space-y-3 overflow-y-auto h-full text-xs">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h3 className="font-medium" style={{ color: 'var(--text-primary)' }}>
          {node.label}
        </h3>
        <span className="px-2 py-0.5 rounded text-xs"
          style={{
            background: validationStatus === 'valid' ? 'var(--success)' :
                        validationStatus === 'invalid' ? 'var(--error)' : 'var(--bg-panel)',
            color: 'var(--bg-primary)',
          }}>
          {validationStatus === 'checking' ? '...' :
           validationStatus === 'valid' ? 'Valid' :
           validationStatus === 'invalid' ? 'Issues' : ''}
        </span>
      </div>

      {/* URI (read-only) */}
      <div>
        <div className="mb-1" style={{ color: 'var(--text-secondary)' }}>URI</div>
        <div className="font-mono px-2 py-1 rounded break-all"
             style={{ background: 'var(--bg-primary)', color: 'var(--text-secondary)' }}>
          {node.uri}
        </div>
      </div>

      {/* Properties table */}
      <div>
        <div className="flex items-center justify-between mb-1">
          <span style={{ color: 'var(--text-secondary)' }}>Properties</span>
          <button
            onClick={() => { setAdding(true); setEditingIdx(null); }}
            className="px-2 py-0.5 rounded text-xs font-medium"
            style={{ background: 'var(--accent)', color: 'var(--bg-primary)' }}
            title="Add property"
          >+ Add</button>
        </div>

        <div className="space-y-0.5">
          {properties.map((prop, i) => (
            <div key={i} className="flex items-start gap-1 group px-2 py-1 rounded"
                 style={{ background: 'var(--bg-primary)' }}>
              <span className="shrink-0 w-24 truncate" style={{ color: 'var(--accent)' }}
                    title={prop.predicate}>
                {prop.predicateLabel}
              </span>
              <span style={{ color: 'var(--text-secondary)' }} className="shrink-0">=</span>

              {editingIdx === i ? (
                <input
                  ref={editRef}
                  className="flex-1 min-w-0 px-1 rounded outline-none text-xs"
                  style={{ background: 'var(--bg-panel)', color: 'var(--text-primary)', border: '1px solid var(--accent)' }}
                  value={editValue}
                  onChange={e => setEditValue(e.target.value)}
                  onKeyDown={e => {
                    if (e.key === 'Enter') saveEdit(i);
                    if (e.key === 'Escape') setEditingIdx(null);
                  }}
                  onBlur={() => saveEdit(i)}
                  disabled={saving}
                />
              ) : (
                <span
                  className="flex-1 min-w-0 truncate cursor-pointer hover:underline"
                  style={{ color: 'var(--text-primary)' }}
                  title={prop.value}
                  onClick={() => { setEditingIdx(i); setEditValue(prop.value); setAdding(false); }}
                >
                  {prop.valueType === 'uri' ? shortUri(prop.value) : prop.value}
                </span>
              )}

              <button
                className="shrink-0 opacity-0 group-hover:opacity-100 ml-1"
                style={{ color: 'var(--error)' }}
                title="Delete"
                onClick={() => deleteProp(i)}
                disabled={saving}
              >×</button>
            </div>
          ))}

          {properties.length === 0 && (
            <div style={{ color: 'var(--text-secondary)' }} className="px-2 py-1">No properties</div>
          )}
        </div>
      </div>

      {/* Add property form */}
      {adding && (
        <div className="rounded p-2 space-y-2" style={{ background: 'var(--bg-primary)', border: '1px solid var(--border)' }}>
          <div className="font-medium" style={{ color: 'var(--text-secondary)' }}>New property</div>

          {/* Predicate input with quick-pick */}
          <div>
            <div style={{ color: 'var(--text-secondary)' }} className="mb-1">Predicate</div>
            <input
              ref={newPredRef}
              className="w-full px-2 py-1 rounded outline-none text-xs"
              style={{ background: 'var(--bg-panel)', color: 'var(--text-primary)', border: '1px solid var(--border)' }}
              placeholder="rdfs:label or full URI..."
              value={newPred}
              onChange={e => setNewPred(e.target.value)}
              onKeyDown={e => { if (e.key === 'Escape') setAdding(false); }}
            />
            <div className="flex flex-wrap gap-1 mt-1">
              {COMMON_PREDICATES.map(p => (
                <button
                  key={p.uri}
                  className="px-1.5 py-0.5 rounded text-xs"
                  style={{
                    background: newPred === p.uri ? 'var(--accent)' : 'var(--bg-panel)',
                    color: newPred === p.uri ? 'var(--bg-primary)' : 'var(--text-secondary)',
                    border: '1px solid var(--border)',
                  }}
                  onClick={() => setNewPred(p.uri)}
                >{p.label}</button>
              ))}
            </div>
          </div>

          {/* Value input */}
          <div>
            <div className="flex items-center gap-2 mb-1">
              <span style={{ color: 'var(--text-secondary)' }}>Value</span>
              <button
                className="px-1.5 py-0.5 rounded text-xs"
                style={{
                  background: newValType === 'literal' ? 'var(--accent)' : 'var(--bg-panel)',
                  color: newValType === 'literal' ? 'var(--bg-primary)' : 'var(--text-secondary)',
                  border: '1px solid var(--border)',
                }}
                onClick={() => setNewValType('literal')}
              >Literal</button>
              <button
                className="px-1.5 py-0.5 rounded text-xs"
                style={{
                  background: newValType === 'uri' ? 'var(--accent)' : 'var(--bg-panel)',
                  color: newValType === 'uri' ? 'var(--bg-primary)' : 'var(--text-secondary)',
                  border: '1px solid var(--border)',
                }}
                onClick={() => setNewValType('uri')}
              >URI</button>
            </div>
            <input
              className="w-full px-2 py-1 rounded outline-none text-xs"
              style={{ background: 'var(--bg-panel)', color: 'var(--text-primary)', border: '1px solid var(--border)' }}
              placeholder={newValType === 'uri' ? 'http://...' : 'value...'}
              value={newVal}
              onChange={e => setNewVal(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') addProperty();
                if (e.key === 'Escape') setAdding(false);
              }}
            />
          </div>

          <div className="flex gap-2">
            <button
              className="flex-1 py-1 rounded text-xs font-medium"
              style={{ background: 'var(--accent)', color: 'var(--bg-primary)' }}
              onClick={addProperty}
              disabled={saving || !newPred.trim() || !newVal.trim()}
            >{saving ? '…' : 'Add'}</button>
            <button
              className="flex-1 py-1 rounded text-xs"
              style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)', border: '1px solid var(--border)' }}
              onClick={() => { setAdding(false); setNewPred(''); setNewVal(''); }}
            >Cancel</button>
          </div>
        </div>
      )}
    </div>
  );
}

function shortUri(uri: string): string {
  const hash = uri.lastIndexOf('#');
  if (hash >= 0) return uri.slice(hash + 1);
  const slash = uri.lastIndexOf('/');
  if (slash >= 0) return uri.slice(slash + 1);
  return uri;
}

function expandPrefix(prefixed: string): string {
  const prefixes: Record<string, string> = {
    'rdfs:': 'http://www.w3.org/2000/01/rdf-schema#',
    'owl:':  'http://www.w3.org/2002/07/owl#',
    'rdf:':  'http://www.w3.org/1999/02/22-rdf-syntax-ns#',
    'skos:': 'http://www.w3.org/2004/02/skos/core#',
    'xsd:':  'http://www.w3.org/2001/XMLSchema#',
    'dcterms:': 'http://purl.org/dc/terms/',
  };
  for (const [prefix, ns] of Object.entries(prefixes)) {
    if (prefixed.startsWith(prefix)) return ns + prefixed.slice(prefix.length);
  }
  return prefixed;
}
