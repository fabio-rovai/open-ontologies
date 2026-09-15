import OOCert.Semantics

/-!
# Witnesses: the semantics has models, and it does not prove everything

`certificate_sound` says an accepted certificate contains only entailed triples. That claim is worth
nothing on its own, and this file closes the two ways it could be hollow by construction rather than
by argument.

1. **If no interpretation satisfied `Conditions`**, `Entails G t` would hold for every `t` and the
   soundness theorem would be vacuously true. `saturated_is_a_model` exhibits a model of an
   arbitrary graph, so the conditions are satisfiable and no graph is inconsistent here. That is the
   expected shape rather than a surprise: the fragment has no negation, so nothing can contradict
   anything.

2. **If `Entails` held of everything anyway**, soundness would still say nothing.
   `not_everything_is_entailed` exhibits a triple that is not entailed.

Three later sections do the same job for the conditions the restriction-ordering and the list rules
added, and each is a PAIR: what the rule draws, proved for an arbitrary model, next to something the
same premises do not give, refuted by a model of them.

| pair | says |
|---|---|
| `the_avf2_direction_the_table_gives_is_entailed` | `scm-avf2` concludes `c2 subClassOf c1` |
| `the_natural_avf2_direction_is_not_entailed` | and `c1 subClassOf c2`, the direction the other three restriction rules run in, is refuted |
| `an_enumerated_member_is_entailed` | `cls-oo` types the members an `owl:oneOf` list names |
| `an_unlisted_individual_is_not_entailed` | and nothing else, because the condition is the "at least" half only |
| `a_member_of_the_intersection_is_entailed` | `cls-int2` takes an intersection apart |
| `membership_in_one_member_does_not_give_the_intersection` | and `cls-int1` still needs every member, so the new condition did not swallow the old one |

The `scm-avf2` pair is the one that earns its place. Three of the four restriction-ordering rules
conclude `c1 subClassOf c2` and that one concludes `c2 subClassOf c1`, so copying the pattern gives
a rule with no model behind it. Nothing downstream would notice: the certificate would check, the
Lean proof would be the only thing that failed, and here it is made to fail on purpose.

The third theorem of the original set is the one worth having.
`the_old_svf_derivation_is_not_entailed` proves that the
derivation the reasoner used to make, `x rdf:type C` from `C rdfs:subClassOf (some p D)` together
with `x p y` and `y rdf:type D`, is **not** a consequence: there is a model of the premises in which
`x` is not a `C`. So that behaviour was unsound in fact, not merely unjustified by the rule set this
checker implements, and `tests/reason_rl_ext_soundness_test.rs` pins a real defect.
`the_sound_half_survives` proves the fix did not overshoot: what the reasoner still derives on the
same input is entailed.

The witness is the Herbrand interpretation of a finite graph: the domain is the set of terms, every
term denotes itself, and a property's extension is the set of pairs the list carries. Every side
condition is `decide`-checked over concrete term strings, so these are executable facts.
-/
namespace OOCert

/-! ## The semantics is satisfiable -/

/-- The interpretation in which every property relates everything. -/
def saturated : Interp where
  D := Unit
  ι := fun _ => ()
  iext := fun _ _ _ => True

/-- Every graph has a model, so `Entails` is never the "anything follows" of an inconsistent
premise set.

`saturated` also satisfies `W3C.lean`'s specification conditions, with
`IP := fun _ => True`, because both sides of every equality there are all of
`Unit`. That is worth knowing and worth NOT reporting as a result: a model that
makes every relation total distinguishes no condition from any other and refutes
nothing, so bare satisfiability of the stronger structure says exactly as little
as bare satisfiability of this one. The non-degenerate witness is
`W3CWitness.lean`'s `live`, and `live_is_live` is what makes it non-degenerate. -/
theorem saturated_is_a_model (G : List Triple) : Model saturated G where
  conds :=
    { sc_sub := fun _ _ _ _ _ => trivial
      sc_trans := fun _ _ _ _ _ => trivial
      sp_sub := fun _ _ _ _ _ _ => trivial
      sp_trans := fun _ _ _ _ _ => trivial
      dom := fun _ _ _ _ _ _ => trivial
      rng := fun _ _ _ _ _ _ => trivial
      trp := fun _ _ _ _ _ _ _ => trivial
      symp := fun _ _ _ _ _ => trivial
      inv := fun _ _ _ _ _ => Iff.intro (fun _ => trivial) (fun _ => trivial)
      same := fun a b _ => by cases a; cases b; rfl
      eqc := fun _ _ _ => ⟨trivial, trivial⟩
      eqp := fun _ _ _ => ⟨trivial, trivial⟩
      svf := fun _ _ _ _ _ _ _ _ _ => trivial
      avf := fun _ _ _ _ _ _ _ _ _ => trivial
      hv := fun _ _ _ _ _ _ => Iff.intro (fun _ => trivial) (fun _ => trivial)
      svf_sc := by intros; trivial
      svf_sp := by intros; trivial
      avf_sc := by intros; trivial
      avf_sp := by intros; trivial
      dom_sc := by intros; trivial
      dom_sp := by intros; trivial
      rng_sc := by intros; trivial
      rng_sp := by intros; trivial }
  facts := fun _ _ => trivial
  int := fun _ _ _ _ _ _ _ => trivial
  int2 := by intros; trivial
  uni := fun _ _ _ _ _ _ _ _ _ => trivial
  oneOf := by intros; trivial

/-! ## Herbrand interpretations -/

/-- Terms denote themselves; a property relates exactly the pairs the graph lists. -/
def herbrand (H : List Triple) : Interp where
  D := Term
  ι := id
  iext := fun p x y => (⟨x, p, y⟩ : Triple) ∈ H

@[simp] theorem herbrand_ι (H : List Triple) (t : Term) : (herbrand H).ι t = t := rfl

@[simp] theorem herbrand_iext (H : List Triple) (p x y : Term) :
    (herbrand H).iext p x y ↔ (⟨x, p, y⟩ : Triple) ∈ H := Iff.rfl

@[simp] theorem herbrand_sat (H : List Triple) (t : Triple) :
    (herbrand H).sat t ↔ t ∈ H := Iff.rfl

@[simp] theorem herbrand_cext (H : List Triple) (c x : Term) :
    (herbrand H).cext c x ↔ (⟨x, V.type, c⟩ : Triple) ∈ H := Iff.rfl

@[simp] theorem herbrand_sc (H : List Triple) (a b : Term) :
    (herbrand H).sc a b ↔ (⟨a, V.subClassOf, b⟩ : Triple) ∈ H := Iff.rfl

@[simp] theorem herbrand_sp (H : List Triple) (a b : Term) :
    (herbrand H).sp a b ↔ (⟨a, V.subPropertyOf, b⟩ : Triple) ∈ H := Iff.rfl

/-- No triple of `H` uses predicate `p`, so nothing is in `p`'s extension. `H` and `p` are explicit:
left implicit, unification instantiates `p` with the unreduced `(herbrand H).ι V.foo`, and the side
condition stops being a decidable proposition. -/
theorem not_mem_pred (H : List Triple) (p : Term) (h : ∀ t ∈ H, t.p ≠ p) (s o : Term) :
    (⟨s, p, o⟩ : Triple) ∉ H := fun hm => h _ hm rfl

/-- Nothing in `H` is typed `c`, so `c`'s class extension is empty. -/
theorem not_typed (H : List Triple) (c : Term) (h : ∀ t ∈ H, ¬(t.p = V.type ∧ t.o = c)) (x : Term) :
    (⟨x, V.type, c⟩ : Triple) ∉ H := fun hm => h _ hm ⟨rfl, rfl⟩

/-- The Herbrand interpretation of the empty graph: every condition holds vacuously and no triple
is satisfied. -/
theorem empty_herbrand_is_a_model : Model (herbrand []) [] where
  conds :=
    { sc_sub := fun a b hab => absurd hab (not_mem_pred [] V.subClassOf (by simp) a b)
      sc_trans := fun a b _ hab => absurd hab (not_mem_pred [] V.subClassOf (by simp) a b)
      sp_sub := fun a b hab => absurd hab (not_mem_pred [] V.subPropertyOf (by simp) a b)
      sp_trans := fun a b _ hab => absurd hab (not_mem_pred [] V.subPropertyOf (by simp) a b)
      dom := fun p c hpc => absurd hpc (not_mem_pred [] V.domain (by simp) p c)
      rng := fun p c hpc => absurd hpc (not_mem_pred [] V.range (by simp) p c)
      trp := fun p hp => absurd hp (not_typed [] V.transitiveProperty (by simp) p)
      symp := fun p hp => absurd hp (not_typed [] V.symmetricProperty (by simp) p)
      inv := fun p q hpq => absurd hpq (not_mem_pred [] V.inverseOf (by simp) p q)
      same := fun a b hab => absurd hab (not_mem_pred [] V.sameAs (by simp) a b)
      eqc := fun a b hab => absurd hab (not_mem_pred [] V.equivalentClass (by simp) a b)
      eqp := fun a b hab => absurd hab (not_mem_pred [] V.equivalentProperty (by simp) a b)
      svf := fun r p _ hop => absurd hop (not_mem_pred [] V.onProperty (by simp) r p)
      avf := fun r p _ hop => absurd hop (not_mem_pred [] V.onProperty (by simp) r p)
      hv := fun r p _ hop => absurd hop (not_mem_pred [] V.onProperty (by simp) r p)
      svf_sc := fun c1 _ _ y1 _ hsv =>
        absurd hsv (not_mem_pred [] V.someValuesFrom (by simp) c1 y1)
      svf_sp := fun c1 _ _ _ y hsv =>
        absurd hsv (not_mem_pred [] V.someValuesFrom (by simp) c1 y)
      avf_sc := fun c1 _ _ y1 _ hav =>
        absurd hav (not_mem_pred [] V.allValuesFrom (by simp) c1 y1)
      avf_sp := fun c1 _ _ _ y hav =>
        absurd hav (not_mem_pred [] V.allValuesFrom (by simp) c1 y)
      dom_sc := fun p c1 _ hd => absurd hd (not_mem_pred [] V.domain (by simp) p c1)
      dom_sp := fun _ p2 c hd => absurd hd (not_mem_pred [] V.domain (by simp) p2 c)
      rng_sc := fun p c1 _ hr => absurd hr (not_mem_pred [] V.range (by simp) p c1)
      rng_sp := fun _ p2 c hr => absurd hr (not_mem_pred [] V.range (by simp) p2 c) }
  facts := by intro t ht; simp at ht
  int := by intro c l _ hc; exact absurd hc (not_mem_pred [] V.intersectionOf (by simp) c l)
  int2 := by intro c l _ hc; exact absurd hc (not_mem_pred [] V.intersectionOf (by simp) c l)
  uni := by intro c l _ hc; exact absurd hc (not_mem_pred [] V.unionOf (by simp) c l)
  oneOf := by intro c l _ hc; exact absurd hc (not_mem_pred [] V.oneOf (by simp) c l)

/-- Entailment is not trivial.

This one transfers to the `W3CModel` class unchanged, and
`W3CWitness.lean`'s `not_everything_is_w3c_entailed` is the transfer: over the
empty graph every antecedent of `W3C` is false and `IP := fun _ => False` makes
the four backward conditions vacuous, so the same interpretation serves.

An earlier version of this docstring added that it was "the only non-entailment
in this file that transfers without a new hand-built structure". It is not. Two
of the three below transfer the same way, with their own witness graphs
unchanged, and the proofs are `an_unlisted_individual_is_not_w3c_entailed` and
`membership_in_one_member_is_not_w3c_enough`. -/
theorem not_everything_is_entailed :
    ¬ Entails [] ⟨"<http://ex.org/a>", "<http://ex.org/b>", "<http://ex.org/c>"⟩ := by
  intro h
  have hs := h (herbrand []) empty_herbrand_is_a_model
  simp at hs

/-! ## The derivation the old `cls-svf1` made, refuted

`svfPremises` is the ontology from `tests/reason_rl_ext_soundness_test.rs`:
`C rdfs:subClassOf R`, `R owl:onProperty p`, `R owl:someValuesFrom D`, `x p y`, `y rdf:type D`.
The reasoner used to conclude `x rdf:type C` from it.

`svfWitness` adds the one triple the semantics does force, `x rdf:type R`, and nothing else. Its
Herbrand interpretation models the premises and refutes the conclusion.
-/

def tC : Term := "<http://ex.org/C>"
def tR : Term := "<http://ex.org/R>"
def tD : Term := "<http://ex.org/D>"
def tp : Term := "<http://ex.org/p>"
def tx : Term := "<http://ex.org/x>"
def ty : Term := "<http://ex.org/y>"

def svfPremises : List Triple :=
  [ ⟨tC, V.subClassOf, tR⟩,
    ⟨tR, V.onProperty, tp⟩,
    ⟨tR, V.someValuesFrom, tD⟩,
    ⟨tx, tp, ty⟩,
    ⟨ty, V.type, tD⟩ ]

/-- The premises plus the one consequence the `svf` condition forces, `x ∈ (some p D)`. Nothing
puts `x` in `C`, which is the whole point. -/
def svfWitness : List Triple := ⟨tx, V.type, tR⟩ :: svfPremises

/-! The three side conditions the witness graph has to satisfy, each a closed statement over a
six-triple list and therefore settled by `decide` rather than by a tactic script. Stating them this
way keeps the model proof free of membership case analysis, which is where the string literals
would otherwise have to be compared by hand. -/

/-- Nothing is an instance of a class that has a superclass: the only `rdfs:subClassOf` triple has
subject `C`, and nothing is typed `C`. This is what makes `sc_sub` hold vacuously. -/
private theorem no_typed_subclass :
    ∀ t ∈ svfWitness, ∀ u ∈ svfWitness, ¬(t.p = V.subClassOf ∧ u.p = V.type ∧ u.o = t.s) := by
  decide

/-- No two `rdfs:subClassOf` triples compose, so `sc_trans` holds vacuously. -/
private theorem no_sc_chain :
    ∀ t ∈ svfWitness, ∀ u ∈ svfWitness, ¬(t.p = V.subClassOf ∧ u.p = V.subClassOf ∧ u.s = t.o) := by
  decide

/-- The one existential witness the graph does contain is already recorded: whenever the graph has
`r owl:onProperty p`, `r owl:someValuesFrom c`, `x p y` and `y rdf:type c`, it also has
`x rdf:type r`. This is the `svf` condition, and it is the only condition the witness graph
satisfies non-vacuously. -/
private theorem svf_closed :
    ∀ t ∈ svfWitness, ∀ u ∈ svfWitness, ∀ v ∈ svfWitness, ∀ w ∈ svfWitness,
      (t.p = V.onProperty ∧ u.p = V.someValuesFrom ∧ u.s = t.s ∧
       v.p = t.o ∧ w.p = V.type ∧ w.o = u.o ∧ w.s = v.o) →
      (⟨v.s, V.type, t.s⟩ : Triple) ∈ svfWitness := by
  decide

/-- The one `owl:someValuesFrom` filler in this graph is `D`, and the one class
with a superclass is `C`, so no filler is ever the subject of a
`rdfs:subClassOf` triple. That is what makes `svf_sc` hold vacuously here. -/
private theorem no_svf_filler_is_a_subclass :
    ∀ t ∈ svfWitness, ∀ u ∈ svfWitness,
      ¬(t.p = V.someValuesFrom ∧ u.p = V.subClassOf ∧ u.s = t.o) := by
  decide

theorem svf_witness_is_a_model : Model (herbrand svfWitness) svfPremises where
  conds :=
    { sc_sub := fun a b hab x hx =>
        absurd ⟨rfl, rfl, rfl⟩ (no_typed_subclass ⟨a, V.subClassOf, b⟩ hab ⟨x, V.type, a⟩ hx)
      sc_trans := fun a b c hab hbc =>
        absurd ⟨rfl, rfl, rfl⟩
          (no_sc_chain ⟨a, V.subClassOf, b⟩ hab ⟨b, V.subClassOf, c⟩ hbc)
      sp_sub := fun a b hab => absurd hab (not_mem_pred svfWitness V.subPropertyOf (by decide) a b)
      sp_trans := fun a b _ hab =>
        absurd hab (not_mem_pred svfWitness V.subPropertyOf (by decide) a b)
      dom := fun p c hpc => absurd hpc (not_mem_pred svfWitness V.domain (by decide) p c)
      rng := fun p c hpc => absurd hpc (not_mem_pred svfWitness V.range (by decide) p c)
      trp := fun p hp => absurd hp (not_typed svfWitness V.transitiveProperty (by decide) p)
      symp := fun p hp => absurd hp (not_typed svfWitness V.symmetricProperty (by decide) p)
      inv := fun p q hpq => absurd hpq (not_mem_pred svfWitness V.inverseOf (by decide) p q)
      same := fun a b hab => absurd hab (not_mem_pred svfWitness V.sameAs (by decide) a b)
      eqc := fun a b hab => absurd hab (not_mem_pred svfWitness V.equivalentClass (by decide) a b)
      eqp := fun a b hab =>
        absurd hab (not_mem_pred svfWitness V.equivalentProperty (by decide) a b)
      svf := fun r p c hop hsv x y hxy hy =>
        svf_closed ⟨r, V.onProperty, p⟩ hop ⟨r, V.someValuesFrom, c⟩ hsv
          ⟨x, p, y⟩ hxy ⟨y, V.type, c⟩ hy ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩
      avf := fun r _ c _ hav =>
        absurd hav (not_mem_pred svfWitness V.allValuesFrom (by decide) r c)
      hv := fun r _ v _ hhv => absurd hhv (not_mem_pred svfWitness V.hasValue (by decide) r v)
      svf_sc := fun c1 _ _ y1 y2 hsv _ _ _ hsc =>
        absurd ⟨rfl, rfl, rfl⟩
          (no_svf_filler_is_a_subclass ⟨c1, V.someValuesFrom, y1⟩ hsv ⟨y1, V.subClassOf, y2⟩ hsc)
      svf_sp := fun _ _ p1 p2 _ _ _ _ _ hsp =>
        absurd hsp (not_mem_pred svfWitness V.subPropertyOf (by decide) p1 p2)
      avf_sc := fun c1 _ _ y1 _ hav =>
        absurd hav (not_mem_pred svfWitness V.allValuesFrom (by decide) c1 y1)
      avf_sp := fun c1 _ _ _ y hav =>
        absurd hav (not_mem_pred svfWitness V.allValuesFrom (by decide) c1 y)
      dom_sc := fun p c1 _ hd => absurd hd (not_mem_pred svfWitness V.domain (by decide) p c1)
      dom_sp := fun _ p2 c hd => absurd hd (not_mem_pred svfWitness V.domain (by decide) p2 c)
      rng_sc := fun p c1 _ hr => absurd hr (not_mem_pred svfWitness V.range (by decide) p c1)
      rng_sp := fun _ p2 c hr => absurd hr (not_mem_pred svfWitness V.range (by decide) p2 c) }
  facts := fun t ht => List.mem_cons_of_mem _ ht
  int := by intro c l _ hc; exact absurd hc (not_mem_pred svfPremises V.intersectionOf (by decide) c l)
  int2 := by intro c l _ hc; exact absurd hc (not_mem_pred svfPremises V.intersectionOf (by decide) c l)
  uni := by intro c l _ hc; exact absurd hc (not_mem_pred svfPremises V.unionOf (by decide) c l)
  oneOf := by intro c l _ hc; exact absurd hc (not_mem_pred svfPremises V.oneOf (by decide) c l)

/-- **The old rule was unsound.** `C ⊑ ∃p.D` together with `x p y` and `y ∈ D` does not entail
`x ∈ C`. The reasoner derived exactly this until 13 September 2026.

**This is the one non-entailment in this file whose own witness is not a
`W3CModel`**, and the field that stops it is checked rather than argued.

`onp_typ` is RBS Table 5.3's `owl:onProperty` row, whose first conjunct is
`z ∈ ICEXT(owl:Restriction)`. `svfWitness` carries `R owl:onProperty p` and types
`R` as nothing at all, so the antecedent holds and the consequent fails.
`svf_typ` fails the same way on `R owl:someValuesFrom D`, and `sc_fwd`, Table 5.8
row 1 forward, fails on `C rdfs:subClassOf R` because `IC` is empty here, no
triple having `rdfs:Class` as its object. None of those three fields mentions
`IP`, so the escape that works for the other three witnesses in this repository,
choosing `IP := fun _ => False` to make the backward conditions vacuous, does not
apply.

That is a fact about THIS witness and it was once written up as though it were a
fact about the statement. It is not: a witness that is not a `W3CModel` is a
reason to build one. `W3CWitness.lean`'s `svfI` is that structure and
`the_old_svf_derivation_is_not_w3c_entailed` is the resulting theorem, which is
strictly stronger than this one.

An earlier version of this paragraph gave a different and wrong reason for the
witness failing: that bridge coherence forces every predicate the witness uses
into `IP` and `sp_bwd` then demands reflexive `rdfs:subPropertyOf` triples.
Bridge coherence is a property `W3CWitness.lean`'s models have, not a field of
`W3CModel`, and `IP` is free.

**The claim itself is not in doubt and must not be withdrawn on the strength of
this note.** `tests/reason_rl_ext_soundness_test.rs` pins a real defect, and the
question that used to be open here, whether a structure meeting the quoted table
cells also refutes it, is answered yes by `svfI`. -/
theorem the_old_svf_derivation_is_not_entailed :
    ¬ Entails svfPremises ⟨tx, V.type, tC⟩ := fun h =>
  absurd (h (herbrand svfWitness) svf_witness_is_a_model)
    (not_typed svfWitness tC (by decide) tx)

/-- The fix did not overshoot into deriving nothing: what the reasoner still concludes on the same
input, `x ∈ (some p D)`, is entailed. Proved for an arbitrary model, not just the witness. -/
theorem the_sound_half_survives : Entails svfPremises ⟨tx, V.type, tR⟩ := by
  intro I M
  have hop := M.facts ⟨tR, V.onProperty, tp⟩ (by simp [svfPremises])
  have hsv := M.facts ⟨tR, V.someValuesFrom, tD⟩ (by simp [svfPremises])
  have hxy := M.facts ⟨tx, tp, ty⟩ (by simp [svfPremises])
  have hy := M.facts ⟨ty, V.type, tD⟩ (by simp [svfPremises])
  exact M.conds.svf _ _ _ hop hsv _ _ hxy hy

/-! ## `scm-avf2` runs one way and not the other

`scm-avf1`, `scm-svf1` and `scm-svf2` conclude `?c1 rdfs:subClassOf ?c2`.
`scm-avf2` concludes `?c2 rdfs:subClassOf ?c1`. Copying the shape of the other
three is the mistake this section is here to make expensive: the wrong
direction has no model behind it, every certificate citing it would check, and
nothing else in the pipeline would notice.

The reason is that a universal restriction is antitone in its property. If
`p1` is a subproperty of `p2` then everything an individual reaches by `p1` it
also reaches by `p2`, so being in `all p2 Y` is the harder condition and
`all p2 Y` is the smaller class.

`avfPremises` is that configuration. `avfWitness` adds the one subsumption the
semantics forces and nothing else. -/

def tY : Term := "<e:Y>"
def tC1 : Term := "<e:C1>"
def tC2 : Term := "<e:C2>"
def tp1 : Term := "<e:p1>"
def tp2 : Term := "<e:p2>"

/-- `C1 = all p1 Y`, `C2 = all p2 Y`, and `p1 rdfs:subPropertyOf p2`. -/
def avfPremises : List Triple :=
  [ ⟨tC1, V.allValuesFrom, tY⟩,
    ⟨tC1, V.onProperty, tp1⟩,
    ⟨tC2, V.allValuesFrom, tY⟩,
    ⟨tC2, V.onProperty, tp2⟩,
    ⟨tp1, V.subPropertyOf, tp2⟩ ]

/-- The premises plus `C2 rdfs:subClassOf C1`, which is what `scm-avf2` draws.
`C1 rdfs:subClassOf C2` is absent, and that absence is the theorem. -/
def avfWitness : List Triple := ⟨tC2, V.subClassOf, tC1⟩ :: avfPremises

/-! The decidable facts the model proof reads off the six-triple list. -/

private theorem avf_only_sp :
    ∀ t ∈ avfWitness, t.p = V.subPropertyOf → t.s = tp1 ∧ t.o = tp2 := by decide
private theorem avf_op_p1 :
    ∀ t ∈ avfWitness, t.p = V.onProperty → t.o = tp1 → t.s = tC1 := by decide
private theorem avf_op_p2 :
    ∀ t ∈ avfWitness, t.p = V.onProperty → t.o = tp2 → t.s = tC2 := by decide
private theorem avf_no_sc_chain :
    ∀ t ∈ avfWitness, ∀ u ∈ avfWitness,
      ¬(t.p = V.subClassOf ∧ u.p = V.subClassOf ∧ u.s = t.o) := by decide
private theorem avf_no_sp_chain :
    ∀ t ∈ avfWitness, ∀ u ∈ avfWitness,
      ¬(t.p = V.subPropertyOf ∧ u.p = V.subPropertyOf ∧ u.s = t.o) := by decide
/-- Nothing in the graph uses `p1` as a predicate, so `sp_sub` is vacuous. -/
private theorem avf_subproperty_unused :
    ∀ t ∈ avfWitness, ∀ u ∈ avfWitness, ¬(t.p = V.subPropertyOf ∧ u.p = t.s) := by decide
/-- The one `owl:allValuesFrom` filler is `Y`, and the one class with a
superclass is `C2`, so no filler is the subject of a `rdfs:subClassOf` triple
and `avf_sc` is vacuous here. -/
private theorem avf_no_filler_is_a_subclass :
    ∀ t ∈ avfWitness, ∀ u ∈ avfWitness,
      ¬(t.p = V.allValuesFrom ∧ u.p = V.subClassOf ∧ u.s = t.o) := by decide

theorem avf_witness_is_a_model : Model (herbrand avfWitness) avfPremises where
  conds :=
    { sc_sub := fun a _ _ x hx => absurd hx (not_mem_pred avfWitness V.type (by decide) x a)
      sc_trans := fun a b c hab hbc =>
        absurd ⟨rfl, rfl, rfl⟩
          (avf_no_sc_chain ⟨a, V.subClassOf, b⟩ hab ⟨b, V.subClassOf, c⟩ hbc)
      sp_sub := fun a b hab x y hxy =>
        absurd ⟨rfl, rfl⟩
          (avf_subproperty_unused ⟨a, V.subPropertyOf, b⟩ hab ⟨x, a, y⟩ hxy)
      sp_trans := fun a b c hab hbc =>
        absurd ⟨rfl, rfl, rfl⟩
          (avf_no_sp_chain ⟨a, V.subPropertyOf, b⟩ hab ⟨b, V.subPropertyOf, c⟩ hbc)
      dom := fun p c hpc => absurd hpc (not_mem_pred avfWitness V.domain (by decide) p c)
      rng := fun p c hpc => absurd hpc (not_mem_pred avfWitness V.range (by decide) p c)
      trp := fun p hp => absurd hp (not_typed avfWitness V.transitiveProperty (by decide) p)
      symp := fun p hp => absurd hp (not_typed avfWitness V.symmetricProperty (by decide) p)
      inv := fun p q hpq => absurd hpq (not_mem_pred avfWitness V.inverseOf (by decide) p q)
      same := fun a b hab => absurd hab (not_mem_pred avfWitness V.sameAs (by decide) a b)
      eqc := fun a b hab => absurd hab (not_mem_pred avfWitness V.equivalentClass (by decide) a b)
      eqp := fun a b hab =>
        absurd hab (not_mem_pred avfWitness V.equivalentProperty (by decide) a b)
      svf := fun r _ c _ hsv =>
        absurd hsv (not_mem_pred avfWitness V.someValuesFrom (by decide) r c)
      avf := fun r _ _ _ _ x _ hx => absurd hx (not_mem_pred avfWitness V.type (by decide) x r)
      hv := fun r _ v _ hhv => absurd hhv (not_mem_pred avfWitness V.hasValue (by decide) r v)
      svf_sc := fun c1 _ _ y1 _ hsv =>
        absurd hsv (not_mem_pred avfWitness V.someValuesFrom (by decide) c1 y1)
      svf_sp := fun c1 _ _ _ y hsv =>
        absurd hsv (not_mem_pred avfWitness V.someValuesFrom (by decide) c1 y)
      avf_sc := fun c1 _ _ y1 y2 hav _ _ _ hsc =>
        absurd ⟨rfl, rfl, rfl⟩
          (avf_no_filler_is_a_subclass ⟨c1, V.allValuesFrom, y1⟩ hav ⟨y1, V.subClassOf, y2⟩ hsc)
      avf_sp := by
        intro c1 c2 q1 q2 _ _ hop1 _ hop2 hsp
        obtain ⟨hs, ho⟩ := avf_only_sp ⟨q1, V.subPropertyOf, q2⟩ hsp rfl
        subst hs
        subst ho
        have h1 : c1 = tC1 := avf_op_p1 ⟨c1, V.onProperty, tp1⟩ hop1 rfl rfl
        have h2 : c2 = tC2 := avf_op_p2 ⟨c2, V.onProperty, tp2⟩ hop2 rfl rfl
        subst h1
        subst h2
        show (⟨tC2, V.subClassOf, tC1⟩ : Triple) ∈ avfWitness
        decide
      dom_sc := fun p c1 _ hd => absurd hd (not_mem_pred avfWitness V.domain (by decide) p c1)
      dom_sp := fun _ p2 c hd => absurd hd (not_mem_pred avfWitness V.domain (by decide) p2 c)
      rng_sc := fun p c1 _ hr => absurd hr (not_mem_pred avfWitness V.range (by decide) p c1)
      rng_sp := fun _ p2 c hr => absurd hr (not_mem_pred avfWitness V.range (by decide) p2 c) }
  facts := fun t ht => List.mem_cons_of_mem _ ht
  int := by
    intro c l _ hc
    exact absurd hc (not_mem_pred avfPremises V.intersectionOf (by decide) c l)
  int2 := by
    intro c l _ hc
    exact absurd hc (not_mem_pred avfPremises V.intersectionOf (by decide) c l)
  uni := by intro c l _ hc; exact absurd hc (not_mem_pred avfPremises V.unionOf (by decide) c l)
  oneOf := by intro c l _ hc; exact absurd hc (not_mem_pred avfPremises V.oneOf (by decide) c l)

/-- **The direction the W3C table gives.** Proved for an arbitrary model, not
just the witness, so it is the rule and not an accident of this graph. -/
theorem the_avf2_direction_the_table_gives_is_entailed :
    Entails avfPremises ⟨tC2, V.subClassOf, tC1⟩ := by
  intro I M
  exact M.conds.avf_sp _ _ _ _ _
    (M.facts ⟨tC1, V.allValuesFrom, tY⟩ (by simp [avfPremises]))
    (M.facts ⟨tC1, V.onProperty, tp1⟩ (by simp [avfPremises]))
    (M.facts ⟨tC2, V.allValuesFrom, tY⟩ (by simp [avfPremises]))
    (M.facts ⟨tC2, V.onProperty, tp2⟩ (by simp [avfPremises]))
    (M.facts ⟨tp1, V.subPropertyOf, tp2⟩ (by simp [avfPremises]))

/-- **And the other direction is unsound.** A rule that concluded
`C1 rdfs:subClassOf C2` here, which is what copying `scm-avf1` gives, would be
making a claim this model refutes.

**This one has a conforming replacement built for it**, and it was the first of
the four that needed a new structure rather than a re-reading of the one it
already had: `W3CWitness.lean`'s
`the_natural_avf2_direction_is_not_w3c_entailed` refutes the same triple from the
same premises with a thirty-five-element bridge-coherent structure that meets the
quoted cells of Tables 5.2, 5.3, 5.6, 5.8, 5.9, 5.12 and 5.13.

`herbrand avfWitness` is not such a structure. `avfWitness` contains no `rdf:type`
triple at all, so every class extension is empty, and `avf_typ` and `onp_typ`
then fail for want of an `owl:Restriction` typing; `onp_typ` fails for a second
reason, its `IP p` conjunct, which `IP := fun _ => False` cannot supply. The
replacement gives `p1` and `p2` real pairs, which is what makes `sp_fwd` and
`sp_bwd` decide the premise `p1 rdfs:subPropertyOf p2` for a reason and what
makes `dom_bwd` and `rng_bwd` fire non-vacuously. -/
theorem the_natural_avf2_direction_is_not_entailed :
    ¬ Entails avfPremises ⟨tC1, V.subClassOf, tC2⟩ := fun h =>
  absurd (h (herbrand avfWitness) avf_witness_is_a_model)
    (by decide : (⟨tC1, V.subClassOf, tC2⟩ : Triple) ∉ avfWitness)

/-! ## The two list conditions, and what they stop short of

`Chain` has no inversion principle in `Semantics.lean`, because nothing needed
one until a witness graph carried a list. This is it, and it belongs beside the
inductive. -/

theorem chain_inv {G : List Triple} {l : Term} {ms : List Term} (h : Chain G l ms) :
    (l = V.nil ∧ ms = []) ∨
    ∃ m l' ms', ms = m :: ms' ∧ (⟨l, V.first, m⟩ : Triple) ∈ G ∧
      (⟨l, V.rest, l'⟩ : Triple) ∈ G ∧ Chain G l' ms' := by
  cases h with
  | nil => exact Or.inl ⟨rfl, rfl⟩
  | cons h1 h2 hr => exact Or.inr ⟨_, _, _, rfl, h1, h2, hr⟩

/-! ### `cls-oo` types the members it lists, and no one else -/

def tE : Term := "<e:E>"
def tA : Term := "<e:a>"
def tB : Term := "<e:b>"
def tZ : Term := "<e:z>"
def oL0 : Term := "_:o0"
def oL1 : Term := "_:o1"

/-- `E = {a, b}` as an RDF list. `z` is mentioned nowhere. -/
def ooPremises : List Triple :=
  [ ⟨tE, V.oneOf, oL0⟩,
    ⟨oL0, V.first, tA⟩, ⟨oL0, V.rest, oL1⟩,
    ⟨oL1, V.first, tB⟩, ⟨oL1, V.rest, V.nil⟩ ]

/-- The premises plus the two typings `cls-oo` draws, and nothing else. -/
def ooWitness : List Triple := ⟨tA, V.type, tE⟩ :: ⟨tB, V.type, tE⟩ :: ooPremises

private theorem oo_l0_first : ∀ t ∈ ooPremises, t.s = oL0 → t.p = V.first → t.o = tA := by decide
private theorem oo_l0_rest : ∀ t ∈ ooPremises, t.s = oL0 → t.p = V.rest → t.o = oL1 := by decide
private theorem oo_l1_first : ∀ t ∈ ooPremises, t.s = oL1 → t.p = V.first → t.o = tB := by decide
private theorem oo_l1_rest : ∀ t ∈ ooPremises, t.s = oL1 → t.p = V.rest → t.o = V.nil := by decide
private theorem oo_no_nil_subject : ∀ t ∈ ooPremises, t.s ≠ V.nil := by decide
/-- Not `private`: `W3CWitness.lean` reads it to transfer
`an_unlisted_individual_is_not_entailed` to the `W3CModel` class. -/
theorem oo_only_oneOf :
    ∀ t ∈ ooPremises, t.p = V.oneOf → t.s = tE ∧ t.o = oL0 := by decide

private theorem oo_chain_nil : ∀ ms, Chain ooPremises V.nil ms → ms = [] := by
  intro ms h
  rcases chain_inv h with ⟨_, rfl⟩ | ⟨_, _, _, _, h1, _, _⟩
  · rfl
  · exact absurd rfl (oo_no_nil_subject _ h1)

private theorem oo_chain_l1 : ∀ ms, Chain ooPremises oL1 ms → ms = [tB] := by
  intro ms h
  rcases chain_inv h with ⟨hn, _⟩ | ⟨m, l', ms', rfl, h1, h2, hr⟩
  · exact absurd hn (by decide)
  · have hm : m = tB := oo_l1_first _ h1 rfl rfl
    have hl : l' = V.nil := oo_l1_rest _ h2 rfl rfl
    subst hm; subst hl
    rw [oo_chain_nil _ hr]

/-- Not `private`, for the same reason as `oo_only_oneOf`. -/
theorem oo_chain_l0 : ∀ ms, Chain ooPremises oL0 ms → ms = [tA, tB] := by
  intro ms h
  rcases chain_inv h with ⟨hn, _⟩ | ⟨m, l', ms', rfl, h1, h2, hr⟩
  · exact absurd hn (by decide)
  · have hm : m = tA := oo_l0_first _ h1 rfl rfl
    have hl : l' = oL1 := oo_l0_rest _ h2 rfl rfl
    subst hm; subst hl
    rw [oo_chain_l1 _ hr]

theorem oo_witness_is_a_model : Model (herbrand ooWitness) ooPremises where
  conds :=
    { sc_sub := fun a b hab => absurd hab (not_mem_pred ooWitness V.subClassOf (by decide) a b)
      sc_trans := fun a b _ hab =>
        absurd hab (not_mem_pred ooWitness V.subClassOf (by decide) a b)
      sp_sub := fun a b hab => absurd hab (not_mem_pred ooWitness V.subPropertyOf (by decide) a b)
      sp_trans := fun a b _ hab =>
        absurd hab (not_mem_pred ooWitness V.subPropertyOf (by decide) a b)
      dom := fun p c hpc => absurd hpc (not_mem_pred ooWitness V.domain (by decide) p c)
      rng := fun p c hpc => absurd hpc (not_mem_pred ooWitness V.range (by decide) p c)
      trp := fun p hp => absurd hp (not_typed ooWitness V.transitiveProperty (by decide) p)
      symp := fun p hp => absurd hp (not_typed ooWitness V.symmetricProperty (by decide) p)
      inv := fun p q hpq => absurd hpq (not_mem_pred ooWitness V.inverseOf (by decide) p q)
      same := fun a b hab => absurd hab (not_mem_pred ooWitness V.sameAs (by decide) a b)
      eqc := fun a b hab => absurd hab (not_mem_pred ooWitness V.equivalentClass (by decide) a b)
      eqp := fun a b hab =>
        absurd hab (not_mem_pred ooWitness V.equivalentProperty (by decide) a b)
      svf := fun r p _ hop => absurd hop (not_mem_pred ooWitness V.onProperty (by decide) r p)
      avf := fun r p _ hop => absurd hop (not_mem_pred ooWitness V.onProperty (by decide) r p)
      hv := fun r p _ hop => absurd hop (not_mem_pred ooWitness V.onProperty (by decide) r p)
      svf_sc := fun c1 _ _ y1 _ hsv =>
        absurd hsv (not_mem_pred ooWitness V.someValuesFrom (by decide) c1 y1)
      svf_sp := fun c1 _ _ _ y hsv =>
        absurd hsv (not_mem_pred ooWitness V.someValuesFrom (by decide) c1 y)
      avf_sc := fun c1 _ _ y1 _ hav =>
        absurd hav (not_mem_pred ooWitness V.allValuesFrom (by decide) c1 y1)
      avf_sp := fun c1 _ _ _ y hav =>
        absurd hav (not_mem_pred ooWitness V.allValuesFrom (by decide) c1 y)
      dom_sc := fun p c1 _ hd => absurd hd (not_mem_pred ooWitness V.domain (by decide) p c1)
      dom_sp := fun _ p2 c hd => absurd hd (not_mem_pred ooWitness V.domain (by decide) p2 c)
      rng_sc := fun p c1 _ hr => absurd hr (not_mem_pred ooWitness V.range (by decide) p c1)
      rng_sp := fun _ p2 c hr => absurd hr (not_mem_pred ooWitness V.range (by decide) p2 c) }
  facts := fun t ht => List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ht)
  int := by
    intro c l _ hc
    exact absurd hc (not_mem_pred ooPremises V.intersectionOf (by decide) c l)
  int2 := by
    intro c l _ hc
    exact absurd hc (not_mem_pred ooPremises V.intersectionOf (by decide) c l)
  uni := by intro c l _ hc; exact absurd hc (not_mem_pred ooPremises V.unionOf (by decide) c l)
  oneOf := by
    intro c l ms hc hchain m hm
    obtain ⟨rfl, rfl⟩ := oo_only_oneOf _ hc rfl
    rw [oo_chain_l0 ms hchain] at hm
    show (⟨m, V.type, tE⟩ : Triple) ∈ ooWitness
    rcases List.mem_cons.mp hm with rfl | hm
    · decide
    · rw [List.mem_singleton] at hm
      subst hm
      decide

/-- **`cls-oo` earns its conclusion.** Proved for an arbitrary model. -/
theorem an_enumerated_member_is_entailed : Entails ooPremises ⟨tA, V.type, tE⟩ := by
  intro I M
  have hchain : Chain ooPremises oL0 [tA, tB] := by
    refine Chain.cons (l' := oL1) (by simp [ooPremises]) (by simp [ooPremises]) ?_
    exact Chain.cons (l' := V.nil) (by simp [ooPremises]) (by simp [ooPremises]) Chain.nil
  exact M.oneOf tE oL0 [tA, tB] (by simp [ooPremises]) hchain tA (by simp)

/-- **And it stops at the members.** The condition says the class holds at
least the listed members; it does not say it holds no others, so an individual
the list never mentions is not entailed to be one.

Two separate things are wrong with the old reading of this theorem, and both are
recorded rather than repaired.

FIRST, it is not the detector `Semantics.lean` used to nominate it as. `ooWitness`
is `[a rdf:type E, b rdf:type E] ++ ooPremises`, so `ICEXT(E)` is exactly the two
listed members and already satisfies Table 5.5's full equality. The witness
passes unchanged whether or not the missing `⊆` half is present, so it cannot
detect that half's absence. That omission is currently undetected by anything in
this repository.

SECOND, an earlier version of this docstring said the result was about the
`Conditions` model class and could not escape it, on the ground that bridge
coherence puts every predicate `ooWitness` uses into `IP` and `sp_bwd` then forces
reflexive `rdfs:subPropertyOf` triples the graph lacks. That is wrong twice over:
bridge coherence is a property one model in `W3CWitness.lean` happens to have and
not a field of `W3CModel`, and `IP` is a free parameter that the refuter chooses.
Choose `IP := fun _ => False` and this very interpretation is a `W3CModel`.
`an_unlisted_individual_is_not_w3c_entailed` is the transfer, and it needs
`ooWitness` exactly as it stands. Table 5.5's equality is the only field it has to
earn, and it satisfies it in both directions, which is the first point read the
other way round. -/
theorem an_unlisted_individual_is_not_entailed : ¬ Entails ooPremises ⟨tZ, V.type, tE⟩ :=
  fun h =>
    absurd (h (herbrand ooWitness) oo_witness_is_a_model)
      (by decide : (⟨tZ, V.type, tE⟩ : Triple) ∉ ooWitness)

/-! ### `cls-int2` takes the intersection apart, and `cls-int1` still needs every member -/

def tK : Term := "<e:K>"
def tM1 : Term := "<e:M1>"
def tM2 : Term := "<e:M2>"
def tw : Term := "<e:w>"
def tv : Term := "<e:v>"
def iL0 : Term := "_:i0"
def iL1 : Term := "_:i1"

/-- `K = M1 ⊓ M2`, `w` is a `K`, and `v` is an `M1` and nothing else. -/
def intPremises : List Triple :=
  [ ⟨tK, V.intersectionOf, iL0⟩,
    ⟨iL0, V.first, tM1⟩, ⟨iL0, V.rest, iL1⟩,
    ⟨iL1, V.first, tM2⟩, ⟨iL1, V.rest, V.nil⟩,
    ⟨tw, V.type, tK⟩,
    ⟨tv, V.type, tM1⟩ ]

/-- The premises plus the two typings `cls-int2` draws for `w`. Nothing puts
`v` in `K`, which is the second theorem. -/
def intWitness : List Triple := ⟨tw, V.type, tM1⟩ :: ⟨tw, V.type, tM2⟩ :: intPremises

private theorem i_l0_first : ∀ t ∈ intPremises, t.s = iL0 → t.p = V.first → t.o = tM1 := by decide
private theorem i_l0_rest : ∀ t ∈ intPremises, t.s = iL0 → t.p = V.rest → t.o = iL1 := by decide
private theorem i_l1_first : ∀ t ∈ intPremises, t.s = iL1 → t.p = V.first → t.o = tM2 := by decide
private theorem i_l1_rest : ∀ t ∈ intPremises, t.s = iL1 → t.p = V.rest → t.o = V.nil := by decide
private theorem i_no_nil_subject : ∀ t ∈ intPremises, t.s ≠ V.nil := by decide
/-- Not `private`: `W3CWitness.lean` reads it to transfer
`membership_in_one_member_does_not_give_the_intersection` to the conforming
model class. -/
theorem i_only_int :
    ∀ t ∈ intPremises, t.p = V.intersectionOf → t.s = tK ∧ t.o = iL0 := by decide

/-- Anything that is both an `M1` and an `M2` in this graph is already a `K`,
so `int` holds. `v` is an `M1` and not an `M2`, so it does not trigger it. -/
private theorem i_int_closed : ∀ t ∈ intWitness, ∀ u ∈ intWitness,
    (t.p = V.type ∧ t.o = tM1 ∧ u.p = V.type ∧ u.o = tM2 ∧ u.s = t.s) →
      (⟨t.s, V.type, tK⟩ : Triple) ∈ intWitness := by decide

/-- Anything that is a `K` is already both, so `int2` holds. -/
private theorem i_int2_closed : ∀ t ∈ intWitness, t.p = V.type → t.o = tK →
    (⟨t.s, V.type, tM1⟩ : Triple) ∈ intWitness ∧
      (⟨t.s, V.type, tM2⟩ : Triple) ∈ intWitness := by decide

private theorem i_chain_nil : ∀ ms, Chain intPremises V.nil ms → ms = [] := by
  intro ms h
  rcases chain_inv h with ⟨_, rfl⟩ | ⟨_, _, _, _, h1, _, _⟩
  · rfl
  · exact absurd rfl (i_no_nil_subject _ h1)

private theorem i_chain_l1 : ∀ ms, Chain intPremises iL1 ms → ms = [tM2] := by
  intro ms h
  rcases chain_inv h with ⟨hn, _⟩ | ⟨m, l', ms', rfl, h1, h2, hr⟩
  · exact absurd hn (by decide)
  · have hm : m = tM2 := i_l1_first _ h1 rfl rfl
    have hl : l' = V.nil := i_l1_rest _ h2 rfl rfl
    subst hm; subst hl
    rw [i_chain_nil _ hr]

/-- Not `private`, for the same reason as `i_only_int`. -/
theorem i_chain_l0 : ∀ ms, Chain intPremises iL0 ms → ms = [tM1, tM2] := by
  intro ms h
  rcases chain_inv h with ⟨hn, _⟩ | ⟨m, l', ms', rfl, h1, h2, hr⟩
  · exact absurd hn (by decide)
  · have hm : m = tM1 := i_l0_first _ h1 rfl rfl
    have hl : l' = iL1 := i_l0_rest _ h2 rfl rfl
    subst hm; subst hl
    rw [i_chain_l1 _ hr]

theorem int_witness_is_a_model : Model (herbrand intWitness) intPremises where
  conds :=
    { sc_sub := fun a b hab => absurd hab (not_mem_pred intWitness V.subClassOf (by decide) a b)
      sc_trans := fun a b _ hab =>
        absurd hab (not_mem_pred intWitness V.subClassOf (by decide) a b)
      sp_sub := fun a b hab => absurd hab (not_mem_pred intWitness V.subPropertyOf (by decide) a b)
      sp_trans := fun a b _ hab =>
        absurd hab (not_mem_pred intWitness V.subPropertyOf (by decide) a b)
      dom := fun p c hpc => absurd hpc (not_mem_pred intWitness V.domain (by decide) p c)
      rng := fun p c hpc => absurd hpc (not_mem_pred intWitness V.range (by decide) p c)
      trp := fun p hp => absurd hp (not_typed intWitness V.transitiveProperty (by decide) p)
      symp := fun p hp => absurd hp (not_typed intWitness V.symmetricProperty (by decide) p)
      inv := fun p q hpq => absurd hpq (not_mem_pred intWitness V.inverseOf (by decide) p q)
      same := fun a b hab => absurd hab (not_mem_pred intWitness V.sameAs (by decide) a b)
      eqc := fun a b hab => absurd hab (not_mem_pred intWitness V.equivalentClass (by decide) a b)
      eqp := fun a b hab =>
        absurd hab (not_mem_pred intWitness V.equivalentProperty (by decide) a b)
      svf := fun r p _ hop => absurd hop (not_mem_pred intWitness V.onProperty (by decide) r p)
      avf := fun r p _ hop => absurd hop (not_mem_pred intWitness V.onProperty (by decide) r p)
      hv := fun r p _ hop => absurd hop (not_mem_pred intWitness V.onProperty (by decide) r p)
      svf_sc := fun c1 _ _ y1 _ hsv =>
        absurd hsv (not_mem_pred intWitness V.someValuesFrom (by decide) c1 y1)
      svf_sp := fun c1 _ _ _ y hsv =>
        absurd hsv (not_mem_pred intWitness V.someValuesFrom (by decide) c1 y)
      avf_sc := fun c1 _ _ y1 _ hav =>
        absurd hav (not_mem_pred intWitness V.allValuesFrom (by decide) c1 y1)
      avf_sp := fun c1 _ _ _ y hav =>
        absurd hav (not_mem_pred intWitness V.allValuesFrom (by decide) c1 y)
      dom_sc := fun p c1 _ hd => absurd hd (not_mem_pred intWitness V.domain (by decide) p c1)
      dom_sp := fun _ p2 c hd => absurd hd (not_mem_pred intWitness V.domain (by decide) p2 c)
      rng_sc := fun p c1 _ hr => absurd hr (not_mem_pred intWitness V.range (by decide) p c1)
      rng_sp := fun _ p2 c hr => absurd hr (not_mem_pred intWitness V.range (by decide) p2 c) }
  facts := fun t ht => List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ht)
  int := by
    intro c l ms hc hchain x hx
    obtain ⟨rfl, rfl⟩ := i_only_int _ hc rfl
    have hms := i_chain_l0 ms hchain
    subst hms
    exact i_int_closed _ (hx tM1 (by simp)) _ (hx tM2 (by simp)) ⟨rfl, rfl, rfl, rfl, rfl⟩
  int2 := by
    intro c l ms hc hchain x hx m hm
    obtain ⟨rfl, rfl⟩ := i_only_int _ hc rfl
    have hms := i_chain_l0 ms hchain
    subst hms
    obtain ⟨h1, h2⟩ := i_int2_closed ⟨x, V.type, tK⟩ hx rfl rfl
    rcases List.mem_cons.mp hm with rfl | hm
    · exact h1
    · rw [List.mem_singleton] at hm
      subst hm
      exact h2
  uni := by intro c l _ hc; exact absurd hc (not_mem_pred intPremises V.unionOf (by decide) c l)
  oneOf := by intro c l _ hc; exact absurd hc (not_mem_pred intPremises V.oneOf (by decide) c l)

/-- **`cls-int2` earns its conclusion.** Being a `K` makes `w` an `M1`. -/
theorem a_member_of_the_intersection_is_entailed : Entails intPremises ⟨tw, V.type, tM1⟩ := by
  intro I M
  have hchain : Chain intPremises iL0 [tM1, tM2] := by
    refine Chain.cons (l' := iL1) (by simp [intPremises]) (by simp [intPremises]) ?_
    exact Chain.cons (l' := V.nil) (by simp [intPremises]) (by simp [intPremises]) Chain.nil
  exact M.int2 tK iL0 [tM1, tM2] (by simp [intPremises]) hchain (I.ι tw)
    (M.facts ⟨tw, V.type, tK⟩ (by simp [intPremises])) tM1 (by simp)

/-- **And the converse still needs every member.** `v` is an `M1` and no model
has to make it a `K`, so `cls-int1` has lost nothing to `cls-int2`.

This transfers to the `W3CModel` class with `intWitness` unchanged, as
`membership_in_one_member_is_not_w3c_enough`, by the same `IP := fun _ => False`
reading as the `owl:oneOf` pair above. Table 5.4's equality holds here in both
directions, because `w` is the only `K` and also the only thing that is both an
`M1` and an `M2`; `v` is an `M1` and not an `M2`, which is what keeps the equality
true and the conclusion false at once.

An earlier version of this docstring said the opposite, on the ground that bridge
coherence forces `rdf:type`, `owl:intersectionOf`, `rdf:first` and `rdf:rest` into
`IP` and `sp_bwd` then demands four `rdfs:subPropertyOf` triples. `W3CModel` does
not require bridge coherence and `IP` is free. -/
theorem membership_in_one_member_does_not_give_the_intersection :
    ¬ Entails intPremises ⟨tv, V.type, tK⟩ := fun h =>
  absurd (h (herbrand intWitness) int_witness_is_a_model)
    (by decide : (⟨tv, V.type, tK⟩ : Triple) ∉ intWitness)

/-! Axioms, pinned. A `sorry` here would quietly restore the vacuity objection this file exists to
close, which is exactly the kind of silent hole the project refuses elsewhere. These lists are
shorter than the one `certificate_sound` carries: the witnesses need no choice, and the two
statements about the removed rule are proved from `propext` alone. -/
/-- info: 'OOCert.saturated_is_a_model' does not depend on any axioms -/
#guard_msgs in
#print axioms saturated_is_a_model

/-- info: 'OOCert.not_everything_is_entailed' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms not_everything_is_entailed

/-- info: 'OOCert.the_old_svf_derivation_is_not_entailed' depends on axioms: [propext] -/
#guard_msgs in
#print axioms the_old_svf_derivation_is_not_entailed

/-- info: 'OOCert.the_sound_half_survives' depends on axioms: [propext] -/
#guard_msgs in
#print axioms the_sound_half_survives

/-- info: 'OOCert.the_avf2_direction_the_table_gives_is_entailed' depends on axioms: [propext] -/
#guard_msgs in
#print axioms the_avf2_direction_the_table_gives_is_entailed

/-- info: 'OOCert.the_natural_avf2_direction_is_not_entailed' depends on axioms: [propext] -/
#guard_msgs in
#print axioms the_natural_avf2_direction_is_not_entailed

/-- info: 'OOCert.an_enumerated_member_is_entailed' depends on axioms: [propext] -/
#guard_msgs in
#print axioms an_enumerated_member_is_entailed

/-- info: 'OOCert.an_unlisted_individual_is_not_entailed' depends on axioms: [propext] -/
#guard_msgs in
#print axioms an_unlisted_individual_is_not_entailed

/-- info: 'OOCert.a_member_of_the_intersection_is_entailed' depends on axioms: [propext] -/
#guard_msgs in
#print axioms a_member_of_the_intersection_is_entailed

/--
info: 'OOCert.membership_in_one_member_does_not_give_the_intersection' depends on axioms: [propext]
-/
#guard_msgs in
#print axioms membership_in_one_member_does_not_give_the_intersection

end OOCert
