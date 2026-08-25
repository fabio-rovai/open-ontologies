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
  warning?: string
}

export function createLiveSource(deps: LiveDeps): DemoSource {
  return {
    async corpus(): Promise<Document[]> {
      return JSON.parse(await deps.callTool('corpus_documents'))
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

    async resolve(id: string, decision: Decision): Promise<void> {
      await deps.callTool('onto_apply', { finding: id, decision: decision.kind })
      await deps.callTool('onto_save')
    },

    async *ask(question: string): AsyncIterable<Chunk> {
      const answer = await deps.callTool('agent_ask', { question })
      yield { type: 'text', value: answer }
    },
  }
}

export const liveSource = () => createLiveSource({ sparqlQuery, callTool })
