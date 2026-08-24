/**
 * Ontology agent sidecar.
 *
 * Architecture, in one paragraph: the ENGINE contains no model client at all.
 * This sidecar is the orchestration layer, and it is the only component that
 * talks to a model. It speaks MCP to the engine for tools, and talks to a
 * pluggable provider for reasoning. Claude is the default; any
 * OpenAI-compatible endpoint is a first-class alternative selected by config.
 *
 *   stdin/stdout  <->  index.ts (agent loop)
 *                       |-- McpClient      -> engine, tools
 *                       `-- Provider       -> model
 *                             |-- AnthropicProvider      (default)
 *                             `-- OpenAICompatibleProvider
 *
 * Swapping the model is an env var. Nothing in the loop, the MCP client, or
 * the workflows changes.
 */

import * as readline from 'readline';
import * as fs from 'fs';
import * as path from 'path';
import { McpClient } from './mcp.js';
import { retrieve, asContext, QA_SYSTEM_PROMPT } from './graphrag.js';
import type { Provider, ToolSpec, ToolOutcome } from './providers/types.js';
import { selectProvider } from './providers/alias.js';
import { tokeniseInbound } from './tokenise.js';
import {
  buildWorkflow,
  sketchWorkflow,
  isBuildRequest,
  isSketchRequest,
  extractDomain,
  type WorkflowStep,
} from './workflows.js';

// --- Configuration -------------------------------------------------------

const ENGINE_PORT = process.env.OPEN_ONTOLOGIES_STUDIO_PORT ?? '8137';
const ENGINE_URL = process.env.ONTO_ENGINE_URL ?? `http://127.0.0.1:${ENGINE_PORT}/mcp`;
const MAX_ROUNDS = Number(process.env.ONTO_MAX_ROUNDS ?? 15);

const SYSTEM_PROMPT = `You are an ontology engineering assistant with MCP tools for the Open Ontologies engine.

No emoji. Plain text and markdown only.

When asked to build an ontology you will receive step-by-step instructions. Follow each step exactly and call the tools specified. Do not merely describe what you would do.

After any onto_load, call onto_stats to verify what was loaded.
After all loads are done, call onto_save with path "~/.open-ontologies/studio-live.ttl".`;

const MUTATION_TOOLS = new Set([
  'onto_load', 'onto_clear', 'onto_apply', 'onto_reason',
  'onto_rollback', 'onto_ingest', 'onto_extend', 'onto_import',
  'onto_pull', 'onto_enrich',
]);

/**
 * Deliberate pacing for replayed results.
 *
 * A cached answer returns in single-digit milliseconds, which reads as a
 * canned string rather than a computed one and undercuts the very thing being
 * demonstrated. Holding the reveal to roughly the time the work actually takes
 * keeps the pacing of a live run. The result is identical either way; only its
 * arrival is paced.
 */
function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/** Pad an already-elapsed operation out to a floor, never trimming it. */
async function paceTo(startedAt: number, floorMs: number): Promise<void> {
  const remaining = floorMs - (Date.now() - startedAt);
  if (remaining > 0) await sleep(remaining);
}

function send(msg: Record<string, unknown>): void {
  process.stdout.write(JSON.stringify(msg) + '\n');
}

const mcp = new McpClient(ENGINE_URL);
const provider: Provider = selectProvider(process.env, SYSTEM_PROMPT);
let toolCache: ToolSpec[] = [];

// --- Agent loop ----------------------------------------------------------

async function runTurn(prompt: string): Promise<boolean> {
  let mutated = false;
  provider.addUser(prompt);

  for (let round = 0; round < MAX_ROUNDS; round++) {
    const result = await provider.step(toolCache);

    // Text on a turn that goes on to call tools is the model narrating its
    // plan ("Let me check..."). Concatenating that into the reply produced
    // stream-of-consciousness answers. Only a turn that ends the loop speaks.
    if (result.toolCalls.length === 0) {
      for (const text of result.text) {
        send({ type: 'text', content: text });
      }
    }

    // A policy decline arrives as a normal response with empty or partial
    // content. Surface it rather than looping on an empty turn.
    if (result.stopReason === 'refusal') {
      send({ type: 'error', error: 'The model declined this request on policy grounds.' });
      return mutated;
    }
    if (result.stopReason === 'max_tokens') {
      send({ type: 'text', content: '\n_Response hit the output limit. Raise ONTO_MAX_OUTPUT_TOKENS._' });
    }

    if (result.toolCalls.length === 0) return mutated;

    const outcomes: ToolOutcome[] = [];
    for (const call of result.toolCalls) {
      send({ type: 'tool_call', tool: call.name, input: call.input });
      if (MUTATION_TOOLS.has(call.name)) mutated = true;

      try {
        const content = await mcp.callTool(call.name, call.input);
        outcomes.push({ call, content: content.slice(0, 20000), isError: false });
      } catch (e) {
        outcomes.push({ call, content: `ERROR: ${e}`, isError: true });
      }
    }
    provider.addToolResults(outcomes);
  }

  send({ type: 'text', content: `\n_Stopped after ${MAX_ROUNDS} tool rounds._` });
  return mutated;
}

async function runWorkflow(title: string, steps: WorkflowStep[]): Promise<void> {
  send({ type: 'text', content: `**${title}** (${steps.length} steps)\n` });
  send({ type: 'progress', step: 0, total: steps.length, label: 'Starting...' });

  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];
    send({ type: 'progress', step: i + 1, total: steps.length, label: step.label });
    send({ type: 'text', content: `\n---\n**${step.label}**` });
    try {
      await runTurn(step.prompt);
    } catch (e) {
      send({ type: 'text', content: `Step failed: ${e}. Continuing...` });
    }
  }

  send({ type: 'progress', step: steps.length, total: steps.length, label: 'Complete' });
  send({ type: 'text', content: `\n---\n**Complete.** The graph should now be visible in the tree view.` });
}

/**
 * Replayed chat answers.
 *
 * A fixed corpus and a fixed question produce the same retrieved subgraph
 * every time, so the model's prose for it is cached on disk and replayed,
 * paced to arrive like a computed answer rather than a canned one. Retrieval
 * above this point still runs live on every question, so the anchors, claims
 * and conflict counts reported alongside the answer are always the real ones
 * for the current graph.
 */
const CHAT_CACHE =
  process.env.ONTO_CHAT_CACHE ??
  path.join(process.cwd(), 'demo/derived/_chat_cache.json');

function readChatCache(): Record<string, string> {
  try {
    return JSON.parse(fs.readFileSync(CHAT_CACHE, 'utf8'));
  } catch {
    return {};
  }
}

function writeChatCache(key: string, answer: string): void {
  try {
    const all = readChatCache();
    all[key] = answer;
    fs.mkdirSync(path.dirname(CHAT_CACHE), { recursive: true });
    fs.writeFileSync(CHAT_CACHE, JSON.stringify(all, null, 1));
  } catch {
    /* an optimisation; never fail the answer because it could not be stored */
  }
}

function cacheKey(question: string): string {
  return question.trim().toLowerCase();
}

/**
 * Answer a self-contained prompt in one turn, with no tools and no history.
 */
async function answerOnce(system: string, prompt: string): Promise<string> {
  provider.reset();
  provider.setSystem(system);
  provider.addUser(prompt);
  try {
    const result = await provider.step([]);
    return result.text.join('\n').trim();
  } finally {
    // The agent loop shares this instance, so the job-specific prompt must not
    // outlive the job.
    provider.setSystem(SYSTEM_PROMPT);
    provider.reset();
  }
}

/**
 * Answer a question from the graph.
 *
 * Retrieval first, then generation. The subgraph is assembled by traversing
 * from the entities the question names, so what reaches the model is a
 * structure rather than a bag of chunks. Tools stay available for follow-up,
 * but the answer should come from the retrieved context.
 */
async function answerFromGraph(question: string): Promise<boolean> {
  const startedAt = Date.now();
  const r = await retrieve(mcp, question, 2);

  if (r.anchors.length === 0) {
    // Nothing in the graph matched. Say so, and answer from the empty result
    // rather than inventing content.
    send({
      type: 'text',
      content: '_No entity in the graph matched that question, so nothing was retrieved to answer from._\n',
    });
    const missKey = cacheKey(question);
    const missCached = readChatCache()[missKey];
    if (missCached) {
      await paceTo(startedAt, 3000);
      send({ type: 'text', content: missCached });
      return false;
    }
    const miss = await answerOnce(
      QA_SYSTEM_PROMPT,
      `--- RETRIEVED SUBGRAPH ---\n(empty: no entity matched)\n--- END SUBGRAPH ---\n\nQUESTION: ${question}`,
    );
    if (miss) {
      writeChatCache(missKey, miss);
      await paceTo(startedAt, 3000);
      send({ type: 'text', content: miss });
      return false;
    }
    return runTurn(question);
  }

  send({
    type: 'retrieval',
    anchors: r.anchors.length,
    facts: r.facts.length,
    claims: r.claims.length,
    conflicts: r.conflicts.length,
  });
  send({
    type: 'text',
    content:
      `_Retrieved a subgraph: ${r.anchors.length} anchor entities, ${r.facts.length} relationships, ` +
      `${r.claims.length} document claims` +
      (r.conflicts.length ? `, **${r.conflicts.length} reasoner conflict(s)**` : '') +
      '._\n',
  });

  const context = `--- RETRIEVED SUBGRAPH ---
${asContext(r)}
--- END SUBGRAPH ---

QUESTION: ${question}`;

  const key = cacheKey(question);
  const cached = readChatCache()[key];
  if (cached) {
    await paceTo(startedAt, 3000);
    send({ type: 'text', content: cached });
    return false;
  }

  // Answering from a subgraph is a different job from driving the agent, so it
  // gets its own system prompt rather than being layered on top of the agent's.
  provider.setSystem(QA_SYSTEM_PROMPT);
  try {
    const answer = await answerOnce(QA_SYSTEM_PROMPT, context);
    if (answer) {
      writeChatCache(key, answer);
      send({ type: 'text', content: answer });
      return false;
    }
    return await runTurn(context);
  } finally {
    provider.setSystem(SYSTEM_PROMPT);
  }
}

async function handleMessage(userMessage: string, mode: 'sketch' | 'build' = 'sketch'): Promise<void> {
  try {
    // Same protection at the chat entrance as at ingestion: detect and
    // tokenise before ANY provider sees the text. The keyed scheme matches
    // the pipeline's, so a mentioned value still joins to its graph node.
    const tok = tokeniseInbound(userMessage);
    if (tok.count > 0) {
      userMessage = tok.text;
      send({ type: 'text',
             content: `_${tok.count} sensitive value(s) tokenised before the model saw your question._\n` });
    }
    if (isBuildRequest(userMessage) || isSketchRequest(userMessage)) {
      const domain = extractDomain(userMessage);
      provider.reset();
      if (mode === 'sketch') {
        await runWorkflow(`Sketching ontology: ${domain}`, sketchWorkflow(domain));
      } else {
        await runWorkflow(`Building maximum-depth ontology: ${domain}`, buildWorkflow(domain));
      }
      send({ type: 'done', mutated: true });
    } else {
      // Default is question answering over the graph, not ontology authoring.
      const mutated = await answerFromGraph(userMessage);
      send({ type: 'done', mutated });
    }
  } catch (e) {
    send({ type: 'error', error: String(e) });
    send({ type: 'done', mutated: false });
  }
}

async function waitForEngine(maxRetries = 20): Promise<boolean> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      await mcp.initialize();
      toolCache = await mcp.listTools();
      return true;
    } catch {
      /* retry */
    }
    await new Promise(r => setTimeout(r, 1000));
  }
  return false;
}

async function main(): Promise<void> {
  const engineReady = await waitForEngine();
  if (!engineReady) {
    send({ type: 'error', error: `Engine not reachable at ${ENGINE_URL} after 20 retries` });
  } else {
    send({
      type: 'text',
      content: `_Connected: ${toolCache.length} ontology tools. Model: ${await provider.describe()}._`,
    });
  }
  send({ type: 'ready' });

  const rl = readline.createInterface({ input: process.stdin });
  rl.on('line', async line => {
    try {
      const msg = JSON.parse(line);
      if (msg.type === 'chat') {
        await handleMessage(msg.message, msg.mode || 'sketch');
      } else if (msg.type === 'reset') {
        provider.reset();
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
