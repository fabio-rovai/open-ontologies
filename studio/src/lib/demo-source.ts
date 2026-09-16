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
  edges: { source: string; target: string; warrant?: Warrant }[]
}
/**
 * Where an edge came from, and what is known about it.
 *
 * `asserted`  someone wrote it down. Nothing is claimed beyond that.
 * `certified` derived, AND the Lean checker accepted the derivation.
 * `rejected`  a derivation the checker REFUSED. Exit 1, with the rule named.
 *
 * There is deliberately no `unproved`. Measured on ies-core, 151 of 151
 * materialised inferences carried an accepted certificate, so a category for
 * "derived but unvouched" would have been drawn rather than observed.
 *
 * Absent means `asserted`, so a source that knows nothing about certificates
 * renders exactly as it did before.
 */
export type Warrant = 'asserted' | 'certified' | 'rejected'
export type Decision = { kind: 'accept' | 'reject'; note?: string }
export interface Chunk { type: 'text' | 'tool_call' | 'unscripted'; value: string }

export interface DemoSource {
  corpus(): Promise<Document[]>
  graph(): Promise<GraphView>
  findings(): Promise<Contradiction[]>
  resolve(id: string, decision: Decision): Promise<void>
  ask(question: string): AsyncIterable<Chunk>
}
