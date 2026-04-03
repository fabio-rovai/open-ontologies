import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import * as mcp from '../lib/mcp-client';

// Sync agent MCP session ID to Rust backend so graph queries use the same store
async function syncSessionId(sessionId: string) {
  try {
    await invoke('set_mcp_session', { sessionId });
  } catch (e) {
    console.error('Failed to sync session ID:', e);
  }
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  toolCalls?: Array<{ tool: string; input?: unknown; result?: string }>;
  timestamp: Date;
}

export type BuildMode = 'sketch' | 'build';

export interface Progress {
  step: number;
  total: number;
  label: string;
}

interface ChatStore {
  messages: ChatMessage[];
  isTyping: boolean;
  mode: BuildMode;
  progress: Progress | null;
  setMode: (mode: BuildMode) => void;
  sendMessage: (text: string) => Promise<void>;
  reset: () => Promise<void>;
}

let msgCounter = 0;

export const useChat = create<ChatStore>((set) => ({
  messages: [{
    id: 'welcome',
    role: 'system',
    content: 'Welcome to Open Ontologies Studio. Ask me to build, expand, or validate ontologies.',
    timestamp: new Date(),
  }],
  isTyping: false,
  mode: 'sketch' as BuildMode,
  progress: null,
  setMode: (mode: BuildMode) => set({ mode }),

  sendMessage: async (text: string) => {
    const userMsg: ChatMessage = {
      id: `msg-${++msgCounter}`,
      role: 'user',
      content: text,
      timestamp: new Date(),
    };
    set(s => ({ messages: [...s.messages, userMsg], isTyping: true }));
    try {
      const { mode } = useChat.getState();
      await invoke('send_chat_message', { message: text, mode });
    } catch (e) {
      set(s => ({
        messages: [...s.messages, {
          id: `err-${msgCounter}`,
          role: 'system',
          content: `Error: ${e}`,
          timestamp: new Date(),
        }],
        isTyping: false,
      }));
    }
  },

  reset: async () => {
    await invoke('reset_chat');
    set({ messages: [{
      id: 'welcome',
      role: 'system',
      content: 'Conversation reset. How can I help?',
      timestamp: new Date(),
    }] });
  },
}));

// Listen for agent messages from sidecar
let currentAssistantMsg: ChatMessage | null = null;

listen<string>('agent-message', (event) => {
  try {
    const data = JSON.parse(event.payload);

    if (data.type === 'session') {
      syncSessionId(data.sessionId);
    }

    if (data.type === 'text') {
      if (!currentAssistantMsg) {
        currentAssistantMsg = {
          id: `msg-${++msgCounter}`,
          role: 'assistant',
          content: data.content,
          toolCalls: [],
          timestamp: new Date(),
        };
        useChat.setState(s => ({
          messages: [...s.messages, currentAssistantMsg!],
        }));
      } else {
        currentAssistantMsg.content += data.content;
        useChat.setState(s => ({
          messages: s.messages.map(m =>
            m.id === currentAssistantMsg!.id ? { ...currentAssistantMsg! } : m
          ),
        }));
      }
    }

    const MUTATION_TOOLS = ['onto_load', 'onto_clear', 'onto_apply', 'onto_reason',
      'onto_rollback', 'onto_ingest', 'onto_extend', 'onto_import', 'onto_pull', 'onto_enrich'];
    const isMutation = (tool: string) => MUTATION_TOOLS.some(t => tool === t || tool.endsWith(`__${t}`));

    const triggerGraphRefresh = (delay = 1000) => setTimeout(() => {
      const refreshGraph = (window as unknown as Record<string, () => void>).__refreshGraph;
      if (refreshGraph) refreshGraph();
    }, delay);

    if (data.type === 'tool_call') {
      if (currentAssistantMsg) {
        currentAssistantMsg.toolCalls = [
          ...(currentAssistantMsg.toolCalls || []),
          { tool: data.tool, input: data.input },
        ];
        useChat.setState(s => ({
          messages: s.messages.map(m =>
            m.id === currentAssistantMsg!.id ? { ...currentAssistantMsg! } : m
          ),
        }));
      }
      // Refresh graph after mutation tool call (engine needs ~1s to execute)
      if (isMutation(data.tool || '')) triggerGraphRefresh(1000);
    }

    if (data.type === 'tool_result') {
      if (currentAssistantMsg?.toolCalls) {
        const tc = currentAssistantMsg.toolCalls.find(t => t.tool === data.tool && !t.result);
        if (tc) tc.result = data.result;
        useChat.setState(s => ({
          messages: s.messages.map(m =>
            m.id === currentAssistantMsg!.id ? { ...currentAssistantMsg! } : m
          ),
        }));
      }
      if (isMutation(data.tool || '')) triggerGraphRefresh(300);
    }

    if (data.type === 'progress') {
      useChat.setState({ progress: { step: data.step, total: data.total, label: data.label } });
    }

    if (data.type === 'done') {
      currentAssistantMsg = null;
      useChat.setState({ isTyping: false, progress: null });
      if (data.mutated) {
        // Persist to file, then refresh graph
        mcp.saveGraphToFile().then(() => triggerGraphRefresh(300));
      }
    }

    if (data.type === 'error') {
      currentAssistantMsg = null;
      useChat.setState(s => ({
        isTyping: false,
        messages: [...s.messages, {
          id: `err-${++msgCounter}`,
          role: 'system',
          content: `Agent error: ${data.error}`,
          timestamp: new Date(),
        }],
      }));
    }
  } catch (e) {
    console.error('Failed to parse agent message:', e);
  }
});
