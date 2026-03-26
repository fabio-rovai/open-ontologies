import { query } from '@anthropic-ai/claude-agent-sdk';
import type { Query } from '@anthropic-ai/claude-agent-sdk';
import * as readline from 'readline';

const ENGINE_URL = 'http://localhost:8080/mcp';

const SYSTEM_PROMPT = `You are an ontology engineering assistant with MCP tools for the Open Ontologies engine.

No emoji. Plain text and markdown only.

CRITICAL: When asked to build an ontology, you will receive step-by-step instructions. Follow each step EXACTLY. Call the tools specified — do NOT just describe what you would do.

After any onto_load, always call onto_stats to verify what was loaded.
After all loads are done, always call onto_save with path "~/.open-ontologies/studio-live.ttl".`;

const MUTATION_TOOLS = new Set([
  'onto_load', 'onto_clear', 'onto_apply', 'onto_reason',
  'onto_rollback', 'onto_ingest', 'onto_extend', 'onto_import',
  'onto_pull', 'onto_enrich'
]);

let sessionId: string | undefined;

function send(msg: Record<string, unknown>): void {
  process.stdout.write(JSON.stringify(msg) + '\n');
}

async function waitForEngine(maxRetries = 15): Promise<boolean> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const resp = await fetch(ENGINE_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream' },
        body: JSON.stringify({
          jsonrpc: '2.0', id: 1, method: 'initialize',
          params: { protocolVersion: '2025-03-26', capabilities: {}, clientInfo: { name: 'probe', version: '1.0.0' } },
        }),
      });
      if (resp.ok) return true;
    } catch { /* retry */ }
    await new Promise(r => setTimeout(r, 1000));
  }
  return false;
}

// --- Run one agent turn within a persistent session ---

async function runTurn(prompt: string): Promise<boolean> {
  let mutated = false;

  const q: Query = query({
    prompt,
    options: {
      systemPrompt: SYSTEM_PROMPT,
      model: 'claude-opus-4-6',
      mcpServers: { 'ontology-engine': { type: 'http', url: ENGINE_URL } },
      allowedTools: ['mcp__ontology-engine__*'],
      tools: [],
      persistSession: true,
      ...(sessionId ? { resume: sessionId } : {}),
      permissionMode: 'bypassPermissions',
      allowDangerouslySkipPermissions: true,
      maxTurns: 15,
    },
  });

  for await (const message of q) {
    if ('session_id' in message && message.session_id) {
      if (!sessionId) {
        sessionId = message.session_id;
        send({ type: 'session', sessionId: message.session_id });
      }
    }

    switch (message.type) {
      case 'assistant': {
        const content = message.message?.content;
        if (Array.isArray(content)) {
          for (const block of content) {
            if (block.type === 'text' && block.text) {
              send({ type: 'text', content: block.text });
            }
            if (block.type === 'tool_use') {
              send({ type: 'tool_call', tool: block.name, input: block.input });
              if ([...MUTATION_TOOLS].some(t => block.name === t || block.name.endsWith(`__${t}`))) {
                mutated = true;
              }
            }
          }
        }
        break;
      }
      case 'result': {
        if (message.subtype !== 'success') {
          const errors = 'errors' in message ? (message as { errors?: string[] }).errors : [];
          send({ type: 'error', error: (errors && errors.length > 0) ? errors.join('; ') : `Agent error: ${message.subtype}` });
        }
        break;
      }
      case 'system': break;
    }
  }

  return mutated;
}

// --- Build request detection ---

function isBuildRequest(msg: string): boolean {
  const lower = msg.toLowerCase();
  return (lower.includes('build') || lower.includes('create') || lower.includes('make') || lower.includes('generate'))
    && (lower.includes('ontology') || lower.includes('about'));
}

function extractDomain(msg: string): string {
  const patterns = [
    /(?:about|for|on|of)\s+(.+)/i,
    /(?:build|create|make|generate)\s+(?:a\s+|an\s+|the\s+)?(?:\w+\s+)?(?:ontology\s+)?(?:about|for|on|of)\s+(.+)/i,
  ];
  for (const p of patterns) {
    const m = msg.match(p);
    if (m) {
      const match = m[2] || m[1];
      if (match) return match.trim().replace(/[.!?]+$/, '');
    }
  }
  return msg.replace(/^(build|create|make|generate)\s+(an?\s+)?ontology\s*/i, '').trim() || msg;
}

// --- Multi-step build within ONE session ---

async function handleBuild(domain: string): Promise<void> {
  const ns = domain.toLowerCase().replace(/[^a-z0-9]+/g, '-');

  // Step 1: Clear + load classes
  send({ type: 'text', content: `**Building ontology: ${domain}**\n\n---\n**Step 1/4:** Classes hierarchy...` });
  await runTurn(
    `Build an ontology about "${domain}". Use namespace @prefix : <http://example.org/${ns}#> .

Step 1: Call onto_clear. Then call onto_load with Turtle containing:
- 80-150 owl:Class declarations organised in a subClassOf hierarchy 5-7 levels deep
- A root class, 5-8 major branches, each with 3-6 sub-branches, each with 2-5 leaves
- Enumerate ALL real-world subtypes exhaustively
- Every class MUST have rdfs:label and rdfs:comment

Call onto_stats after loading to verify. Do NOT call onto_save yet — more content coming.`
  );

  // Step 2: Properties (SAME session — agent has context)
  send({ type: 'text', content: `\n---\n**Step 2/4:** Properties...` });
  await runTurn(
    `Now add properties to the ontology. Call onto_load with Turtle using the SAME namespace as before containing:

- 25-40 owl:ObjectProperty declarations, each with rdfs:domain, rdfs:range, rdfs:label, rdfs:comment
- Build rdfs:subPropertyOf hierarchies (e.g., hasParticipant > hasAgent > hasDriver)
- Add owl:inverseOf pairs for bidirectional relationships
- Mark owl:TransitiveProperty (isPartOf), owl:SymmetricProperty (isRelatedTo), owl:FunctionalProperty where appropriate
- 15-20 owl:DatatypeProperty declarations with rdfs:domain, rdfs:range (xsd types), rdfs:label, rdfs:comment

Call onto_stats after loading. Do NOT call onto_save yet.`
  );

  // Step 3: Axioms + individuals (SAME session)
  send({ type: 'text', content: `\n---\n**Step 3/4:** Axioms + individuals...` });
  await runTurn(
    `Now add axioms and individuals. Call onto_load with Turtle using the SAME namespace containing:

- owl:disjointWith between ALL sibling classes that cannot overlap
- 15-20 owl:NamedIndividual instances — real-world examples with rdf:type and property values

Call onto_stats after loading. Do NOT call onto_save yet.`
  );

  // Step 4: Reason + save (SAME session)
  send({ type: 'text', content: `\n---\n**Step 4/4:** Reasoning + save...` });
  await runTurn(
    `Final step. Run:
1. onto_reason with profile "rdfs"
2. onto_stats — report the final counts
3. onto_save with path "~/.open-ontologies/studio-live.ttl"

Report the final ontology statistics.`
  );

  send({ type: 'text', content: `\n---\n**Build complete.** Refresh the tree view to see the full graph.` });
}

// --- Handle a chat message ---

async function handleMessage(userMessage: string): Promise<void> {
  try {
    if (isBuildRequest(userMessage)) {
      const domain = extractDomain(userMessage);
      // Fresh session for each new build
      sessionId = undefined;
      await handleBuild(domain);
      send({ type: 'done', mutated: true });
    } else {
      const mutated = await runTurn(userMessage);
      send({ type: 'done', mutated });
    }
  } catch (e) {
    send({ type: 'error', error: String(e) });
    send({ type: 'done', mutated: false });
  }
}

// --- Main ---

async function main(): Promise<void> {
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
