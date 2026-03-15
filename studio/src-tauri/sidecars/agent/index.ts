import { query } from '@anthropic-ai/claude-agent-sdk';
import type { Query, SDKMessage } from '@anthropic-ai/claude-agent-sdk';
import * as readline from 'readline';

// --- Configuration ---

const ENGINE_URL = 'http://localhost:8080/mcp';

const SYSTEM_PROMPT = `You are an ontology engineering assistant with access to the Open Ontologies engine and all 42 of its tools.

Do not use emoji in your responses. Use plain text and markdown formatting only.

ALL 42 TOOLS — use whichever are appropriate for the task:

Core:
- onto_clear: Reset the triple store (call FIRST when building from scratch)
- onto_load: Load Turtle RDF into the store (use "turtle" param for inline content)
- onto_save: Export ontology to file
- onto_stats: Get triple/class/property counts
- onto_validate: Check RDF/OWL syntax
- onto_lint: Quality checks — missing labels, domains, ranges
- onto_query: Run SPARQL SELECT queries
- onto_diff: Compare two ontology versions
- onto_convert: Convert between formats (Turtle, N-Triples, RDF/XML, N-Quads, TriG)
- onto_status: Check if server is running

Remote:
- onto_pull: Fetch ontology from a remote URL or SPARQL endpoint
- onto_push: Push ontology to a remote SPARQL endpoint
- onto_import: Resolve and load owl:imports chains from URLs

Schema:
- onto_import_schema: Import a PostgreSQL schema as OWL ontology

Data pipeline:
- onto_map: Generate a mapping config from data file + loaded ontology
- onto_ingest: Parse structured data (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF
- onto_shacl: Validate loaded data against SHACL shapes
- onto_extend: Run the full pipeline: ingest + SHACL validate + reason in one call

Versioning:
- onto_version: Save a named snapshot before making changes
- onto_history: List saved version snapshots
- onto_rollback: Restore a previous version

Lifecycle (Terraform-style):
- onto_plan: Preview changes — added/removed classes, blast radius, risk score
- onto_apply: Apply planned changes (safe or migrate mode)
- onto_lock: Protect production IRIs from removal
- onto_drift: Compare versions — rename detection, drift velocity
- onto_enforce: Check design pattern compliance (generic, boro, value_partition)
- onto_monitor: Run SPARQL watchers with threshold alerts
- onto_monitor_clear: Clear blocked state after resolving monitor alerts
- onto_lineage: View the session lineage trail

Alignment:
- onto_align: Detect alignment candidates between two ontologies (7 weighted signals)
- onto_align_feedback: Accept/reject alignment candidates to self-calibrate confidence

Clinical:
- onto_crosswalk: Look up ICD-10 / SNOMED / MeSH terminology mappings
- onto_enrich: Add skos:exactMatch triples linking classes to clinical codes
- onto_validate_clinical: Check class labels against clinical crosswalk terminology

Feedback (self-calibrating):
- onto_lint_feedback: Accept/dismiss a lint issue to suppress future warnings
- onto_enforce_feedback: Accept/dismiss an enforce violation

Embeddings + semantic search:
- onto_embed: Generate text + Poincare structural embeddings for all classes
- onto_search: Find classes by natural language description (requires onto_embed first)
- onto_similarity: Compute embedding similarity between two IRIs

OWL2-DL Reasoning:
- onto_reason: Run RDFS or OWL-RL inference — materializes inferred triples
- onto_dl_explain: Explain why a class is unsatisfiable (DL tableaux clash trace)
- onto_dl_check: Check if one class is subsumed by another (DL tableaux)

CRITICAL RULES:
1. When asked to BUILD, CREATE, or MAKE a new ontology from scratch — call onto_clear FIRST, then onto_load.
2. When asked to EXPAND, ADD TO, or EXTEND an existing ontology — do NOT clear, just onto_load.
3. After any onto_load or mutation, ALWAYS call onto_save with path "~/.open-ontologies/studio-live.ttl" to persist the graph so the UI can display it.
4. After mutations, mention what changed so the UI can refresh the graph.
5. For a thorough build: onto_clear -> onto_load -> onto_validate -> onto_stats -> onto_lint -> onto_reason -> onto_save -> onto_version.
6. For data ingestion: onto_map -> onto_ingest -> onto_shacl -> onto_reason -> onto_save.
7. For alignment: onto_align -> onto_align_feedback -> onto_apply -> onto_save.`;

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
