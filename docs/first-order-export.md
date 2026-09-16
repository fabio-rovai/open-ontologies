# First-order export, and what an ATP verdict is worth

The engine can hand an ontology to the automated-theorem-proving ecosystem, in TPTP FOF, in
ISO/IEC 24707 CLIF, in ISO/IEC 24707 CGIF, in SMT-LIB 2 and in LADR. The translation it emits is
the one a machine-checked adequacy theorem is about, so a reader of the file can cite a
kernel-checked result about what it means.

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

# ISO/IEC 24707 CGIF, the second Common Logic dialect. CORE CGIF, and a compact
# sub-dialect of it. No dialect or comment flag: there is one spelling.
printf 'load ontology.ttl\nfol --out /tmp/fol --format cgif\n' \
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

`ontology.p` (or `ontology.clif`, or `ontology.cgif`) lands in the directory. With `--goals FILE`, where FILE is a TSV
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

ISO/IEC 24707 Annex A.4.2, verbatim, in the **2018 second edition** and identically in the first:
"The subdialect of CLIF which does not use numerals or quoted strings is exactly semantically
conformant, as can be shown by inverting the above construction of J from I". A numeral and a
quoted string are
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

**This page used to say that nobody here had read the second edition.** That stopped being true on
15 September 2026 and the sentence has been removed rather than left standing; see the limitations
section for what the second edition turned out to say. In particular this page does not say that
Common Logic requires infinite universes: abstract Common Logic requires only non-emptiness
(clause 4.2), and the infinite-universe requirement is CLIF-specific.

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

## CGIF, the second Common Logic dialect

ISO/IEC 24707 defines **three** dialects: CLIF (Annex A), CGIF (Annex B) and XCL (Annex C). This
engine emitted one of them and called the result Common Logic support. That is the shape of
overclaim this project spends its time finding in other people's work, so `--format cgif` closes
half of it. XCL is still absent, and the export report says so in a field
(`common_logic_dialects_not_emitted`) rather than leaving a reader to assume otherwise.

It matters for the same reason CLIF does. ISO/IEC 21838-1:2021 clause 4.3 asks that a top-level
ontology be available through an axiomatisation in a language conforming to ISO/IEC 24707, and its
note names CLIF, CGIF and XCL as the three that qualify. CGIF is not a lesser member of that list:
Annex B.4 states that core CGIF, extended CGIF and the abstract CG syntax are all "fully conformant
CL dialects in the sense that every CL sentence can be translated to a semantically equivalent
sentence in each of them".

### Core, not extended, and what that costs

Annex B specifies two concrete syntaxes. **Core CGIF (B.2) is what this emitter writes.** B.3.1's
own list of what extended CGIF adds is: type labels and type expressions on concepts, universal
quantifiers, the Boolean contexts `[If: … [Then: …]]`, `[Either: [Or: …]]` and `[Equiv: [Iff: …]]`,
concepts inside an arc sequence, and importing a text into a text. None of that is emitted.

Core costs exactly one thing in readability, and B.1.2 states the reduction itself: "The concept
`[Go:*x]`, for example, becomes an untyped concept `[*x]` and a conceptual relation `(Go ?x)`."
So a class membership is written `("c:http://e/A" ?X0)` and not `[A: *x]`. Taking that reduction
means the file needs no Annex B.3 rewrite pass before anything can read it.

### The encoding, production by production

| `Form` | CGIF | clause |
|---|---|---|
| `App1(p,t)` | `(p t)` | B.2.7 `ordinaryRelation` |
| `App2(p,t,u)` | `(p t u)` | B.2.7, at arity two |
| `Eq(t,u)` | `[: t u]` | B.2.5 `coreferenceConcept`. **CGIF has no `=`** |
| `Tru` | `[]` | B.2.5: "an empty context `[ ]` is translated to CLIF as `(and)`, which is true by definition" |
| `Fls` | `~[]` | B.2.8: "The negation of the blank CG, written `~[ ]`, is always false" |
| `Neg(g)` | `~[ g ]` | B.2.8 `negation = "~", context` |
| `And(g,h)` | `g h` | juxtaposition. B.2.6 makes a CG's nodes a conjunction and gives it no operator |
| `Or(g,h)` | `~[ ~[g] ~[h] ]` | B.3.5's own `eitherOr` rewrite into core |
| `Imp(g,h)` | `~[ g ~[h] ]` | B.3.5's own `ifThen` rewrite into core |
| `All(n,g)` | `~[ [*Xn] ~[g] ]` | B.3.7: a CG with universal concepts becomes "a nest of two negations" |
| `Ex(n,g)` | `[ [*Xn] g ]` | B.2.6: a CG with existential concepts is `∃names.` over the rest |

The three derived forms are not this project's encoding choices. B.3.5 and B.3.7 are rewrite rules
FROM extended CGIF INTO core, and the right-hand sides above are what those rules produce.

**Equality is the one that surprises people.** There is no equality relation in CGIF and there
could not be: `=` is a CLIF reserved token and a CGIF `identifier` must begin with a letter. B.2.5
gives `[: ?x Cicero Tully ?abcd]` as translating to `(and (= x Cicero) (= x Tully) (= x abcd))`.
One caveat travels with that and is written down rather than smoothed over: B.2.5 defines the
references as a SET, so `[: n n]` denotes `(and)`, truth, rather than `n = n`. The two agree in
every model, and the only axiom that could produce one is a self-`owl:sameAs`, which is a tautology
either way.

### Every binder gets a context of its own

B.2.10: "If a concept x with a defining label with name n is directly contained in some context c,
then c shall not contain any concept other than x with a defining label with the same CG name n."

`OwlLean.trAx` restarts its variable counter per axiom and calls `tr c 0 2` and `tr d 0 2` for an
axiom's antecedent and its consequent, so two `[*X2]` really can arise in one sentence. Renumbering
them is not available: `owl-lean` uses named variables rather than de Bruijn indices precisely so
that freshness is a proof obligation, and an exporter that renumbered would be emitting a different
formula. So an existential is written `[ [*Xn] … ]` rather than as a bare `[*Xn]` juxtaposed with
its body, and each sentence is wrapped in a context of its own. No context then directly contains
two defining labels at all, whatever the counter did.

**B.2.10 is self-contradictory about shadowing** and the emitter does not depend on the answer. It
says both that a context "shall not contain any concept other than x with a defining label with the
same CG name n" — where *contains* is transitive — and, in the next sentence, that a nested context
may redeclare one. `no_defining_label_is_ever_shadowed` checks that the emitted text is legal under
either reading.

### The restriction, in the standard's own vocabulary

Clause 7.1.1 names three sub-dialects and the emitted text is all three:

| clause 7.1.1 term | verbatim | ours |
|---|---|---|
| compact sub-dialect | "a dialect that does not recognize sequence markers" | no `[*...x]`, no `?...x` |
| unstructured sub-dialect | "a dialect that does not recognize titlings and importation statements" | neither is emitted |
| single domain sub-dialect | "a dialect that does not recognize domain restrictions" | none is emitted |

The first is the one that matters, and clause 6.5 says why: Common Logic with sequence markers "is
not compact, and therefore not first-order". `OwlLean.adequacy` is about plain first-order logic,
so a text using one would sit outside the theorem.

Two further exclusions have no clause-7 name and would leave first-order logic just as surely. **No
`#?` type label**, which B.2.7's own comment says is how "CGIF supports the CL ability to quantify
over relations and functions" — the CGIF spelling of the thing the CLIF restriction already
excludes. And **no actor**, B.2.1, which is how a CL function is written; `OwlLean.FOL`'s `FSig`
has P1, P2 and Const and no function symbol of positive arity, so `Form` cannot express one.

### Interpreted names, and the exception CGIF does not need

A.4.2's "exactly semantically conformant" result is stated for **CLIF** and Annex B states no
analogue for CGIF. This page therefore does not borrow the label. What the emitter does is hold the
property: no numeral and no single-quoted string stands anywhere in the text, checked by
`cgif_uses_no_interpreted_name`.

CGIF holds it with **no exception to declare**, where CLIF needs one. A CLIF label has to ride on
`cl:comment`, whose argument is a quoted string by definition, so the CLIF page above has to carve
comment annotations out of the claim. B.2.4 makes a CGIF comment a LEXICAL construct, `/* … */`,
which is not a name at all.

The same clause is why a comment can be **refused**: "The string enclosed by the delimiters `/*`
and `*/` shall not contain a substring `*/`", and no escape is defined. Rewriting the text would
emit a different comment and dropping it would lose the label that says which axiom a sentence is,
so `to_cgif` returns an `UnwritableComment` and writes no file. That path is reachable rather than
theoretical: an earlier draft of the CGIF header named the delimiters literally and tripped it on
every run.

### Names

B.1.1's `identifier` is `letter, {letter | digit | "_"}` — no colon, no slash, no dot, no hyphen —
and B.1.1 states the consequence: "the category CGname requires that all CLIF name sequences except
those in the CGIF category identifier shall be enclosed in quotes". So every IRI is a double-quoted
enclosed name, **including the text's own name**, where the CLIF writer emits a bare IRI because
Macleod's lexer has no double-quote token. The two writers share `clif::enclosed` rather than each
having its own, because B.1.1 defines `CGname` to include CLIF's `enclosedname` category.

`X0`, `thing` and `lit` are identifiers. A defining label here is `Xn` and a constant here begins
`i:`, so the two can never collide, which B.2.10's last clause forbids outright: "No constant with
CG name n shall be in the scope associated with some concept with a defining label with CG name n."

`letter` is used in that production and **never defined anywhere in ISO/IEC 24707:2018**. That is a
defect in the standard, not a reading of it. The emitter produces ASCII letters only and the
checker reads it that way, and the choice is written down here rather than assumed.

### How CGIF conformance is checked, and how that differs from CLIF

**By our own checker, not by an independent parser.** This is the weaker of the two footings in
this document and the difference is stated rather than blurred.

For CLIF, two external parsers were run over five exports and the table above records what they do,
including that both return an EMPTY theory from the wrapped comment shape ISO's own BFO files use.
Nothing of that kind exists for CGIF. There is no `cgif` package on PyPI; py-typedlogic, which
reads the CLIF output, has no CGIF front end; Macleod reads CLIF. So conformance here is pinned by
the lexer, parser and checker in `tests/fol_cgif_export_test.rs`, transcribed from Annex B's EBNF
with the clause cited at each rule.

What that transcription is worth: it is a SECOND reading of the grammar, written against the
emitter rather than with it, so a shape both agree on is a shape two readings of Annex B agree on.
It is not an independent implementation and this page does not call it one.

Alongside the syntax check there is a **round trip**. Every sentence is read back into a `Form` and
must equal the formula it was built from, up to the two rewrites core CGIF forces: implication and
disjunction have no operators (B.3.5 defines both by the rewrite), and conjunction is "an unordered
set of concepts, conceptual relations, negations, and comments" (B.2.6) rather than a binary
connective, so the associativity of an `And` tree cannot survive. Both sides are normalised by
those two facts and by nothing else.

One source was deliberately NOT used. `jfsowa.com/cg/annexb.htm` is a **pre-publication draft** of
the same annex, not the published standard, and the 2018 Foreword says "the list of syntactic
errors that have already been identified in the Defect Report has been fixed". Measured differences
that would have mattered: the draft's `equiv` rewrite emits `~[` where the published text emits
`[`, turning a conjunction of two implications into a tautology; its `nestedOrs` termination test
drops the last disjunct; its `text` name slot is `constant` rather than `CGname`; and its B.4 says
"[This section is incomplete.]" where the published B.4 carries the complete Table B.1.

### Defects found in ISO/IEC 24707:2018 itself

Listed because a checker written from a standard inherits the standard's gaps, and a reader of the
emitted file should know which lines rest on a reading rather than on a rule.

| clause | what |
|---|---|
| B.1.1 | `letter` is used in `identifier` and defined nowhere in the document |
| B.2.10 | the no-duplicate clause uses *contains* transitively and the next sentence licenses shadowing; the two cannot both hold |
| B.3.1 / Table B.1 E17 | "the ability to import text into a text" is listed as a feature and no production defines it; only `[cg_Imports N]` in the table hints at the syntax |
| B.3.9 | cross-references "A.3" for module elimination; A.3 in this edition is CLIF semantics |
| Table B.1 E6, E19 | write a coreference concept and a text without the `:` that B.2.5 and B.2.11 make mandatory |
| B.1.1 | `CGname`'s second branch is subsumed by its fourth, so the rule is ambiguous (harmless) |
| B.1.4.1 | "zero or more characters of white space may occur" at every comma is false for two adjacent identifiers |
| B.2.5 vs B.3.6 | `[]` is truth under core and `∃x.⊤` under extended. Both true given clause 4.2's non-empty universe, and not the same graph |
| B.2.11 vs B.3.6 | `text` and an extended `concept` both match `[Proposition: N …]` with different readings, and `text` is not a member of `CG` while Table B.1 E19 can generate nested texts |
| B.2.5 | `existentialConcept` admits `[*7]` and `[*'str']`, which A.2.3.8's `bvar` cannot bind. The checker refuses them |

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
- **Nothing runs the differential in CI.** It needs a prover on `PATH` and skips loudly without
  one. A CI leg that installs E is not wired.
- **CGIF conformance is pinned by our own checker and not by an independent parser.** There is no
  installable CGIF parser to differ from: no `cgif` package on PyPI, no CGIF front end in
  py-typedlogic, and Macleod reads CLIF. The CLIF claims on this page rest on two external parsers
  disagreeing with us in recorded ways; the CGIF claims rest on a second reading of Annex B's EBNF
  in `tests/fol_cgif_export_test.rs`. That is a real difference in footing and it is not narrowed
  by calling both "checked".
- **XCL is not emitted.** ISO/IEC 24707 has three dialects and two are written here. The report
  says which, in `common_logic_dialects_emitted` and `common_logic_dialects_not_emitted`, so a
  reader is not left to infer that "Common Logic support" means all three.
- **ISO/IEC 24707:2018 HAS now been read; neither part of ISO/IEC 21838 has.** This item said the
  opposite until 15 September 2026, and the claim it made about ACCESS was wrong as well as stale:
  24707:2018 is in ISO's *Publicly Available Standards* list and needs no account at all, only a
  click-through licence, which is a POST with a cookie jar rather than an owner action. It was
  downloaded (`standards.iso.org/ittf/PubliclyAvailableStandards/c066249_ISO_IEC_24707_2018.zip`,
  sha256 `e920b0c43a932e1ad0e2c76d3501f56e2b11ee5547265b14aeb69a5866eae5e3`) and Annexes A and B
  were read in full for the CGIF work. Three things follow. A.4.2's exactly-conformant sentence is
  in the second edition verbatim, so the CLIF claim above no longer needs its edition caveat. The
  2007 first edition is **no longer in the PAS list**, so the URL Sowa and Wikipedia cite now
  fails; 2018 cancels and replaces it. And ISO/IEC 21838-1 and -2 are still unread, so every claim
  on this page sourced to them is still scoped to the text actually read, including that clause
  4.3's "shall be available" modality comes from the Russian identical adoption and not from the
  English.
- **Only E has actually been run.** Every number on this page comes from E 3.2.5. The Vampire
  branch of the prover detection has never executed, because Vampire is not installed here; its
  argument vector and its SZS parsing are written from the documented interface and are untested.
  Treat `--atp vampire` as unverified until someone runs it.
- **`AGREE` is evidence, not proof.** It means a second implementation reached the same conclusion.

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
