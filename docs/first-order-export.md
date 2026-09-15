# First-order export, and what an ATP verdict is worth

The engine can hand an ontology to the automated-theorem-proving ecosystem, in TPTP FOF, in
ISO/IEC 24707 CLIF, in SMT-LIB 2 and in LADR. The translation it emits is the one a machine-checked
adequacy theorem is about, so a reader of the file can cite a kernel-checked result about what it
means.

This page is the how-to. The design and its limits are in
[decision 0005](decisions/0005-a-prover-is-an-oracle-and-a-translation-is-a-theorem.md) for the
refutation direction and
[decision 0006](decisions/0006-a-model-is-a-certificate-and-a-refutation-is-not.md) for the model
direction.

**Read this first.** A prover's answer about an exported file is an ORACLE OPINION, not a
certificate. Decision 0002 lets this engine say an OWL-RL inference is *checked*, because `lean/`
holds a checker whose soundness is a theorem. A superposition refutation cannot be checked that
way: it needs a verified first-order calculus with unification, which does not exist in core Lean.
When E says `SZS status Theorem`, that carries exactly the weight of pyshacl agreeing with the
SHACL validator, and nothing here calls it a proof.

**And read what that word now comes with.** A prover will also print the DERIVATION it found, and a
derivation is a finite object. `fol-prove` reads one back, matches every leaf against the problem
this engine emitted, checks the DAG, and recomputes the resolution-family steps. It does not turn
the opinion into a proof and no word in its output says otherwise. It does answer the question the
opinion never could: whether the prover was answering about our file. See
[reading the derivation back](#reading-the-derivation-back).

**And read the asymmetry second.** A MODEL is the exact opposite of a refutation. It is a finite
object, checking a formula against it is decidable, and `lean/Fol/` holds a verified evaluator for
that, so the SATISFIABILITY direction CAN be certified. `fol --format smtlib` and
`fol --format ladr` write the files a model finder reads, and `fol-model` drives the whole loop and
hands the structure to the checker. See
[lean-certificates.md](lean-certificates.md#first-order-model-certificates-oo-folmodel).

## Export

```bash
# TPTP FOF, which is what E, Vampire and every other first-order prover read.
printf 'load ontology.ttl\nfol --out /tmp/fol --format tptp\n' \
  | open-ontologies --no-connect --data-dir /tmp/store batch -

# ISO/IEC 24707 CLIF. `iso` (default) writes cl:text; `colore` writes cl-text.
printf 'load ontology.ttl\nfol --out /tmp/fol --format clif --clif-dialect iso\n' \
  | open-ontologies --no-connect --data-dir /tmp/store batch -

# SMT-LIB 2, for Z3. Omit --smt-domain for the UNBOUNDED encoding, where `unsat`
# really is unsatisfiability; give it k for an enumeration carrier of exactly k
# elements, where a `sat` comes with a structure the verified checker can check.
printf 'load ontology.ttl\nfol --out /tmp/fol --format smtlib --smt-domain 4\n' \
  | open-ontologies --no-connect --data-dir /tmp/store batch -

# LADR, for Mace4. Every symbol is MANGLED; the table lands in symbols.tsv.
printf 'load ontology.ttl\nfol --out /tmp/fol --format ladr\n' \
  | open-ontologies --no-connect --data-dir /tmp/store batch -
```

Over MCP, `onto_fol_export` takes `out_dir`, `format`, `smt_domain`, `clif_dialect`, `goals_file`
and `goals_skip_columns`.

Every run also writes `problem.tsv`, the format `oo-folmodel` reads, with its digest in the report.
A solver result can therefore be handed to the verified checker without going back through the
engine; `onto_fol_model` / `fol-model` does all of it in one call.

**The two model-finding formats assert the NEGATED goal.** TPTP and CLIF carry a conjecture and the
consumer negates it internally. A countermodel to `Γ ⊨ φ` is a model of `Γ ∪ {¬φ}`, so the SMT-LIB
and LADR files carry `¬φ` as an assertion and nothing downstream negates again. A `sat` answer on
one of those files therefore says the conjecture is NOT entailed. The negation happens once, in
`FolProblem::checker_entries`, and `problem.tsv` is a fold over the same list, which is the
mechanical reason the solver cannot be asked a different question from the one the checker checks.

`ontology.p` (or `ontology.clif`) lands in the directory. With `--goals FILE`, where FILE is a TSV
whose first three columns are a triple, one problem per goal lands under `goals/` with a
`goals.json` manifest. `derivations.tsv` from `reason --certificate` is the intended input; it puts
the rule in column one, so pass `--goals-skip-columns 1`.

## What comes out

```
fof(background_1, axiom, ! [X0] : (~ (thing(X0) & lit(X0)))).
fof(background_2, axiom, ? [X0] : thing(X0)).
fof(ind_typing_1, axiom, thing('i:http://example.org/a')).
fof(owl_1_subClassOf, axiom, ! [X0] : (thing(X0) => ('c:http://example.org/A'(X0) =>
  ? [X2] : (thing(X2) & ('op:http://example.org/r'(X0,X2) & 'c:http://example.org/B'(X2)))))).
```

`thing` and `lit` are the soft-typing predicates. The Direct Semantics requires the object domain
and the data domain to be disjoint, and an unsorted first-order target has one domain, so object
positions are relativised by `thing` and data positions by `lit`. Omit this and the translation is
unsound for every data-property axiom.

Symbols carry a kind prefix: `c:` for a class, `d:` for a datatype, `op:` for an object property,
`dp:` for a data property, `i:` for an individual. That is not decoration. `OwlP1` and `OwlP2` are
disjoint sums in the Lean, so an IRI used as both a class and an object property is two unrelated
symbols, and a rendering that collapsed them would emit a different theory. Every prefix contains a
colon, which no unquoted TPTP lower-word may, so a prefixed symbol can never collide with the bare
`thing` and `lit`.

The two `background` axioms and the `ind_typing_*` axioms are not optional. See below.

## Ask a prover something

```bash
# Reason with a certificate, then export one problem per derived triple.
printf 'load ontology.ttl\nreason --profile owl-rl-ext --certificate /tmp/cert\n' \
  | open-ontologies --no-connect --data-dir /tmp/a batch -
printf 'load ontology.ttl\nfol --out /tmp/fol --format tptp --goals /tmp/cert/derivations.tsv --goals-skip-columns 1\n' \
  | open-ontologies --no-connect --data-dir /tmp/b batch -

eprover --auto --cpu-limit=10 /tmp/fol/goals/goal_00000.p
```

Note the two separate data directories. `reason` materialises its inferences into the default
graph, so exporting from the same store would put the conclusion into the axioms and make every
conjecture trivially entailed. `tools/fol_differential.py` does this correctly and checks that it
did; see below.

## The differential oracle

```bash
cargo build --release
python3 tools/fol_differential.py ontology.ttl --profile owl-rl-ext --timeout 10 --json out.json
```

It reasons with a certificate, exports one problem per claimed entailment from a separate
unreasoned store, and runs an ATP on each.

| verdict | means |
|---|---|
| `CLAIMED_NOT_ENTAILED` | the engine derived it, every premise of every derivation it found is expressible in the fragment, and the ATP still found a model of the axioms with the conclusion false. One of the two is wrong. Exits 1 |
| `ATP_ERROR` | the prover could not read the file. A defect in the exporter until shown otherwise. Exits 1 |
| `WEAKER_EXPORT` | the ATP is right and neither side is wrong: the engine's derivation rests on a triple the fragment cannot express, so the exported theory really does not entail the conclusion. The blocking triple is named. Does not fail the run |
| `UNDETERMINED` | the prover ran out of time or gave up. Common and honest, and NOT agreement |
| `NOT_ASKED` | no axiom form in `OwlLean/Syntax.lean` corresponds to the derived triple, with the reason |
| `AGREE` | the ATP also finds the conclusion entailed |

The `CLAIMED_NOT_ENTAILED` / `WEAKER_EXPORT` split is computed exactly, not guessed. Every triple
in the certificate is put to the exporter as a goal of its own, so the exporter's own
`triple_as_axiom` decides which are expressible. A conclusion is reachable in the exported theory
when it is an asserted triple the fragment can express, or when some derivation of it has every
premise reachable; the tool walks that relation with memoisation and reports the blocking triple
when no derivation survives. **Read the named triple.** The verdict is only as good as the
expressibility judgement behind it, so if that judgement ever refused a triple the export does in
fact carry, a real bug would be filed as `WEAKER_EXPORT` instead of as `CLAIMED_NOT_ENTAILED`. The
triple is printed rather than summarised for that reason.

E or Vampire, whichever is on `PATH`. Prover9 is deliberately not probed: it reads LADR, not TPTP.
With no prover installed the run skips loudly with the install line and exits 0; set
`FOL_DIFF_REQUIRE_ATP=1` to make that a failure, as `OO_REQUIRE_FIXTURES=1` does in the Rust suite.

`UNDETERMINED` is the normal outcome for a non-entailment. The translated theory is not decidable
in general, and a conclusion that does not follow often has only infinite countermodels, so a
prover that cannot refute it has told you nothing either way. The tool never reads that as
agreement.

## Reading the derivation back

```bash
# Prove the loaded ontology's claimed entailments and CHECK every derivation.
printf 'load ontology.ttl\nfol-prove --out /tmp/proofs --prover vampire --goals /tmp/cert/derivations.tsv --goals-skip-columns 1\n' \
  | open-ontologies --no-connect --data-dir /tmp/store batch -

# Or check a problem and a recorded prover output, with no store and no run.
vampire --proof tptp problem.p > proof.tstp
open-ontologies --no-connect fol-prove --problem problem.p --proof proof.tstp
```

Over MCP the tool is `onto_fol_prove`, with the same two modes.

Three things are established, in increasing order of what they cost.

**The prover refuted OUR problem.** Every leaf of the derivation is matched against the problem file
three ways: by the name its own `file('…', NAME)` annotation gives, by the PARSED formula, and by
the ROLE. Each catches something different. The name catches a prover pointed at a stale or
different file. The formula catches a file edited since under the same names. The role catches a
conjecture presented as an axiom, which would make the refutation say nothing about entailment while
looking perfect. Until this existed, an `AGREE` row above simply assumed all three.

**The derivation is a well-founded DAG ending in `$false`.** Every parent reference resolves, no
name is used twice, the parent relation is acyclic, and the node nothing else cites is the empty
clause. Nodes the empty clause does not depend on are counted separately and left out of every
tally.

**Some steps are recomputed.** `resolution`, `subsumption_resolution` and its forward and backward
spellings, `factoring`, `duplicate_literal_removal`, `flattening`, `trivial_inequality_removal`,
`equality_resolution`, and the negation of the conjecture. Each is replayed from its premises with a
syntactic unifier with an occurs check, and the conclusion must agree up to a bijective renaming of
variables. **Everything else is named and counted as unchecked**, with a reason: clausification,
Skolemisation, AVATAR splitting, every SAT-solver step, and every step of E's whose premise is an
inline inference record and therefore carries no formula.

### The verdict is one of eight words

| verdict | means |
|---|---|
| `problem_unparsed` | the TPTP problem could not be read. Not a statement about the prover |
| `derivation_unparsed` | the output is not a TSTP derivation. Usually the proof option was missing |
| `no_refutation_offered` | no empty clause. A satisfiable problem, a timeout or a give-up. Evidence of nothing |
| `derivation_rejected` | THE CHECKER SAID NO: a dangling parent, a cycle, or a leaf that is not a formula of the problem. Exits 1 |
| `refutation_step_not_reconstructed` | a step whose rule IS implemented did not reconstruct. EITHER the derivation is wrong OR this checker is incomplete, and it decides neither. Exits 1 |
| `refutation_structure_checked` | structure holds, no step replayed |
| `refutation_partially_replayed` | structure holds, some steps replayed. The normal outcome |
| `refutation_fully_replayed` | structure holds, EVERY step replayed. Still not unsatisfiability |

**An unchecked step prevents the strongest word**, and so does a leaf the prover invented. That is
mechanical and it is the reason the ladder has three rungs rather than a boolean. Even
`refutation_fully_replayed` is not a proof: the calculus's soundness is machine-checked nowhere in
`lean/`, and the replayer is ordinary Rust.

### Measured, over FOAF

One problem per claimed entailment, 181 of them, exported from an unreasoned store.

| prover | refuted | steps replayed | leaves matched | rejected | not reconstructed |
|---|---:|---:|---|---:|---:|
| Vampire 5.1.0 | 181 | 1279 of 3314 | 534, all `identical` | 0 | 0 |
| E 3.2.5 | 181 | 181 of 4644 | 534, all `alpha_equivalent` | 0 | 0 |

The order-of-magnitude gap is structural and worth knowing before choosing a prover for a pipeline
that wants to inspect its own evidence. Vampire prints each inference as its own annotated formula
with the conclusion attached; E nests inference records inside parent positions, and a nested record
carries a rule and parents but no formula, so neither it nor the step it feeds can be replayed.

`alpha_equivalent` against `identical` is not cosmetic either. E renames the bound variables of
every axiom it reads, so a leaf check demanding byte identity of the parsed AST would reject every E
proof of every problem. The three match levels — `identical`, `alpha_equivalent`,
`associativity_normalised` — are reported separately so that how much normalisation a leaf needed is
visible rather than absorbed.

### What the differential does with it

`tools/fol_differential.py` now runs each prover with its proof option and hands every derivation to
the checker. Each row carries `proof_check` and `proof_steps` beside its `AGREE`, and a
`PROOF_REJECTED` row exits 1. `--no-proof-check` turns it off, and a run with that flag reports what
the prover SAID and nothing about what it said it about.

## What is proved, and where the proof stops

`OwlLean.adequacy`, in the sibling `owl-lean` project:

> for an ontology `O`, an axiom `a`, and `inds` enumerating the individual vocabulary,
> `Entails O a ↔ FOL.Entails (background ++ indAxioms inds ++ O.map trAx) (trAx a)`

No `sorry`, no Mathlib, axioms `propext`, `Classical.choice` and `Quot.sound`, reprinted by
`lake env lean Audit.lean`. It covers every constructor of the fragment, cardinality restrictions
and `propertyChainAxiom` included, which is where LATIN's `OWL2toFOL.elf` stops: its cardinality
cases and `objectPropertyChain` are commented out, and 45.9% of constrained real ontologies use
property chains while 37.2% use qualified cardinality.

**The proof stops at the Lean.** `src/tptp.rs` is a transcription of `OwlLean/Translation.lean`
into Rust, and nothing mechanically checks that the transcription is faithful.
`tests/fol_translation_correspondence_test.rs` pins it: worked cases whose expected strings were
computed by hand from the Lean, unrolling `tr`, `trAx`, `mkEx`, `mkAll`, `distinctPairs`,
`somePairEq`, `Form.conj` and the counter arithmetic. A golden file captured from the emitter would
agree with the emitter by construction and would catch nothing, which is why the strings are
hand-derived. **The correspondence is pinned by tests and is not itself proved**, and that sentence
is in the module docs, in the JSON report and in the header of every emitted file.

## The two conditions the theorem carries

Both were real defects found while proving it, and both have machine-checked countermodels in
`OwlLean/Refutations.lean`.

**Freshness.** `tr` allocates bound variables from a counter and `tr_bridge` holds only under
`Fresh n x`, that is `x < n`. `tr_bridge_needs_freshness` exhibits an interpretation where the
translation of `∃r.⊤` at subject 0 with the counter also at 0 captures its own subject, so the
formula stops saying "`a` has an `r`-successor" and starts saying "something is `r`-related to
itself". The exporter never picks a counter: every entry goes through
`Translation::concept_fresh`, which refuses unless the subject is strictly below the counter, and
the call sites reproduce `trAx`'s own `tr c 0 2`, `tr c 1 2`, `tr c 0 1`. Break one and the export
fails with a message naming the countermodel, rather than writing a captured file.

**Individual typing axioms.** Nothing in `background` forces a constant to denote an object, so an
arbitrary first-order model may interpret an individual name outside `thing`.
`adequacy_needs_ind_axioms` refutes the left-to-right direction of adequacy outright, with the
empty ontology and the axiom `⊤(a)`: the countermodel `Mbad` sends the name to a literal. The fix
is `indAxioms`, `thing(a)` for every individual name, and the exporter emits one per individual in
the signature. The theorem's hypothesis is `∀ a : S.Ind, a ∈ inds`; the export's signature is the
individual vocabulary occurring in the axioms and the goal, so the hypothesis holds for the
signature the export defines.

## What is dropped, and where it is said

In the output, not in a comment. The report carries `exports_a_weaker_axiom_set` and
`constructs_not_exported`, the same shape the description-logic layer uses for
`certifies_a_weaker_axiom_set`:

```json
{
  "exports_a_weaker_axiom_set": true,
  "constructs_not_exported": [
    {"construct": "restriction on an inverse property expression", "occurrences": 6,
     "why": "Concept.some_ / all_ / minCard / maxCard take a named property (S.OProp), not an OPE"}
  ],
  "reduced_to_fragment": [
    {"construct": "owl:AllDisjointClasses", "occurrences": 2,
     "how": "expanded pairwise into Axiom.disjointWith"}
  ],
  "annotations_ignored": {"count": 206, "why": "an annotation carries no Direct Semantics content"}
}
```

An ASSERTION whose subject is declared a class or a property is also outside the fragment and is
counted as `assertion on a punned entity`. OWL 2 DL allows one IRI to be a class and an individual
at once; `OwlLean/Syntax.lean` does not, because `Sig` gives each entity kind its own type. The
subsumptions, domains and ranges on the same subject ARE exported, which is why this used to be
invisible: the file looks complete. It was found by running the model-certificate pipeline over
`case-studies/blast-furnace-ironmaking/`, where five of the nine triples the OWL-RL reasoner
derives came back with a machine-checked countermodel, because `bf:Hanging` is an `owl:Class` that
also carries `bf:hasSeverity bf:HighSeverity` and `rdfs2` is what the reasoner used. The export was
genuinely weaker than the graph and the report said `exports_a_weaker_axiom_set: false`.

Outside the fragment: `owl:hasKey`, datatype facets (`owl:withRestrictions`, `owl:onDatatype`),
`owl:datatypeComplementOf`, `owl:NegativePropertyAssertion`, data cardinality restrictions,
data-property assertions (`Axiom` has `oPropAssert` and no `dPropAssert`), property characteristics
on a data property, restrictions and `owl:inverseOf` over an anonymous property expression,
`rdfs:subPropertyOf` outside the object-property case, datatype expressions, and any cardinality
above 25, which is capped because `minCard n` emits `n(n-1)/2` distinctness literals.

Rewritten into the fragment before translation, exactly and at the OWL level:
`owl:AllDisjointClasses`, `owl:AllDifferent`, `owl:disjointUnionOf`, `owl:equivalentProperty` (two
`subOProp` axioms, which is what `EquivalentObjectProperties` means under the Direct Semantics),
and exact cardinality (as `min` and `max`).

Annotations are counted separately from the drops, because ignoring one does not weaken the axiom
set and folding 206 `rdfs:label` triples into the list would bury the constructs that do.

## CLIF, and its two dialects

Common Logic is **not** plain first-order logic. It has sequence markers, arity-free predicates,
and a universe in which relations are themselves individuals; ISO/IEC 24707 clause 6.5 states that
sequence markers make the logic non-compact and therefore not first-order. The adequacy theorem is
about plain first-order logic, so the emitted text is restricted to the fragment where the two
coincide: no sequence markers, fixed arity everywhere, no quantification into a predicate position.
The restriction is in the file header and is checked by `clif_uses_only_fol_fragment`.

`(and)` with no arguments is truth and `(or)` with no arguments is falsity (A.2.3.7). CLIF defines
no truth constants.

### The quoting trap points opposite ways in the two syntaxes

The TPTP writer uses **single**-quoted atoms for IRIs and the CLIF writer uses **double**-quoted
enclosed names for the same IRIs. That is deliberate, and it is the reverse of what most people
assume.

In TPTP a single-quoted atom is an ordinary constant, and a **double**-quoted string is a *distinct
object*, pairwise unequal to every other distinct object by fiat. Had the TPTP writer used double
quotes for IRIs it would have silently asserted that all individuals are pairwise distinct. OWL has
no unique name assumption, so every `owl:sameAs` export would become unsound, and the file would
still parse and still prove things.

In CLIF the polarity flips. A **single**-quoted string is an *interpreted name* that denotes
itself; a **double**-quoted enclosed name is an ordinary interpretable name, which is why A.2.2.4
recommends the enclosed-name syntax for writing IRIs. The vertical bar is an ordinary name
character in CLIF and is Common Lisp's convention, not CLIF's.

### The exactly semantically conformant subdialect

ISO/IEC 24707 first edition Annex A.4.2, verbatim: "The subdialect of CLIF which does not use
numerals or quoted strings is exactly semantically conformant". A numeral and a quoted string are
interpreted names, and a text containing them constrains its own interpretations in a way abstract
Common Logic does not. Stay out of them and CLIF entailment and Common Logic entailment coincide,
so the adequacy theorem's biconditional needs no qualification at the CLIF end.

**The scope of that claim, exactly: no decimal numerals and no quoted strings IN SENTENCE
POSITIONS.** Comment annotations are the named exception, because a comment's text is a quoted
string by definition, so a strict reading of "does not use quoted strings" would exclude any file
carrying a comment at all. The CLIF files ISO hosts for ISO/IEC 21838-2 are in the same position,
using single quotes only inside comment headers.

The property is checked rather than asserted:
`clif_stays_in_the_exactly_conformant_subdialect` walks every emitted sentence, skips the comment
annotations, and fails on a bare decimal or a single-quoted string.

Nothing here is claimed about the second edition's Annex A.3, which this project has not read. In
particular this page does not say that Common Logic requires infinite universes; abstract Common
Logic requires only non-emptiness, and the infinite-universe requirement is CLIF-specific, appears
in the withdrawn first edition, and sits awkwardly beside A.4.2 in that same edition.

### The two dialects, and what actually reads them

| dialect | spelling |
|---|---|
| `iso` (default) | `cl:text`, `cl:comment` |
| `colore` | `cl-text`, `cl-comment` |

Comment strings are **single**-quoted in both. Quote style is not a dialect matter: A.2.2.2 makes
the single quote the string delimiter, and BFO's release notes of 7 December 2025 retract the
double-quoted form, saying "Comment texts are surrounded by single, not double quotes". This
exporter emitted double quotes until that was measured. The ISO-hosted 21838-2 files carry 369
double-quoted `cl:comment` forms across 15 files; BFO master carries 356 single-quoted and none
double-quoted across 13. Matching the corpus was the wrong test, because the corpus had been
withdrawn by its own maintainers. Binding quote style to operator spelling, as this exporter did at
first, meant **no combination of flags could emit conforming CLIF**.

It is **not** true that a file in one spelling fails in the other's tools. Measured:
py-typedlogic maps both spellings to identical results with identical sentence counts. What is true
is narrower and is set out below.

### Why CLIF at all, given TPTP exists

They are not competing. Macleod translates CLIF to TPTP and to LADR and then calls Vampire,
Prover9, Paradox or Mace4, so TPTP is the execution format and CLIF is the interchange format.

More than a preference: **ISO/IEC 21838-1:2021 clause 4.3 requires that a top-level ontology be
available through an axiomatisation in a language conforming to ISO/IEC 24707**, and its note names
CLIF, CGIF and XCL as the qualifying dialects. Two qualifications travel with that sentence and
must not be dropped. First, the English clause text was **not** read here: it is paywalled, and the
"shall be available" modality comes from the Russian identical adoption of the same standard.
Second, CLIF specifically is not mandatory, because CGIF or XCL would satisfy clause 4.3 equally;
CLIF is named for Basic Formal Ontology in particular by ISO/IEC 21838-2 clause 4.4.1(a), which
**was** read verbatim. BFO's own documentation adds that the OWL version is an approximation to the
CLIF one.

### What the two existing CLIF parsers actually do with our output

Measured on this machine with py-typedlogic's `ClifParser` and the Macleod toolchain, over five
exports.

| | py-typedlogic | Macleod |
|---|---|---|
| default shape (named text, standalone comments, bare sentences) | accepts, recovers every sentence | accepts the shape; rejects our symbols |
| `--clif-comments wrapped` | accepts and recovers **zero** sentences | no production for a commented sentence |

Sentence counts recovered from the default shape, against the exporter's own report of
`background + individual typing + axioms`:

| ontology | exporter | py-typedlogic |
|---|---:|---:|
| the smoke ontology | 2+1+4 = 7 | 7 |
| `pizza.ttl` | 2+5+698 = 705 | 705 |
| `foaf.ttl` | 2+1+158 = 161 | 161 |
| `gufo.ttl` | 2+2+247 = 251 | 251 |
| `mushroom-ontology.ttl` | 2+0+105 = 107 | 107 |

**Why the default is not the BFO shape.** py-typedlogic treats a `(cl:comment ...)` form as
discardable, so a file whose sentences all sit inside one parses cleanly and yields zero sentences.
It does the same to BFO's own files. Macleod has no commented-sentence production at all. A file
that is formally valid and practically empty is the assurance-laundering shape this project exists
to attack, so it is not the default; `--clif-comments wrapped` remains available and the row above
says what it costs.

**Why the text is named.** All 227 COLORE texts are named. Macleod refuses an unnamed text with
"Error in ontology: bad URI", and py-typedlogic otherwise reports the first comment as the theory's
name. The name is the ontology's own `owl:Ontology` IRI when it declares one. It is written bare
rather than double-quoted because Macleod's lexer has no double-quote token.

**Macleod cannot consume an IRI-named ontology, and that is not fixable from this side.** Its lexer
rejects a double-quoted enclosed name outright, and it also rejects most bare IRI-shaped symbols:
measured, `cA:x` and `cA/x` are accepted while `c:http://e/A` and `http://e/A` are not, which is a
PLY token-precedence accident rather than a documented rule. It further rejects the zero-ary
`(and)` that A.2.3.7 explicitly licenses, and rejects a backtick, a slash, a double quote or a
percent sign inside a comment string. COLORE ontologies use short bare identifiers, which is the
world Macleod is built for. The kind prefixes `c:` / `op:` are load-bearing here, because `OwlP1`
and `OwlP2` are disjoint sums, so this exporter will not mangle IRIs into opaque identifiers to
satisfy a lexer.

### An external parser rejects ISO's own normative corpus

Both parsers independently reject the **same 8 of the 227** Common Logic files ISO hosts for
ISO/IEC 21838-4, while accepting every one of our exports. An independent bracket-balance check,
counting only brackets outside quoted strings and enclosed names, finds that **4 of the 8 are
literally unbalanced**:

| file | bracket imbalance |
|---|---|
| `simple_curve_segment_theorems.clif` | +3 unclosed |
| `region_mt.clif` | +1 unclosed |
| `psl_actors.clif` | -2, closes past the top level |
| `requires.clif` | -1, closes past the top level |
| `atomic_act_mereology.clif`, `multig_complex.clif`, `possibly_consumable.clif`, `reusable.clif` | balanced; rejected for other reasons |

py-typedlogic rejects 8 of 227; Macleod rejects 18 of 227, a superset of those 8. This is the
strongest available argument that a fragment gate and an external parser check are worth having: a
normative corpus published by a standards body does not survive contact with either of the two
parsers that read its language.

## Known limitations

Stated rather than discovered later.

- **The reader is a front end and no theorem touches it.** `owl-lean` lists OWL-file-to-abstract-syntax
  as not started: IRI resolution, the imports closure, the OWL 2 datatype map and facets, punning,
  blank node scoping. `src/tptp.rs`'s reader is that front end. Everything it cannot read it names,
  and that is the whole of its guarantee.
- **Entity kinds are inferred when undeclared.** A property with no `owl:ObjectProperty` or
  `owl:DatatypeProperty` declaration is classified by whether it ever takes a literal object, and
  every such inference is listed under `entity_kinds_inferred`. An explicit `owl:DatatypeProperty`
  declaration beats a property characteristic: FOAF declares `foaf:msnChatID` as both a datatype
  property and an inverse-functional property, which OWL 2 DL forbids.
- **Blank node labels are not stable across loads.** A goal naming an anonymous class expression
  must come from the same graph the export was built from, which is why the differential loads the
  certificate's own `asserted.tsv` rather than the source file twice. Otherwise the goal is refused
  with that reason.
- **Nothing runs the differential in CI, and nothing runs the derivation check there either.** Both
  need a prover on `PATH` and both skip loudly without one. The recorded Vampire and E derivations
  under `tests/fixtures/tstp/` DO run everywhere, over a problem the test regenerates from the
  exporter so the fixture cannot drift; the live run skips. A CI leg that installs a prover is not
  wired.
- **Superposition and demodulation are not replayed.** They are the steps that do the work on any
  problem with equality, and replaying one needs a term ordering. Until then an equality-heavy proof
  sits near the bottom of the ladder.
- **Nobody here has read ISO/IEC 24707:2018 or either part of ISO/IEC 21838.** All three are priced
  at CHF 0, 70 pages for 24707, but downloading them needs a free ISO account, which is an owner
  action. The old ITTF free-standards site closed in 2025; catalogue pages are reachable at
  `committee.iso.org` when `www.iso.org` returns 403. Every clause cited on this page is scoped to
  the edition and the text actually read, and the second edition's Annex A.3 has not been read at
  all.
- **Vampire has now been run, and this bullet used to say it had not.** Until 15 September 2026
  every number on this page came from E 3.2.5 and the Vampire branch of the prover detection had
  never executed. Vampire 5.1.0 is installed on this machine, `--atp vampire` has been run over
  FOAF, and the measured table above is from both. What was found in the process: `--mode casc`
  alone prints a proof in a display format, and `--proof tptp` is what makes it a TSTP derivation
  the checker can read. The argument vector now carries it, and so does E's `--proof-object`.
- **`AGREE` is evidence, not proof.** It means a second implementation reached the same conclusion.
  A checked derivation beside it adds that the second implementation was reading our file. It does
  not add a proof and the summary says so in the same paragraph as the counts.

## What it caught

Run against FOAF on the day it was written, the differential reported 58 `CLAIMED_NOT_ENTAILED`
out of 181 claimed entailments. All 58 were defects in this layer, found in three classes: the
first two accounted for 52 and the third for the remaining 6.

1. The goal builder did not map `owl:Thing` and `owl:Nothing` to the translation's `top` and `bot`,
   so `X rdfs:subClassOf owl:Thing` asked about an atomic class symbol occurring in no axiom.
2. The goal builder did not consult the entity-kind classification, so a goal about a data property
   asked about `op:p` while the axioms spoke about `dp:p`. `OwlP2` is a disjoint sum, so those are
   unrelated predicates.
3. **In the exporter itself, not only in the goals**: a property characteristic was allowed to
   override an explicit `owl:DatatypeProperty` declaration, so `foaf:msnChatID` was exported as an
   object property throughout.

After the fixes, 181 of 181 agree.

It also forced the `WEAKER_EXPORT` verdict into existence. OWL TIME and GoodRelations both produced
disagreements that were neither side's fault: `rdfs2` derives `time:unitDay a
time:GeneralDurationDescription` from the data-property assertion `time:unitDay time:days "1"`, and
`scm-rng2` derives `gr:hasMinValueFloat rdfs:range rdfs:Literal` from
`gr:hasMinValueFloat rdfs:subPropertyOf gr:hasMinValue` between two data properties. The fragment
has no `dPropAssert` and no data-property hierarchy, so the exported theory genuinely does not
entail either conclusion. Reporting those as `CLAIMED_NOT_ENTAILED` would have sent someone hunting
a bug that was a stated limitation all along. The tool's own first shape had a worse defect: it exported from
the store the reasoner had just materialised into, so the conclusion was already among the axioms
and a deliberately broken exporter still scored a clean run. That was found by running the
stop-the-line gate on deliberately broken input, which is the only reason it was found at all.
