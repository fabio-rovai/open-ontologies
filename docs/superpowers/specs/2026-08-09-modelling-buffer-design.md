# Modelling Buffer: keeping proprietary data out of LLM-authored ontologies

Status: draft for review
Date: 2026-08-09

## The question this answers

> "You'll need to consider security. If the ontology is created by LLMs then how
> do you address the question of LLM 'seeing' sensitive/proprietary data?"

This is the obvious objection to AI-native ontology engineering, and it is a
**build-time** question. It is not about controlling what an agent may query at
runtime.

## Non-goals

Runtime access control (RBAC, ABAC, query-time DLP, information-flow labels) is
a separate problem with a separate design. Nothing here depends on it, and it
does not depend on this.

## Principle

**Ontology construction needs the shape of a domain, not its contents.**

`Candidate hasTarget Protein` can be modelled from a data dictionary, a database
schema, or a conversation with a domain expert. It does not require knowing
which candidates or which proteins exist.

The buffer enforces that principle mechanically instead of trusting an operator
to remember it.

## Why a buffer is tractable here

Three properties hold at build time that do not hold at query time:

1. **It is offline and batch.** Ontology construction is not latency-sensitive,
   so a human can inspect the payload before it leaves. A control that depends
   on a classifier being perfect degrades the first time someone finds an edge
   case. A control with an approval checkpoint does not.
2. **Instance data is dispensable.** At query time an agent needs values to be
   useful. Here it does not, because a good ontology models the domain rather
   than its individuals.
3. **The output is verifiable.** `onto_validate`, `onto_vocab_check`,
   `onto_shacl`, the reasoner, and `onto_verify_cq` check the returned ontology
   mechanically. Correctness does not rest on trusting how it was produced.

Point 3 carries more weight than it first appears. See *Verification* below.

## Four dispositions

Every term reaching the boundary passes, is stripped, is tokenised, or is
surrogated. Getting this split right is the substance of the design.

### Pass

Generic domain vocabulary leaves unchanged. "Candidate", "Protein", "Phase III"
and "Assay" are industry-standard terms and nobody's intellectual property.
Substituting them would destroy the model's ability to name classes meaningfully
while protecting nothing.

Pass applies only to terms on an explicit allowlist. It is never a default.

### Strip

Instance data. Rows, literal values, individuals. These are never required to
build a TBox and never leave.

`onto_import_schema` already works this way: `SchemaIntrospector` operates over
`TableInfo`/`ColumnInfo`, carrying table name, column name, declared type,
nullability and primary-key flag. No rows. This is the default path and it is
already shipping.

### Tokenise

An opaque, meaningless replacement: `ENT_7f3a`.

Use when the model needs **identity and equality only**, not meaning:

- unique identifiers, keys, accession and record numbers
- internal codenames (`project_cardinal`)
- any term whose mere name discloses intent

Properties: no semantic leakage and no contamination risk, because the token
asserts nothing. Cost: the model treats it as an opaque symbol and cannot bring
domain knowledge to bear on it.

Tokens are deterministic within a session so the model can join and group, and
salted per session so surrogates do not accumulate across sessions.

### Surrogate

A plausible same-class substitute: a proprietary target protein replaced by a
well-known public one; a codenamed compound replaced by a generic exemplar.

Use when the model's **domain knowledge must fire** for the modelling to be any
good. An LLM shown `ENT_7f3a` can only produce `ClassA subClassOf ClassB`. An
LLM shown a recognisable protein produces a correct hierarchy, sensible property
domains and ranges, and appropriate disjointness axioms.

### Choosing

```
Is it an instance?                           -> strip
Is it on the generic-vocabulary allowlist?   -> pass
Does the model need only identity/equality?  -> tokenise
Does the model need domain knowledge?        -> surrogate
Anything else                                -> strip
```

The classifier proposes a disposition per term. The human confirms it at the
review gate. Unclassified defaults to **strip**.

One fail-safe matters: if a term is classified `Surrogate` but no substitute is
available for its class, it falls back to `Tokenise`, never to `Pass`. Every
fallback in the system moves toward disclosing less.

## The contamination hazard, and the rule that contains it

Tokens are honestly opaque. **Surrogates lie plausibly**, and that is a real
hazard with no counterpart on the token side.

If a proprietary target is surrogated to a public protein, the model will
import facts true of the *public* protein and false of yours. Those facts can
end up as axioms, and after de-substitution they are asserted about your term,
silently and with full confidence.

**Rule: a surrogate may inform structure and typing. It may never contribute
content.**

In practice:

- Permitted: `Compound-X rdf:type SmallMolecule`, `hasTarget rdfs:range Protein`,
  placement in a class hierarchy, disjointness between siblings.
- Rejected: any axiom whose truth depends on the surrogate's specific identity,
  for example a stated binding affinity, a named pathway, a measured property.

Enforcement is a post-processing pass over the returned ontology: axioms that
mention a surrogate in a content-bearing position are dropped and reported
rather than silently de-substituted. `onto_vocab_check` catches the adjacent
failure of invented terms. This pass is not optional; without it surrogates are
a correctness bug dressed as a security control.

## The review gate

The buffered payload is rendered for human inspection before egress. Terms are
shown with their disposition and, for surrogates, the substitution.

This is the load-bearing component. "Our classifier catches sensitive terms"
invites an argument about recall that you will lose. "Nothing leaves without a
human seeing exactly what leaves, and here is the log" ends the conversation.
It is only affordable because ontology construction is infrequent.

## De-substitution

The returned ontology is mapped back to real vocabulary from the local vault.
The delivered artefact uses your terms. The vault never crosses the boundary.

## Verification, and why it licenses a local model

The usual reason to send sensitive material to a frontier model is that you are
trusting its judgment, so you want the best judgment available.

Here the output is checked mechanically: syntax by `onto_validate`, hallucinated
terms by `onto_vocab_check`, constraints by `onto_shacl`, consistency by the
reasoner, and fitness for purpose by `onto_verify_cq`.

When output is verified rather than trusted, a **local** model becomes
sufficient for the steps that genuinely need real data, because correctness is
established after the fact rather than assumed. This converts the objection into
a differentiator: confidentiality is not traded against correctness.

## Residual risks

Stated plainly, because a reviewer will find them.

1. **Intent leakage through selection.** Asking for an ontology of PD-1
   inhibitor resistance discloses the research direction regardless of how the
   payload is scrubbed. The leak is in *what was asked*, not what was sent. No
   content filter addresses this. Mitigation: route such requests to the local
   model.
2. **Novel proprietary concepts.** A genuinely unique mechanism has no generic
   equivalent to substitute. Surrogating it removes the thing that needed
   modelling. Mitigation: local model.
3. **Schema metadata is not neutral.** A table named `project_cardinal_phase3`
   discloses a programme. Schema-first shrinks exposure to metadata; it does not
   eliminate it. Mitigation: tokenise identifiers at the schema level.
4. **Surrogate class disclosure.** Choosing a surrogate reveals the class of the
   thing it replaces. Usually acceptable, occasionally not.
5. **Equality and cardinality.** Deterministic substitution preserves which
   terms are the same. This is inherent, not an implementation gap. See Naveed,
   Kamara and Wright, CCS 2015, on recovering plaintext from
   deterministically-encrypted data using auxiliary distributions.

Do not describe the system as solving this "forever" or completely. The accurate
claim is: instances never leave, identifiers leave only as substitutes, a human
approves every payload, and what a buffer provably cannot protect is routed to a
local model.

## Phase 1 scope

- Term extraction producing a buffered payload from a source (schema first)
- Disposition classifier with strip / tokenise / surrogate and a strip default
- Vault in the existing SQLite state DB, session-salted
- Review gate rendering the payload with dispositions before egress
- De-substitution on return
- Contamination pass rejecting content-bearing axioms about surrogates
- Lineage records for every disposition and every egress

Not in phase 1: automatic sensitivity inference from ontology annotations,
local-model routing as an automated decision (manual to begin with), runtime
access control of any kind.

## Testing

**Canary suite.** Source material whose sensitive values are unique generated
tokens. Assert no canary appears in any buffered payload, including on error
paths. Fail the build otherwise. This is the same technique as
`scripts/check-test-collection.sh`: convert a property you want to claim into a
gate that breaks when it stops holding.

**Contamination suite.** Feed a returned ontology containing axioms that depend
on surrogate identity. Assert they are rejected and reported, not
de-substituted.

**Disposition suite.** Table-driven over term kinds, asserting the default is
strip for anything unrecognised.

## Open questions

1. Where does the disposition classifier run? It must see real terms, so it must
   be local. A rules-plus-dictionary approach is probably sufficient and is
   auditable, which a model-based classifier is not.
2. Should surrogate selection be deterministic across sessions? Stability aids
   review and comparison; it also accumulates a corpus at the provider.
3. Does `onto_map` sample data values, or read field names only? Not yet
   audited, and it is on the build-time path.
