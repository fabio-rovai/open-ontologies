# SHIQ Reasoning

Native Rust SHIQ tableaux reasoner. No JVM is required.

The implemented logic is SHIQ: ALC with transitive roles, role hierarchies,
inverse roles and qualified number restrictions. Nominals are not implemented.
The `Concept` enum has no nominal constructor, `owl:oneOf` is never parsed and
falls through to an opaque atomic class, `owl:hasValue` is approximated as an
existential restriction whose filler is an atomic concept named after the
individual, and datatype ranges are skipped. An ontology that uses
`owl:hasValue` returns undetermined classes rather than a classification. The
measurements are in
[benchmark/reasoner/regressions/README.md](../benchmark/reasoner/regressions/README.md).

| DL Feature | Symbol | OWL Construct |
| ---------- | ------ | ------------- |
| Atomic negation | not A | complementOf |
| Conjunction | C and D | intersectionOf |
| Disjunction | C or D | unionOf |
| Existential | exists R.C | someValuesFrom |
| Universal | forall R.C | allValuesFrom |
| Min cardinality | >=n R.C | minQualifiedCardinality |
| Max cardinality | <=n R.C | maxQualifiedCardinality |
| Role hierarchy | R subprop S | subPropertyOf |
| Transitive roles | Trans(R) | TransitiveProperty |
| Inverse roles | R inverse | inverseOf |
| Symmetric roles | Sym(R) | SymmetricProperty |
| Functional | Fun(R) | FunctionalProperty |
| ABox reasoning | a:C | NamedIndividual |
| Nominals | {a} | oneOf: not implemented |
| Nominal in a restriction | exists R.{a} | hasValue: approximated as an atomic concept |
| Datatypes | d | Datatype ranges are skipped |

## Agent-Based Parallel Classification

1. **Satisfiability Agent** — Tests each class in parallel using rayon
2. **Subsumption Agent** — Pairwise subsumption tests, pruned by told-subsumer closure
3. **Explanation Agent** — Traces clash derivations for unsatisfiable classes
4. **ABox Agent** — Individual consistency and type inference

| Reasoner | Language | JVM | Parallel | Logic |
| -------- | -------- | --- | -------- | ----- |
| **Open Ontologies** | Rust | No | Yes (rayon) | SHIQ |
| HermiT | Java | Yes | No | SROIQ(D) |
| Pellet | Java | Yes | No | SROIQ(D) |

## Reasoning Profiles

| Profile | What it does |
| ------- | ------------ |
| `rdfs` | Subclass closure, domain/range inference |
| `owl-rl` | + transitive/symmetric/inverse, sameAs, equivalentClass |
| `owl-rl-ext` | + someValuesFrom, allValuesFrom, hasValue, intersectionOf, unionOf |
| `owl-dl` | SHIQ tableaux: satisfiability, classification, ABox reasoning. Nominals are not implemented and datatype ranges are skipped |

## Tools

| Tool | Purpose |
| ---- | ------- |
| `onto_reason` | Run inference with selected profile |
| `onto_dl_explain` | Explain why a class is unsatisfiable (clash trace) |
| `onto_dl_check` | Check if one class is subsumed by another |

## Which graphs a run reads

By default, all of them: the default graph and every named graph, flattened.
That is the right dataset for a store that holds one version of everything,
and the wrong one for a store that keeps several versions of an entity in
several named graphs, where the union is a state that held at no instant.

`reason` and `shacl` therefore take `--valid-at`, `--as-of` and
`--all-versions` (`valid_at`, `as_of`, `all_versions` over MCP). Over a store
that describes its named graphs with the temporal vocabulary
(`https://open-ontologies.org/temporal#`), a run that names none of them is
REFUSED rather than answered. A store that uses no temporal vocabulary is
unaffected.

A scoped run reads the in-scope named graphs plus the default graph and drops
any graph holding this engine's own materialised inferences. NO run over a
versioned store materialises, scoped or `--all-versions`: every graph it could
write to is in scope at every instant, so a conclusion written there becomes an
axiom of every snapshot. The CLI makes any run carrying a scope argument a dry
one; over MCP, pass `materialize: false`. Every report carries `scope`, naming
what was read. See [the temporal module](../src/temporal.rs) and issue #108.

## Certified inference

Every profile except `owl-dl` can emit a derivation certificate
(`reason --certificate DIR`) that the checker in `lean/` verifies against a
machine-checked soundness theorem. What is proved, what is not, and how to run
it: [lean-certificates.md](lean-certificates.md).

The certificate directory also gets `scope.tsv`, the record of which graphs
`asserted.tsv` was built from. The Lean checker does not read it and cannot:
it verifies the derivations against the triples in front of it, and cannot ask
whether those triples are the graph anyone meant.
