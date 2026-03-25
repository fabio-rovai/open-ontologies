import { useState, useEffect, useRef } from 'react';
import { useEngine } from '../hooks/useEngine';
import { GraphCanvas } from './GraphCanvas';
import { TreeView } from './TreeView';
import { ChatPanel } from './ChatPanel';
import { PropertyInspector } from './PropertyInspector';
import { LineagePanel } from './LineagePanel';
import * as mcp from '../lib/mcp-client';

type ViewMode = '3d' | 'dag' | 'tree';

export function Layout() {
  const [showChat, setShowChat] = useState(true);
  const [showInspector, setShowInspector] = useState(false);
  const [showLineage, setShowLineage] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>('3d');
  const [selectedNode, setSelectedNode] = useState<{ id: string; label: string; uri: string } | null>(null);
  const [projectName, setProjectName] = useState('studio-live');
  const [savingAs, setSavingAs] = useState(false);
  const [saveAsName, setSaveAsName] = useState('');
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const saveInputRef = useRef<HTMLInputElement>(null);
  const { status, stats, connect } = useEngine();

  useEffect(() => {
    connect();
  }, [connect]);

  // Auto-show inspector when a node is selected
  useEffect(() => {
    if (selectedNode) setShowInspector(true);
  }, [selectedNode]);

  // Global keyboard shortcuts
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
      {/* Toolbar */}
      <div className="h-10 flex items-center px-4 border-b gap-3"
           style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
        <span className="text-sm font-semibold shrink-0" style={{ color: 'var(--accent)' }}>
          Open Ontologies
        </span>

        {/* Project name / save-as input */}
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
                    style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)' }}>✕</button>
          </div>
        ) : (
          <button
            onClick={openSaveAs}
            className="text-xs px-2 py-0.5 rounded flex items-center gap-1.5"
            style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)', border: '1px solid var(--border)' }}
            title="Save as… (⌘S)"
          >
            <span>💾</span>
            <span style={{ color: 'var(--text-primary)' }}>{projectName}.ttl</span>
          </button>
        )}

        {saveMsg && (
          <span className="text-xs" style={{ color: 'var(--success)' }}>{saveMsg}</span>
        )}

        <div className="ml-auto flex gap-2">
          {/* View mode toggle */}
          {(['3d', 'dag', 'tree'] as ViewMode[]).map(mode => (
            <button key={mode} onClick={() => setViewMode(mode)}
                    className="text-xs px-2 py-1 rounded"
                    style={{ background: viewMode === mode ? 'var(--accent)' : 'var(--bg-panel)',
                             color: viewMode === mode ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
              {mode === '3d' ? '3D' : mode === 'dag' ? 'DAG' : 'Tree'}
            </button>
          ))}
          <div className="w-px mx-1" style={{ background: 'var(--border)' }} />
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
        </div>
      </div>

      {/* Main area */}
      <div className="flex-1 flex overflow-hidden">
        {/* Graph canvas */}
        <div className="flex-1 relative">
          {viewMode === 'tree' ? (
            <TreeView onNodeSelect={setSelectedNode} />
          ) : (
            <GraphCanvas onNodeSelect={setSelectedNode} dagMode={viewMode === 'dag'} />
          )}
        </div>

        {/* Inspector panel */}
        {showInspector && (
          <div className="w-72 border-l overflow-hidden"
               style={{ borderColor: 'var(--border)', background: 'var(--bg-panel)' }}>
            <PropertyInspector
              node={selectedNode}
              onGraphChanged={() => {
                const refresh = (window as unknown as Record<string, () => void>).__refreshGraph;
                if (refresh) refresh();
              }}
            />
          </div>
        )}

        {/* Lineage panel */}
        {showLineage && (
          <div className="w-72 border-l flex flex-col overflow-hidden"
               style={{ borderColor: 'var(--border)', background: 'var(--bg-panel)' }}>
            <LineagePanel />
          </div>
        )}

        {/* Chat panel */}
        {showChat && (
          <div className="w-96 border-l flex flex-col"
               style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
            <ChatPanel />
          </div>
        )}
      </div>

      {/* Status bar */}
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
