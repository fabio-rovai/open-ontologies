import { invoke } from '@tauri-apps/api/core';

// All MCP calls go through the Tauri backend (Rust → localhost:8080)
// This avoids webview fetch restrictions and handles SSE parsing in Rust

async function mcpCall(method: string, params: Record<string, unknown> = {}): Promise<unknown> {
  return invoke('mcp_call', { method, params });
}

// Sessionless REST API — direct access to shared graph store, no MCP session required
const API = 'http://127.0.0.1:8080/api';

async function apiGet(path: string): Promise<unknown> {
  const resp = await fetch(`${API}${path}`);
  return resp.json();
}

async function apiPost(path: string, body: Record<string, unknown>): Promise<unknown> {
  const resp = await fetch(`${API}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  return resp.json();
}

// --- Public API ---

export async function initialize(): Promise<boolean> {
  try {
    await mcpCall('initialize', {
      protocolVersion: '2025-03-26',
      capabilities: {},
      clientInfo: { name: 'open-ontologies-studio', version: '1.0.0' },
    });
    // notifications/initialized is a one-way notification, ignore errors
    try { await mcpCall('notifications/initialized', {}); } catch { /* ok */ }
    return true;
  } catch (e) {
    console.error('MCP init failed:', e);
    return false;
  }
}

export async function callTool(name: string, args: Record<string, unknown> = {}): Promise<string> {
  const result = await mcpCall('tools/call', { name, arguments: args }) as {
    content?: Array<{ type: string; text?: string }>;
  };
  if (result?.content) {
    return result.content
      .filter((c: { type: string }) => c.type === 'text')
      .map((c: { text?: string }) => c.text || '')
      .join('\n');
  }
  return JSON.stringify(result);
}

export async function listTools(): Promise<Array<{ name: string; description: string; inputSchema: unknown }>> {
  const result = await mcpCall('tools/list', {}) as {
    tools?: Array<{ name: string; description: string; inputSchema: unknown }>;
  };
  return result?.tools || [];
}

export async function getStats(): Promise<{ triples: number; classes: number; properties: number; individuals: number }> {
  try {
    const result = await apiGet('/stats') as Record<string, number>;
    return {
      triples: result.triples ?? 0,
      classes: result.classes ?? 0,
      properties: result.properties ?? 0,
      individuals: result.individuals ?? 0,
    };
  } catch {
    return { triples: 0, classes: 0, properties: 0, individuals: 0 };
  }
}

export async function loadTurtle(turtle: string): Promise<string> {
  const result = await apiPost('/load-turtle', { turtle }) as Record<string, unknown>;
  if (result.error) throw new Error(String(result.error));
  return JSON.stringify(result);
}

export async function sparqlQuery(query: string): Promise<string> {
  const result = await apiPost('/query', { query });
  return JSON.stringify(result);
}

export async function sparqlUpdate(query: string): Promise<void> {
  await apiPost('/update', { query });
}

export async function validate(turtle: string): Promise<string> {
  return callTool('onto_validate', { input: turtle, inline: true });
}

export async function lint(turtle: string): Promise<string> {
  return callTool('onto_lint', { input: turtle, inline: true });
}

const LIVE_FILE = '~/.open-ontologies/studio-live.ttl';

export async function saveGraphToFile(): Promise<void> {
  try {
    const result = await apiPost('/save', { path: LIVE_FILE, format: 'turtle' }) as Record<string, unknown>;
    if (result.error) console.warn('Failed to save graph to file:', result.error);
  } catch (e) {
    console.warn('Failed to save graph to file:', e);
  }
}

export async function saveGraphAs(path: string): Promise<void> {
  const result = await apiPost('/save', { path, format: 'turtle' }) as Record<string, unknown>;
  if (result.error) throw new Error(String(result.error));
}

export async function loadGraphFromFile(): Promise<boolean> {
  try {
    const result = await apiPost('/load', { path: LIVE_FILE }) as Record<string, unknown>;
    return result.ok === true && ((result.triples_loaded as number) ?? 0) > 0;
  } catch {
    return false;
  }
}

export interface LineageEvent {
  session: string;
  seq: number;
  ts: string;
  type: string;
  op: string;
  details: string;
}

export async function getLineage(): Promise<LineageEvent[]> {
  try {
    const result = await apiGet('/lineage') as { events: LineageEvent[] };
    return result.events ?? [];
  } catch { return []; }
}

export async function enforce(rulePack: string): Promise<string> {
  return callTool('onto_enforce', { rule_pack: rulePack });
}

export async function clearStore(): Promise<string> {
  return callTool('onto_clear');
}
