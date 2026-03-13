# OWL2-DL Reasoning

Native Rust SHOIQ tableaux reasoner — no JVM required.

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

## Agent-Based Parallel Classification

1. **Satisfiability Agent** — Tests each class in parallel using rayon
2. **Subsumption Agent** — Pairwise subsumption tests, pruned by told-subsumer closure
3. **Explanation Agent** — Traces clash derivations for unsatisfiable classes
4. **ABox Agent** — Individual consistency and type inference

| Reasoner | Language | JVM | Parallel | SHOIQ |
| -------- | -------- | --- | -------- | ----- |
| **Open Ontologies** | Rust | No | Yes (rayon) | Yes |
| HermiT | Java | Yes | No | Yes |
| Pellet | Java | Yes | No | Yes |

## Reasoning Profiles

| Profile | What it does |
| ------- | ------------ |
| `rdfs` | Subclass closure, domain/range inference |
| `owl-rl` | + transitive/symmetric/inverse, sameAs, equivalentClass |
| `owl-rl-ext` | + someValuesFrom, allValuesFrom, hasValue, intersectionOf, unionOf |
| `owl-dl` | Full SHOIQ tableaux: satisfiability, classification, ABox reasoning |

## Tools

| Tool | Purpose |
| ---- | ------- |
| `onto_reason` | Run inference with selected profile |
| `onto_dl_explain` | Explain why a class is unsatisfiable (clash trace) |
| `onto_dl_check` | Check if one class is subsumed by another |
