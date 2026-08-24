/**
 * Minimal MCP client over streamable HTTP.
 *
 * Talks JSON-RPC to the Open Ontologies engine. The engine answers either as
 * plain JSON or as an SSE stream depending on the request, so both are parsed.
 * Sessions can expire during long model turns, which surfaces as a bare HTTP
 * 404; that is treated as "reconnect and retry once" rather than fatal.
 */

import type { ToolSpec } from './providers/types.js';

interface McpToolEnvelope {
  name: string;
  description?: string;
  inputSchema?: Record<string, unknown>;
}

export class McpClient {
  private sessionId: string | undefined;
  private nextId = 1;

  constructor(private endpoint: string) {}

  private async rpc(method: string, params: unknown): Promise<any> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    };
    if (this.sessionId) headers['Mcp-Session-Id'] = this.sessionId;

    const resp = await fetch(this.endpoint, {
      method: 'POST',
      headers,
      body: JSON.stringify({ jsonrpc: '2.0', id: this.nextId++, method, params }),
    });

    const newSession = resp.headers.get('Mcp-Session-Id');
    if (newSession) this.sessionId = newSession;

    // A dead session can arrive as 404, 422, or any error body naming it:
    // engines evict sessions, and the exact shape varies. Whatever the form,
    // the recovery is identical: new session, retry once.
    if (!resp.ok && method !== 'initialize') {
      const body = await resp.text();
      if (resp.status === 404 || resp.status === 422 || /session not found/i.test(body)) {
        this.sessionId = undefined;
        await this.initialize();
        return this.rpc(method, params);
      }
      throw new Error(`MCP ${method} failed: HTTP ${resp.status} ${body}`);
    }
    if (!resp.ok) {
      throw new Error(`MCP ${method} failed: HTTP ${resp.status} ${await resp.text()}`);
    }

    const payload = this.parseBody(await resp.text());
    if (payload?.error) {
      throw new Error(`MCP ${method} error: ${payload.error.message ?? JSON.stringify(payload.error)}`);
    }
    return payload?.result;
  }

  /** Accepts a bare JSON body or an SSE stream, returning the JSON-RPC frame. */
  private parseBody(body: string): any {
    const trimmed = body.trim();
    if (!trimmed) return undefined;
    if (trimmed.startsWith('{')) return JSON.parse(trimmed);

    for (const line of trimmed.split('\n')) {
      if (!line.startsWith('data:')) continue;
      const data = line.slice(5).trim();
      if (!data || data === '[DONE]') continue;
      try {
        const frame = JSON.parse(data);
        if (frame && (frame.result !== undefined || frame.error !== undefined)) return frame;
      } catch {
        /* keep scanning */
      }
    }
    return undefined;
  }

  async initialize(): Promise<void> {
    await this.rpc('initialize', {
      protocolVersion: '2025-03-26',
      capabilities: {},
      clientInfo: { name: 'ontology-studio-sidecar', version: '3.0.0' },
    });
  }

  async listTools(): Promise<ToolSpec[]> {
    const result = await this.rpc('tools/list', {});
    const tools: McpToolEnvelope[] = result?.tools ?? [];
    return tools.map(t => ({
      name: t.name,
      description: t.description ?? '',
      inputSchema: t.inputSchema ?? { type: 'object', properties: {} },
    }));
  }

  async callTool(name: string, args: unknown): Promise<string> {
    const result = await this.rpc('tools/call', { name, arguments: args ?? {} });
    const content = result?.content;
    if (Array.isArray(content)) {
      const text = content
        .filter((c: any) => c?.type === 'text' && typeof c.text === 'string')
        .map((c: any) => c.text)
        .join('\n');
      if (text) return text;
    }
    return JSON.stringify(result ?? {});
  }
}
