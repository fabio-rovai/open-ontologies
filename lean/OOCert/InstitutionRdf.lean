import OOCert.Comorphism
import OOCert.Witness

/-!
# Two RDF institutions, and the comorphism between them that does not reflect

`Institution.lean` is worth nothing until something is one. This file makes two
things one, out of material that was already in this repository, and proves the
satisfaction condition for both.

| institution | models | morphisms may rename |
|---|---|---|
| `Rdf.simple` | every `OOCert.Interp` | ANY term, subject only to landing in the target vocabulary |
| `Rdf.rl` | the `OOCert.Interp`s satisfying `OOCert.Conditions` | the same, EXCEPT the reserved vocabulary, which it must fix |

The second column is the content. Satisfaction of a triple is invariant under
any renaming at all, which is why `Rdf.simple` can take arbitrary renamings; the
OWL 2 RL semantic conditions are NOT, because they are conditions stated at
fixed terms, and `conditions_reduct` is the theorem that says a renaming fixing
those terms carries them along. A renaming that moved `rdf:type` would take a
model of the conditions to something that is not one, so `Rdf.rl` cannot admit
it and says so in its morphism type rather than in a comment.

## The design choices, and which way each one cuts

Read these against the weakest-conditions section of `Institution.lean`.

* **`Sen S` constrains the PREDICATE position only.** A sentence over `S` is a
  triple whose predicate is in `S`; subject and object are unconstrained. That is
  the weakest restriction that lets `senMap` land in `Sen S'`, and it is all that
  `InstitutionFol.lean`'s comorphism needs: its background axioms range over the
  signature, and the predicate position is the only one they have to cover.
  Constraining all three would remove sentences, and removing sentences removes
  instances of the satisfaction condition.
* **`Mod S` does not depend on `S`.** An `OOCert.Interp` denotes EVERY term, so
  there is no reduct-to-the-signature to take and nothing for `S` to cut down.
  Cutting the class down anyway would strengthen `Entails` and weaken the
  satisfaction condition at the same time, for nothing.
* **`Rdf.simple` morphisms carry no condition beyond mapping the signature into
  the target.** Requiring them to fix the reserved vocabulary, so that the two
  institutions could share a morphism type, would remove morphisms from
  `Rdf.simple` and weaken its satisfaction condition. The two therefore have
  DIFFERENT morphism types, which is the honest outcome: the logical vocabulary
  of a logic is whatever its semantic conditions pin down, and for simple RDF
  interpretations that is nothing.

## What this file proves about the checker

`rl_entails_gives_entails` connects the institution to `Soundness.lean`: an
entailment in `Rdf.rl` is an `OOCert.Entails`. **The converse is FALSE, and it is
refuted rather than merely unclaimed.** `OOCert.Model` carries four conditions
(`int`, `int2`, `uni`, `oneOf`) stated over `Chain G`, the RDF list read off the
ASSERTED GRAPH. They are relative to a graph, not to a signature, so they are not
sentences of this institution and there is nowhere in an institution to put them.
`OOCert.Entails` therefore quantifies over a SMALLER class of interpretations and
is the weaker relation, strictly:
`InstitutionWitness.the_checker_entails_more_than_the_institution` exhibits a
premise set and a conclusion that `Entails` holds of and `Rdf.rl.EntailsL` does
not, using `cls-oo` and the Herbrand model of its premises. Decision 0009 records
this as the open edge of the work.
-/
namespace OOCert

namespace Rdf

/-! ## The reserved vocabulary

Exactly the terms `OOCert.Conditions` mentions. `owl:intersectionOf`,
`owl:unionOf`, `owl:oneOf`, `rdf:first`, `rdf:rest` and `rdf:nil` are NOT here:
they occur in `OOCert.Model`'s list fields and in no field of `Conditions`, and
this institution's models are `Conditions` only. Adding them would constrain
morphisms that nothing here needs constrained.

`Reserved` is a disjunction of equations rather than membership in a list on
purpose. Every proof below is `Or.inl rfl` or a chain of `Or.inr`, which the
kernel checks in no time, where `t ∈ reservedList` would put fifteen
fifty-character IRIs through `String.decEq` at every use site. -/

/-- A term whose denotation `OOCert.Conditions` constrains. -/
def Reserved (t : Term) : Prop :=
  t = V.type ∨ t = V.subClassOf ∨ t = V.subPropertyOf ∨ t = V.domain ∨ t = V.range ∨
  t = V.transitiveProperty ∨ t = V.symmetricProperty ∨ t = V.inverseOf ∨ t = V.sameAs ∨
  t = V.equivalentClass ∨ t = V.equivalentProperty ∨ t = V.onProperty ∨
  t = V.someValuesFrom ∨ t = V.allValuesFrom ∨ t = V.hasValue

namespace Reserved
theorem type : Reserved V.type := Or.inl rfl
theorem subClassOf : Reserved V.subClassOf := Or.inr (Or.inl rfl)
theorem subPropertyOf : Reserved V.subPropertyOf := Or.inr (Or.inr (Or.inl rfl))
theorem domain : Reserved V.domain := Or.inr (Or.inr (Or.inr (Or.inl rfl)))
theorem range : Reserved V.range := Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))
theorem transitiveProperty : Reserved V.transitiveProperty :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl)))))
theorem symmetricProperty : Reserved V.symmetricProperty :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))))
theorem inverseOf : Reserved V.inverseOf :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl)))))))
theorem sameAs : Reserved V.sameAs :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))))))
theorem equivalentClass : Reserved V.equivalentClass :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl)))))))))
theorem equivalentProperty : Reserved V.equivalentProperty :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr
    (Or.inr (Or.inl rfl))))))))))
theorem onProperty : Reserved V.onProperty :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr
    (Or.inr (Or.inr (Or.inl rfl)))))))))))
theorem someValuesFrom : Reserved V.someValuesFrom :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr
    (Or.inr (Or.inr (Or.inr (Or.inl rfl))))))))))))
theorem allValuesFrom : Reserved V.allValuesFrom :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr
    (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl)))))))))))))
theorem hasValue : Reserved V.hasValue :=
  Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr
    (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr rfl)))))))))))))
end Reserved

/-! ## Renaming, and the reduct it induces -/

/-- A renaming applied to a triple. -/
def renameT (r : Term → Term) (t : Triple) : Triple := ⟨r t.s, r t.p, r t.o⟩

/-- The reduct of an interpretation along a renaming: the same domain and the
same property extensions, with each term denoting what its image denoted.

This is model reduction in the institution sense. It runs AGAINST the renaming,
which is the contravariance that makes the satisfaction condition say something:
a sentence is pushed forward and a model is pulled back, and they meet. -/
def reduct (r : Term → Term) (I : Interp) : Interp where
  D := I.D
  ι := fun t => I.ι (r t)
  iext := I.iext

/-- **Satisfaction of a triple is invariant under any renaming whatever.** It is
`Iff.rfl`, and that is the point: nothing about a triple's truth depends on the
spelling of its terms, so the simple institution below can admit every renaming
as a signature morphism. -/
theorem reduct_sat (r : Term → Term) (I : Interp) (t : Triple) :
    (reduct r I).sat t ↔ I.sat (renameT r t) := Iff.rfl

/-! ### The conditions travel exactly as far as the renaming leaves them alone -/

section Transport

/-! `hr` is written out on every statement below rather than carried by a
`variable`, because a `variable` that appears only in a proof term is not
included in the signature and every one of these needs it. -/

theorem reduct_ι_res {r : Term → Term} (hr : ∀ t, Reserved t → r t = t) (I : Interp)
    {v : Term} (hv : Reserved v) : (reduct r I).ι v = I.ι v :=
  congrArg I.ι (hr v hv)

theorem reduct_iext_res {r : Term → Term} (hr : ∀ t, Reserved t → r t = t) (I : Interp)
    {v : Term} (hv : Reserved v) (x y : I.D) :
    (reduct r I).iext ((reduct r I).ι v) x y ↔ I.iext (I.ι v) x y := by
  rw [reduct_ι_res hr I hv]
  exact Iff.rfl

theorem reduct_cext {r : Term → Term} (hr : ∀ t, Reserved t → r t = t) (I : Interp)
    (c x : I.D) : (reduct r I).cext c x ↔ I.cext c x := by
  show (reduct r I).iext ((reduct r I).ι V.type) x c ↔ _
  rw [reduct_ι_res hr I Reserved.type]
  exact Iff.rfl

theorem reduct_sc {r : Term → Term} (hr : ∀ t, Reserved t → r t = t) (I : Interp)
    (a b : I.D) : (reduct r I).sc a b ↔ I.sc a b :=
  reduct_iext_res hr I Reserved.subClassOf a b

theorem reduct_sp {r : Term → Term} (hr : ∀ t, Reserved t → r t = t) (I : Interp)
    (a b : I.D) : (reduct r I).sp a b ↔ I.sp a b :=
  reduct_iext_res hr I Reserved.subPropertyOf a b

/-- **The OWL 2 RL semantic conditions survive a renaming that fixes the
reserved vocabulary.** Twenty-three fields, each one a rewrite at the terms the
condition names, and nothing else changes: the domain and the property
extensions are the same objects.

This is the obligation `Rdf.rl.modRed` cannot be written without, and it is the
reason `Rdf.rl` and `Rdf.simple` have different morphism types. -/
theorem conditions_reduct {r : Term → Term} (hr : ∀ t, Reserved t → r t = t) (I : Interp)
    (h : Conditions I) : Conditions (reduct r I) where
  sc_sub := fun a b hab x hx =>
    (reduct_cext hr I b x).mpr
      (h.sc_sub a b ((reduct_sc hr I a b).mp hab) x ((reduct_cext hr I a x).mp hx))
  sc_trans := fun a b c hab hbc =>
    (reduct_sc hr I a c).mpr
      (h.sc_trans a b c ((reduct_sc hr I a b).mp hab) ((reduct_sc hr I b c).mp hbc))
  sp_sub := fun a b hab x y hxy =>
    h.sp_sub a b ((reduct_sp hr I a b).mp hab) x y hxy
  sp_trans := fun a b c hab hbc =>
    (reduct_sp hr I a c).mpr
      (h.sp_trans a b c ((reduct_sp hr I a b).mp hab) ((reduct_sp hr I b c).mp hbc))
  dom := fun p c hpc x y hxy =>
    (reduct_cext hr I c x).mpr
      (h.dom p c ((reduct_iext_res hr I Reserved.domain p c).mp hpc) x y hxy)
  rng := fun p c hpc x y hxy =>
    (reduct_cext hr I c y).mpr
      (h.rng p c ((reduct_iext_res hr I Reserved.range p c).mp hpc) x y hxy)
  trp := fun p hp x y z hxy hyz =>
    h.trp p (by
      have := (reduct_cext hr I ((reduct r I).ι V.transitiveProperty) p).mp hp
      rwa [reduct_ι_res hr I Reserved.transitiveProperty] at this) x y z hxy hyz
  symp := fun p hp x y hxy =>
    h.symp p (by
      have := (reduct_cext hr I ((reduct r I).ι V.symmetricProperty) p).mp hp
      rwa [reduct_ι_res hr I Reserved.symmetricProperty] at this) x y hxy
  inv := fun p q hpq x y =>
    h.inv p q ((reduct_iext_res hr I Reserved.inverseOf p q).mp hpq) x y
  same := fun a b hab =>
    h.same a b ((reduct_iext_res hr I Reserved.sameAs a b).mp hab)
  eqc := fun a b hab => by
    have := h.eqc a b ((reduct_iext_res hr I Reserved.equivalentClass a b).mp hab)
    exact ⟨(reduct_sc hr I a b).mpr this.1, (reduct_sc hr I b a).mpr this.2⟩
  eqp := fun a b hab => by
    have := h.eqp a b ((reduct_iext_res hr I Reserved.equivalentProperty a b).mp hab)
    exact ⟨(reduct_sp hr I a b).mpr this.1, (reduct_sp hr I b a).mpr this.2⟩
  svf := fun rr p c hop hsv x y hxy hy =>
    (reduct_cext hr I rr x).mpr
      (h.svf rr p c ((reduct_iext_res hr I Reserved.onProperty rr p).mp hop)
        ((reduct_iext_res hr I Reserved.someValuesFrom rr c).mp hsv)
        x y hxy ((reduct_cext hr I c y).mp hy))
  avf := fun rr p c hop hav x y hx hxy =>
    (reduct_cext hr I c y).mpr
      (h.avf rr p c ((reduct_iext_res hr I Reserved.onProperty rr p).mp hop)
        ((reduct_iext_res hr I Reserved.allValuesFrom rr c).mp hav)
        x y ((reduct_cext hr I rr x).mp hx) hxy)
  hv := fun rr p v hop hhv x =>
    (reduct_cext hr I rr x).trans
      (h.hv rr p v ((reduct_iext_res hr I Reserved.onProperty rr p).mp hop)
        ((reduct_iext_res hr I Reserved.hasValue rr v).mp hhv) x)
  svf_sc := fun c1 c2 p y1 y2 h1 h2 h3 h4 h5 =>
    (reduct_sc hr I c1 c2).mpr
      (h.svf_sc c1 c2 p y1 y2
        ((reduct_iext_res hr I Reserved.someValuesFrom c1 y1).mp h1)
        ((reduct_iext_res hr I Reserved.onProperty c1 p).mp h2)
        ((reduct_iext_res hr I Reserved.someValuesFrom c2 y2).mp h3)
        ((reduct_iext_res hr I Reserved.onProperty c2 p).mp h4)
        ((reduct_sc hr I y1 y2).mp h5))
  svf_sp := fun c1 c2 p1 p2 y h1 h2 h3 h4 h5 =>
    (reduct_sc hr I c1 c2).mpr
      (h.svf_sp c1 c2 p1 p2 y
        ((reduct_iext_res hr I Reserved.someValuesFrom c1 y).mp h1)
        ((reduct_iext_res hr I Reserved.onProperty c1 p1).mp h2)
        ((reduct_iext_res hr I Reserved.someValuesFrom c2 y).mp h3)
        ((reduct_iext_res hr I Reserved.onProperty c2 p2).mp h4)
        ((reduct_sp hr I p1 p2).mp h5))
  avf_sc := fun c1 c2 p y1 y2 h1 h2 h3 h4 h5 =>
    (reduct_sc hr I c1 c2).mpr
      (h.avf_sc c1 c2 p y1 y2
        ((reduct_iext_res hr I Reserved.allValuesFrom c1 y1).mp h1)
        ((reduct_iext_res hr I Reserved.onProperty c1 p).mp h2)
        ((reduct_iext_res hr I Reserved.allValuesFrom c2 y2).mp h3)
        ((reduct_iext_res hr I Reserved.onProperty c2 p).mp h4)
        ((reduct_sc hr I y1 y2).mp h5))
  avf_sp := fun c1 c2 p1 p2 y h1 h2 h3 h4 h5 =>
    (reduct_sc hr I c2 c1).mpr
      (h.avf_sp c1 c2 p1 p2 y
        ((reduct_iext_res hr I Reserved.allValuesFrom c1 y).mp h1)
        ((reduct_iext_res hr I Reserved.onProperty c1 p1).mp h2)
        ((reduct_iext_res hr I Reserved.allValuesFrom c2 y).mp h3)
        ((reduct_iext_res hr I Reserved.onProperty c2 p2).mp h4)
        ((reduct_sp hr I p1 p2).mp h5))
  dom_sc := fun p c1 c2 hp hsc =>
    (reduct_iext_res hr I Reserved.domain p c2).mpr
      (h.dom_sc p c1 c2 ((reduct_iext_res hr I Reserved.domain p c1).mp hp)
        ((reduct_sc hr I c1 c2).mp hsc))
  dom_sp := fun p1 p2 c hp hsp =>
    (reduct_iext_res hr I Reserved.domain p1 c).mpr
      (h.dom_sp p1 p2 c ((reduct_iext_res hr I Reserved.domain p2 c).mp hp)
        ((reduct_sp hr I p1 p2).mp hsp))
  rng_sc := fun p c1 c2 hp hsc =>
    (reduct_iext_res hr I Reserved.range p c2).mpr
      (h.rng_sc p c1 c2 ((reduct_iext_res hr I Reserved.range p c1).mp hp)
        ((reduct_sc hr I c1 c2).mp hsc))
  rng_sp := fun p1 p2 c hp hsp =>
    (reduct_iext_res hr I Reserved.range p1 c).mpr
      (h.rng_sp p1 p2 c ((reduct_iext_res hr I Reserved.range p2 c).mp hp)
        ((reduct_sp hr I p1 p2).mp hsp))

end Transport

/-! ## The two institutions -/

/-- A signature is the vocabulary a sentence may use in PREDICATE position. -/
abbrev Sig : Type := List Term

/-- A sentence over `S`: a triple whose predicate is in `S`. Subject and object
are free. See the header for why this is the weakest useful restriction. -/
def Sen (S : Sig) : Type := { t : Triple // t.p ∈ S }

/-- A signature morphism of `Rdf.simple`: a renaming that takes `S` into `S'`.
That condition is exactly what `senMap` needs and there is nothing else. -/
structure Mor (S S' : Sig) where
  /-- The renaming. Total on terms: an RDF interpretation denotes every term, so
  a partial renaming would have nothing to mean. -/
  map : Term → Term
  /-- Predicates land in the target vocabulary, which is what makes the
  translation of a sentence a sentence. -/
  into : ∀ t ∈ S, map t ∈ S'

/-- A signature morphism of `Rdf.rl`: a `Mor` that additionally fixes every
term the OWL 2 RL semantic conditions name. Without this field, `modRed` cannot
be defined: `conditions_reduct` is exactly what fails. -/
structure MorRl (S S' : Sig) extends Mor S S' where
  /-- The reserved vocabulary is logical vocabulary and a renaming may not touch
  it. -/
  fixes : ∀ t, Reserved t → toMor.map t = t

/-- **The institution of simple RDF interpretations.** Models are arbitrary
`OOCert.Interp`s: no semantic conditions at all, so no renaming is forbidden.
Its entailment relation is simple RDF entailment restricted to this repository's
shape of interpretation. -/
def simple : Institution where
  Sig := Sig
  SigMor := Mor
  Sen := Sen
  Mod := fun _ => Interp
  senMap := fun σ φ => ⟨renameT σ.map φ.1, σ.into φ.1.p φ.2⟩
  modRed := fun σ I => reduct σ.map I
  sat := fun I φ => I.sat φ.1
  satCond := fun _ _ _ => Iff.rfl

/-- **The institution of OWL 2 RL interpretations**, the ones `Semantics.lean`
calls `Conditions`. Everything `Soundness.lean` proves is about this model
class.

The `modRed` field is where `conditions_reduct` is spent: reducing a model along
a morphism has to produce a model, and it only does because `MorRl` fixes the
reserved vocabulary. -/
def rl : Institution where
  Sig := Sig
  SigMor := MorRl
  Sen := Sen
  Mod := fun _ => { I : Interp // Conditions I }
  senMap := fun σ φ => ⟨renameT σ.toMor.map φ.1, σ.toMor.into φ.1.p φ.2⟩
  modRed := fun σ M => ⟨reduct σ.toMor.map M.1, conditions_reduct σ.fixes M.1 M.2⟩
  sat := fun M φ => M.1.sat φ.1
  satCond := fun _ _ _ => Iff.rfl

/-! ## The bridge to the checker

`Soundness.lean` is stated over `OOCert.Entails`, which quantifies over
`OOCert.Model I G`. That is a `Conditions` interpretation which additionally
satisfies `G` and the four list-constructor conditions. So an `Rdf.rl`
entailment, which quantifies over MORE interpretations, is the stronger
statement and gives an `OOCert.Entails`. -/

/-- An entailment in the OWL 2 RL institution is an `OOCert.Entails`. The
converse is false: see the file header and
`InstitutionWitness.the_checker_entails_more_than_the_institution`. -/
theorem rl_entails_gives_entails {S : Sig} (Γ : List (Sen S)) (φ : Sen S)
    (h : rl.EntailsL Γ φ) : Entails (Γ.map Subtype.val) φ.1 := by
  intro I M
  exact h ⟨I, M.conds⟩ (fun ψ hψ => M.facts ψ.1 (List.mem_map_of_mem hψ))

/-! ## The comorphism, and the thing it does not do

Forgetting the OWL 2 RL conditions is a comorphism from `Rdf.simple` to
`Rdf.rl`. It is as simple as a comorphism gets: signatures and sentences are
untouched and the model map is the inclusion of the smaller model class into the
larger one, running backwards, which is the direction a comorphism's model map
runs.

It preserves entailment, by the general theorem and nothing else. It does NOT
reflect entailment, and that is proved below rather than suspected: `rdfs9` is
an entailment of the target that is not an entailment of the source. -/

/-- **The comorphism.** `Comorphism.mk` would not accept this without the last
field, which is the whole design. -/
def forgetConditions : Comorphism simple rl where
  sigMap := fun S => S
  senMap := fun φ => φ
  modMap := fun M => M.1
  satCond := fun _ _ => Iff.rfl

/-- Simple RDF entailment is OWL 2 RL entailment, by the general preservation
theorem applied and not by a separate argument. -/
theorem simple_entailment_is_rl_entailment {S : Sig} {Γ : List (Sen S)} {φ : Sen S}
    (h : simple.EntailsL Γ φ) : rl.EntailsL Γ φ := by
  have hmap : Γ.map forgetConditions.senMap = Γ := List.map_id Γ
  have h2 := forgetConditions.preserves_entailmentL h
  rw [hmap] at h2
  exact h2

end Rdf

end OOCert
