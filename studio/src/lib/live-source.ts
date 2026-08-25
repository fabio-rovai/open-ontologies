import type { Chunk, Claim, Contradiction, Decision, DemoSource, Document, GraphView } from './demo-source'
import { sparqlQuery, callTool } from './mcp-client'

export interface LiveDeps {
  sparqlQuery: typeof sparqlQuery
  callTool: typeof callTool
}

const PREFIXES = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>`

// The same class and subclass queries Graph3D.tsx issues (studio/src/components/Graph3D.tsx),
// moved here so the component reads through DemoSource instead of talking to the engine directly.
const CLASSES = `${PREFIXES}
SELECT ?c ?l WHERE {
  { ?c a owl:Class } UNION { ?c rdfs:subClassOf ?x }
  OPTIONAL { ?c rdfs:label ?l }
  FILTER(!isBlank(?c))
} LIMIT 300`

const SUBCLASSES = `${PREFIXES}
SELECT ?a ?b WHERE {
  ?a rdfs:subClassOf ?b .
  FILTER(!isBlank(?a) && !isBlank(?b))
}`

// The provenance convention already used elsewhere in this codebase
// (Graph3D.tsx's document-and-entity projection, demo/contradiction_scan.py's
// PROVENANCE query): a fact extracted from a source document carries
// prov:wasDerivedFrom pointing at a resource labelled with that document's name.
const PROVENANCE = `PREFIX prov: <http://www.w3.org/ns/prov#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?thing ?doc WHERE {
  ?thing prov:wasDerivedFrom ?source .
  ?source rdfs:label ?doc .
}`

// The corpus this demonstration validates: DCAT-US's own recovered SHACL
// shapes, the exact file demo/dcat_conformance.py validates against
// (demo/corpus/dcat-us/recovered-shapes.ttl). Passed as a path rather than
// inline content: the live source runs in the webview and has no filesystem
// access of its own, only the engine process does.
const SHAPES_PATH = 'demo/corpus/dcat-us/recovered-shapes.ttl'

function rows(raw: string): Record<string, string>[] {
  const parsed = JSON.parse(raw)
  const bindings = parsed?.results?.bindings ?? []
  return bindings.map((b: Record<string, { value: string }>) =>
    Object.fromEntries(Object.entries(b).map(([k, v]) => [k, v.value])),
  )
}

interface ShaclViolation {
  severity: string
  focus_node: string
  path?: string
  constraint: string
  message: string
}

interface ShaclReport {
  error?: string
  conforms?: boolean | null
  violation_count?: number
  violations?: ShaclViolation[]
  focus_nodes?: number
  warning?: string
}

export function createLiveSource(deps: LiveDeps): DemoSource {
  return {
    // No MCP tool named corpus_documents exists (see src/server.rs's tool
    // list), and none of the Tauri commands the desktop shell exposes for
    // corpus work (corpus_presets, ingest_corpus, read_store, list_graphs,
    // read_decisions, revert_type, list_saved, pick_ontology_file, in
    // studio/src-tauri/src/corpus.rs) return document text either:
    // read_store returns the merged ontology store, list_graphs returns
    // per-document vocabulary file PATHS, and neither is the Document{id,
    // title, text} shape this method promises. Returning an empty array or
    // documents with a blank text field would look like "a corpus with no
    // content" rather than "no tool reaches the content", so this throws
    // instead of manufacturing that.
    async corpus(): Promise<Document[]> {
      throw new Error(
        'The live engine exposes no tool or Tauri command that returns document text ' +
          '(no "corpus_documents" MCP tool exists, and none of corpus.rs\'s commands ' +
          'read individual document bodies). The corpus panel can only be populated ' +
          'from the precomputed replay bundle today.',
      )
    },

    async graph(): Promise<GraphView> {
      const [classRows, subRows] = await Promise.all([
        deps.sparqlQuery(CLASSES).then(rows),
        deps.sparqlQuery(SUBCLASSES).then(rows),
      ])
      return {
        classes: classRows.map((r) => (r.l ? { iri: r.c, label: r.l } : { iri: r.c })),
        properties: [],
        edges: subRows.map((r) => ({ source: r.a, target: r.b })),
      }
    },

    // Findings no longer come from a document-contradiction scan: no
    // onto_contradiction_scan tool exists on the engine (see the PIVOT section
    // of docs/superpowers/plans/2026-08-24-studio-public-port.md). The
    // demonstration is the validator finding instead: corpus documents checked
    // against SHACL shapes, the same measurement demo/dcat_conformance.py runs
    // offline. This calls the engine's real onto_shacl tool and joins each
    // violation's focus node against the corpus's own provenance triples to
    // name the document that asserted the offending fact.
    //
    // Two failure shapes are both surfaced, never swallowed: a thrown engine
    // error (network down, session lost) propagates as-is, and onto_shacl's
    // own {"error": ...} response (a missing or unreadable shapes file) is
    // turned into a thrown error too. Neither collapses into an empty array,
    // because an empty findings list must mean "validated clean", never
    // "the engine could not be asked".
    async findings(): Promise<Contradiction[]> {
      const raw = await deps.callTool('onto_shacl', { shapes: SHAPES_PATH, inline: false })
      const report = JSON.parse(raw) as ShaclReport
      if (report.error) {
        throw new Error(`SHACL validation failed: ${report.error}`)
      }
      // The engine deliberately reports `conforms: null` (src/shacl.rs, the
      // block just above `report["conforms"] = serde_json::Value::Null`) when
      // the shapes graph selects zero focus nodes: reporting `conforms: true`
      // over nothing checked would be exactly the lie this demonstration
      // exists to expose, and this is precisely the profile-as-published
      // case (0 focus nodes). Returning [] here would render as "no
      // findings" in the panel -- indistinguishable from a genuine clean
      // pass -- so an undetermined result throws instead of going quiet.
      if (report.conforms === null || report.conforms === undefined) {
        throw new Error(
          `SHACL validation is undetermined, not clean: conforms is null with ` +
            `${report.focus_nodes ?? 0} focus node(s). ${report.warning ?? 'The shapes graph selected nothing in the data.'}`,
        )
      }
      const violations = report.violations ?? []

      const provenance = rows(await deps.sparqlQuery(PROVENANCE))
      const docsByThing = new Map<string, string[]>()
      for (const r of provenance) {
        if (!r.thing || !r.doc) continue
        const list = docsByThing.get(r.thing) ?? []
        list.push(r.doc)
        docsByThing.set(r.thing, list)
      }

      return violations.map((v, i) => {
        // If the store carries no provenance for this focus node, cite the
        // node itself rather than a name invented for the occasion.
        const documents = docsByThing.get(v.focus_node) ?? [v.focus_node]
        const claims: Claim[] = documents.map((document) => ({
          document,
          predicate: v.constraint,
          object: v.message,
        }))
        return {
          id: `shacl:${v.focus_node}|${v.path ?? ''}|${v.constraint}|${i}`,
          subject: v.focus_node,
          kind: 'conformance' as const,
          claims,
        }
      })
    },

    // onto_apply (src/server.rs) applies a PLAN produced by onto_plan: its
    // only inputs are plan_id and mode ("safe" | "force" | "migrate"). It has
    // no "finding"/"decision" parameters, and nothing else on the engine or
    // in corpus.rs's Tauri commands accepts a SHACL finding id (the shape
    // findings() now returns, "shacl:<focus_node>|<path>|<constraint>|<i>")
    // together with an accept/reject verdict. corpus.rs's revert_type comes
    // closest, but it reverts a typing decision keyed by (doc, subject,
    // from, to) from the old contradiction-scan demonstration, not a
    // conformance finding keyed by id. There is no honest call to make here.
    async resolve(id: string, decision: Decision): Promise<void> {
      throw new Error(
        `No engine tool or Tauri command resolves a conformance finding by id. onto_apply ` +
          `applies a plan (plan_id, mode), not a finding decision, and corpus.rs's ` +
          `revert_type reverts a typing correction keyed by document/subject, not a finding ` +
          `id. Cannot resolve "${id}" with decision "${decision.kind}".`,
      )
    },

    // No "agent_ask" MCP tool exists. Chat is not an MCP call at all: it runs
    // through the agent sidecar process, driven by the Tauri commands
    // send_chat_message / reset_chat (studio/src-tauri/src/chat.rs) and
    // consumed via the "agent-message" event stream in
    // studio/src/hooks/useChat.ts, which ChatPanel already wires up directly
    // and outside of DemoSource. LiveDeps only carries the MCP surface
    // (sparqlQuery, callTool), which has no path to that sidecar, so there is
    // no honest way to answer a question from here.
    async *ask(_question: string): AsyncIterable<Chunk> {
      throw new Error(
        'Chat does not run through DemoSource in live mode. There is no "agent_ask" MCP ' +
          'tool; real chat goes through the agent sidecar (send_chat_message / ' +
          '"agent-message" events, see chat.rs and useChat.ts) and the desktop app talks ' +
          'to it directly via the useChat hook and ChatPanel, not through this method.',
      )
    },
  }
}

export const liveSource = () => createLiveSource({ sparqlQuery, callTool })
