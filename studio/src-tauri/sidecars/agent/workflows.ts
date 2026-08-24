/**
 * Multi-step ontology build workflows.
 *
 * These are prompt sequences, not code paths: each step is one agent turn
 * against the same conversation. Kept separate from the agent loop so the
 * loop stays about orchestration and these stay about ontology engineering.
 */

export interface WorkflowStep {
  label: string;
  prompt: string;
}

function namespaceFor(domain: string): { ns: string; prefix: string } {
  const ns = domain.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  return { ns, prefix: `@prefix : <http://example.org/${ns}#> .` };
}

const deepen = (prefix: string, branchDesc: string): string =>
  `Call onto_query with this SPARQL to find leaf classes in ${branchDesc}:
SELECT ?leaf ?label WHERE { ?leaf rdfs:subClassOf+ ?branch . FILTER NOT EXISTS { ?child rdfs:subClassOf ?leaf } . OPTIONAL { ?leaf rdfs:label ?label } } LIMIT 30

Then call onto_load with Turtle using the SAME namespace ${prefix} adding DEEPER subclass chains. For each leaf class:
1. Add 3-5 rdfs:subClassOf children
2. For each of THOSE children, add 2-4 more subclasses
3. If possible, add one more level below that

The goal is DEPTH not width. Each new class needs rdfs:label and rdfs:comment.
Add at most 80-120 classes in this step. Call onto_stats after. Do NOT save yet.`;

/** Full build: maximum depth, properties, axioms, individuals. */
export function buildWorkflow(domain: string): WorkflowStep[] {
  const { ns, prefix } = namespaceFor(domain);
  const root = ns.charAt(0).toUpperCase() + ns.slice(1).replace(/-./g, m => m[1].toUpperCase());

  return [
    {
      label: 'Step 1: Foundation, root + 5 levels deep',
      prompt: `Build an ontology about "${domain}". Use namespace ${prefix}

Call onto_clear. Then call onto_load with Turtle containing:
- An owl:Ontology declaration
- A root class :${root}
- 6-10 major branch classes as rdfs:subClassOf the root (Level 1)
- For each branch, 3-5 subclasses (Level 2)
- For each of those, 2-4 further subclasses (Level 3)
- For at least half of Level 3, add 2-3 more subclasses (Level 4)
- For at least a quarter of Level 4, add 2 more subclasses (Level 5)

Structure this as a DEEP tree, not a wide one. Every class MUST have rdfs:label and rdfs:comment.

Call onto_stats after. Do NOT save yet, many more steps coming.`,
    },
    { label: 'Step 2: Deepen the first major branch', prompt: deepen(prefix, 'the FIRST major branch') },
    { label: 'Step 3: Deepen the second major branch', prompt: deepen(prefix, 'the SECOND major branch') },
    { label: 'Step 4: Deepen the third major branch', prompt: deepen(prefix, 'the THIRD major branch') },
    { label: 'Step 5: Deepen the fourth and fifth branches', prompt: deepen(prefix, 'the FOURTH and FIFTH major branches') },
    { label: 'Step 6: Deepen all remaining branches', prompt: deepen(prefix, 'ALL remaining major branches not yet deepened') },
    {
      label: 'Step 7: Object properties',
      prompt: `Add object properties. Call onto_load with Turtle containing 50-70 owl:ObjectProperty declarations.

EVERY property MUST have rdfs:domain, rdfs:range, rdfs:label, rdfs:comment.

Cover compositional (hasPart/isPartOf), causal (causes, prevents, inhibits), associative (dependsOn, influences), role and participation, temporal, and spatial relations. Build rdfs:subPropertyOf hierarchies 3-4 levels deep, add owl:inverseOf for every directional property, and mark owl:TransitiveProperty / owl:SymmetricProperty / owl:FunctionalProperty where they apply.

Call onto_stats after. Do NOT save yet.`,
    },
    {
      label: 'Step 8: Datatype properties',
      prompt: `Add datatype properties. Call onto_load with Turtle containing 40-60 owl:DatatypeProperty declarations.

Each with rdfs:domain, rdfs:range (xsd types), rdfs:label, rdfs:comment. Go through every major branch and cover identifiers, names, codes, dates, quantities, measurements, boolean flags, statuses and text fields.

Call onto_stats after. Do NOT save yet.`,
    },
    {
      label: 'Step 9: Disjointness axioms',
      prompt: `Add owl:disjointWith between ALL sibling classes that cannot overlap. Target 60+ disjoint pairs minimum.

Call onto_stats after. Do NOT save yet.`,
    },
    {
      label: 'Step 10: Named individuals',
      prompt: `Add 25-40 owl:NamedIndividual instances spread across ALL major branches.

Each individual needs rdf:type (most specific class), rdfs:label, rdfs:comment, and 3-5 property values.

Call onto_stats after. Do NOT save yet.`,
    },
    {
      label: 'Step 11: Reason and save',
      prompt: `Final step. Run onto_reason with profile "rdfs", then onto_stats, then onto_save with path "~/.open-ontologies/studio-live.ttl". Report the final statistics.`,
    },
  ];
}

/** Quick sketch: a usable ontology in five steps. */
export function sketchWorkflow(domain: string): WorkflowStep[] {
  const { prefix } = namespaceFor(domain);

  return [
    {
      label: 'Step 1/5: Foundation, root + 4 levels deep',
      prompt: `Build an ontology about "${domain}". Use namespace ${prefix}

Call onto_clear. Then call onto_load with ONE Turtle block containing:
- An owl:Ontology declaration
- A root class for the domain
- 5-8 major branch classes under the root (Level 1)
- For each branch, 3-4 subclasses (Level 2)
- For each of those, 2-3 further subclasses (Level 3)
- For at least half of Level 3, add 2 more subclasses (Level 4)

Prioritise depth over width. Every class MUST have rdfs:label and rdfs:comment.

Call onto_stats after. Do NOT save yet.`,
    },
    {
      label: 'Step 2/5: Deepen and add properties',
      prompt: `Call onto_query to find leaf classes:
SELECT ?leaf ?label WHERE { ?leaf a owl:Class . FILTER NOT EXISTS { ?child rdfs:subClassOf ?leaf } . OPTIONAL { ?leaf rdfs:label ?label } } LIMIT 30

Then call onto_load with Turtle using namespace ${prefix} adding:
- For each leaf that can be subdivided, 2-3 subclasses, and for each of those 1-2 more
- 15-25 owl:ObjectProperty each with rdfs:domain, rdfs:range, rdfs:label, rdfs:comment
- owl:inverseOf pairs for directional properties
- 8-12 owl:DatatypeProperty with rdfs:domain, rdfs:range (xsd types), rdfs:label

Call onto_stats after. Do NOT save yet.`,
    },
    {
      label: 'Step 3/5: Axioms and individuals',
      prompt: `Call onto_load with Turtle using namespace ${prefix} adding:
- owl:disjointWith between sibling classes that cannot overlap (15+ pairs)
- 12-20 owl:NamedIndividual across different branches, each with rdf:type, rdfs:label, rdfs:comment and 2-4 property values

Call onto_stats after. Do NOT save yet.`,
    },
    {
      label: 'Step 4/5: Verify and fix gaps',
      prompt: `Call onto_stats to check counts, and onto_lint to find missing labels, comments, domains or ranges.

Fix anything lint reports by calling onto_load with corrective Turtle. If any branch is shallower than 4 levels, add 2-3 more levels below its leaves.

Call onto_stats after. Do NOT save yet.`,
    },
    {
      label: 'Step 5/5: Reason and save',
      prompt: `Run onto_reason (profile "rdfs"), then onto_stats, then onto_save ("~/.open-ontologies/studio-live.ttl"). Report final class, property, individual and triple counts.`,
    },
  ];
}

const BUILD_VERBS = ['build', 'create', 'make', 'generate'];

export function isBuildRequest(msg: string): boolean {
  const lower = msg.toLowerCase();
  return BUILD_VERBS.some(v => lower.includes(v)) && (lower.includes('ontology') || lower.includes('about'));
}

export function isSketchRequest(msg: string): boolean {
  return msg.toLowerCase().includes('sketch');
}

export function extractDomain(msg: string): string {
  const patterns = [
    /(?:about|for|on|of)\s+(.+)/i,
    /(?:build|create|make|generate|sketch)\s+(?:a\s+|an\s+|the\s+)?(?:\w+\s+)?(?:ontology\s+)?(?:about|for|on|of)\s+(.+)/i,
  ];
  for (const p of patterns) {
    const m = msg.match(p);
    const match = m?.[2] || m?.[1];
    if (match) return match.trim().replace(/[.!?]+$/, '');
  }
  return msg.replace(/^(build|create|make|generate|sketch)\s+(an?\s+)?ontology\s*/i, '').trim() || msg;
}
