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
| `rdfs` | Subclass closure, domain/range inference. 6 of the 29 rules |
| `owl-rl` | + transitive/symmetric/inverse, sameAs, equivalentClass. 17 of the 29 |
| `owl-rl-ext` | + someValuesFrom, allValuesFrom, hasValue, intersectionOf, unionOf. All 29 |
| `owl-dl` | SHIQ tableaux: satisfiability, classification, ABox reasoning. Nominals are not implemented and datatype ranges are skipped |

## Tools

| Tool | Purpose |
| ---- | ------- |
| `onto_reason` | Run inference with selected profile |
| `onto_dlp_boundary` | Which of YOUR axioms the rule table can see. Run it BEFORE trusting a result |
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

## What the rule table cannot see, and the three reasons it cannot

A certificate from `onto_reason` is a sound proof about **the axioms the rules
read**. It says nothing about the ones no rule fires on. Both halves of that
sentence are true at once, which is why a green machine-checked certificate over
an ontology whose TBox is half invisible is misleading without being wrong.

`onto_dlp_boundary` is the report that closes the gap. It is the mirror of the
discipline `onto_rules_import` already applies on the way in: that tool refuses
a non-Horn SWRL or RIF construct by name and count, so nobody gets a certificate
over a rule set they did not write. This one says which of the axioms they DID
write the rule table never looked at.

**It separates three failures that a single "not covered" bucket would destroy.**
Two DIMENSIONS produce them: whether the axiom is Horn at all, and whether this
engine runs the rule for it.

| bucket | what it means | what fixes it |
| ------ | ------------- | ------------- |
| `outside_the_fragment` | the axiom is not a Horn rule over triple patterns and no implementation makes it one. Disjunction in the consequent, existential in the head, cardinality restriction, negation in the antecedent | rewriting the ontology, or a different reasoner family |
| `inside_the_fragment_but_a_rule_is_not_implemented` | the axiom IS Horn and OWL 2 RL has a rule for it. This engine does not run that rule | a patch to `src/reason.rs`. Nothing about the ontology is wrong |
| `partially_inside_the_fragment` | part of it splits off soundly and is seen. `A ⊑ B ⊓ Out` keeps `A ⊑ B`; `A ≡ ∃r.B` keeps the direction `cls-svf1` evaluates | depends on which half you needed |

`not_fully_seen_by_the_rule_table` is the headline and carries all three counts
separately rather than adding them. They are fixed in three different places, so
one number would be the least useful summary available.

`owl:hasKey` and `owl:propertyChainAxiom` are the two that bite most often: both
are Horn, OWL 2 RL has `prp-key` and `prp-spo2` for them, and this engine has
neither. Calling those "outside the fragment" would send someone rewriting an
ontology that is already fine.

The splitting asymmetry is not decoration. A conjunction splits in a CONSEQUENT
and does not split in an ANTECEDENT, because dropping a conjunct from a rule
body makes the rule fire **more** often, which is unsound rather than merely
weaker. A disjunction splits the other way round, in an antecedent.

The line the classifier draws is OWL 2 RL's class grammar (OWL 2 Profiles
section 4.3), because OWL 2 RL is the standardised descendant of Description
Logic Programs that this rule table targets. Where the abstract DLP line and the
OWL 2 RL line differ, the difference is named rather than smoothed over:
`owl:ReflexiveProperty` is a Horn clause (`⊤(x) → r(x,x)`) that OWL 2 RL
excludes as an axiom form, and it carries `horn_but_outside_owl2_rl` saying so.

### Where the figures come from

`78`, `29`, `10`, `7` and `32` are derived, not typed. `OWL2_RL_RULES` in
`src/dlp.rs` is the profile's 78 inference rules; `reason::RULES_EVALUATED` is
the 29 the fixpoint runs and `reason::CLASH_RULES_NOT_DETECTED` is the 7 that
conclude `false` and are not looked for, and the rest falls out.

Three tests hold it together. `the_evaluated_rule_list_is_the_rules_this_file_emits`
greps `src/reason.rs` for its own `emit` calls and fails if the constant and the
code disagree. `the_seventeen_false_concluding_rules_agree_with_the_engine`
checks the rules marked as concluding `false` against the independently written
list in `src/reason.rs`'s own comment — a list that said SIXTEEN until it was
checked against the W3C source. `the_rule_table_adds_up` checks the partition.

## Certified inference

Every profile except `owl-dl` can emit a derivation certificate
(`reason --certificate DIR`) that the checker in `lean/` verifies against a
machine-checked soundness theorem. What is proved, what is not, and how to run
it: [lean-certificates.md](lean-certificates.md).

The certificate directory also gets `scope.tsv`, the record of which graphs
`asserted.tsv` was built from. The Lean checker does not read it and cannot:
it verifies the derivations against the triples in front of it, and cannot ask
whether those triples are the graph anyone meant.
