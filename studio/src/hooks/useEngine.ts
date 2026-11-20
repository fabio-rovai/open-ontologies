import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';
import * as mcp from '../lib/mcp-client';
import { notifyGraphChanged } from '../lib/graph-refresh';

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

interface EngineStore {
  status: ConnectionStatus;
  stats: { triples: number; classes: number; properties: number; individuals: number } | null;
  error: string | null;
  connect: () => Promise<void>;
  refreshStats: () => Promise<void>;
}

export const useEngine = create<EngineStore>((set, get) => ({
  status: 'disconnected',
  stats: null,
  error: null,

  connect: async () => {
    set({ status: 'connecting', error: null });

    for (let i = 0; i < 10; i++) {
      try {
        // Use REST API for connectivity check, no MCP session needed
        const stats = await mcp.getStats();
        if (stats) {
          set({ status: 'connected' });
          // Also init MCP session for write operations
          mcp.initialize().catch(() => {/* MCP writes degrade gracefully */});
          // Load persisted graph from file
          await mcp.loadGraphFromFile();
          await get().refreshStats();
          // Tell every mounted graph view to re-read the store
          setTimeout(notifyGraphChanged, 300);
          return;
        }
      } catch {
        // Engine not ready yet
      }
      await new Promise(r => setTimeout(r, 1000));
    }

    set({ status: 'error', error: 'Could not connect to engine after 10 attempts' });
  },

  refreshStats: async () => {
    try {
      const stats = await mcp.getStats();
      set({ stats });
    } catch (e) {
      console.error('Failed to refresh stats:', e);
    }
  },
}));

// Listen for engine lifecycle events from Tauri
listen('engine-ready', () => {
  useEngine.getState().connect();
});

listen('engine-stopped', () => {
  useEngine.setState({ status: 'disconnected', error: 'Engine stopped' });
});
