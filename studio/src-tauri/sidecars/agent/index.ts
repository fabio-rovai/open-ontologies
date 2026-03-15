import { query } from '@anthropic-ai/claude-agent-sdk';
import type { Query, SDKMessage } from '@anthropic-ai/claude-agent-sdk';
import * as readline from 'readline';

// --- Configuration ---

const ENGINE_URL = 'http://localhost:8080/mcp';

const SYSTEM_PROMPT = `You are an ontology engineering assistant. You have access to the Open Ontologies engine with 42 tools for creating, validating, reasoning over, and managing OWL ontologies.

Do not use emoji in your responses. Use plain text and markdown formatting only.

Key tools:
- onto_clear: ALWAYS call this first before building a brand-new ontology from scratch
- onto_load: Load Turtle RDF into the store (use the "turtle" parameter for inline content)
- onto_query: Run SPARQL queries
- onto_stats: Get ontology statistics
- onto_validate: Check RDF/OWL syntax
- onto_lint: Quality checks
- onto_reason: Run OWL reasoning (rdfs, owl-rl)
- onto_enforce: Check design patterns
- onto_save: Export ontology
- onto_diff: Compare ontologies
- onto_plan: Preview changes (terraform-style)
- onto_apply: Apply planned changes
- onto_version/onto_history/onto_rollback: Version management

CRITICAL RULES:
1. When asked to BUILD, CREATE, or MAKE a new ontology from scratch — call onto_clear FIRST, then onto_load.
2. When asked to EXPAND, ADD TO, or EXTEND an existing ontology — do NOT clear, just onto_load.
3. After any onto_load or mutation, ALWAYS call onto_save with path "~/.open-ontologies/studio-live.ttl" to persist the graph so the UI can display it.
4. After mutations, mention what changed so the UI can refresh the graph.`;

const MUTATION_TOOLS = new Set([
  'onto_load', 'onto_clear', 'onto_apply', 'onto_reason',
  'onto_rollback', 'onto_ingest', 'onto_extend', 'onto_import',
  'onto_pull', 'onto_enrich'
]);

// --- State ---

let sessionId: string | undefined;

// --- stdout Protocol ---

function send(msg: Record<string, unknown>): void {
  process.stdout.write(JSON.stringify(msg) + '\n');
}

// --- Wait for engine to be ready ---

async function waitForEngine(maxRetries = 15): Promise<boolean> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const resp = await fetch(ENGINE_URL, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json, text/event-stream',
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'initialize',
          params: {
            protocolVersion: '2025-03-26',
            capabilities: {},
            clientInfo: { name: 'ontology-agent-probe', version: '1.0.0' },
          },
        }),
      });
      if (resp.ok) return true;
    } catch {
      // Engine not ready yet
    }
    await new Promise(r => setTimeout(r, 1000));
  }
  return false;
}

// --- Handle a chat message using the Claude Agent SDK ---

async function handleMessage(userMessage: string): Promise<void> {
  let mutated = false;

  try {
    // Build the prompt with system context prepended
    const fullPrompt = userMessage;

    const q: Query = query({
      prompt: fullPrompt,
      options: {
        systemPrompt: SYSTEM_PROMPT,
        model: 'claude-sonnet-4-6',
        mcpServers: {
          'ontology-engine': {
            type: 'http',
            url: ENGINE_URL,
          },
        },
        // Allow all MCP tools without prompting
        allowedTools: ['mcp__ontology-engine__*'],
        // No built-in tools needed - only MCP tools from the engine
        tools: [],
        // Persist sessions to disk so resume works across turns
        // Don't persist to disk — prevents auto-resuming stale sessions after engine restart
        persistSession: false,
        // Resume in-memory session for multi-turn within same sidecar process
        ...(sessionId ? { resume: sessionId } : {}),
        // Accept edits mode to avoid permission prompts
        permissionMode: 'bypassPermissions',
        allowDangerouslySkipPermissions: true,
      },
    });

    for await (const message of q) {
      // Capture session ID for multi-turn AND share with frontend for SPARQL queries
      if ('session_id' in message && message.session_id && !sessionId) {
        sessionId = message.session_id;
        // Tell the frontend which MCP session to use for graph queries
        send({ type: 'session', sessionId: message.session_id });
      }

      switch (message.type) {
        case 'assistant': {
          // Extract text content from the assistant message
          const content = message.message?.content;
          if (Array.isArray(content)) {
            for (const block of content) {
              if (block.type === 'text' && block.text) {
                send({ type: 'text', content: block.text });
              }
              if (block.type === 'tool_use') {
                send({ type: 'tool_call', tool: block.name, input: block.input });
                // Tool names are namespaced: "mcp__ontology-engine__onto_load"
                // Check if the name ends with any known mutation tool
                if ([...MUTATION_TOOLS].some(t => block.name === t || block.name.endsWith(`__${t}`))) {
                  mutated = true;
                }
              }
            }
          }
          break;
        }

        case 'result': {
          // Final result message
          if (message.subtype === 'success') {
            // Success - text already sent via assistant messages
          } else {
            // Error subtypes: error_during_execution, error_max_turns, etc.
            const errors = 'errors' in message ? (message as { errors?: string[] }).errors : [];
            send({ type: 'error', error: (errors && errors.length > 0) ? errors.join('; ') : `Agent error: ${message.subtype}` });
          }
          break;
        }

        case 'system': {
          // System messages (compact boundaries, etc.) - ignore
          break;
        }
      }
    }

    send({ type: 'done', mutated });
  } catch (e) {
    send({ type: 'error', error: String(e) });
    send({ type: 'done', mutated });
  }
}

// --- stdin/stdout Protocol ---

async function main(): Promise<void> {
  // Wait for the ontology engine to be ready
  const engineReady = await waitForEngine();
  if (!engineReady) {
    send({ type: 'error', error: 'Engine not reachable after 15 retries' });
  }

  send({ type: 'ready' });

  const rl = readline.createInterface({ input: process.stdin });

  rl.on('line', async (line) => {
    try {
      const msg = JSON.parse(line);
      if (msg.type === 'chat') {
        await handleMessage(msg.message);
      } else if (msg.type === 'reset') {
        sessionId = undefined;
        send({ type: 'reset_done' });
      }
    } catch (e) {
      send({ type: 'error', error: String(e) });
    }
  });
}

main().catch(e => {
  send({ type: 'error', error: String(e) });
  process.exit(1);
});
