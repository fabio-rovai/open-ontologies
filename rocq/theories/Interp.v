(** * An RDF-based interpretation, and the semantic conditions the built-in
      rules are discharged against.

    [Semantics.v] gives the RULE-RELATIVE warrant, which assumes the rule table.
    This file is what is needed to get rid of that assumption for the built-in
    table: a notion of model in which the RDFS and OWL vocabulary MEANS
    something, so that the built-in rules can be derived rather than assumed.

    R-INT-1. THE SHAPE IS THE RDF-BASED ONE: a domain, a total denotation, and a
    ternary extension relation. [iext I p x y] reads "the pair <x, y> is in
    IEXT(p)". Classes are read off the extension relation in the standard way,
    [ICEXT(c) = { x : <x, c> in IEXT(I(rdf:type)) }], rather than carried as a
    second field, because carrying it as a field would let an interpretation
    make a class extension disagree with the type triples that name it.

    R-INT-2. [iden] IS TOTAL ON TERMS. Every field of the certificate denotes,
    including one that is not writable RDF. This is a real restriction on the
    model class and it runs in the UNSAFE direction: interpretations in which
    some literal fails to denote are outside the claim. It is stated here rather
    than buried because the alternative, a partial denotation, would make
    [itrue] partial, and a three-valued truth would change the meaning of every
    theorem in the development. OWL 2 RDF-Based Semantics takes IL total, which
    is the precedent, and the claim is scoped to interpretations that do.

    R-INT-3. IC AND IP ARE FIELDS, not derived. RDFS conditions are stated with
    membership in IC and IP as a side condition ("then x, y are in IC and
    ICEXT(x) is a subset of ICEXT(y)"), and the conditions that run backwards
    need those memberships as HYPOTHESES. Deriving IC from the graph would make
    the backward conditions harder to satisfy and the model class smaller, which
    is the wrong direction.

    R-INT-4. THE CONDITIONS ARE SEPARATE DEFINITIONS, NOT FIELDS OF ONE RECORD,
    AND EVERY RULE LEMMA TAKES EXACTLY THE ONES IT USES. This is the only part
    of the design that is about honesty rather than about mathematics. A rule
    lemma proved from a record can be proved from a field that says the rule,
    and nothing in the proof script shows the difference. With the conditions as
    explicit hypotheses the dependency is IN THE STATEMENT, so a reader counting
    which arms consume a backward condition is reading a machine-checked fact
    rather than a comment. The count is fourteen and [Builtin.v] is where it can
    be checked.

    R-INT-5. THE BACKWARD CONDITIONS ARE A MODELLING DECISION AND THEY ARE THE
    WEAKEST POINT OF THE ABSOLUTE CLAIM. RDFS alone states its conditions as
    implications: "if <x, y> is in IEXT(I(rdfs:subClassOf)) then ...". Fourteen
    of the twenty-seven built-in rows CONCLUDE a [subClassOf], [subPropertyOf],
    [domain] or [range] triple, and no implication in that direction licenses
    them. The OWL 2 RDF-Based Semantics states the corresponding conditions as
    biconditionals, which is what makes those rows derivable, and a
    biconditional is a STRONGER condition: it admits fewer interpretations, so
    the model class is smaller and entailment over it is a WEAKER claim than
    entailment over the RDFS conditions alone. Adding [C_..._bwd] to the
    hypothesis list of a lemma is therefore never free, and the lemmas that need
    it say so in their own statement.

    WHERE THE BICONDITIONAL READING COMES FROM, AND WHAT WAS NOT DONE. It is
    taken from this repository's own correction to decision 0002, which names
    the connective in the OWL 2 RDF-Based Semantics table that carries
    [rowspan="4"] in the specification's HTML and therefore states an [iff]
    rather than the [if-then] RDFS alone gives. It was NOT re-checked cell by
    cell against the Recommendation in this work. That check is one of the
    things [rocq/README.md] lists as not covered, and it is the largest single
    piece of prose the absolute theorem rests on. A reader who wants the
    absolute verdict to mean what it says should do that reading before
    trusting it. *)

From Stdlib Require Import String List Bool.
From OOCertRocq Require Import Syntax Semantics.
Import ListNotations.
Open Scope string_scope.

(** ** The vocabulary, as the bytes the rule table actually carries *)

Definition rdf_type      := "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".
Definition rdfs_domain   := "<http://www.w3.org/2000/01/rdf-schema#domain>".
Definition rdfs_range    := "<http://www.w3.org/2000/01/rdf-schema#range>".
Definition rdfs_subClassOf    := "<http://www.w3.org/2000/01/rdf-schema#subClassOf>".
Definition rdfs_subPropertyOf := "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>".
Definition owl_TransitiveProperty := "<http://www.w3.org/2002/07/owl#TransitiveProperty>".
Definition owl_SymmetricProperty  := "<http://www.w3.org/2002/07/owl#SymmetricProperty>".
Definition owl_inverseOf          := "<http://www.w3.org/2002/07/owl#inverseOf>".
Definition owl_sameAs             := "<http://www.w3.org/2002/07/owl#sameAs>".
Definition owl_equivalentClass    := "<http://www.w3.org/2002/07/owl#equivalentClass>".
Definition owl_equivalentProperty := "<http://www.w3.org/2002/07/owl#equivalentProperty>".
Definition owl_onProperty         := "<http://www.w3.org/2002/07/owl#onProperty>".
Definition owl_someValuesFrom     := "<http://www.w3.org/2002/07/owl#someValuesFrom>".
Definition owl_allValuesFrom      := "<http://www.w3.org/2002/07/owl#allValuesFrom>".
Definition owl_hasValue           := "<http://www.w3.org/2002/07/owl#hasValue>".

(** ** Interpretations *)

Record interp : Type := Interp {
  idom : Type ;
  iden : term -> idom ;                     (* IS union IL, total: R-INT-2 *)
  iext : idom -> idom -> idom -> Prop ;     (* iext p x y  <->  <x,y> in IEXT(p) *)
  ic   : idom -> Prop ;                     (* IC *)
  ip   : idom -> Prop                       (* IP *)
}.

Arguments iext {i} _ _ _.
Arguments ic {i} _.
Arguments ip {i} _.

(** Truth of a ground triple. *)
Definition itrue (I : interp) (t : triple) : Prop :=
  iext (iden I (tpred t)) (iden I (tsubj t)) (iden I (tobj t)).

(** ICEXT, read off IEXT(I(rdf:type)) rather than carried. *)
Definition icext (I : interp) (c x : idom I) : Prop :=
  iext (iden I rdf_type) x c.

(** A named property's extension, at the level of the domain. *)
Definition rel (I : interp) (v : term) (x y : idom I) : Prop :=
  iext (iden I v) x y.

(** ** The semantic conditions

    Each is the specification clause it is named for, stated over the domain and
    never over the graph. The forward halves are RDF 1.1 Semantics section 9.2.1
    (the RDFS semantic conditions); the backward halves are the biconditional
    reading the OWL 2 RDF-Based Semantics gives the same vocabulary, and R-INT-5
    is the warning about what they cost. *)

(* "If <x,y> is in IEXT(I(rdfs:domain)) and <u,v> is in IEXT(x) then u is in ICEXT(y)."
   The IP and IC memberships come with it, as they do for every RDFS condition
   of this shape. *)
Definition C_dom_fwd (I : interp) : Prop :=
  forall x y, rel I rdfs_domain x y ->
    ip x /\ ic y /\ (forall u v, iext x u v -> icext I y u).

Definition C_dom_bwd (I : interp) : Prop :=
  forall x y, ip x -> ic y ->
    (forall u v, iext x u v -> icext I y u) -> rel I rdfs_domain x y.

Definition C_rng_fwd (I : interp) : Prop :=
  forall x y, rel I rdfs_range x y ->
    ip x /\ ic y /\ (forall u v, iext x u v -> icext I y v).

Definition C_rng_bwd (I : interp) : Prop :=
  forall x y, ip x -> ic y ->
    (forall u v, iext x u v -> icext I y v) -> rel I rdfs_range x y.

(* "If <x,y> is in IEXT(I(rdfs:subPropertyOf)) then x and y are in IP and
   IEXT(x) is a subset of IEXT(y)." *)
Definition C_sp_fwd (I : interp) : Prop :=
  forall x y, rel I rdfs_subPropertyOf x y ->
    ip x /\ ip y /\ (forall u v, iext x u v -> iext y u v).

Definition C_sp_bwd (I : interp) : Prop :=
  forall x y, ip x -> ip y ->
    (forall u v, iext x u v -> iext y u v) -> rel I rdfs_subPropertyOf x y.

(* "If <x,y> is in IEXT(I(rdfs:subClassOf)) then x and y are in IC and
   ICEXT(x) is a subset of ICEXT(y)." *)
Definition C_sc_fwd (I : interp) : Prop :=
  forall x y, rel I rdfs_subClassOf x y ->
    ic x /\ ic y /\ (forall u, icext I x u -> icext I y u).

Definition C_sc_bwd (I : interp) : Prop :=
  forall x y, ic x -> ic y ->
    (forall u, icext I x u -> icext I y u) -> rel I rdfs_subClassOf x y.

(* A transitive property's extension is transitive. *)
Definition C_trp (I : interp) : Prop :=
  forall p, icext I (iden I owl_TransitiveProperty) p ->
    forall x y z, iext p x y -> iext p y z -> iext p x z.

Definition C_symp (I : interp) : Prop :=
  forall p, icext I (iden I owl_SymmetricProperty) p ->
    forall x y, iext p x y -> iext p y x.

(* owl:inverseOf relates two properties whose extensions are converses. *)
Definition C_inv (I : interp) : Prop :=
  forall p q, rel I owl_inverseOf p q ->
    forall x y, iext p x y <-> iext q y x.

(* IEXT(I(owl:sameAs)) is the identity on the domain. *)
Definition C_sameAs (I : interp) : Prop :=
  forall x y, rel I owl_sameAs x y <-> x = y.

Definition C_eqc (I : interp) : Prop :=
  forall x y, rel I owl_equivalentClass x y <->
    (ic x /\ ic y /\ (forall u, icext I x u <-> icext I y u)).

Definition C_eqp (I : interp) : Prop :=
  forall x y, rel I owl_equivalentProperty x y <->
    (ip x /\ ip y /\ (forall u v, iext x u v <-> iext y u v)).

(* The three restriction constructors this table uses. Each says the restriction
   IS a class, and says exactly which one. *)
Definition C_svf (I : interp) : Prop :=
  forall r p c, rel I owl_onProperty r p -> rel I owl_someValuesFrom r c ->
    ic r /\ (forall x, icext I r x <-> (exists y, iext p x y /\ icext I c y)).

Definition C_avf (I : interp) : Prop :=
  forall r p c, rel I owl_onProperty r p -> rel I owl_allValuesFrom r c ->
    ic r /\ (forall x, icext I r x <-> (forall y, iext p x y -> icext I c y)).

Definition C_hv (I : interp) : Prop :=
  forall r p v, rel I owl_onProperty r p -> rel I owl_hasValue r v ->
    ic r /\ (forall x, icext I r x <-> iext p x v).

(** ** The bundle

    A record, so that the top-level theorem has one hypothesis. Every field is
    one of the conditions above, and no field is a rule. *)

Record RL (I : interp) : Prop := MkRL {
  rl_dom_fwd : C_dom_fwd I ;
  rl_dom_bwd : C_dom_bwd I ;
  rl_rng_fwd : C_rng_fwd I ;
  rl_rng_bwd : C_rng_bwd I ;
  rl_sp_fwd  : C_sp_fwd I ;
  rl_sp_bwd  : C_sp_bwd I ;
  rl_sc_fwd  : C_sc_fwd I ;
  rl_sc_bwd  : C_sc_bwd I ;
  rl_trp     : C_trp I ;
  rl_symp    : C_symp I ;
  rl_inv     : C_inv I ;
  rl_sameAs  : C_sameAs I ;
  rl_eqc     : C_eqc I ;
  rl_eqp     : C_eqp I ;
  rl_svf     : C_svf I ;
  rl_avf     : C_avf I ;
  rl_hv      : C_hv I
}.

(** ** THE ABSOLUTE ENTAILMENT RELATION

    True in every model of the asserted graph. No rule table appears in it,
    which is the whole difference from [entails_rel]. *)

Definition entails_abs (G : list triple) (t : triple) : Prop :=
  forall I : interp, RL I -> (forall u, In u G -> itrue I u) -> itrue I t.
