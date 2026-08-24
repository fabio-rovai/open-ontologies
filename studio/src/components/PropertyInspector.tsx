import { useState, useEffect, useRef, useMemo } from 'react';
import * as mcp from '../lib/mcp-client';

interface Props {
  node: { id: string; label: string; uri: string } | null;
  onGraphChanged: () => void;
}

interface NodeProperty {
  predicate: string;
  value: string;
  valueType: 'uri' | 'literal';
  language?: string;
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
  const [newLang, setNewLang] = useState('');
  const [saving, setSaving] = useState(false);
  const [langFilter, setLangFilter] = useState<string>('all');
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
      let rows: NodeProperty[] = [];
      if (Array.isArray(bindings)) {
        rows = bindings.map((b: Record<string, { value: string; type?: string; 'xml:lang'?: string } | string>) => {
          let pVal: string, oVal: string, oType: 'uri' | 'literal', lang: string | undefined;
          if (typeof b.p === 'string') {
            // Engine-native: { p: "<uri>", o: "\"lit\"@en" | "<uri>" }
            const ps = b.p as string;
            const os = b.o as string;
            pVal = ps.startsWith('<') ? ps.slice(1, -1) : ps;
            if (os.startsWith('<')) {
              oVal = os.slice(1, -1);
              oType = 'uri';
            } else {
              const m = os.match(/^"((?:[^"\\]|\\.)*)"(?:@([\w-]+))?(?:\^\^.*)?$/);
              oVal = m ? m[1].replace(/\\"/g, '"') : os;
              lang = m?.[2] || undefined;
              oType = 'literal';
            }
          } else {
            // Standard SPARQL JSON
            const pb = b.p as { value: string };
            const ob = b.o as { value: string; type?: string; 'xml:lang'?: string };
            pVal = pb?.value || '';
            oVal = ob?.value || '';
            lang = ob?.['xml:lang'] || undefined;
            oType = ob?.type === 'uri' ? 'uri' : 'literal';
          }
          return { predicate: pVal, value: oVal, valueType: oType, language: lang, predicateLabel: shortUri(pVal) };
        });
      }
      // Engine-native flat results array
      if (!Array.isArray(bindings) && Array.isArray(data?.results)) {
        rows = (data.results as Record<string, string>[]).map(row => {
          const ps = row.p || '';
          const os = row.o || '';
          const pVal = ps.startsWith('<') ? ps.slice(1, -1) : ps;
          let oVal: string, oType: 'uri' | 'literal', lang: string | undefined;
          if (os.startsWith('<')) {
            oVal = os.slice(1, -1);
            oType = 'uri';
          } else {
            const m = os.match(/^"((?:[^"\\]|\\.)*)"(?:@([\w-]+))?(?:\^\^.*)?$/);
            oVal = m ? m[1].replace(/\\"/g, '"') : os;
            lang = m?.[2] || undefined;
            oType = 'literal';
          }
          return { predicate: pVal, value: oVal, valueType: oType, language: lang, predicateLabel: shortUri(pVal) };
        });
      }
      setProperties(rows);
    } catch { setProperties([]); }
  }

  // Derive available languages from current properties
  const availableLangs = useMemo(() => {
    const langs = new Set<string>();
    for (const p of properties) {
      if (p.valueType === 'literal' && p.language) langs.add(p.language);
    }
    return Array.from(langs).sort();
  }, [properties]);

  // Reset filter when node changes or new langs appear
  useEffect(() => {
    if (langFilter !== 'all' && !availableLangs.includes(langFilter)) {
      setLangFilter('all');
    }
  }, [availableLangs, langFilter]);

  // `editingIdx` is a row index into `visibleProperties`, and the language filter
  // reshuffles that list underneath it. Left alone, an open edit would resolve to
  // whichever property had moved into that row and `saveEdit`/`deleteProp` would
  // run their DELETE/INSERT against the wrong triple, then persist it. The filter
  // can change without the user touching a chip: the effect above flips it back to
  // `all` when the last literal of the filtered language is deleted, which is
  // reachable from the row's own delete button while an edit input is open. It can
  // also change from a chip click that never blurs the input, as in Tauri's
  // WKWebView, where buttons do not take focus. Closing the edit is the only
  // response that cannot write to the wrong row.
  useEffect(() => {
    setEditingIdx(null);
    setAdding(false);
  }, [langFilter]);

  const visibleProperties = useMemo(() => {
    if (langFilter === 'all') return properties;
    return properties.filter(p =>
      p.valueType === 'uri' ||
      !p.language ||
      p.language === langFilter
    );
  }, [properties, langFilter]);

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
    if (adding && newPredRef.current) newPredRef.current.focus();
  }, [adding]);

  function literalPart(prop: NodeProperty): string {
    if (prop.valueType === 'uri') return `<${prop.value}>`;
    const escaped = prop.value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
    return prop.language ? `"${escaped}"@${prop.language}` : `"${escaped}"`;
  }

  async function saveEdit(idx: number) {
    if (!node) return;
    const prop = visibleProperties[idx];
    if (editValue === prop.value) { setEditingIdx(null); return; }
    setSaving(true);
    try {
      const oldPart = literalPart(prop);
      // The language tag rides along with the rest of the term. Built through `literalPart` rather than inlining the escapes again: the
      // DELETE and the INSERT below have to agree about escaping exactly, and two
      // copies of the rule are two things to keep in step.
      const newPart = literalPart(
        editValue.startsWith('http') && prop.valueType !== 'uri'
          ? { ...prop, valueType: 'uri', value: editValue }
          : { ...prop, value: editValue },
      );
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
    const prop = visibleProperties[idx];
    setSaving(true);
    try {
      const valPart = literalPart(prop);
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
    let valPart: string;
    if (newValType === 'uri') {
      valPart = `<${newVal.startsWith('<') ? newVal.slice(1, -1) : newVal}>`;
    } else {
      const escaped = newVal.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
      valPart = newLang.trim() ? `"${escaped}"@${newLang.trim()}` : `"${escaped}"`;
    }
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
    setNewLang('');
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
          {validationStatus === 'checking' ? '…' :
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

        {/* Language filter */}
        {availableLangs.length > 0 && (
          <div className="flex flex-wrap gap-1 mb-2">
            <button
              onClick={() => setLangFilter('all')}
              className="px-1.5 py-0.5 rounded text-xs font-mono"
              style={{
                background: langFilter === 'all' ? 'var(--accent)' : 'var(--bg-panel)',
                color: langFilter === 'all' ? 'var(--bg-primary)' : 'var(--text-secondary)',
                border: '1px solid var(--border)',
              }}
            >all</button>
            {availableLangs.map(lang => (
              <button
                key={lang}
                onClick={() => setLangFilter(lang)}
                className="px-1.5 py-0.5 rounded text-xs font-mono"
                style={{
                  background: langFilter === lang ? 'var(--accent)' : 'var(--bg-panel)',
                  color: langFilter === lang ? 'var(--bg-primary)' : 'var(--text-secondary)',
                  border: '1px solid var(--border)',
                }}
              >{lang}</button>
            ))}
          </div>
        )}

        <div className="space-y-0.5">
          {visibleProperties.map((prop, i) => (
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

              {/* Language tag badge */}
              {prop.valueType === 'literal' && prop.language && (
                <span
                  className="shrink-0 px-1 rounded font-mono"
                  style={{
                    background: 'var(--bg-panel)',
                    color: 'var(--accent)',
                    border: '1px solid var(--border)',
                    fontSize: '0.65rem',
                    lineHeight: '1.4',
                  }}
                  title={`Language: ${prop.language}`}
                >{prop.language}</span>
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

          {visibleProperties.length === 0 && (
            <div style={{ color: 'var(--text-secondary)' }} className="px-2 py-1">
              {properties.length > 0 ? `No properties for language "${langFilter}"` : 'No properties'}
            </div>
          )}
        </div>
      </div>

      {/* Add property form */}
      {adding && (
        <div className="rounded p-2 space-y-2" style={{ background: 'var(--bg-primary)', border: '1px solid var(--border)' }}>
          <div className="font-medium" style={{ color: 'var(--text-secondary)' }}>New property</div>

          <div>
            <div style={{ color: 'var(--text-secondary)' }} className="mb-1">Predicate</div>
            <input
              ref={newPredRef}
              className="w-full px-2 py-1 rounded outline-none text-xs"
              style={{ background: 'var(--bg-panel)', color: 'var(--text-primary)', border: '1px solid var(--border)' }}
              placeholder="rdfs:label or full URI…"
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
              placeholder={newValType === 'uri' ? 'http://…' : 'value…'}
              value={newVal}
              onChange={e => setNewVal(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') addProperty();
                if (e.key === 'Escape') setAdding(false);
              }}
            />
          </div>

          {/* Language tag input — only for literals */}
          {newValType === 'literal' && (
            <div>
              <div style={{ color: 'var(--text-secondary)' }} className="mb-1">Language tag <span style={{ color: 'var(--text-secondary)', opacity: 0.6 }}>(optional)</span></div>
              <div className="flex gap-1.5 items-center">
                <input
                  className="w-20 px-2 py-1 rounded outline-none text-xs font-mono"
                  style={{ background: 'var(--bg-panel)', color: 'var(--text-primary)', border: '1px solid var(--border)' }}
                  placeholder="en, fr, de…"
                  value={newLang}
                  onChange={e => setNewLang(e.target.value.toLowerCase())}
                  onKeyDown={e => {
                    if (e.key === 'Enter') addProperty();
                    if (e.key === 'Escape') setAdding(false);
                  }}
                />
                {availableLangs.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {availableLangs.map(lang => (
                      <button
                        key={lang}
                        onClick={() => setNewLang(lang)}
                        className="px-1.5 py-0.5 rounded text-xs font-mono"
                        style={{
                          background: newLang === lang ? 'var(--accent)' : 'var(--bg-panel)',
                          color: newLang === lang ? 'var(--bg-primary)' : 'var(--text-secondary)',
                          border: '1px solid var(--border)',
                        }}
                      >{lang}</button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

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
              onClick={() => { setAdding(false); setNewPred(''); setNewVal(''); setNewLang(''); }}
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
