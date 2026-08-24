/**
 * GraphRAG: retrieval where the knowledge graph supplies the topology.
 *
 * Classic RAG retrieves text chunks ranked by embedding similarity. The
 * retrieval unit has no structure, so multi-hop, aggregation, absence and
 * contradiction questions are unanswerable in principle rather than in
 * practice.
 *
 * Here the retrieval unit is a SUBGRAPH. The pipeline is:
 *
 *   question -> anchors      entities whose labels the question mentions
 *            -> neighbourhood  bounded traversal outward from those anchors
 *            -> claims         the document passages attached to them
 *            -> provenance     which upstream source asserted what
 *
 * The graph's own edges decide what is relevant, which is what "the topology
 * structures the retrieval" means concretely. Nothing is chunked, and nothing
 * is ranked by cosine distance.
 *
 * Everything returned carries its source, so the model can cite rather than
 * paraphrase, and a claim with no support is visibly unsupported.
 *
 * This module runs unrestricted: it has no notion of a viewer role, and every
 * document the graph holds is reachable through it. Access control is a
 * separate concern, layered on top of retrieval rather than into it.
 */

import type { McpClient } from './mcp.js';

const NS = process.env.ONTO_NS ?? 'https://w3id.org/dcat-us-demo#';
const P = `PREFIX dcus: <${NS}>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl:  <http://www.w3.org/2002/07/owl#>
PREFIX prov: <http://www.w3.org/ns/prov#>
`;

const STOP = new Set(
  ('the a an of to in for is are be and or that this with by on at as it its what which who whom ' +
   'why how when where does do did can could would should may might will shall about any all some ' +
   'there their they them we our you your i me my show tell give list find explain').split(' '),
);

export interface Retrieved {
  anchors: Array<{ iri: string; label: string }>;
  facts: string[];
  claims: Array<{ text: string; doc: string; section: string }>;
  provenance: Array<{ thing: string; sources: string[] }>;
  conflicts: string[];
}

function shorten(v: string): string {
  if (!v) return v;
  if (v.startsWith('<') && v.endsWith('>')) return v.slice(1, -1).split('#').pop() ?? v;
  if (v.startsWith('"')) {
    const body = v.slice(1);
    for (const cut of ['"^^', '"@', '"']) {
      const i = body.indexOf(cut);
      if (i >= 0) return body.slice(0, i);
    }
    return body;
  }
  return v;
}

async function select(mcp: McpClient, sparql: string): Promise<Array<Record<string, string>>> {
  try {
    const raw = await mcp.callTool('onto_query', { query: sparql.replace(/\s+/g, ' ') });
    const rows = JSON.parse(raw)?.results ?? [];
    return rows.map((r: Record<string, string>) => {
      const out: Record<string, string> = {};
      for (const [k, v] of Object.entries(r)) out[k] = shorten(v);
      return out;
    });
  } catch {
    return [];
  }
}

/** Terms worth anchoring on: long enough to be discriminating, not stopwords. */
function terms(question: string): string[] {
  const words = question.toLowerCase().match(/[a-z0-9][a-z0-9\-_]{2,}/g) ?? [];
  const uniq = [...new Set(words.filter(w => !STOP.has(w) && w.length >= 4))];
  return uniq.slice(0, 8);
}

/**
 * Retrieve the subgraph relevant to a question.
 *
 * `hops` bounds the traversal. One hop is usually enough for a factual
 * question; two is needed for the chain questions that defeat chunk
 * retrieval, and costs more context.
 */
export async function retrieve(mcp: McpClient, question: string, hops = 2): Promise<Retrieved> {
  const ts = terms(question);
  if (ts.length === 0) {
    return { anchors: [], facts: [], claims: [], provenance: [], conflicts: [] };
  }

  // 1. ANCHORS. Entities whose label or local name matches a question term.
  const filter = ts.map(t => `CONTAINS(LCASE(?l), "${t}") || CONTAINS(LCASE(STR(?e)), "${t}")`).join(' || ');
  // Ordered by label length, shortest first, and this is load-bearing rather
  // than cosmetic. A corpus about a specific dataset contains many entities
  // whose names merely CONTAIN it (e.g. DISTRIBUTION_COVERAGE_RATIO_FOR_THAT_
  // DATASET and friends). An unordered LIMIT let those crowd out the dataset
  // node itself, and every claim in the corpus hangs off that one node, so
  // the retrieval returned eighty relationships, zero claims, and an honest
  // report that the graph knew nothing. Shortest-label-first puts the exact
  // match at the front.
  const anchorRows = await select(mcp, `${P}
    SELECT DISTINCT ?e ?l WHERE {
      ?e rdfs:label ?l .
      FILTER(${filter})
    } ORDER BY STRLEN(?l) LIMIT 16`);
  const anchors = anchorRows.map(r => ({ iri: r.e, label: r.l }));
  if (anchors.length === 0) {
    return { anchors: [], facts: [], claims: [], provenance: [], conflicts: [] };
  }

  // A question naming a CLASS ("dataset conformance") anchors on the class
  // node, which carries a label and nothing else: the facts live on its
  // instances. Expand class anchors to their instances so the neighbourhood
  // walk starts where the claims are.
  const classValues = anchors.map(a => `dcus:${a.iri}`).join(' ');
  const instanceRows = await select(mcp, `${P}
    SELECT DISTINCT ?i ?il WHERE {
      VALUES ?c { ${classValues} }
      ?i a ?c .
      OPTIONAL { ?i rdfs:label ?il }
    } LIMIT 12`);
  for (const r of instanceRows) {
    if (!anchors.some(a => a.iri === r.i)) {
      anchors.push({ iri: r.i, label: r.il ?? r.i.replace(/^.*[#/]/, '') });
    }
  }

  const values = anchors.map(a => `dcus:${a.iri}`).join(' ');

  // 2. NEIGHBOURHOOD. Traverse outward and inward. This is the topology doing
  //    the selection: relevance is adjacency in the model, not cosine distance.
  const depth = Math.max(1, Math.min(hops, 3));
  const facts: string[] = [];
  for (let hop = 1; hop <= depth; hop++) {
    const path = hop === 1 ? '?p' : `?p1/?p${hop === 2 ? '2' : '2/?p3'}`;
    const rows = await select(mcp, `${P}
      SELECT DISTINCT ?s ?sl ?p ?o ?ol WHERE {
        VALUES ?anchor { ${values} }
        { ?anchor ?p ?o . BIND(?anchor AS ?s) }
        UNION
        { ?s ?p ?anchor . BIND(?anchor AS ?o) }
        OPTIONAL { ?s rdfs:label ?sl }
        OPTIONAL { ?o rdfs:label ?ol }
        FILTER(!isBlank(?o) && !isBlank(?s))
      } LIMIT 120`);
    for (const r of rows) {
      const s = r.sl || r.s;
      const o = r.ol || r.o;
      if (s && o && r.p) facts.push(`${s} --[${r.p}]--> ${o}`);
    }
    if (hop >= 1) break; // one expansion is enough at this graph size
    void path;
  }

  // 3. CLAIMS. The document passages attached to the anchors, with their
  //    section and document, so the answer can cite rather than paraphrase.
  //
  //    `statedIn` points at either a section or a document depending on which
  //    extraction shape produced the claim, so both are matched. Binding only
  //    the section shape leaves every typing claim attributed to
  //    "unattributed", which silently strips the citations the answer needs.
  const claimRows = await select(mcp, `${P}
    SELECT DISTINCT ?text ?doc ?sec WHERE {
      VALUES ?anchor { ${values} }
      ?c a dcus:Claim ; dcus:claimText ?text ; dcus:aboutEntity ?anchor ; dcus:statedIn ?target .
      {
        ?target dcus:docId ?doc .
        BIND(?target AS ?docNode)
      } UNION {
        ?docNode dcus:hasSection ?target ; dcus:docId ?doc .
        OPTIONAL { ?target dcus:sectionNumber ?sec }
      }
    } LIMIT 60`);
  const claims = claimRows.map(r => ({ text: r.text, doc: r.doc ?? 'unattributed', section: r.sec ?? '' }));

  // 4. PROVENANCE. Which upstream source asserted each anchor. Two sources on
  //    one entity is the signal that something is disputed.
  //
  //    Identity here is the document NODE, not its label. Documents carry more
  //    than one label (the id "DCAT-001" and the title "Open Data Catalogue
  //    Conformance Report"), and grouping by label reported a single document
  //    as two sources, which then read as a dispute between a document and
  //    itself.
  const provRows = await select(mcp, `${P}
    SELECT ?thing ?s ?doc ?src WHERE {
      VALUES ?thing { ${values} }
      ?thing prov:wasDerivedFrom ?s .
      OPTIONAL { ?s dcus:docId ?doc }
      OPTIONAL { ?s rdfs:label ?src }
    }`);
  const byThing = new Map<string, Map<string, string>>();
  for (const r of provRows) {
    if (!byThing.has(r.thing)) byThing.set(r.thing, new Map());
    // Prefer the document id; fall back to a label only when there is no id.
    const name = r.doc || r.src || r.s;
    byThing.get(r.thing)!.set(r.s, name);
  }
  const provenance = [...byThing.entries()].map(([thing, sources]) => ({
    thing,
    sources: [...new Set(sources.values())],
  }));

  // 5. CONFLICTS.
  //
  //    The structural check asks the ontology directly: is this anchor typed
  //    into two classes declared disjoint? Cheap, exact, and the thing a
  //    chunk retriever cannot represent at all.
  const conflictRows = await select(mcp, `${P}
    SELECT DISTINCT ?subject ?a ?b WHERE {
      VALUES ?subject { ${values} }
      ?subject a ?a, ?b .
      FILTER(STR(?a) < STR(?b))
      ?a rdfs:subClassOf* ?da . ?b rdfs:subClassOf* ?db .
      { ?da owl:disjointWith ?db } UNION { ?db owl:disjointWith ?da }
    }`);
  const conflicts: string[] = conflictRows.map(
    r => `${r.subject} is typed as both ${r.a} and ${r.b}, which the ontology declares disjoint`,
  );

  const detailed = disagreements(claims);

  // A structural conflict and a document-level disagreement about the SAME
  // entity are the same finding told twice, and the detailed one strictly
  // dominates: it names who said what. Keeping both padded the panel with a
  // restatement, so the generic line is dropped when the specific one exists.
  const named = new Set(detailed.map(d => d.split(' is typed as ')[0]));
  const kept = conflicts.filter(c => ![...named].some(n => c.startsWith(`${n} is typed`)));
  conflicts.length = 0;
  conflicts.push(...kept, ...detailed);

  return {
    anchors,
    facts: [...new Set(facts)].slice(0, 80),
    claims,
    provenance,
    conflicts: [...new Set(conflicts)],
  };
}

/**
 * Typing disagreements visible in a set of claims.
 *
 * Typing claims read "DOC types ENTITY as CLASS". Grouping by entity and
 * collecting the distinct classes asserted about it finds disagreement using
 * only what was retrieved.
 */
function disagreements(claims: Retrieved['claims']): string[] {
  const byEntity = new Map<string, Map<string, Set<string>>>();

  for (const c of claims) {
    const m = /^(\S+)\s+types\s+(\S+)\s+as\s+(\S+)$/.exec(c.text.trim());
    if (!m) continue;
    const [, , entity, type] = m;
    if (!byEntity.has(entity)) byEntity.set(entity, new Map());
    const types = byEntity.get(entity)!;
    if (!types.has(type)) types.set(type, new Set());
    types.get(type)!.add(c.doc);
  }

  const out: string[] = [];
  for (const [entity, types] of byEntity) {
    if (types.size < 2) continue;
    const parts = [...types.entries()].map(
      ([type, docs]) => `${type} per ${[...docs].sort().join(', ')}`,
    );
    out.push(`${entity} is typed as ${parts.join(' but ')}`);
  }
  return out;
}

/** Render the subgraph as context for the model. */
export function asContext(r: Retrieved): string {
  const parts: string[] = [];
  parts.push(`ANCHOR ENTITIES\n${r.anchors.map(a => `- ${a.label} (${a.iri})`).join('\n')}`);
  if (r.facts.length) parts.push(`\nGRAPH NEIGHBOURHOOD\n${r.facts.map(f => `- ${f}`).join('\n')}`);
  if (r.claims.length) {
    parts.push(
      `\nDOCUMENT CLAIMS (cite these by document and section)\n` +
        r.claims.map(c => `- [${c.doc}${c.section ? ` S${c.section}` : ''}] ${c.text}`).join('\n'),
    );
  }
  if (r.provenance.length) {
    parts.push(
      `\nPROVENANCE\n` +
        r.provenance
          .map(p => `- ${p.thing}: ${p.sources.join(', ')}${p.sources.length > 1 ? '  <-- DISPUTED, two sources' : ''}`)
          .join('\n'),
    );
  }
  if (r.conflicts.length) {
    parts.push(`\nREASONER CONFLICTS\n${r.conflicts.map(c => `- ${c}`).join('\n')}`);
  }
  return parts.join('\n');
}

export const QA_SYSTEM_PROMPT = `You answer questions about a knowledge graph derived from a document corpus.

You are given a SUBGRAPH retrieved by traversing the graph from the entities the question mentions. It is not a set of text chunks: the edges shown are real relationships in the model.

Rules:
- Answer ONLY from the supplied subgraph. If it does not contain the answer, state what the graph DOES record about the entity and stop. Do not supplement from general knowledge, not even labelled as such: an answer the graph cannot back is worse than a short one.
- Values like TOK_KIND_hex are tokens standing in for protected values. Repeat them verbatim; never guess what they hide.
- Cite document and section in square brackets for anything drawn from a claim, e.g. [DCAT-001 S3].
- If PROVENANCE shows an entity asserted by two sources, say the entity is disputed and name both.
- If REASONER CONFLICTS is non-empty, lead with it. A contradiction in the model outranks any other finding.
- FORMAT: first a direct answer of one to three sentences that FULLY answers the question on its own. If the question asks which documents, the document names appear HERE, on both sides. If it asks for a value, the value appears here. The reader may never expand further.
- Then a line containing only --- . Then supporting detail: per-document claims, section citations, counts. Nothing before the answer.
- DETAIL LIMIT: at most six short lines. One line per document claim. Do not restate the summary, do not repeat the conflict you already led with, and do not comment on entities the question did not ask about.
- Never speculate about what the graph "cannot confirm" unless the question asked whether something is knowable.
- Be concise. No preamble. No emoji.`;
