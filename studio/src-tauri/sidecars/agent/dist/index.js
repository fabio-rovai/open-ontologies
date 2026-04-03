import { query } from '@anthropic-ai/claude-agent-sdk';
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
let sessionId;
function send(msg) {
    process.stdout.write(JSON.stringify(msg) + '\n');
}
async function waitForEngine(maxRetries = 15) {
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
            if (resp.ok)
                return true;
        }
        catch { /* retry */ }
        await new Promise(r => setTimeout(r, 1000));
    }
    return false;
}
// --- Run one agent turn within a persistent session ---
async function runTurn(prompt) {
    let mutated = false;
    const q = query({
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
                    const errors = 'errors' in message ? message.errors : [];
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
function isBuildRequest(msg) {
    const lower = msg.toLowerCase();
    return (lower.includes('build') || lower.includes('create') || lower.includes('make') || lower.includes('generate'))
        && (lower.includes('ontology') || lower.includes('about'));
}
function isSketchRequest(msg) {
    return msg.toLowerCase().includes('sketch');
}
function extractDomain(msg) {
    const patterns = [
        /(?:about|for|on|of)\s+(.+)/i,
        /(?:build|create|make|generate|sketch)\s+(?:a\s+|an\s+|the\s+)?(?:\w+\s+)?(?:ontology\s+)?(?:about|for|on|of)\s+(.+)/i,
    ];
    for (const p of patterns) {
        const m = msg.match(p);
        if (m) {
            const match = m[2] || m[1];
            if (match)
                return match.trim().replace(/[.!?]+$/, '');
        }
    }
    return msg.replace(/^(build|create|make|generate|sketch)\s+(an?\s+)?ontology\s*/i, '').trim() || msg;
}
// --- Multi-step build within ONE session ---
async function handleBuild(domain) {
    const ns = domain.toLowerCase().replace(/[^a-z0-9]+/g, '-');
    const prefix = `@prefix : <http://example.org/${ns}#> .`;
    const DEEPEN = (branchDesc) => `Call onto_query to find leaf classes (classes with no subclasses) in ${branchDesc}. ` +
        `Then call onto_load with Turtle using the SAME namespace adding more subclass levels. ` +
        `For every leaf, ask yourself: "Can this be subdivided further?" If yes, add 3-8 subclasses. ` +
        `Enumerate ALL real-world subtypes — do not stop at 2-3 examples. ` +
        `Every class needs rdfs:label and rdfs:comment. ` +
        `IMPORTANT: Add at most 80-120 classes in this step, then stop. Call onto_stats after. Do NOT save yet.`;
    const steps = [
        {
            label: 'Step 1: Foundation — root + major branches + 3 levels',
            prompt: `Build an ontology about "${domain}". Use namespace ${prefix}

Call onto_clear. Then call onto_load with Turtle containing:
- An owl:Ontology declaration
- A root class for the domain
- 8-12 major branch classes under the root (think of ALL major aspects/dimensions of "${domain}")
- For each branch, 4-8 subclasses
- For each of those, 3-6 further subclasses
- Every class MUST have rdfs:label and rdfs:comment
- Be EXHAUSTIVE — list every subtype you can think of

Call onto_stats after. Do NOT save yet — many more steps coming.`,
        },
        {
            label: 'Step 2: Deepen — first major branch to maximum depth',
            prompt: DEEPEN('the FIRST major branch (the first child of the root)'),
        },
        {
            label: 'Step 3: Deepen — second major branch to maximum depth',
            prompt: DEEPEN('the SECOND major branch'),
        },
        {
            label: 'Step 4: Deepen — third major branch to maximum depth',
            prompt: DEEPEN('the THIRD major branch'),
        },
        {
            label: 'Step 5: Deepen — fourth and fifth major branches',
            prompt: DEEPEN('the FOURTH and FIFTH major branches'),
        },
        {
            label: 'Step 6: Deepen — all remaining branches',
            prompt: DEEPEN('ALL remaining major branches that have not been deepened yet'),
        },
        {
            label: 'Step 7: Deepen — second pass on shallow areas',
            prompt: `Call onto_query to find ALL leaf classes. For any branch that is still less than 6 levels deep, add more subclasses. ` +
                `Also think: are there any major subtypes or categories I missed entirely? Add them now. ` +
                `Be ruthlessly exhaustive. Call onto_load with the additional Turtle. Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 8: Object properties — structural relationships',
            prompt: `Now add object properties. Call onto_load with Turtle containing 50-70 owl:ObjectProperty declarations.

EVERY property MUST have: rdfs:domain, rdfs:range, rdfs:label, rdfs:comment.

Cover ALL relationship types:
- Compositional: hasPart/isPartOf, contains/isContainedIn, hasComponent/isComponentOf
- Causal: causes, prevents, triggers, treats, inhibits, enables
- Associative: isAssociatedWith, isRelatedTo, dependsOn, influences
- Hierarchical: hasSubtype, isExampleOf, instantiates
- Build rdfs:subPropertyOf hierarchies (3-4 levels deep)
- Add owl:inverseOf for EVERY directional property
- Mark owl:TransitiveProperty, owl:SymmetricProperty, owl:FunctionalProperty

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 9: Object properties — roles, temporal, spatial',
            prompt: `Add MORE object properties. Call onto_load with Turtle containing 50-70 MORE owl:ObjectProperty declarations.

Focus on what's MISSING — look at the classes and ask "how does X relate to Y?" for every pair of branches:
- Role/participation: hasRole, participatesIn, performs, undergoes, produces, consumes
- Temporal: precedes, follows, during, overlaps, startsWith, endsWith
- Spatial: isLocatedIn, isNear, surrounds, isAdjacentTo, isWithin
- Ownership: owns, belongsTo, isOwnedBy, manages, controls
- Agent: hasAgent, hasPatient, hasBeneficiary, hasInstrument
- More rdfs:subPropertyOf hierarchies and owl:inverseOf pairs

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 10: Datatype properties — all attributes',
            prompt: `Add datatype properties. Call onto_load with Turtle containing 40-60 owl:DatatypeProperty declarations.

Each with: rdfs:domain, rdfs:range (xsd types), rdfs:label, rdfs:comment.

Go through EVERY major branch and add ALL attributes:
- Identifiers, names, codes, labels, descriptions, titles
- Dates (birth, creation, modification, expiry, start, end)
- Quantities (weight, height, length, count, duration, price, score, rating, percentage)
- Measurements (temperature, speed, volume, area, concentration)
- Boolean flags (isActive, isVerified, isPublic, isRequired, isOptional, isDeprecated)
- Statuses, categories, priorities, levels, grades
- Text fields (notes, comments, instructions, definitions)

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 11: Axioms — disjoints everywhere',
            prompt: `Add disjoint axioms. Call onto_load with Turtle containing owl:disjointWith between ALL sibling classes that cannot overlap.

Go through EVERY branch systematically:
- Root children: all major branches that are mutually exclusive
- Within each branch: siblings that cannot overlap
- Target: 60+ disjoint axiom pairs minimum

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 12: Individuals — real-world examples',
            prompt: `Add named individuals. Call onto_load with Turtle containing 25-40 owl:NamedIndividual instances.

Spread them across ALL major branches — at least 3-4 individuals per branch.
Each individual needs:
- rdf:type (the most specific class)
- rdfs:label and rdfs:comment
- 3-5 property values (both object and datatype properties)

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 13: Reason + save',
            prompt: `Final step. Run:
1. onto_reason with profile "rdfs"
2. onto_stats — report the final counts
3. onto_save with path "~/.open-ontologies/studio-live.ttl"

Report the final ontology statistics.`,
        },
    ];
    send({ type: 'text', content: `**Building maximum-depth ontology: ${domain}** (${steps.length} steps)\n` });
    send({ type: 'progress', step: 0, total: steps.length, label: 'Starting build...' });
    for (let i = 0; i < steps.length; i++) {
        const step = steps[i];
        send({ type: 'progress', step: i + 1, total: steps.length, label: step.label });
        send({ type: 'text', content: `\n---\n**${step.label}**` });
        try {
            await runTurn(step.prompt);
        }
        catch (e) {
            send({ type: 'text', content: `Step failed: ${e}. Continuing...` });
        }
    }
    send({ type: 'progress', step: steps.length, total: steps.length, label: 'Build complete' });
    send({ type: 'text', content: `\n---\n**Build complete.** The graph should now be visible in the tree view.` });
}
// --- Quick sketch: 3-step lightweight build ---
async function handleSketch(domain) {
    const ns = domain.toLowerCase().replace(/[^a-z0-9]+/g, '-');
    const prefix = `@prefix : <http://example.org/${ns}#> .`;
    const steps = [
        {
            label: 'Step 1/5: Foundation — root + branches',
            prompt: `Build an ontology about "${domain}". Use namespace ${prefix}

Call onto_clear. Then call onto_load with ONE Turtle block containing:
- An owl:Ontology declaration
- A root class for the domain
- 6-10 major branch classes under the root
- For each branch, 3-5 subclasses (2-3 levels deep)
- Every class MUST have rdfs:label and rdfs:comment

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 2/5: Deepen + properties',
            prompt: `Call onto_query to find leaf classes (classes with no subclasses).
Then call onto_load with Turtle using the SAME namespace ${prefix} adding:
- 2-4 more subclass levels under leaf classes that can be subdivided further
- 15-25 owl:ObjectProperty each with rdfs:domain, rdfs:range, rdfs:label, rdfs:comment
- owl:inverseOf pairs for directional properties
- 8-12 owl:DatatypeProperty with rdfs:domain, rdfs:range (xsd types), rdfs:label

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 3/5: Axioms + individuals',
            prompt: `Call onto_load with Turtle using namespace ${prefix} adding:
- owl:disjointWith between sibling classes that cannot overlap (15+ pairs)
- 12-20 owl:NamedIndividual spread across different branches, each with:
  - rdf:type (the most specific class)
  - rdfs:label and rdfs:comment
  - 2-4 property values (both object and datatype properties)

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 4/5: Verify + fix gaps',
            prompt: `Call onto_stats and onto_query to check: How many individuals exist? What is the max depth? Are there branches with no properties?

If individuals < 10, call onto_load with more individuals.
If max depth < 5, call onto_load deepening the shallowest branches.
If any branch has no object properties connecting it, add some.

Call onto_stats after. Do NOT save yet.`,
        },
        {
            label: 'Step 5/5: Reason + save',
            prompt: `Run onto_reason (profile "rdfs"), then onto_stats, then onto_save ("~/.open-ontologies/studio-live.ttl"). Report final statistics including class count, property count, individual count, max depth, and triple count.`,
        },
    ];
    send({ type: 'text', content: `**Sketching ontology: ${domain}** (${steps.length} steps)\n` });
    send({ type: 'progress', step: 0, total: steps.length, label: 'Starting sketch...' });
    for (let i = 0; i < steps.length; i++) {
        const step = steps[i];
        send({ type: 'progress', step: i + 1, total: steps.length, label: step.label });
        send({ type: 'text', content: `\n---\n**${step.label}**` });
        try {
            await runTurn(step.prompt);
        }
        catch (e) {
            send({ type: 'text', content: `Step failed: ${e}. Continuing...` });
        }
    }
    send({ type: 'progress', step: steps.length, total: steps.length, label: 'Sketch complete' });
    send({ type: 'text', content: `\n---\n**Sketch complete.** Use /expand to deepen any branch.` });
}
// --- Handle a chat message ---
async function handleMessage(userMessage, mode = 'sketch') {
    try {
        const isBuildLike = isBuildRequest(userMessage) || isSketchRequest(userMessage);
        if (isBuildLike) {
            const domain = extractDomain(userMessage);
            sessionId = undefined;
            if (mode === 'sketch') {
                await handleSketch(domain);
            }
            else {
                await handleBuild(domain);
            }
            send({ type: 'done', mutated: true });
        }
        else {
            const mutated = await runTurn(userMessage);
            send({ type: 'done', mutated });
        }
    }
    catch (e) {
        send({ type: 'error', error: String(e) });
        send({ type: 'done', mutated: false });
    }
}
// --- Main ---
async function main() {
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
                await handleMessage(msg.message, msg.mode || 'sketch');
            }
            else if (msg.type === 'reset') {
                sessionId = undefined;
                send({ type: 'reset_done' });
            }
        }
        catch (e) {
            send({ type: 'error', error: String(e) });
        }
    });
}
main().catch(e => {
    send({ type: 'error', error: String(e) });
    process.exit(1);
});
