import OOCert.Refute
import OOCert.W3C
import OOCert.Witness

/-!
# Witnesses for the refutation layer

`refutation_sound` says an accepted refutation means `Unsat G`. That claim is
worth nothing on its own, and it can be hollow in a way the derivation checker's
soundness theorem cannot be.

`Unsat G` means "no interpretation satisfies both `Model I G` and
`RefuteConditions I`". If NOTHING ever satisfied both, `Unsat G` would hold of
every graph, the checker could accept anything, and `refutation_sound` would
still be a theorem. `lean/OOCert/Witness.lean` closes the matching hole for the
derivation checker with `saturated_is_a_model`, which exhibits a model of an
arbitrary graph. That route is closed here by construction: disjointness is a
NEGATIVE condition and the saturated interpretation, which puts everything in
every class, violates it. The witness has to be rebuilt, not reused.

| theorem | says |
|---|---|
| `closure_is_a_model` | the OWL-RL closure of ANY graph, read as a Herbrand interpretation, is a model of it |
| `Der_sound` | that closure contains only entailed triples, so it is the closure and not a junk superset |
| `no_violation_means_a_joint_model` | a graph whose closure has no disjointness violation HAS a model respecting disjointness |
| `unsat_means_a_violation` | and the converse: refutability is exactly a violation in the closure |
| `graze_is_refuted` | a concrete graph is refuted, through a derived type rather than an asserted one |
| `feed_is_not_refuted` | a concrete graph that HAS a disjointness axiom is not refutable, so the checker's rejections are not an accident |
| `the_checker_rejects_the_attempt_on_the_consistent_graph` | and the checker really does reject it |
| six `rejects_*` theorems | six ways to forge a refutation, each caught |
| `an_ordinary_certificate_is_worthless_over_the_refuted_graph` | the explosion, stated for the graph above |
| `and_the_old_verdict_does_not_notice` | and `Entails` does not notice, which is the trap |
| `w3cUnsat_of_unsat` | a refutation rules out the interpretations meeting the W3C tables too, which is the easy direction |
| `feed_is_not_w3c_refuted` | and the hard one: a thirteen-element model of `feed` that meets those tables AND respects disjointness |
| `the_old_verdict_does_not_notice_over_w3c` | the trap again, over that same class, on the same carrier with one row changed |

## The one hypothesis the general theorem carries, named rather than hidden

`closure_is_a_model` needs `∀ a b, Der G ⟨a, owl:sameAs, b⟩ → a = b`. Every other
semantic condition is a closure rule and becomes a constructor of `Der`, but
`Conditions.same` is not a closure rule: it says the denotations of two terms
coincide, and in a Herbrand interpretation a term denotes itself, so two
different spellings cannot be equal. A graph whose closure asserts an identity
between distinct terms therefore needs a quotient of the term domain, which is a
larger construction and is NOT done here. The hypothesis is an explicit argument
of every theorem that needs it, and both concrete graphs discharge it by
containing no `owl:sameAs` at all.
-/
namespace OOCert

/-! ## The closure of a graph, as an inductive predicate

One constructor per condition of `Model`, `Conditions.same` excepted. Read it as
the definition of "the semantics forces this triple": `Der_sound` proves it
derives nothing that is not entailed, and `closure_is_a_model` proves it derives
everything a model must contain.

The pair matters. Without `Der_sound` the predicate could be everything, which
would make `no_violation_means_a_joint_model` vacuous because no graph would
satisfy its hypothesis. Without `closure_is_a_model` it could be nothing, and the
theorem would be about an interpretation that is not a model. -/
inductive Der (G : List Triple) : Triple → Prop
  | base {t : Triple} : t ∈ G → Der G t
  | sc_sub {a b x : Term} :
      Der G ⟨a, V.subClassOf, b⟩ → Der G ⟨x, V.type, a⟩ → Der G ⟨x, V.type, b⟩
  | sc_trans {a b c : Term} :
      Der G ⟨a, V.subClassOf, b⟩ → Der G ⟨b, V.subClassOf, c⟩ → Der G ⟨a, V.subClassOf, c⟩
  | sp_sub {a b x y : Term} :
      Der G ⟨a, V.subPropertyOf, b⟩ → Der G ⟨x, a, y⟩ → Der G ⟨x, b, y⟩
  | sp_trans {a b c : Term} :
      Der G ⟨a, V.subPropertyOf, b⟩ → Der G ⟨b, V.subPropertyOf, c⟩ →
      Der G ⟨a, V.subPropertyOf, c⟩
  | dom {p c x y : Term} : Der G ⟨p, V.domain, c⟩ → Der G ⟨x, p, y⟩ → Der G ⟨x, V.type, c⟩
  | rng {p c x y : Term} : Der G ⟨p, V.range, c⟩ → Der G ⟨x, p, y⟩ → Der G ⟨y, V.type, c⟩
  | trp {p x y z : Term} :
      Der G ⟨p, V.type, V.transitiveProperty⟩ → Der G ⟨x, p, y⟩ → Der G ⟨y, p, z⟩ →
      Der G ⟨x, p, z⟩
  | symp {p x y : Term} :
      Der G ⟨p, V.type, V.symmetricProperty⟩ → Der G ⟨x, p, y⟩ → Der G ⟨y, p, x⟩
  | inv1 {p q x y : Term} : Der G ⟨p, V.inverseOf, q⟩ → Der G ⟨x, p, y⟩ → Der G ⟨y, q, x⟩
  | inv2 {p q x y : Term} : Der G ⟨p, V.inverseOf, q⟩ → Der G ⟨y, q, x⟩ → Der G ⟨x, p, y⟩
  | eqc1 {a b : Term} : Der G ⟨a, V.equivalentClass, b⟩ → Der G ⟨a, V.subClassOf, b⟩
  | eqc2 {a b : Term} : Der G ⟨a, V.equivalentClass, b⟩ → Der G ⟨b, V.subClassOf, a⟩
  | eqp1 {a b : Term} : Der G ⟨a, V.equivalentProperty, b⟩ → Der G ⟨a, V.subPropertyOf, b⟩
  | eqp2 {a b : Term} : Der G ⟨a, V.equivalentProperty, b⟩ → Der G ⟨b, V.subPropertyOf, a⟩
  | svf {r p c x y : Term} :
      Der G ⟨r, V.onProperty, p⟩ → Der G ⟨r, V.someValuesFrom, c⟩ → Der G ⟨x, p, y⟩ →
      Der G ⟨y, V.type, c⟩ → Der G ⟨x, V.type, r⟩
  | avf {r p c x y : Term} :
      Der G ⟨r, V.onProperty, p⟩ → Der G ⟨r, V.allValuesFrom, c⟩ → Der G ⟨x, V.type, r⟩ →
      Der G ⟨x, p, y⟩ → Der G ⟨y, V.type, c⟩
  | hv1 {r p v x : Term} :
      Der G ⟨r, V.onProperty, p⟩ → Der G ⟨r, V.hasValue, v⟩ → Der G ⟨x, V.type, r⟩ →
      Der G ⟨x, p, v⟩
  | hv2 {r p v x : Term} :
      Der G ⟨r, V.onProperty, p⟩ → Der G ⟨r, V.hasValue, v⟩ → Der G ⟨x, p, v⟩ →
      Der G ⟨x, V.type, r⟩
  | svf_sc {c1 c2 p y1 y2 : Term} :
      Der G ⟨c1, V.someValuesFrom, y1⟩ → Der G ⟨c1, V.onProperty, p⟩ →
      Der G ⟨c2, V.someValuesFrom, y2⟩ → Der G ⟨c2, V.onProperty, p⟩ →
      Der G ⟨y1, V.subClassOf, y2⟩ → Der G ⟨c1, V.subClassOf, c2⟩
  | svf_sp {c1 c2 p1 p2 y : Term} :
      Der G ⟨c1, V.someValuesFrom, y⟩ → Der G ⟨c1, V.onProperty, p1⟩ →
      Der G ⟨c2, V.someValuesFrom, y⟩ → Der G ⟨c2, V.onProperty, p2⟩ →
      Der G ⟨p1, V.subPropertyOf, p2⟩ → Der G ⟨c1, V.subClassOf, c2⟩
  | avf_sc {c1 c2 p y1 y2 : Term} :
      Der G ⟨c1, V.allValuesFrom, y1⟩ → Der G ⟨c1, V.onProperty, p⟩ →
      Der G ⟨c2, V.allValuesFrom, y2⟩ → Der G ⟨c2, V.onProperty, p⟩ →
      Der G ⟨y1, V.subClassOf, y2⟩ → Der G ⟨c1, V.subClassOf, c2⟩
  /-- The reversed one. `scm-avf2` concludes `c2 rdfs:subClassOf c1`. -/
  | avf_sp {c1 c2 p1 p2 y : Term} :
      Der G ⟨c1, V.allValuesFrom, y⟩ → Der G ⟨c1, V.onProperty, p1⟩ →
      Der G ⟨c2, V.allValuesFrom, y⟩ → Der G ⟨c2, V.onProperty, p2⟩ →
      Der G ⟨p1, V.subPropertyOf, p2⟩ → Der G ⟨c2, V.subClassOf, c1⟩
  | dom_sc {p c1 c2 : Term} :
      Der G ⟨p, V.domain, c1⟩ → Der G ⟨c1, V.subClassOf, c2⟩ → Der G ⟨p, V.domain, c2⟩
  | dom_sp {p1 p2 c : Term} :
      Der G ⟨p2, V.domain, c⟩ → Der G ⟨p1, V.subPropertyOf, p2⟩ → Der G ⟨p1, V.domain, c⟩
  | rng_sc {p c1 c2 : Term} :
      Der G ⟨p, V.range, c1⟩ → Der G ⟨c1, V.subClassOf, c2⟩ → Der G ⟨p, V.range, c2⟩
  | rng_sp {p1 p2 c : Term} :
      Der G ⟨p2, V.range, c⟩ → Der G ⟨p1, V.subPropertyOf, p2⟩ → Der G ⟨p1, V.range, c⟩
  | int {c l x : Term} {ms : List Term} :
      (⟨c, V.intersectionOf, l⟩ : Triple) ∈ G → Chain G l ms →
      (∀ m ∈ ms, Der G ⟨x, V.type, m⟩) → Der G ⟨x, V.type, c⟩
  | int2 {c l x m : Term} {ms : List Term} :
      (⟨c, V.intersectionOf, l⟩ : Triple) ∈ G → Chain G l ms →
      Der G ⟨x, V.type, c⟩ → m ∈ ms → Der G ⟨x, V.type, m⟩
  | uni {c l x m : Term} {ms : List Term} :
      (⟨c, V.unionOf, l⟩ : Triple) ∈ G → Chain G l ms →
      m ∈ ms → Der G ⟨x, V.type, m⟩ → Der G ⟨x, V.type, c⟩
  | oneOf {c l m : Term} {ms : List Term} :
      (⟨c, V.oneOf, l⟩ : Triple) ∈ G → Chain G l ms → m ∈ ms → Der G ⟨m, V.type, c⟩

/-- **The closure derives only what the semantics forces.** Every case is a
one-line appeal to the matching field of `Model`, which is what it means for the
constructors to mirror the conditions rather than to invent rules. -/
theorem Der_sound {G : List Triple} {I : Interp} (M : Model I G) :
    ∀ {t : Triple}, Der G t → I.sat t := by
  intro t h
  induction h with
  | @base _ hm => exact M.facts _ hm
  | @sc_sub a b x _ _ ih1 ih2 => exact M.conds.sc_sub _ _ ih1 _ ih2
  | @sc_trans a b c _ _ ih1 ih2 => exact M.conds.sc_trans _ _ _ ih1 ih2
  | @sp_sub a b x y _ _ ih1 ih2 => exact M.conds.sp_sub _ _ ih1 _ _ ih2
  | @sp_trans a b c _ _ ih1 ih2 => exact M.conds.sp_trans _ _ _ ih1 ih2
  | @dom p c x y _ _ ih1 ih2 => exact M.conds.dom _ _ ih1 _ _ ih2
  | @rng p c x y _ _ ih1 ih2 => exact M.conds.rng _ _ ih1 _ _ ih2
  | @trp p x y z _ _ _ ih1 ih2 ih3 => exact M.conds.trp _ ih1 _ _ _ ih2 ih3
  | @symp p x y _ _ ih1 ih2 => exact M.conds.symp _ ih1 _ _ ih2
  | @inv1 p q x y _ _ ih1 ih2 => exact (M.conds.inv _ _ ih1 _ _).mp ih2
  | @inv2 p q x y _ _ ih1 ih2 => exact (M.conds.inv _ _ ih1 _ _).mpr ih2
  | @eqc1 a b _ ih => exact (M.conds.eqc _ _ ih).1
  | @eqc2 a b _ ih => exact (M.conds.eqc _ _ ih).2
  | @eqp1 a b _ ih => exact (M.conds.eqp _ _ ih).1
  | @eqp2 a b _ ih => exact (M.conds.eqp _ _ ih).2
  | @svf r p c x y _ _ _ _ ih1 ih2 ih3 ih4 => exact M.conds.svf _ _ _ ih1 ih2 _ _ ih3 ih4
  | @avf r p c x y _ _ _ _ ih1 ih2 ih3 ih4 => exact M.conds.avf _ _ _ ih1 ih2 _ _ ih3 ih4
  | @hv1 r p v x _ _ _ ih1 ih2 ih3 => exact (M.conds.hv _ _ _ ih1 ih2 _).mp ih3
  | @hv2 r p v x _ _ _ ih1 ih2 ih3 => exact (M.conds.hv _ _ _ ih1 ih2 _).mpr ih3
  | @svf_sc c1 c2 p y1 y2 _ _ _ _ _ ih1 ih2 ih3 ih4 ih5 =>
      exact M.conds.svf_sc _ _ _ _ _ ih1 ih2 ih3 ih4 ih5
  | @svf_sp c1 c2 p1 p2 y _ _ _ _ _ ih1 ih2 ih3 ih4 ih5 =>
      exact M.conds.svf_sp _ _ _ _ _ ih1 ih2 ih3 ih4 ih5
  | @avf_sc c1 c2 p y1 y2 _ _ _ _ _ ih1 ih2 ih3 ih4 ih5 =>
      exact M.conds.avf_sc _ _ _ _ _ ih1 ih2 ih3 ih4 ih5
  | @avf_sp c1 c2 p1 p2 y _ _ _ _ _ ih1 ih2 ih3 ih4 ih5 =>
      exact M.conds.avf_sp _ _ _ _ _ ih1 ih2 ih3 ih4 ih5
  | @dom_sc p c1 c2 _ _ ih1 ih2 => exact M.conds.dom_sc _ _ _ ih1 ih2
  | @dom_sp p1 p2 c _ _ ih1 ih2 => exact M.conds.dom_sp _ _ _ ih1 ih2
  | @rng_sc p c1 c2 _ _ ih1 ih2 => exact M.conds.rng_sc _ _ _ ih1 ih2
  | @rng_sp p1 p2 c _ _ ih1 ih2 => exact M.conds.rng_sp _ _ _ ih1 ih2
  | @int c l x ms hin hch _ ih => exact M.int _ _ _ hin hch _ ih
  | @int2 c l x m ms hin hch _ hm ih => exact M.int2 _ _ _ hin hch _ ih _ hm
  | @uni c l x m ms hin hch hm _ ih => exact M.uni _ _ _ hin hch _ _ hm ih
  | @oneOf c l m ms hin hch hm => exact M.oneOf _ _ _ hin hch _ hm

/-- The closure is contained in every model, in particular in every finite
Herbrand model, which is how the concrete graphs below settle negatives. -/
theorem Der_entails {G : List Triple} {t : Triple} (h : Der G t) : Entails G t :=
  fun _ M => Der_sound M h

/-- **The closure of any graph is a model of it.** Each field is the matching
constructor, because that is how the constructors were chosen. The one
hypothesis is `Conditions.same`, which is not a closure rule: see the module
docstring. -/
theorem closure_is_a_model (G : List Triple)
    (hsame : ∀ a b : Term, Der G ⟨a, V.sameAs, b⟩ → a = b) :
    Model (herbrandP (Der G)) G where
  conds :=
    { sc_sub := fun _ _ h _ hx => Der.sc_sub h hx
      sc_trans := fun _ _ _ h1 h2 => Der.sc_trans h1 h2
      sp_sub := fun _ _ h _ _ hxy => Der.sp_sub h hxy
      sp_trans := fun _ _ _ h1 h2 => Der.sp_trans h1 h2
      dom := fun _ _ h _ _ hxy => Der.dom h hxy
      rng := fun _ _ h _ _ hxy => Der.rng h hxy
      trp := fun _ h _ _ _ h1 h2 => Der.trp h h1 h2
      symp := fun _ h _ _ h1 => Der.symp h h1
      inv := fun _ _ h _ _ => ⟨fun h1 => Der.inv1 h h1, fun h1 => Der.inv2 h h1⟩
      same := hsame
      eqc := fun _ _ h => ⟨Der.eqc1 h, Der.eqc2 h⟩
      eqp := fun _ _ h => ⟨Der.eqp1 h, Der.eqp2 h⟩
      svf := fun _ _ _ h1 h2 _ _ h3 h4 => Der.svf h1 h2 h3 h4
      avf := fun _ _ _ h1 h2 _ _ h3 h4 => Der.avf h1 h2 h3 h4
      hv := fun _ _ _ h1 h2 _ => ⟨fun h3 => Der.hv1 h1 h2 h3, fun h3 => Der.hv2 h1 h2 h3⟩
      svf_sc := fun _ _ _ _ _ h1 h2 h3 h4 h5 => Der.svf_sc h1 h2 h3 h4 h5
      svf_sp := fun _ _ _ _ _ h1 h2 h3 h4 h5 => Der.svf_sp h1 h2 h3 h4 h5
      avf_sc := fun _ _ _ _ _ h1 h2 h3 h4 h5 => Der.avf_sc h1 h2 h3 h4 h5
      avf_sp := fun _ _ _ _ _ h1 h2 h3 h4 h5 => Der.avf_sp h1 h2 h3 h4 h5
      dom_sc := fun _ _ _ h1 h2 => Der.dom_sc h1 h2
      dom_sp := fun _ _ _ h1 h2 => Der.dom_sp h1 h2
      rng_sc := fun _ _ _ h1 h2 => Der.rng_sc h1 h2
      rng_sp := fun _ _ _ h1 h2 => Der.rng_sp h1 h2 }
  facts := fun _ ht => Der.base ht
  int := fun _ _ _ hin hch _ hall => Der.int hin hch hall
  int2 := fun _ _ _ hin hch _ hx _ hm => Der.int2 hin hch hx hm
  uni := fun _ _ _ hin hch _ _ hm hx => Der.uni hin hch hm hx
  oneOf := fun _ _ _ hin hch _ hm => Der.oneOf hin hch hm

/-- **The replacement for `saturated_is_a_model`.** A graph whose closure has no
disjointness violation has a model that satisfies `Model` AND `RefuteConditions`,
so `refutation_sound` is not a theorem about an empty model class.

Conditional on the `owl:sameAs` hypothesis, which is stated and not buried. -/
theorem no_violation_means_a_joint_model (G : List Triple)
    (hsame : ∀ a b : Term, Der G ⟨a, V.sameAs, b⟩ → a = b)
    (hdw : ∀ c1 c2 x : Term, Der G ⟨c1, RV.disjointWith, c2⟩ → Der G ⟨x, V.type, c1⟩ →
      Der G ⟨x, V.type, c2⟩ → False) :
    ∃ I : Interp, Model I G ∧ RefuteConditions I :=
  joint_model_of_closure (closure_is_a_model G hsame) hdw

/-- The converse, and the more useful direction in practice: if a graph IS
refutable then its closure contains a disjointness violation. So `cax-dw` is not
merely sound for this fragment, it is the only way to be unsatisfiable in it, and
a consumer who finds no violation has a positive result rather than a failure to
find one. Same `owl:sameAs` hypothesis.

Read "unsatisfiable" here as `Unsat`, which is the whole content of the
statement and is narrower than a consumer's word for it. `Unsat` quantifies over
`RefuteConditions`, and `RefuteConditions` reads `owl:disjointWith` and nothing
else, so this theorem says `cax-dw` is the only clash IN THAT MODEL CLASS. It
does not say a graph with no disjointness violation is consistent under OWL 2
RL: sixteen further rules of the profile conclude `false` and none of them has a
condition here, so a graph refutable only by `prp-irp` or `cls-nothing2` passes
this test and is contradictory anyway. The positive result is "no clash of the
one kind this layer can see", and it is worth having for exactly that. -/
theorem unsat_means_a_violation (G : List Triple)
    (hsame : ∀ a b : Term, Der G ⟨a, V.sameAs, b⟩ → a = b) (h : Unsat G) :
    ∃ c1 c2 x : Term, Der G ⟨c1, RV.disjointWith, c2⟩ ∧ Der G ⟨x, V.type, c1⟩ ∧
      Der G ⟨x, V.type, c2⟩ :=
  Classical.byContradiction fun hno =>
    not_unsat_of_joint_model
      (no_violation_means_a_joint_model G hsame
        (fun c1 c2 x h1 h2 h3 => hno ⟨c1, c2, x, h1, h2, h3⟩)) h

/-! ## A fragment in which a plain Herbrand model is easy to exhibit

Both concrete graphs below use only `rdf:type`, `rdfs:subClassOf` and
`owl:disjointWith`. In that fragment every semantic condition except `sc_sub`
and `sc_trans` has a premise no triple can supply, so a list closed under those
two is a model of everything it contains. `Plain` packages the four side
conditions in a form `decide` can settle over a concrete list. -/

/-- The side conditions, all decidable over a concrete list. `scClosed` and
`scTrans` are written over pairs of triples of `H` rather than over arbitrary
terms for exactly that reason. -/
structure Plain (H : List Triple) : Prop where
  vocab : ∀ t ∈ H, t.p = V.type ∨ t.p = V.subClassOf ∨ t.p = RV.disjointWith
  noMeta : ∀ t ∈ H, ¬(t.p = V.type ∧ (t.o = V.transitiveProperty ∨ t.o = V.symmetricProperty))
  scClosed : ∀ t ∈ H, ∀ u ∈ H, t.p = V.subClassOf → u.p = V.type → u.o = t.s →
    (⟨u.s, V.type, t.o⟩ : Triple) ∈ H
  scTrans : ∀ t ∈ H, ∀ u ∈ H, t.p = V.subClassOf → u.p = V.subClassOf → u.s = t.o →
    (⟨t.s, V.subClassOf, u.o⟩ : Triple) ∈ H

namespace Plain

/-- A predicate outside the fragment has an empty extension in `H`. -/
theorem absent {H : List Triple} (hP : Plain H) (p : Term) (h1 : p ≠ V.type)
    (h2 : p ≠ V.subClassOf) (h3 : p ≠ RV.disjointWith) (s o : Term) :
    (⟨s, p, o⟩ : Triple) ∉ H := by
  intro hm
  rcases hP.vocab _ hm with h | h | h
  · exact h1 h
  · exact h2 h
  · exact h3 h

theorem notTrans {H : List Triple} (hP : Plain H) (p : Term) :
    (⟨p, V.type, V.transitiveProperty⟩ : Triple) ∉ H :=
  fun hm => hP.noMeta _ hm ⟨rfl, Or.inl rfl⟩

theorem notSymm {H : List Triple} (hP : Plain H) (p : Term) :
    (⟨p, V.type, V.symmetricProperty⟩ : Triple) ∉ H :=
  fun hm => hP.noMeta _ hm ⟨rfl, Or.inr rfl⟩

/-- The Herbrand interpretation of a `Plain` list models every graph contained
in it. -/
theorem model {H G : List Triple} (hP : Plain H) (hGH : ∀ t ∈ G, t ∈ H) :
    Model (herbrandL H) G where
  conds :=
    { sc_sub := fun a b hab x hx => hP.scClosed ⟨a, V.subClassOf, b⟩ hab ⟨x, V.type, a⟩ hx rfl rfl rfl
      sc_trans := fun a b c hab hbc =>
        hP.scTrans ⟨a, V.subClassOf, b⟩ hab ⟨b, V.subClassOf, c⟩ hbc rfl rfl rfl
      sp_sub := fun a b hab =>
        absurd hab (hP.absent V.subPropertyOf (by decide) (by decide) (by decide) a b)
      sp_trans := fun a b _ hab =>
        absurd hab (hP.absent V.subPropertyOf (by decide) (by decide) (by decide) a b)
      dom := fun p c h => absurd h (hP.absent V.domain (by decide) (by decide) (by decide) p c)
      rng := fun p c h => absurd h (hP.absent V.range (by decide) (by decide) (by decide) p c)
      trp := fun p h => absurd h (hP.notTrans p)
      symp := fun p h => absurd h (hP.notSymm p)
      inv := fun p q h => absurd h (hP.absent V.inverseOf (by decide) (by decide) (by decide) p q)
      same := fun a b h => absurd h (hP.absent V.sameAs (by decide) (by decide) (by decide) a b)
      eqc := fun a b h =>
        absurd h (hP.absent V.equivalentClass (by decide) (by decide) (by decide) a b)
      eqp := fun a b h =>
        absurd h (hP.absent V.equivalentProperty (by decide) (by decide) (by decide) a b)
      svf := fun r p _ h => absurd h (hP.absent V.onProperty (by decide) (by decide) (by decide) r p)
      avf := fun r p _ h => absurd h (hP.absent V.onProperty (by decide) (by decide) (by decide) r p)
      hv := fun r p _ h => absurd h (hP.absent V.onProperty (by decide) (by decide) (by decide) r p)
      svf_sc := fun c1 _ _ y1 _ h =>
        absurd h (hP.absent V.someValuesFrom (by decide) (by decide) (by decide) c1 y1)
      svf_sp := fun c1 _ _ _ y h =>
        absurd h (hP.absent V.someValuesFrom (by decide) (by decide) (by decide) c1 y)
      avf_sc := fun c1 _ _ y1 _ h =>
        absurd h (hP.absent V.allValuesFrom (by decide) (by decide) (by decide) c1 y1)
      avf_sp := fun c1 _ _ _ y h =>
        absurd h (hP.absent V.allValuesFrom (by decide) (by decide) (by decide) c1 y)
      dom_sc := fun p c1 _ h =>
        absurd h (hP.absent V.domain (by decide) (by decide) (by decide) p c1)
      dom_sp := fun _ p2 c h =>
        absurd h (hP.absent V.domain (by decide) (by decide) (by decide) p2 c)
      rng_sc := fun p c1 _ h =>
        absurd h (hP.absent V.range (by decide) (by decide) (by decide) p c1)
      rng_sp := fun _ p2 c h =>
        absurd h (hP.absent V.range (by decide) (by decide) (by decide) p2 c) }
  facts := fun t ht => hGH t ht
  int := fun c l _ hin =>
    absurd (hGH _ hin) (hP.absent V.intersectionOf (by decide) (by decide) (by decide) c l)
  int2 := fun c l _ hin =>
    absurd (hGH _ hin) (hP.absent V.intersectionOf (by decide) (by decide) (by decide) c l)
  uni := fun c l _ hin =>
    absurd (hGH _ hin) (hP.absent V.unionOf (by decide) (by decide) (by decide) c l)
  oneOf := fun c l _ hin =>
    absurd (hGH _ hin) (hP.absent V.oneOf (by decide) (by decide) (by decide) c l)

/-- The closure of `G` is inside `H`, which is what turns a `decide` over the
list into a fact about the inductive predicate. -/
theorem der_mem {H G : List Triple} (hP : Plain H) (hGH : ∀ t ∈ G, t ∈ H) {t : Triple}
    (h : Der G t) : t ∈ H :=
  Der_sound (hP.model hGH) h

end Plain

/-! ## Two concrete graphs

`graze` is refutable and `feed` is not, and they differ by one triple. Both
carry the same `owl:disjointWith` axiom and the same subclass chain, so the
difference cannot be read off the schema. -/

def rHerbivore : Term := "<http://ex.org/Herbivore>"
def rCarnivore : Term := "<http://ex.org/Carnivore>"
def rLion : Term := "<http://ex.org/Lion>"
def rLeo : Term := "<http://ex.org/leo>"

/-- Herbivore and Carnivore are disjoint, every Lion is a Carnivore, leo is a
Lion, and leo is a Herbivore. Contradictory, and NOT syntactically: the
Carnivore membership has to be derived before the clash is visible. -/
def graze : List Triple :=
  [ ⟨rHerbivore, RV.disjointWith, rCarnivore⟩,
    ⟨rLion, V.subClassOf, rCarnivore⟩,
    ⟨rLeo, V.type, rLion⟩,
    ⟨rLeo, V.type, rHerbivore⟩ ]

/-- The same ontology with the Herbivore assertion removed. Consistent. -/
def feed : List Triple :=
  [ ⟨rHerbivore, RV.disjointWith, rCarnivore⟩,
    ⟨rLion, V.subClassOf, rCarnivore⟩,
    ⟨rLeo, V.type, rLion⟩ ]

/-! ### The refutation of `graze` -/

/-- `rdfs9` premise order is `x rdf:type a`, `a rdfs:subClassOf b`. -/
def grazeStep : Step :=
  { rule := .rdfs9
    premises := [⟨rLeo, V.type, rLion⟩, ⟨rLion, V.subClassOf, rCarnivore⟩]
    conclusion := ⟨rLeo, V.type, rCarnivore⟩ }

/-- `cax-dw` premise order is `c1 owl:disjointWith c2`, `x rdf:type c1`,
`x rdf:type c2`. The third premise is the step above, not an asserted triple. -/
def grazeFinal : RefuteStep :=
  { rule := .caxDw
    premises :=
      [ ⟨rHerbivore, RV.disjointWith, rCarnivore⟩,
        ⟨rLeo, V.type, rHerbivore⟩,
        ⟨rLeo, V.type, rCarnivore⟩ ] }

def grazeRefutation : Refutation := ⟨[grazeStep], grazeFinal⟩

/-- The checker accepts it, by kernel computation rather than by assertion. -/
theorem the_checker_accepts_the_refutation :
    checkRefutation graze grazeRefutation = true := by decide

/-- **A concrete graph is refuted**, with the clash reached through an
inference. -/
theorem graze_is_refuted : Unsat graze :=
  refutation_sound graze grazeRefutation the_checker_accepts_the_refutation

/-! ### Six forgeries, each rejected

A gate that cannot fail is decoration. Each of these is a different way to lie
and each is settled by `decide`, so the rejection is computed and not claimed. -/

/-- The disjointness axiom is invented: `Herbivore owl:disjointWith Lion` is not
in the graph. -/
def forgeUnassertedAxiom : Refutation :=
  ⟨[grazeStep],
   { rule := .caxDw
     premises :=
       [ ⟨rHerbivore, RV.disjointWith, rLion⟩,
         ⟨rLeo, V.type, rHerbivore⟩,
         ⟨rLeo, V.type, rLion⟩ ] }⟩

/-- The first premise is a `rdfs:subClassOf` triple dressed as the axiom. It IS
in the graph, and it still is not a disjointness axiom. -/
def forgeWrongPredicate : Refutation :=
  ⟨[grazeStep],
   { rule := .caxDw
     premises :=
       [ ⟨rLion, V.subClassOf, rCarnivore⟩,
         ⟨rLeo, V.type, rLion⟩,
         ⟨rLeo, V.type, rCarnivore⟩ ] }⟩

/-- The two class memberships are about different individuals, which is the
whole content of `cax-dw` and the easiest thing for a hand-written engine to get
wrong. -/
def forgeTwoIndividuals : Refutation :=
  ⟨[grazeStep],
   { rule := .caxDw
     premises :=
       [ ⟨rHerbivore, RV.disjointWith, rCarnivore⟩,
         ⟨rLeo, V.type, rHerbivore⟩,
         ⟨rLion, V.type, rCarnivore⟩ ] }⟩

/-- The derivation prefix is forged: `rdfs9` does not conclude this. -/
def forgePrefix : Refutation :=
  ⟨[{ grazeStep with conclusion := ⟨rLeo, V.type, rHerbivore⟩ }], grazeFinal⟩

/-- No prefix at all, so the third premise is neither asserted nor derived. This
is the one a careless format would let through, because the triple is true. -/
def forgeMissingPrefix : Refutation := ⟨[], grazeFinal⟩

/-- The right three triples in the wrong order. Premise order is part of the
contract. -/
def forgeWrongOrder : Refutation :=
  ⟨[grazeStep],
   { rule := .caxDw
     premises :=
       [ ⟨rLeo, V.type, rHerbivore⟩,
         ⟨rHerbivore, RV.disjointWith, rCarnivore⟩,
         ⟨rLeo, V.type, rCarnivore⟩ ] }⟩

theorem rejects_an_unasserted_axiom : checkRefutation graze forgeUnassertedAxiom = false := by
  decide
theorem rejects_a_predicate_that_is_not_disjointWith :
    checkRefutation graze forgeWrongPredicate = false := by decide
theorem rejects_two_different_individuals : checkRefutation graze forgeTwoIndividuals = false := by
  decide
theorem rejects_a_forged_derivation_prefix : checkRefutation graze forgePrefix = false := by decide
theorem rejects_a_premise_that_was_never_derived :
    checkRefutation graze forgeMissingPrefix = false := by decide
theorem rejects_premises_in_the_wrong_order : checkRefutation graze forgeWrongOrder = false := by
  decide

/-! ### `feed` is not refutable, and that is proved rather than observed -/

/-- The closure of `feed`, written out. One derived triple: leo is a Carnivore. -/
def feedClosure : List Triple := ⟨rLeo, V.type, rCarnivore⟩ :: feed

theorem feed_closure_is_plain : Plain feedClosure := by
  constructor <;> decide

theorem feed_inside_its_closure : ∀ t ∈ feed, t ∈ feedClosure := by decide

/-- No `owl:sameAs` anywhere in the closure, so the hypothesis of the general
theorem is discharged rather than assumed. -/
theorem feed_has_no_sameAs (a b : Term) (h : Der feed ⟨a, V.sameAs, b⟩) : a = b :=
  absurd (feed_closure_is_plain.der_mem feed_inside_its_closure h)
    (feed_closure_is_plain.absent V.sameAs (by decide) (by decide) (by decide) a b)

/-- No individual is in both halves of the disjointness axiom, checked over the
closure by `decide` rather than by reading the file. -/
private theorem feed_closure_has_no_clash :
    ∀ t ∈ feedClosure, ∀ u ∈ feedClosure, ∀ v ∈ feedClosure,
      ¬(t.p = RV.disjointWith ∧ u.p = V.type ∧ u.o = t.s ∧
        v.p = V.type ∧ v.o = t.o ∧ v.s = u.s) := by decide

/-! `feed_is_not_refuted` below is about `Semantics.lean`'s `Conditions` and not
about the conditions `W3C.lean` quotes out of the OWL 2 RDF-Based Semantics
tables, because the interpretation it uses is a Herbrand one. `feedClosure`
carries `Lion rdfs:subClassOf Carnivore` and types nothing as an `rdfs:Class`, so
`W3C.sc_fwd`, Table 5.8 row 1 forward, fails on the `Lion ∈ IC` conjunct it
concludes, for every choice of `IP`.

The direction of transfer runs the unhelpful way here as well. `Unsat G` is a
universal negative over a model class, so a SMALLER class makes it EASIER to hold
and `¬ Unsat` HARDER. Positive refutations transfer outward for free, because
`W3CModel I IP G → Model I G`, and `w3cUnsat_of_unsat` at the end of this file
records that; `¬ Unsat` needs a `W3CModel` that also satisfies `RefuteConditions`.

Such a model is built at the end of this file and `feed_is_not_w3c_refuted` is
the theorem it carries, which is strictly stronger than the statement below. The
statement below is kept because it is proved by a different route, through the
closure machinery rather than through a hand-built finite structure, so the two
fail in different ways if either is broken. Until 15 September 2026 the caveat
was recorded only in `Semantics.lean`'s docstring, with a reason that turned out
to be wrong, and this file was byte-identical to the one that predates
`W3C.lean`. -/

/-- **A graph with a disjointness axiom that is NOT refutable.** Without this the
refutation layer would be consistent with a checker that accepts everything.

Relative to `Conditions` and to `RefuteConditions`. The same sentence over the
specification's model class is `feed_is_not_w3c_refuted` at the end of this
file, which is strictly stronger and implies this one. -/
theorem feed_is_not_refuted : ¬ Unsat feed := by
  refine not_unsat_of_joint_model (no_violation_means_a_joint_model feed feed_has_no_sameAs ?_)
  intro c1 c2 x h1 h2 h3
  have m1 := feed_closure_is_plain.der_mem feed_inside_its_closure h1
  have m2 := feed_closure_is_plain.der_mem feed_inside_its_closure h2
  have m3 := feed_closure_is_plain.der_mem feed_inside_its_closure h3
  exact feed_closure_has_no_clash _ m1 _ m2 _ m3 ⟨rfl, rfl, rfl, rfl, rfl, rfl⟩

/-- The attempt to refute the consistent graph, which differs from the accepted
refutation only in that `leo rdf:type Herbivore` is not available. -/
def feedAttempt : Refutation :=
  ⟨[{ rule := .rdfs9
      premises := [⟨rLeo, V.type, rLion⟩, ⟨rLion, V.subClassOf, rCarnivore⟩]
      conclusion := ⟨rLeo, V.type, rCarnivore⟩ }],
   grazeFinal⟩

/-- And the checker rejects it. Taken with `feed_is_not_refuted`, the rejection
is correct and not a limitation. -/
theorem the_checker_rejects_the_attempt_on_the_consistent_graph :
    checkRefutation feed feedAttempt = false := by decide

/-! ## The explosion, and the trap inside it

Over a refuted graph the disjointness-aware warrant is empty, so a derivation
certificate says nothing. The trap is that `certificate_sound`'s own verdict does
NOT collapse, because `Entails` quantifies over `Model I G` alone and that class
is never empty. A consumer reading `"ok":true` from `oo-cert` over a
self-contradicting ontology is reading a true sentence about a model class that
ignores disjointness. -/

def rJunk : Triple := ⟨"<http://ex.org/a>", "<http://ex.org/b>", "<http://ex.org/c>"⟩

/-- Any triple at all has the disjointness-aware warrant over `graze`. -/
theorem an_ordinary_certificate_is_worthless_over_the_refuted_graph :
    EntailsIn (RModel graze) rJunk :=
  unsat_entails_everything graze_is_refuted rJunk

def grazeClosure : List Triple := ⟨rLeo, V.type, rCarnivore⟩ :: graze

theorem graze_closure_is_plain : Plain grazeClosure := by
  constructor <;> decide

theorem graze_inside_its_closure : ∀ t ∈ graze, t ∈ grazeClosure := by decide

/-- **And the old verdict does not notice.** `graze` is refuted, yet `Entails
graze` is still a non-trivial relation: `herbrandL grazeClosure` is a model of
`graze` in which the junk triple is false. So a triple certificate over a
refuted graph checks green and means nothing, and nothing in
`certificate_sound` reports that. This is why `oo-refute guard` exists. -/
theorem the_junk_triple_is_not_in_the_closure : rJunk ∉ grazeClosure := by decide

/-- About `Semantics.lean`'s `Conditions`, because the interpretation is a
Herbrand one and `grazeClosure` types nothing as an `rdfs:Class`, so `W3C.sc_fwd`
fails on the `IC` conjunct it concludes. The same sentence over the
specification's model class is `the_old_verdict_does_not_notice_over_w3c` at the
end of this file, on a finite structure built for it. -/
theorem and_the_old_verdict_does_not_notice : ¬ Entails graze rJunk := fun h =>
  the_junk_triple_is_not_in_the_closure
    (h (herbrandL grazeClosure) (graze_closure_is_plain.model graze_inside_its_closure))

/-! ## The two remaining negative results, over the specification's model class

`feed_is_not_refuted` and `and_the_old_verdict_does_not_notice` are the last two
results in this repository that were about `Semantics.lean`'s `Conditions` and
not about the conditions `W3C.lean` quotes out of the tables. Until now the
reason was recorded as a checked obstruction: `feedClosure` and `grazeClosure`
both carry `Lion rdfs:subClassOf Carnivore` and type nothing as an `rdfs:Class`,
so `W3C.sc_fwd` fails on the `IC` conjunct it concludes, for every choice of
`IP`.

That obstruction was true and it was about the HERBRAND interpretations those two
theorems use. It was never an obstruction to the results themselves, and the
structure below settles both. It is the same kind of object as
`W3CWitness.lean`'s `live` and `svfI`: a hand-built finite model with the four
`rdfs:*` tables at the fixpoint Table 5.8's two directions force.

One carrier serves both graphs, because `feed` and `graze` differ by one triple
and the models differ by one row. `ICEXT(Herbivore)` is EMPTY over `feed` and
`{leo}` over `graze`, and everything else is shared, so the Boolean parameter
`leoH` below is exactly the difference between a graph with a model that respects
disjointness and a graph without one.

**The direction of transfer is the point, and it runs opposite ways for the two
theorems.** `W3CEntails` quantifies over a SMALLER class than `Entails`, so a
non-entailment over it is a STRONGER claim and needs its own witness, which is
`grazeI` below. `Unsat` is itself a universal negative over a model class, so
`W3CUnsat` is a WEAKER claim than `Unsat` and `¬ W3CUnsat` is the stronger one;
`w3cUnsat_of_unsat` records the easy half, and `feed_is_not_w3c_refuted` is the
half that needed the model. Both of the old statements follow from the new ones,
and `feed_is_not_refuted_via_the_w3c_model` derives one of them to show that nothing was
lost. -/

/-- The thirteen elements. One individual, three classes, seven vocabulary
denotations that have to be told apart, and a junk element absorbing every other
IRI with both extensions empty. -/
inductive RefD where
  /-- `leo`. -/
  | leo
  /-- `Herbivore`. Its class extension is EMPTY over `feed` and `{leo}` over
  `graze`, and that one row is the entire difference between the two models. -/
  | herb
  /-- `Carnivore`, with `ICEXT(Carnivore) = {leo}` in both, FORCED by
  `Lion rdfs:subClassOf Carnivore` through Table 5.8 row 1 forward. -/
  | carn
  /-- `Lion`, with `ICEXT(Lion) = {leo}`, asserted by both graphs. -/
  | lion
  /-- `rdf:type`. -/
  | ty
  /-- `rdfs:subClassOf`. -/
  | sco
  /-- `rdfs:subPropertyOf`. -/
  | spo
  /-- `rdfs:domain`, whose extension is empty in both models. -/
  | dm
  /-- `rdfs:range`, likewise. -/
  | rg
  /-- `owl:disjointWith`. `W3C` states no row for it, because no rule of the
  derivation checker consumes one; `RefuteConditions` is where its reading
  lives. -/
  | dw
  /-- `owl:sameAs`, carrying the diagonal, as RBS Table 5.9 row 1 requires. -/
  | sa
  /-- `rdfs:Class`. -/
  | cls
  /-- Every other IRI, the junk triple's three IRIs included, which is what makes
  that triple false here. -/
  | other
deriving DecidableEq, Repr

namespace RefD

/-- The carrier as a list, so quantification over it is decidable without
Mathlib. -/
def all : List RefD :=
  [leo, herb, carn, lion, ty, sco, spo, dm, rg, dw, sa, cls, other]

theorem mem_all (w : RefD) : w ∈ all := by cases w <;> decide

instance decForall (p : RefD → Prop) [DecidablePred p] : Decidable (∀ w, p w) :=
  decidable_of_iff (∀ w ∈ all, p w) ⟨fun h w => h w (mem_all w), fun h w _ => h w⟩

instance decExists (p : RefD → Prop) [DecidablePred p] : Decidable (∃ w, p w) :=
  decidable_of_iff (∃ w ∈ all, p w)
    ⟨fun ⟨w, _, h⟩ => ⟨w, h⟩, fun ⟨w, h⟩ => ⟨w, mem_all w, h⟩⟩

end RefD

/-- The denotations. Everything absent denotes `other`, `rJunk`'s three IRIs
included. -/
def refTable : List (Term × RefD) :=
  [ (V.type, .ty), (V.subClassOf, .sco), (V.subPropertyOf, .spo),
    (V.domain, .dm), (V.range, .rg), (V.sameAs, .sa), (V.Class, .cls),
    (RV.disjointWith, .dw),
    (rHerbivore, .herb), (rCarnivore, .carn), (rLion, .lion), (rLeo, .leo) ]

/-- Terms denote their table entry, or `other`. -/
def refι (t : Term) : RefD := (List.lookup t refTable).getD .other

/-- `IP`: the five elements that carry pairs, so the structure is
bridge-coherent. `rdfs:domain` and `rdfs:range` are deliberately NOT in it, and
the reason is worth reading once because it is not a preference. Their extensions
are empty here. Put a property with an empty extension into `IP` and Table 5.8
row 3 BACKWARD fires on it for every class, because the empty set of subjects is
contained in every class extension, so `IEXT(rdfs:domain)` acquires the pairs
`(rdfs:domain, c)` for each `c` in `IC`. Its subject set is then
`{rdfs:domain}`, and the largest class extension in this model is `{leo}`, so row
3 FORWARD immediately fails on those same pairs. There is no fixpoint with them
in; leaving them out of `IP` is what makes one exist. -/
def refIsIP : RefD → Bool
  | .ty | .sco | .spo | .dw | .sa => true
  | _ => false

/-- The extensions, parameterised by whether `leo` is a `Herbivore`. `false` is
the model of `feed` and `true` the model of `graze`.

THREE rows are chosen: `ICEXT(Lion)`, `ICEXT(Herbivore)` and
`IEXT(owl:disjointWith)`. `ICEXT(Carnivore)` is forced by the asserted subclass
triple, the `owl:sameAs` diagonal by Table 5.9, and the four `rdfs:*` tables are
the fixpoint of Table 5.8's two directions. `rdfs:domain` and `rdfs:range` come
out EMPTY, because every property here has a subject or an object that is a class
rather than `leo`, and `{leo}` is the largest class extension in the model. -/
def refIext (leoH : Bool) : RefD → RefD → RefD → Bool
  -- ICEXT(rdfs:Class) = IC = {Herbivore, Carnivore, Lion}.
  | .ty, w, .cls => w == .herb || w == .carn || w == .lion
  -- ICEXT(Lion) = ICEXT(Carnivore) = {leo}. The Carnivore row is forced: Table 5.8
  -- row 1 forward on the asserted `Lion rdfs:subClassOf Carnivore` demands it.
  | .ty, w, .lion | .ty, w, .carn => w == .leo
  -- The one row that differs. Over `feed` it is empty, which is what lets
  -- RefuteConditions hold; over `graze` it is {leo}, which is what makes `graze`
  -- refutable and is why no model of `graze` respects disjointness.
  | .ty, w, .herb => leoH && w == .leo
  -- ICEXT of everything else is empty.
  | .ty, _, _ => false
  -- IEXT(rdfs:subClassOf): the pairs of IC whose extensions nest. Over `feed`
  -- Herbivore is below everything because its extension is empty; over `graze` all
  -- three extensions are equal and the table is all nine pairs.
  | .sco, .herb, v => v == .herb || v == .carn || v == .lion
  | .sco, .carn, v | .sco, .lion, v => (leoH && v == .herb) || v == .carn || v == .lion
  | .sco, _, _ => false
  -- IEXT(rdfs:subPropertyOf): the pairs of IP whose extensions nest. The one
  -- off-diagonal entry is FORCED and looks odd until it is checked:
  -- IEXT(owl:disjointWith) is the single pair (Herbivore, Carnivore), that pair is
  -- also in IEXT(rdfs:subClassOf), and Table 5.8 row 2 is an iff, so
  -- `owl:disjointWith rdfs:subPropertyOf rdfs:subClassOf` holds here. RDF has no
  -- sortal separation and this is what that costs.
  | .spo, .ty, v => v == .ty
  | .spo, .sco, v => v == .sco
  | .spo, .spo, v => v == .spo
  | .spo, .dw, v => v == .sco || v == .dw
  | .spo, .sa, v => v == .sa
  | .spo, _, _ => false
  -- The asserted disjointness axiom.
  | .dw, .herb, v => v == .carn
  | .dw, _, _ => false
  -- RBS Table 5.9 row 1.
  | .sa, u, v => u == v
  -- Everything else, `rdfs:domain`, `rdfs:range` and `other` included, is empty.
  | _, _, _ => false

/-- The interpretation, at either row. -/
def refI (leoH : Bool) : Interp where
  D := RefD
  ι := refι
  iext := fun p x y => refIext leoH p x y = true

/-- `IP` as a predicate. -/
def refIP (x : RefD) : Prop := refIsIP x = true

instance : DecidablePred refIP := fun x => decidable_of_iff (refIsIP x = true) Iff.rfl

/-! Each condition is checked for BOTH models at once, by quantifying over the
Boolean row. Two models for the price of one `decide`, and neither can drift from
the other. -/

theorem ref_sc_fwd : ∀ (h : Bool) (a b : RefD), refIext h .sco a b = true →
    refIext h .ty a .cls = true ∧ refIext h .ty b .cls = true ∧
    ∀ x, refIext h .ty x a = true → refIext h .ty x b = true := by decide

theorem ref_sc_bwd : ∀ (h : Bool) (a b : RefD), refIext h .ty a .cls = true →
    refIext h .ty b .cls = true →
    (∀ x, refIext h .ty x a = true → refIext h .ty x b = true) →
    refIext h .sco a b = true := by decide

theorem ref_sp_fwd : ∀ (h : Bool) (a b : RefD), refIext h .spo a b = true →
    refIP a ∧ refIP b ∧ ∀ x y, refIext h a x y = true → refIext h b x y = true := by decide

theorem ref_sp_bwd : ∀ (h : Bool) (a b : RefD), refIP a → refIP b →
    (∀ x y, refIext h a x y = true → refIext h b x y = true) →
    refIext h .spo a b = true := by decide

theorem ref_dom_fwd : ∀ (h : Bool) (p c : RefD), refIext h (refι V.domain) p c = true →
    refIP p ∧ refIext h .ty c .cls = true ∧
    ∀ x y, refIext h p x y = true → refIext h .ty x c = true := by decide

theorem ref_dom_bwd : ∀ (h : Bool) (p c : RefD), refIP p → refIext h .ty c .cls = true →
    (∀ x y, refIext h p x y = true → refIext h .ty x c = true) →
    refIext h (refι V.domain) p c = true := by decide

theorem ref_rng_fwd : ∀ (h : Bool) (p c : RefD), refIext h (refι V.range) p c = true →
    refIP p ∧ refIext h .ty c .cls = true ∧
    ∀ x y, refIext h p x y = true → refIext h .ty y c = true := by decide

theorem ref_rng_bwd : ∀ (h : Bool) (p c : RefD), refIP p → refIext h .ty c .cls = true →
    (∀ x y, refIext h p x y = true → refIext h .ty y c = true) →
    refIext h (refι V.range) p c = true := by decide

theorem ref_eqc_fwd : ∀ (h : Bool) (a b : RefD),
    refIext h (refι V.equivalentClass) a b = true →
    refIext h .ty a .cls = true ∧ refIext h .ty b .cls = true ∧
    ∀ x, (refIext h .ty x a = true ↔ refIext h .ty x b = true) := by decide

theorem ref_eqp_fwd : ∀ (h : Bool) (a b : RefD),
    refIext h (refι V.equivalentProperty) a b = true →
    refIP a ∧ refIP b ∧ ∀ x y, (refIext h a x y = true ↔ refIext h b x y = true) := by decide

theorem ref_same_fwd : ∀ (h : Bool) (a b : RefD), refIext h (refι V.sameAs) a b = true →
    a = b := by decide

theorem ref_inv_fwd : ∀ (h : Bool) (p r : RefD), refIext h (refι V.inverseOf) p r = true →
    refIP p ∧ refIP r ∧ ∀ x y, (refIext h p x y = true ↔ refIext h r y x = true) := by decide

theorem ref_sym_fwd : ∀ (h : Bool) (p : RefD),
    refIext h .ty p (refι V.symmetricProperty) = true →
    ∀ x y, refIext h p x y = true → refIext h p y x = true := by decide

theorem ref_trp_fwd : ∀ (h : Bool) (p : RefD),
    refIext h .ty p (refι V.transitiveProperty) = true →
    ∀ x y z, refIext h p x y = true → refIext h p y z = true →
      refIext h p x z = true := by decide

theorem ref_svf_eq : ∀ (h : Bool) (z c p : RefD),
    refIext h (refι V.someValuesFrom) z c = true → refIext h (refι V.onProperty) z p = true →
    ∀ x, (refIext h .ty x z = true ↔ ∃ y, refIext h p x y = true ∧
      refIext h .ty y c = true) := by decide

theorem ref_avf_eq : ∀ (h : Bool) (z c p : RefD),
    refIext h (refι V.allValuesFrom) z c = true → refIext h (refι V.onProperty) z p = true →
    ∀ x, (refIext h .ty x z = true ↔ ∀ y, refIext h p x y = true →
      refIext h .ty y c = true) := by decide

theorem ref_hv_eq : ∀ (h : Bool) (z a p : RefD), refIext h (refι V.hasValue) z a = true →
    refIext h (refι V.onProperty) z p = true →
    ∀ x, (refIext h .ty x z = true ↔ refIext h p x a = true) := by decide

theorem ref_restr_IC : ∀ (h : Bool) (x : RefD),
    refIext h .ty x (refι V.Restriction) = true → refIext h .ty x .cls = true := by decide

theorem ref_svf_typ : ∀ (h : Bool) (z c : RefD),
    refIext h (refι V.someValuesFrom) z c = true →
    refIext h .ty z (refι V.Restriction) = true ∧ refIext h .ty c .cls = true := by decide

theorem ref_avf_typ : ∀ (h : Bool) (z c : RefD),
    refIext h (refι V.allValuesFrom) z c = true →
    refIext h .ty z (refι V.Restriction) = true ∧ refIext h .ty c .cls = true := by decide

theorem ref_onp_typ : ∀ (h : Bool) (z p : RefD), refIext h (refι V.onProperty) z p = true →
    refIext h .ty z (refι V.Restriction) = true ∧ refIP p := by decide

/-- The conditions, at either row. -/
theorem ref_meets_the_w3c_conditions (h : Bool) : W3C (refI h) refIP where
  sc_fwd := ref_sc_fwd h
  sc_bwd := ref_sc_bwd h
  sp_fwd := ref_sp_fwd h
  sp_bwd := ref_sp_bwd h
  dom_fwd := ref_dom_fwd h
  dom_bwd := ref_dom_bwd h
  rng_fwd := ref_rng_fwd h
  rng_bwd := ref_rng_bwd h
  eqc_fwd := ref_eqc_fwd h
  eqp_fwd := ref_eqp_fwd h
  same_fwd := ref_same_fwd h
  inv_fwd := ref_inv_fwd h
  sym_fwd := ref_sym_fwd h
  trp_fwd := ref_trp_fwd h
  svf_eq := ref_svf_eq h
  avf_eq := ref_avf_eq h
  hv_eq := ref_hv_eq h
  restr_IC := ref_restr_IC h
  svf_typ := ref_svf_typ h
  avf_typ := ref_avf_typ h
  onp_typ := ref_onp_typ h

theorem ref_feed_facts :
    ∀ t ∈ feed, refIext false (refι t.p) (refι t.s) (refι t.o) = true := by decide

theorem ref_graze_facts :
    ∀ t ∈ graze, refIext true (refι t.p) (refι t.s) (refι t.o) = true := by decide

/-- **A `W3CModel` of `feed`.** -/
theorem feedI_is_a_w3c_model : W3CModel (refI false) refIP feed where
  conds := ref_meets_the_w3c_conditions false
  facts := ref_feed_facts
  int_eq := fun c l _ hc => absurd hc (not_mem_pred feed V.intersectionOf (by decide) c l)
  uni_eq := fun c l _ hc => absurd hc (not_mem_pred feed V.unionOf (by decide) c l)
  oneOf_eq := fun c l _ hc => absurd hc (not_mem_pred feed V.oneOf (by decide) c l)

/-- **A `W3CModel` of `graze`.** `graze` is refuted, and that does not stop it
having models: `Model` and `W3CModel` both ignore disjointness, which is the
whole content of `and_the_old_verdict_does_not_notice`. -/
theorem grazeI_is_a_w3c_model : W3CModel (refI true) refIP graze where
  conds := ref_meets_the_w3c_conditions true
  facts := ref_graze_facts
  int_eq := fun c l _ hc => absurd hc (not_mem_pred graze V.intersectionOf (by decide) c l)
  uni_eq := fun c l _ hc => absurd hc (not_mem_pred graze V.unionOf (by decide) c l)
  oneOf_eq := fun c l _ hc => absurd hc (not_mem_pred graze V.oneOf (by decide) c l)

theorem ref_feed_dw :
    ∀ a b : RefD, refIext false (refι RV.disjointWith) a b = true →
      ∀ x, refIext false .ty x a = true → refIext false .ty x b = true → False := by decide

/-- And the `feed` model respects disjointness, which is what
`RefuteConditions` asks and what `graze`'s model cannot do. -/
theorem feedI_respects_disjointness : RefuteConditions (refI false) where
  dw := ref_feed_dw

theorem ref_models_are_live_raw :
    (refIext false .ty .leo .lion = true ∧ refIext false .ty .leo .carn = true ∧
      ¬ refIext false .ty .leo .herb = true) ∧
    (refIext true .ty .leo .herb = true) ∧
    (refIext false .sco .lion .carn = true ∧ ¬ refIext false .sco .carn .herb = true) ∧
    (refIext false (refι RV.disjointWith) .herb .carn = true) ∧
    (refIext false .spo .dw .sco = true) ∧
    (∀ h : Bool, ∀ p x y : RefD, refIext h p x y = true → refIP p) := by decide

/-- **Neither model is degenerate.** `leo` is a `Lion` and a `Carnivore` in both
and a `Herbivore` in exactly one; the subclass table is not everything, so
`sc_bwd` is not satisfied by collapse; the disjointness axiom is really there;
the one forced off-diagonal `rdfs:subPropertyOf` entry is really there; and both
models are bridge-coherent, so both are in the image of the bridge in `W3C.lean`
where the Herbrand interpretations they replace are not. -/
theorem ref_models_are_live :
    ((refI false).cext .lion .leo ∧ (refI false).cext .carn .leo ∧
      ¬ (refI false).cext .herb .leo) ∧
    (refI true).cext .herb .leo ∧
    ((refI false).sc .lion .carn ∧ ¬ (refI false).sc .carn .herb) ∧
    (refI false).iext ((refI false).ι RV.disjointWith) .herb .carn ∧
    (refI false).sp .dw .sco ∧
    (∀ h : Bool, ∀ p x y : RefD, (refI h).iext p x y → refIP p) :=
  ref_models_are_live_raw

/-- `Unsat` over the specification's model class. A SMALLER class of
interpretations makes this EASIER to satisfy than `Unsat`, which is the direction
that matters: `w3cUnsat_of_unsat` is free and `¬ W3CUnsat` is the thing that
needs a model. -/
def W3CUnsat (G : List Triple) : Prop :=
  ∀ (I : Interp) (IP : I.D → Prop), ¬ (W3CModel I IP G ∧ RefuteConditions I)

/-- **A refutation rules out the conforming interpretations too.** The easy half,
and the one `refutation_sound`'s verdict needs: every `W3CModel` of `G` is a
`Model` of `G`, so a graph with no disjointness-respecting `Model` has no
disjointness-respecting `W3CModel` either. -/
theorem w3cUnsat_of_unsat {G : List Triple} (h : Unsat G) : W3CUnsat G :=
  fun I _ hw => h I ⟨hw.1.toModel, hw.2⟩

/-- The `W3CUnsat` counterpart of `not_unsat_of_joint_model`, and it is trivial
where that one is not: `W3CUnsat` is stated directly over `W3CModel`, so a joint
model refutes it by unfolding. Recorded so that the transfer table in
`W3CWitness.lean` has no row without a theorem behind it. -/
theorem not_w3cUnsat_of_joint_w3c_model {G : List Triple}
    (h : ∃ (I : Interp) (IP : I.D → Prop), W3CModel I IP G ∧ RefuteConditions I) :
    ¬ W3CUnsat G := by
  obtain ⟨I, IP, hM, hR⟩ := h
  exact fun hu => hu I IP ⟨hM, hR⟩

/-- **`feed` is not refutable over the specification's model class**, which is
strictly stronger than `feed_is_not_refuted` and is what that theorem's note said
was missing. The model is `refI false` and it satisfies `RefuteConditions`
because `ICEXT(Herbivore)` is empty there, which is exactly what removing
`leo rdf:type Herbivore` from `graze` buys. -/
theorem feed_is_not_w3c_refuted : ¬ W3CUnsat feed :=
  fun h => h (refI false) refIP ⟨feedI_is_a_w3c_model, feedI_respects_disjointness⟩

/-- And the old statement follows, so nothing was lost in the strengthening.
`feed_is_not_refuted` above proves the same thing by a different route, through
the closure machinery rather than through a hand-built model, and the two are
kept side by side because they fail in different ways if either is broken. -/
theorem feed_is_not_refuted_via_the_w3c_model : ¬ Unsat feed :=
  fun h => feed_is_not_w3c_refuted (w3cUnsat_of_unsat h)

theorem grazeI_refutes_the_junk_triple :
    ¬ refIext true (refι rJunk.p) (refι rJunk.s) (refι rJunk.o) = true := by decide

/-- **And the old verdict does not notice over the specification's model class
either.** `graze` is refuted, and `W3CEntails graze` is still a non-trivial
relation: `refI true` is a `W3CModel` of `graze` in which the junk triple is
false, because its three IRIs all denote the junk element, whose extension is
empty.

So a derivation certificate over a refuted graph checks green and means nothing,
and that is now a statement about the class `W3C.lean` quotes out of the tables
rather than only about `Semantics.lean`'s weaker `Conditions`. This is why
`oo-refute guard` exists. -/
theorem the_old_verdict_does_not_notice_over_w3c : ¬ W3CEntails graze rJunk := fun h =>
  absurd (h (refI true) refIP grazeI_is_a_w3c_model) grazeI_refutes_the_junk_triple

/-! ## Axioms, pinned -/

/-- info: 'OOCert.closure_is_a_model' does not depend on any axioms -/
#guard_msgs in
#print axioms closure_is_a_model

/-- info: 'OOCert.Der_sound' does not depend on any axioms -/
#guard_msgs in
#print axioms Der_sound

/-- info: 'OOCert.no_violation_means_a_joint_model' does not depend on any axioms -/
#guard_msgs in
#print axioms no_violation_means_a_joint_model

/-- info: 'OOCert.unsat_means_a_violation' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms unsat_means_a_violation

/-- info: 'OOCert.graze_is_refuted' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms graze_is_refuted

/-- info: 'OOCert.feed_is_not_refuted' depends on axioms: [propext] -/
#guard_msgs in
#print axioms feed_is_not_refuted

/-- info: 'OOCert.and_the_old_verdict_does_not_notice' depends on axioms: [propext] -/
#guard_msgs in
#print axioms and_the_old_verdict_does_not_notice

/-- info: 'OOCert.feedI_is_a_w3c_model' depends on axioms: [propext] -/
#guard_msgs in
#print axioms feedI_is_a_w3c_model

/-- info: 'OOCert.grazeI_is_a_w3c_model' depends on axioms: [propext] -/
#guard_msgs in
#print axioms grazeI_is_a_w3c_model

/-- info: 'OOCert.ref_models_are_live' depends on axioms: [propext] -/
#guard_msgs in
#print axioms ref_models_are_live

/-- info: 'OOCert.w3cUnsat_of_unsat' does not depend on any axioms -/
#guard_msgs in
#print axioms w3cUnsat_of_unsat

/-- info: 'OOCert.not_w3cUnsat_of_joint_w3c_model' does not depend on any axioms -/
#guard_msgs in
#print axioms not_w3cUnsat_of_joint_w3c_model

/-- info: 'OOCert.feed_is_not_w3c_refuted' depends on axioms: [propext] -/
#guard_msgs in
#print axioms feed_is_not_w3c_refuted

/-- info: 'OOCert.the_old_verdict_does_not_notice_over_w3c' depends on axioms: [propext] -/
#guard_msgs in
#print axioms the_old_verdict_does_not_notice_over_w3c

end OOCert
