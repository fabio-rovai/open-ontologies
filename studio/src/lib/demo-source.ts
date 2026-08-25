export interface Document { id: string; title: string; text: string }
export interface Claim { document: string; predicate: string; object: string }
export interface Contradiction {
  id: string
  subject: string
  // 'provenance-split' and 'typing': two documents type the same individual
  // incompatibly. 'disjointness': a reasoner-caught axiom violation.
  // 'conformance': a claim contradicted by the artifacts published alongside
  // it, e.g. a README asserting a standard the corpus does not exhibit,
  // established by a validator run rather than the document-to-ontology
  // pipeline. Every kind still cites real documents in its claims.
  kind: 'provenance-split' | 'disjointness' | 'typing' | 'conformance'
  claims: Claim[]
}
export interface GraphView {
  classes: { iri: string; label?: string }[]
  properties: { iri: string; label?: string }[]
  edges: { source: string; target: string }[]
}
export type Decision = { kind: 'accept' | 'reject'; note?: string }
export interface Chunk { type: 'text' | 'tool_call' | 'unscripted'; value: string }

export interface DemoSource {
  corpus(): Promise<Document[]>
  graph(): Promise<GraphView>
  findings(): Promise<Contradiction[]>
  resolve(id: string, decision: Decision): Promise<void>
  ask(question: string): AsyncIterable<Chunk>
}
