import OOCert.Soundness

/-!
# `oo-refute/1`: certifying that a graph has NO model

Seventeen rules in the OWL 2 RL profile conclude `false` rather than a triple.
Counted off OWL 2 Web Ontology Language Profiles (Second Edition), W3C
Recommendation 11 December 2012, section 4.3, the whole list is:

| Table | Rules concluding `false` |
| --- | --- |
| 4, Equality | `eq-diff1`, `eq-diff2`, `eq-diff3` |
| 5, Axioms about Properties | `prp-irp`, `prp-asyp`, `prp-pdw`, `prp-adp`, `prp-npa1`, `prp-npa2` |
| 6, Classes | `cls-nothing2`, `cls-com`, `cls-maxc1`, `cls-maxqc1`, `cls-maxqc2` |
| 7, Class Axioms | `cax-dw`, `cax-adc` |
| 8, Datatypes | `dt-not-type` |

The list is written out rather than summarised because the number alone is a
claim nobody can re-check. Three plus six plus five plus two plus one is
seventeen, out of the 78 rules named across Tables 4 to 9. Table 9, Schema
Vocabulary, contributes none of them: all 20 of its rules conclude a triple, so
the ones this project implements live in `Semantics.lean` as ordinary
conditions.

`cax-dw` is the one implemented here, and the one with the most axioms waiting
for it: given `c1 owl:disjointWith c2`, `x rdf:type c1` and `x rdf:type c2`, the
graph is contradictory. The certificate format in `Rules.lean` cannot express
that, because `Step.conclusion` is a `Triple` and `certificate_sound` concludes
`Entails`. There is no triple to conclude. This file adds the second format.

Sixteen of the seventeen are rules this layer can state, and it states one of
them, `cax-dw`. The seventeenth, `dt-not-type`, it cannot state at all. The
distinction is drawn where the count is, below, because a reader who meets only
the number will otherwise take "not implemented" and "not expressible" for the
same thing, and they are different claims about this checker.

## Why the negative conditions live HERE and not in `Semantics.lean`

`lean/OOCert/Witness.lean` proves `saturated_is_a_model`: every graph has a
model, the one in which every property relates everything. That theorem is what
stops `certificate_sound` being vacuously true, and its truth depends on the
fragment having no negation. A disjointness condition is negative: it says
nothing is in both of two disjoint classes, and the saturated interpretation
puts everything in every class. Adding such a condition to `Conditions` would
make `saturated_is_a_model` FALSE, and that theorem cannot be patched because
being unfalsifiable is the whole point of it.

So `RefuteConditions` is a SEPARATE layer. `Conditions` is untouched, every
existing theorem keeps its meaning, and the two model classes are named
differently so that no report can confuse them.

## What `Unsat` means, and what it does not

`Unsat G` says: no interpretation satisfies BOTH `Model I G` AND
`RefuteConditions I`. It is unsatisfiability RELATIVE TO THOSE CONDITIONS, not
absolute unsatisfiability. A reader who takes it for the latter will be wrong in
a specific way: `Model I G` alone is still satisfiable for every graph, refuted
or not, because `saturated` ignores disjointness. `Unsat G` is the statement
that reading `owl:disjointWith` as disjointness leaves no models, which is what
an OWL 2 RL consumer means by "inconsistent" and is not the same sentence.

`RefuteConditions` carries exactly one field, because exactly one rule is
implemented. The other sixteen fall into two groups, and the difference between
them is the difference between work not done and work that cannot be done here.

FIFTEEN are missing and expressible. They are `eq-diff1`, `eq-diff2`,
`eq-diff3`, `prp-irp`, `prp-asyp`, `prp-pdw`, `prp-adp`, `prp-npa1`,
`prp-npa2`, `cls-nothing2`, `cls-com`, `cls-maxc1`, `cls-maxqc1`, `cls-maxqc2`
and `cax-adc`. Each needs its own field on `RefuteConditions` and its own
constructor on `RefuteRule`, and none of them is here. Adding one is ordinary
work: the condition is a sentence about `I.iext` and `I.cext`, which `Interp`
already provides, and the four list-valued ones (`eq-diff2`, `eq-diff3`,
`prp-adp`, `cax-adc`) read their members through `Chain` as `Model` already
does for `owl:intersectionOf`. The three cardinality rules (`cls-maxc1`,
`cls-maxqc1`, `cls-maxqc2`) would match `"0"^^xsd:nonNegativeInteger` by its
spelling, as this layer matches every other term. That is sound and it is not
complete: a graph writing the same value some other way would be missed, which
is a rejection and never a false pass.

ONE is not missing. `dt-not-type` fires when the data value of a literal falls
outside the value space of the datatype it is typed with, and `Semantics.lean`
has no value spaces to fall outside of. An `Interp` there is a domain, a
denotation `ι : Term → D` and one ternary relation; a literal is its N-Triples
spelling like any other term; and that file's "What is not here" section states
the restriction and its consequence, that `Entails` must not be read as
datatype-aware. No field added to `RefuteConditions` could cover the rule,
because what it needs is absent from `Interp` rather than from the condition
structure. Covering it means extending the signature with a datatype map and a
lexical-to-value reading, which is a different piece of work from adding a
clash rule and would change what every existing theorem quantifies over.
`src/reason.rs` keeps the whole `dt-*` family out of the engine on the same
ground, and `tests/reason_rl_coverage_test.rs` pins that exclusion.

A graph that only a missing rule could refute is not refutable by this checker,
whichever group the rule falls in, and the checker says so by rejecting, never
by passing.

## The explosion, and what a consumer must do about it

Once a graph is refutable, every triple is entailed under the disjointness-aware
reading, so an ordinary triple certificate over that graph carries no
information at all. `a_certificate_adds_nothing_when_the_graph_is_refuted`
states this as a theorem rather than as advice: for a refuted `G`, EVERY
conclusion of EVERY list of steps has the disjointness-aware warrant, whether
the checker accepted the certificate or not.

The trap here is subtler than plain explosion, and it is worth being exact.
`certificate_sound`'s verdict, `Entails G t`, is NOT trivialised by a refutation,
because `Entails` quantifies over `Model I G` alone and that class is never
empty. So a triple certificate over a self-contradicting ontology still checks
green, still says something literally true, and still says nothing a consumer
wants. The guarantee the consumer wants is `EntailsIn (RModel G)`, and that one
is empty. A consumer must therefore run the refutation checker FIRST and treat a
successful refutation as invalidating every derivation over the same graph.
`oo-refute guard` does that in one command and refuses the triple certificate.

## The format

```
oo-refute/1
<rule>  <s>  <p>  <o>  [<ps>  <pp>  <po>]*        (zero or more, tab separated)
refute  cax-dw  <c1>  <p>  <c2>  <x>  <p>  <c1>  <x>  <p>  <c2>
```

The first line is the version marker. The lines between it and the last are
ordinary derivation steps in the `derivations.tsv` spelling, so a refutation may
reach its contradiction through inference: the disjoint types are usually
derived by `rdfs9` off a subclass chain rather than asserted. The last line is
the contradiction step, and there is exactly one. A `derivations.tsv` handed to
this checker is rejected, because it has no `refute` line and no version marker.
-/
namespace OOCert

/-! ## Vocabulary

`V` lives in `Triple.lean` and this round belongs to another file's owner, so
the one term this layer adds is kept here. It is the same N-Triples spelling the
engine's interner produces. -/
namespace RV
/-- `owl:disjointWith`. -/
def disjointWith : Term := "<http://www.w3.org/2002/07/owl#disjointWith>"
end RV

/-! ## The negative conditions -/

/-- The conditions a model must satisfy for a REFUTATION to mean anything.
Minimal by design: one field, for the one rule implemented.

`dw` is the semantic reading of `owl:disjointWith`: if `(a, b)` is in the
extension of `owl:disjointWith` then no domain element is in both class
extensions. It is the *if* direction only, which is all `cax-dw` uses, and it is
a consequence of the OWL 2 condition rather than a strengthening of it. -/
structure RefuteConditions (I : Interp) : Prop where
  /-- cax-dw. -/
  dw : ∀ a b, I.iext (I.ι RV.disjointWith) a b → ∀ x, I.cext a x → I.cext b x → False

/-- A model of `G` that ALSO respects disjointness. Named apart from `Model` so
that no verdict can quietly move between the two classes. -/
def RModel (G : List Triple) (I : Interp) : Prop := Model I G ∧ RefuteConditions I

/-- `G` has no model that respects disjointness.

Relative, not absolute: see the module docstring. `Model I G` on its own is
satisfiable for every graph, by `saturated`. -/
def Unsat (G : List Triple) : Prop := ∀ I : Interp, ¬ RModel G I

theorem RModel.model {G : List Triple} {I : Interp} (h : RModel G I) : Model I G := h.1

/-! ## The refutation step -/

/-- The clash rules, one constructor per rule implemented. One so far. -/
inductive RefuteRule
  | caxDw
deriving DecidableEq, Repr

def RefuteRule.name : RefuteRule → String
  | .caxDw => "cax-dw"

def RefuteRule.all : List RefuteRule := [.caxDw]

def RefuteRule.ofName? (s : String) : Option RefuteRule :=
  RefuteRule.all.find? (fun r => r.name == s)

/-- The final step of a refutation. It has premises and no conclusion, because
what it concludes is contradiction.

`cax-dw` premise order, and the checker matches on it: `c1 owl:disjointWith c2`,
`x rdf:type c1`, `x rdf:type c2`. -/
structure RefuteStep where
  rule : RefuteRule
  premises : List Triple
deriving Repr

/-- A refutation: a derivation prefix in the ordinary format, then one
contradiction step. The prefix is what lets the disjoint types be derived rather
than asserted, which is the normal case in a real ontology. -/
structure Refutation where
  steps : List Step
  final : RefuteStep
deriving Repr

/-! ## The checker

`checkAll` in `Rules.lean` threads its derived set through a `Std.HashSet`,
whose `contains` the Lean kernel cannot evaluate: `String.hash` is `opaque`, so
`decide` cannot run it. `checkAllL` is the same function over a list, which the
kernel can run, so the witness file can settle acceptance and rejection by
`decide` instead of by assertion. Both are proved sound, and the CLI uses the
hashed one because it is the one that scales. -/

/-- `checkAll` with a list for the derived set. Quadratic, and the kernel can
run it. -/
def checkAllL (inG : Triple → Bool) : List Step → List Triple → Bool
  | [], _ => true
  | st :: rest, derived =>
      checkStep inG (fun t => decide (t ∈ derived)) st &&
      checkAllL inG rest (st.conclusion :: derived)

/-- Check the contradiction step. `k` answers "asserted, or concluded by the
prefix". Premise order is part of the contract, as it is for every other rule in
this checker: a refutation with the right triples in the wrong order is
rejected, which is a false alarm and never a false pass. -/
def checkRefuteStep (k : Triple → Bool) (rs : RefuteStep) : Bool :=
  match rs.rule, rs.premises with
  | .caxDw, [⟨c1, dw, c2⟩, ⟨x, t1, c1'⟩, ⟨x', t2, c2'⟩] =>
      dw = RV.disjointWith ∧ t1 = V.type ∧ c1' = c1 ∧
      x' = x ∧ t2 = V.type ∧ c2' = c2 ∧
      k ⟨c1, dw, c2⟩ ∧ k ⟨x, t1, c1'⟩ ∧ k ⟨x', t2, c2'⟩
  | _, _ => false

/-- The conclusions the prefix made available to the contradiction step. A
premise of the final step may be asserted or derived by the prefix; it may not
be assumed. -/
def prefixConclusions (r : Refutation) : List Triple := r.steps.map Step.conclusion

/-- The checker, with the "is asserted" test left abstract so that the list
version and the hashed version are the same function. -/
def checkRefutationWith (inG : Triple → Bool) (r : Refutation) : Bool :=
  checkAllL inG r.steps [] &&
  checkRefuteStep (fun t => inG t || decide (t ∈ prefixConclusions r)) r.final

/-- The kernel-runnable checker. `decide (t ∈ G)` walks the list, so this one is
linear in the size of the asserted graph per premise. The witness file uses it
because the kernel can run it; a caller with a real ontology should not. -/
def checkRefutation (G : List Triple) (r : Refutation) : Bool :=
  checkRefutationWith (fun t => decide (t ∈ G)) r

/-- The checker the CLI runs. Exactly what is hashed, and nothing more: the
ASSERTED graph, which is the part that is large. The derivation prefix is still
scanned as a list, so the check is quadratic in the NUMBER OF STEPS in the
refutation. That is deliberate rather than overlooked. A refutation carries only
the steps needed to reach one contradiction, which is a handful of `rdfs9` hops
down a subclass chain, and buying a hash table for a list of four would cost
more than it saves. A caller who writes thousand-step refutations will notice,
and should say so. -/
def checkRefutationFast (G : List Triple) (r : Refutation) : Bool :=
  let gset := Std.HashSet.ofList G
  checkRefutationWith (fun t => gset.contains t) r

/-! ## Soundness -/

theorem checkAllL_sound {G : List Triple} {P : Interp → Prop} {inG : Triple → Bool}
    (hP : ∀ I, P I → Model I G) (hG : ∀ t, inG t = true → t ∈ G) :
    ∀ (steps : List Step) (derived : List Triple),
      (∀ t ∈ derived, EntailsIn P t) →
      checkAllL inG steps derived = true →
      ∀ st ∈ steps, EntailsIn P st.conclusion := by
  intro steps
  induction steps with
  | nil =>
    intro _ _ _ st hst
    simp at hst
  | cons st rest ih =>
    intro derived hD h st' hst'
    simp only [checkAllL, Bool.and_eq_true] at h
    obtain ⟨h1, h2⟩ := h
    have hD' : ∀ t, decide (t ∈ derived) = true → EntailsIn P t := by
      intro t ht
      exact hD t (of_decide_eq_true ht)
    have hst : EntailsIn P st.conclusion := checkStep_sound hP hG hD' h1
    have hD'' : ∀ t ∈ st.conclusion :: derived, EntailsIn P t := by
      intro t ht
      rcases List.mem_cons.mp ht with rfl | ht
      · exact hst
      · exact hD t ht
    rcases List.mem_cons.mp hst' with rfl | hmem
    · exact hst
    · exact ih _ hD'' h2 st' hmem

/-- The contradiction step, discharged against `RefuteConditions.dw`. -/
theorem checkRefuteStep_sound {G : List Triple} {k : Triple → Bool} {rs : RefuteStep}
    (hk : ∀ t, k t = true → EntailsIn (RModel G) t)
    (h : checkRefuteStep k rs = true) : Unsat G := by
  obtain ⟨rule, premises⟩ := rs
  unfold checkRefuteStep at h
  dsimp only at h
  cases rule
  case caxDw =>
    rcases premises with
      _ | ⟨⟨c1, dw, c2⟩, _ | ⟨⟨x, t1, c1'⟩, _ | ⟨⟨x', t2, c2'⟩, _ | _⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, h1, h2, h3⟩ := h
    intro I hI
    exact hI.2.dw _ _ (hk _ h1 I hI) _ (hk _ h2 I hI) (hk _ h3 I hI)

/-- **The theorem the refutation checker is trusted for.** An accepted
refutation means no interpretation models `G` and respects disjointness.

Conditional on exactly one thing, and it is named in the statement rather than
in a footnote: the reading of `owl:disjointWith` given by `RefuteConditions`.
Drop that reading and the graph has models again, `saturated` among them. -/
theorem refutation_sound_with {G : List Triple} {inG : Triple → Bool}
    (hG : ∀ t, inG t = true → t ∈ G) (r : Refutation)
    (h : checkRefutationWith inG r = true) : Unsat G := by
  unfold checkRefutationWith at h
  simp only [Bool.and_eq_true] at h
  obtain ⟨h1, h2⟩ := h
  have hsteps : ∀ st ∈ r.steps, EntailsIn (RModel G) st.conclusion :=
    checkAllL_sound (P := RModel G) (fun _ hI => hI.1) hG r.steps [] (by intro t ht; simp at ht) h1
  refine checkRefuteStep_sound (k := fun t => inG t || decide (t ∈ prefixConclusions r)) ?_ h2
  intro t ht
  simp only [Bool.or_eq_true, decide_eq_true_eq] at ht
  rcases ht with hx | hx
  · exact fun I hI => hI.1.facts t (hG t hx)
  · obtain ⟨st, hst, rfl⟩ := List.mem_map.mp hx
    exact hsteps st hst

/-- The kernel-runnable checker is sound. -/
theorem refutation_sound (G : List Triple) (r : Refutation)
    (h : checkRefutation G r = true) : Unsat G :=
  refutation_sound_with (fun _ ht => of_decide_eq_true ht) r h

/-- The hashed checker the CLI runs is sound, by the same theorem. -/
theorem refutation_fast_sound (G : List Triple) (r : Refutation)
    (h : checkRefutationFast G r = true) : Unsat G := by
  refine refutation_sound_with (inG := fun t => (Std.HashSet.ofList G).contains t) ?_ r h
  intro t ht
  rw [Std.HashSet.contains_ofList] at ht
  simpa using ht

/-! ## What a refutation costs the triple certificates over the same graph -/

/-- Once `G` is refuted, every triple has the disjointness-aware warrant. -/
theorem unsat_entails_everything {G : List Triple} (h : Unsat G) (t : Triple) :
    EntailsIn (RModel G) t := fun I hI => absurd hI (h I)

/-- **Why `oo-refute guard` refuses.** Over a refuted graph, EVERY conclusion of
EVERY list of steps already has the disjointness-aware warrant, whether the
checker accepted the certificate or not, whether the steps are derivations or
forgeries. Running the derivation checker on such a graph therefore establishes
nothing that was not already true of an empty file. -/
theorem a_certificate_adds_nothing_when_the_graph_is_refuted {G : List Triple}
    (h : Unsat G) (steps : List Step) :
    ∀ st ∈ steps, EntailsIn (RModel G) st.conclusion :=
  fun st _ => unsat_entails_everything h st.conclusion

/-! ## The replacement for the lost witness, in its general form

`saturated_is_a_model` is not available here, so the non-vacuity of
`refutation_sound` has to be re-earned. If nothing ever satisfied `RModel G`,
`Unsat G` would hold for every graph and the theorem above would say nothing.

`joint_model_of_closure` is the general shape of the answer, and it is stated
without mentioning any particular condition of `Model`, so it does not move when
`Semantics.lean` gains a rule: hand it ANY triple-predicate `D` whose Herbrand
interpretation models `G`, plus the fact that `D` contains no disjointness
violation, and it hands back a joint model. `RefuteWitness.lean` supplies such a
`D` for an arbitrary graph, by induction, and discharges it concretely. -/

/-- Terms denote themselves and a property relates exactly the pairs the
predicate `D` holds of. The list-valued `herbrand` of `Witness.lean` is the
special case `D t := t ∈ H`, and the predicate version is what lets the closure
of an infinite or inductively defined set be used. -/
def herbrandP (D : Triple → Prop) : Interp where
  D := Term
  ι := id
  iext := fun p x y => D ⟨x, p, y⟩

@[simp] theorem herbrandP_iext (D : Triple → Prop) (p x y : Term) :
    (herbrandP D).iext p x y ↔ D ⟨x, p, y⟩ := Iff.rfl

@[simp] theorem herbrandP_sat (D : Triple → Prop) (t : Triple) :
    (herbrandP D).sat t ↔ D t := Iff.rfl

@[simp] theorem herbrandP_cext (D : Triple → Prop) (c x : Term) :
    (herbrandP D).cext c x ↔ D ⟨x, V.type, c⟩ := Iff.rfl

/-- The finite-list special case. This is `Witness.lean`'s `herbrand` with the
membership predicate written through `herbrandP`, definitionally the same
interpretation, and it is spelled again here so that this layer's witnesses
depend on `Semantics.lean` and nothing else. -/
def herbrandL (H : List Triple) : Interp := herbrandP (fun t => t ∈ H)

@[simp] theorem herbrandL_sat (H : List Triple) (t : Triple) :
    (herbrandL H).sat t ↔ t ∈ H := Iff.rfl

@[simp] theorem herbrandL_cext (H : List Triple) (c x : Term) :
    (herbrandL H).cext c x ↔ (⟨x, V.type, c⟩ : Triple) ∈ H := Iff.rfl

/-- **A closure with no disjointness violation is a joint model.** -/
theorem joint_model_of_closure {G : List Triple} {D : Triple → Prop}
    (hD : Model (herbrandP D) G)
    (hdw : ∀ c1 c2 x, D ⟨c1, RV.disjointWith, c2⟩ → D ⟨x, V.type, c1⟩ →
      D ⟨x, V.type, c2⟩ → False) :
    ∃ I : Interp, Model I G ∧ RefuteConditions I :=
  ⟨herbrandP D, hD, ⟨fun a b hab x h1 h2 => hdw a b x hab h1 h2⟩⟩

/-- A graph with a joint model is not refutable, so `refutation_sound` is not a
theorem about an empty model class.

**Relative to `Conditions` and `RefuteConditions`, and it is not itself the
statement over the OWL 2 RDF-Based Semantics conditions.** This note is here rather than only in
`Semantics.lean` because a caveat that lives in one file is a caveat that goes
stale; until 15 September 2026 this one did, and this file was byte-identical to
the version that predates `lean/OOCert/W3C.lean`.

The direction is what stops it. `Unsat G` quantifies negatively over a model
class, so shrinking the class to `W3CModel` makes `Unsat` EASIER and `¬ Unsat`
HARDER. The consequence runs the useful way for the positive result and the
useless way for this one: `refutation_sound`'s `Unsat G` does transfer, because
every `W3CModel` of `G` is a `Model` of `G`, so an accepted refutation rules out
conforming interpretations too. A `¬ Unsat` needs a `W3CModel` that also
satisfies `RefuteConditions`, and neither this lemma nor
`RefuteWitness.lean`'s `feed_is_not_refuted` supplies one.

`RefuteWitness.lean` now does supply one, and this lemma is the shape it takes
over there: `W3CUnsat` is `Unsat` over the smaller class, `w3cUnsat_of_unsat`
is the transfer that does run, `not_w3cUnsat_of_joint_w3c_model` is this lemma's
counterpart, and `feed_is_not_w3c_refuted` is the result. The obstruction that
used to be recorded here, that `feedClosure` types nothing as an `rdfs:Class` and
so `W3C.sc_fwd` fails on it, was a fact about the HERBRAND witness and not about
the statement; a thirteen-element model built for it settles the statement. -/
theorem not_unsat_of_joint_model {G : List Triple}
    (h : ∃ I : Interp, Model I G ∧ RefuteConditions I) : ¬ Unsat G := by
  obtain ⟨I, hM, hR⟩ := h
  intro hu
  exact hu I ⟨hM, hR⟩

/-! ## Reading a refutation file

Not part of the proof. A parse error is a rejected file, never an accepted
refutation. -/
namespace RefuteParse

def version : String := "oo-refute/1"

/-- The version marker, as it may arrive. A file written on Windows ends its
lines with a carriage return and the marker would then never match. Nothing
else in the format is trimmed: a term's spelling is its identity, and stripping
whitespace from one would be a silent rewrite of the certificate. -/
def isVersionLine (s : String) : Bool := s == version || s == version ++ "\r"

/-- Three tab fields at a time into triples, as `Parse.groupTriples` does for
the derivation format. -/
def groupTriples : List String → Option (List Triple)
  | [] => some []
  | s :: p :: o :: rest => (groupTriples rest).map (⟨s, p, o⟩ :: ·)
  | _ => none

/-- Parse one contradiction line, already split on tabs and with the leading
`refute` removed. An unimplemented clash rule gets its own message: naming it
"bad syntax" would hide the real answer, which is that the checker has no
condition for that rule and cannot judge a refutation citing it. -/
def refuteStepOf (fields : List String) : Except String RefuteStep :=
  match fields with
  | rule :: rest =>
      match RefuteRule.ofName? rule with
      | none =>
          .error s!"unknown or unimplemented clash rule '{rule}'. This checker implements \
            {String.intercalate ", " (RefuteRule.all.map RefuteRule.name)} and nothing else; \
            the other OWL 2 RL clash rules have no semantic condition here, so a refutation \
            citing one cannot be judged and is refused rather than guessed at."
      | some r =>
          match groupTriples rest with
          | some ps => .ok ⟨r, ps⟩
          | none => .error "the premises of the contradiction step are not whole triples"
  | _ => .error "the contradiction step names no rule"

def stepOf? (fields : List String) : Option Step :=
  match fields with
  | rule :: s :: p :: o :: prem =>
      match Rule.ofName? rule, groupTriples prem with
      | some r, some ps => some ⟨r, ps, ⟨s, p, o⟩⟩
      | _, _ => none
  | _ => none

/-- `oo-refute/1`, then derivation steps, then exactly one `refute` line, last. -/
def parseRefutation (content : String) : Except String Refutation := do
  let lines := (content.splitOn "\n").filter (fun l => !l.isEmpty)
  match lines with
  | [] => throw "refutation: the file is empty; expected the line 'oo-refute/1'"
  | header :: body =>
    if !isVersionLine header then
      throw s!"refutation: first line is '{header}', expected '{version}'. \
        A derivations.tsv is not a refutation and is refused here on purpose."
    let mut steps : Array Step := #[]
    let mut final : Option RefuteStep := none
    let mut n := 1
    for line in body do
      n := n + 1
      if final.isSome then
        throw s!"refutation line {n}: the contradiction step must be the last line"
      match line.splitOn "\t" with
      | "refute" :: rest =>
        match refuteStepOf rest with
        | .ok rs => final := some rs
        | .error e => throw s!"refutation line {n}: {e}"
      | fields =>
        match stepOf? fields with
        | some st => steps := steps.push st
        | none => throw s!"refutation line {n}: bad derivation step; expected \
            'rule TAB s TAB p TAB o' then premises in whole triples"
    match final with
    | some rs => return ⟨steps.toList, rs⟩
    | none => throw "refutation: no 'refute' line. A refutation must reach a \
        contradiction; a file of derivations alone proves nothing here."

end RefuteParse

/-! ## Axioms, pinned

The same tripwire `Soundness.lean` and `Horn.lean` carry. A `sorry` or a
`native_decide` under any of these fails the build here. `joint_model_of_closure`
needs neither choice nor quotients: it is a construction.

The list checker needs no choice. The hashed one does, and only because
`Std.HashSet.contains_ofList` does. A SHORTER footprint than the house one is
the safe direction; a longer one is the alarm. -/

/-- info: 'OOCert.refutation_sound' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms refutation_sound

/-- info: 'OOCert.refutation_fast_sound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms refutation_fast_sound

/-- info: 'OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted' does not depend on any axioms -/
#guard_msgs in
#print axioms a_certificate_adds_nothing_when_the_graph_is_refuted

/-- info: 'OOCert.joint_model_of_closure' does not depend on any axioms -/
#guard_msgs in
#print axioms joint_model_of_closure

end OOCert
