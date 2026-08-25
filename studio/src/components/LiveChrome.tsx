import { useState, useEffect, useRef } from 'react';
import { useEngine } from '../hooks/useEngine';
import { useDemoStore } from '../state/demo-store';
import { TreeView } from './TreeView';
import { Graph3D } from './Graph3D';
import { ChatPanel } from './ChatPanel';
import { PropertyInspector } from './PropertyInspector';
import { LineagePanel } from './LineagePanel';
import { GovernancePanel } from './GovernancePanel';
import * as mcp from '../lib/mcp-client';

/**
 * Desktop-only chrome: everything here needs a running engine and a live
 * agent sidecar, neither of which exists in the static web build. AppShell
 * lazy-loads this only when the source kind is "live", so none of it (or
 * the Tauri-coupled hooks it pulls in, useEngine and useChat via ChatPanel)
 * is ever evaluated in the replay bundle.
 *
 * The demonstration surfaces themselves (corpus, findings, resolution,
 * compare, the 3D graph) are NOT reimplemented here: this wraps them, it
 * does not duplicate them. AppShell renders that shared body and passes it
 * in as `children`; this component adds the ontology-authoring chrome
 * around it (chat, save/open, inspector, lineage, the 2D tree view) that
 * has no offline equivalent.
 */
export function LiveChrome({ children }: { children: React.ReactNode }) {
  const [showChat, setShowChat] = useState(true);
  const [graphMode, setGraphMode] = useState<'2d' | '3d'>('2d');
  const [showInspector, setShowInspector] = useState(false);
  const [showLineage, setShowLineage] = useState(false);
  const [showGovernance, setShowGovernance] = useState(false);
  const [selectedNode, setSelectedNode] = useState<{ id: string; label: string; uri: string } | null>(null);
  const [projectName, setProjectName] = useState('studio-live');
  const [savingAs, setSavingAs] = useState(false);
  const [saveAsName, setSaveAsName] = useState('');
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const saveInputRef = useRef<HTMLInputElement>(null);
  const { status, stats, connect } = useEngine();
  const graph = useDemoStore((s) => s.graph);
  const refreshGraph = useDemoStore((s) => s.refreshGraph);

  useEffect(() => {
    connect();
  }, [connect]);

  // Chat-driven ontology mutations (onto_load, onto_apply, ...) happen over
  // the agent sidecar, entirely outside the demo-store. Graph3D no longer
  // queries the engine itself, so it needs telling when something changed.
  // useChat.ts and PropertyInspector already call window.__refreshGraph
  // after a mutation; this keeps that same contract rather than inventing
  // a second one, and now points it at the store instead of a component's
  // own local reload.
  useEffect(() => {
    const w = window as unknown as { __refreshGraph?: () => void };
    const prev = w.__refreshGraph;
    w.__refreshGraph = () => { prev?.(); refreshGraph(); };
    const onLineageRefresh = () => refreshGraph();
    window.addEventListener('lineage-refresh', onLineageRefresh);
    return () => {
      window.removeEventListener('lineage-refresh', onLineageRefresh);
      w.__refreshGraph = prev;
    };
  }, [refreshGraph]);

  useEffect(() => {
    if (selectedNode) setShowInspector(true);
  }, [selectedNode]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'j') {
        e.preventDefault();
        setShowChat(c => !c);
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'i') {
        e.preventDefault();
        setShowInspector(i => !i);
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        openSaveAs();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectName]);

  function openSaveAs() {
    setSaveAsName(projectName);
    setSavingAs(true);
    setTimeout(() => saveInputRef.current?.select(), 50);
  }

  async function confirmSaveAs() {
    const name = saveAsName.trim().replace(/\.ttl$/i, '') || projectName;
    setSavingAs(false);
    const path = `~/.open-ontologies/${name}.ttl`;
    try {
      await mcp.saveGraphAs(path);
      setProjectName(name);
      setSaveMsg(`Saved as "${name}.ttl"`);
      setTimeout(() => setSaveMsg(null), 3000);
    } catch (e) {
      setSaveMsg(`Save failed: ${e instanceof Error ? e.message : String(e)}`);
      setTimeout(() => setSaveMsg(null), 4000);
    }
  }

  return (
    <div className="h-screen flex flex-col" style={{ background: 'var(--bg-primary)' }}>
      <div className="h-10 flex items-center px-4 border-b gap-3"
           style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
        <span className="text-sm font-semibold shrink-0" style={{ color: 'var(--accent)' }}>
          Open Ontologies
        </span>

        {savingAs ? (
          <div className="flex items-center gap-1">
            <input
              ref={saveInputRef}
              className="text-xs px-2 py-0.5 rounded outline-none w-44"
              style={{ background: 'var(--bg-panel)', color: 'var(--text-primary)', border: '1px solid var(--accent)' }}
              value={saveAsName}
              onChange={e => setSaveAsName(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') confirmSaveAs();
                if (e.key === 'Escape') setSavingAs(false);
              }}
              autoFocus
            />
            <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>.ttl</span>
            <button onClick={confirmSaveAs}
                    className="text-xs px-2 py-0.5 rounded"
                    style={{ background: 'var(--accent)', color: 'var(--bg-primary)' }}>Save</button>
            <button onClick={() => setSavingAs(false)}
                    className="text-xs px-2 py-0.5 rounded"
                    style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)' }}>&#10005;</button>
          </div>
        ) : (
          <button
            onClick={openSaveAs}
            className="text-xs px-2 py-0.5 rounded flex items-center gap-1.5"
            style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)', border: '1px solid var(--border)' }}
            title="Save as… (⌘S)"
          >
            <span style={{ color: 'var(--text-primary)' }}>{projectName}.ttl</span>
          </button>
        )}

        {saveMsg && (
          <span className="text-xs" style={{ color: 'var(--success)' }}>{saveMsg}</span>
        )}

        <div className="ml-auto flex gap-2">
          <div className="w-px mx-1" style={{ background: 'var(--border)' }} />
          <div className="flex items-center rounded text-xs overflow-hidden"
               style={{ border: '1px solid var(--border)' }}>
            {(['2d', '3d'] as const).map(m => (
              <button key={m} onClick={() => setGraphMode(m)}
                      className="px-2 py-1 uppercase"
                      style={{ background: graphMode === m ? 'var(--accent)' : 'var(--bg-panel)',
                               color: graphMode === m ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
                {m}
              </button>
            ))}
          </div>
          <button onClick={() => setShowChat(!showChat)}
                  className="text-xs px-2 py-1 rounded"
                  style={{ background: showChat ? 'var(--accent)' : 'var(--bg-panel)',
                           color: showChat ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
            Chat
          </button>
          <button onClick={() => setShowInspector(!showInspector)}
                  className="text-xs px-2 py-1 rounded"
                  style={{ background: showInspector ? 'var(--accent)' : 'var(--bg-panel)',
                           color: showInspector ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
            Inspector
          </button>
          <button onClick={() => setShowLineage(l => !l)}
                  className="text-xs px-2 py-1 rounded"
                  style={{ background: showLineage ? 'var(--accent)' : 'var(--bg-panel)',
                           color: showLineage ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
            Lineage
          </button>
          <button onClick={() => setShowGovernance(g => !g)}
                  className="text-xs px-2 py-1 rounded"
                  style={{ background: showGovernance ? 'var(--accent)' : 'var(--bg-panel)',
                           color: showGovernance ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
            Governance
          </button>
        </div>
      </div>

      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 relative">
          {graphMode === '2d'
            ? <TreeView onNodeSelect={setSelectedNode} />
            : <Graph3D graph={graph} onNodeSelect={setSelectedNode} />}
        </div>

        {showInspector && (
          <div className="w-72 border-l overflow-hidden"
               style={{ borderColor: 'var(--border)', background: 'var(--bg-panel)' }}>
            <PropertyInspector node={selectedNode} onGraphChanged={refreshGraph} />
          </div>
        )}

        {showLineage && (
          <div className="w-72 border-l flex flex-col overflow-hidden"
               style={{ borderColor: 'var(--border)', background: 'var(--bg-panel)' }}>
            <LineagePanel />
          </div>
        )}

        {showGovernance && (
          <div className="w-72 border-l overflow-hidden"
               style={{ borderColor: 'var(--border)', background: 'var(--bg-panel)' }}>
            <GovernancePanel />
          </div>
        )}

        {/* The shared demonstration body (corpus / findings / resolution /
            compare), the same tree the replay build renders. */}
        <div className="w-96 border-l overflow-hidden flex flex-col"
             style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
          {children}
        </div>

        {showChat && (
          <div className="w-96 border-l flex flex-col"
               style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
            <ChatPanel />
          </div>
        )}
      </div>

      <div className="h-6 flex items-center px-4 text-xs border-t gap-4"
           style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)',
                    color: 'var(--text-secondary)' }}>
        <span style={{ color: status === 'connected' ? 'var(--success)' :
                              status === 'error' ? 'var(--error)' : 'var(--text-secondary)' }}>
          {status === 'connected' ? 'Connected' :
           status === 'connecting' ? 'Connecting...' :
           status === 'error' ? 'Error' : 'Disconnected'}
        </span>
        {stats && (
          <span>{stats.triples} triples | {stats.classes} classes | {stats.properties} properties</span>
        )}
      </div>
    </div>
  );
}
