(** * The built-in rule table, and the twenty-seven arms that discharge it.

    [Sound.v] gives the relative warrant, which assumes the table. This file
    removes the assumption for ONE table, the built-in one, by deriving each of
    its rows from the semantic conditions in [Interp.v]. The result is
    [entails_of_builtin_horn]: a certificate over exactly this table has
    conclusions true in every model of the asserted graph, with no rule
    assumed.

    R-BLT-1. THE TABLE IS TRANSCRIBED FROM [tests/fixtures/horn/builtin_rules.tsv]
    AND FROM NOWHERE ELSE. [Fixtures.v] parses those committed bytes with this
    development's own parser and checks the result equals the table below, so
    the transcription is a gate rather than a claim. Twenty-seven rows for
    twenty-five W3C rules, because [scm-eqc1] and [scm-eqp1] each license two
    conclusions and each therefore occupies two rows.

    R-BLT-2. EVERY ARM TAKES EXACTLY THE CONDITIONS IT USES, as separate
    hypotheses. See R-INT-4. Reading the statements gives the dependency without
    reading a proof, and in particular gives the count of arms that consume a
    BACKWARD condition, which is the number that says how much of the absolute
    claim rests on the biconditional reading R-INT-5 warns about. It is
    fourteen: [rdfs5], [rdfs11], both [scm-eqc1] rows, both [scm-eqp1] rows,
    [scm-svf1], [scm-svf2], [scm-avf1], [scm-avf2], [scm-dom1], [scm-dom2],
    [scm-rng1] and [scm-rng2]. Every one of them concludes a [subClassOf],
    [subPropertyOf], [domain] or [range] triple, and nothing that only runs
    forwards can conclude one.

    R-BLT-3. A QUALIFICATION THE COUNT NEEDS, OR IT IS SELECTIVE. Six other
    conditions are ALSO stated as biconditionals in [Interp.v]: [C_eqc], [C_eqp],
    [C_svf], [C_avf], [C_hv] and [C_sameAs]. Several arms consume the reverse
    direction of those, [cls-svf1] and [cls-hv2] among them. They are not counted
    in the fourteen, and the reason is a real distinction rather than a
    convenience.

    The RDFS conditions for [subClassOf], [subPropertyOf], [domain] and [range]
    are stated by RDFS as implications, and reading them as biconditionals is a
    CHANGE to what RDFS says. The conditions for [owl:someValuesFrom],
    [owl:allValuesFrom], [owl:hasValue], [owl:equivalentClass],
    [owl:equivalentProperty] and [owl:sameAs] are biconditionals because that is
    what those constructors MEAN: a someValuesFrom restriction is the class of
    things with such a value, not merely a subset of it, and [owl:sameAs] is
    identity rather than something implied by identity. Reading them as one-way
    implications would not be a weaker reading of the same condition, it would be
    a different condition describing a different vocabulary.

    So the fourteen is the count of arms that rest on a reading STRONGER than the
    specification they come from. It is not the count of arms that use an [iff].
    A reader who wants the second number can get it from the statements, which is
    the point of writing them this way.
*)

From Stdlib Require Import String List Bool.
From OOCertRocq Require Import Syntax Semantics Checker Sound Interp.
Import ListNotations.
Open Scope string_scope.

Notation V := PVar.
Notation T := PTerm.

(** ** The table *)

Definition r_rdfs2 := Rule "rdfs2"
  [TP (V "s") (V "p") (V "o"); TP (V "p") (T rdfs_domain) (V "c")]
  (TP (V "s") (T rdf_type) (V "c")).

Definition r_rdfs3 := Rule "rdfs3"
  [TP (V "s") (V "p") (V "o"); TP (V "p") (T rdfs_range) (V "c")]
  (TP (V "o") (T rdf_type) (V "c")).

Definition r_rdfs5 := Rule "rdfs5"
  [TP (V "a") (T rdfs_subPropertyOf) (V "b");
   TP (V "b") (T rdfs_subPropertyOf) (V "c")]
  (TP (V "a") (T rdfs_subPropertyOf) (V "c")).

Definition r_rdfs7 := Rule "rdfs7"
  [TP (V "s") (V "p") (V "o"); TP (V "p") (T rdfs_subPropertyOf) (V "q")]
  (TP (V "s") (V "q") (V "o")).

Definition r_rdfs9 := Rule "rdfs9"
  [TP (V "x") (T rdf_type) (V "a"); TP (V "a") (T rdfs_subClassOf) (V "b")]
  (TP (V "x") (T rdf_type) (V "b")).

Definition r_rdfs11 := Rule "rdfs11"
  [TP (V "a") (T rdfs_subClassOf) (V "b"); TP (V "b") (T rdfs_subClassOf) (V "c")]
  (TP (V "a") (T rdfs_subClassOf) (V "c")).

Definition r_prp_trp := Rule "prp-trp"
  [TP (V "p") (T rdf_type) (T owl_TransitiveProperty);
   TP (V "x") (V "p") (V "y");
   TP (V "y") (V "p") (V "z")]
  (TP (V "x") (V "p") (V "z")).

Definition r_prp_symp := Rule "prp-symp"
  [TP (V "p") (T rdf_type) (T owl_SymmetricProperty); TP (V "x") (V "p") (V "y")]
  (TP (V "y") (V "p") (V "x")).

Definition r_prp_inv1 := Rule "prp-inv1"
  [TP (V "p") (T owl_inverseOf) (V "q"); TP (V "x") (V "p") (V "y")]
  (TP (V "y") (V "q") (V "x")).

Definition r_prp_inv2 := Rule "prp-inv2"
  [TP (V "p") (T owl_inverseOf) (V "q"); TP (V "x") (V "q") (V "y")]
  (TP (V "y") (V "p") (V "x")).

Definition r_eq_sym := Rule "eq-sym"
  [TP (V "a") (T owl_sameAs) (V "b")]
  (TP (V "b") (T owl_sameAs) (V "a")).

Definition r_scm_eqc1a := Rule "scm-eqc1"
  [TP (V "a") (T owl_equivalentClass) (V "b")]
  (TP (V "a") (T rdfs_subClassOf) (V "b")).

Definition r_scm_eqc1b := Rule "scm-eqc1"
  [TP (V "a") (T owl_equivalentClass) (V "b")]
  (TP (V "b") (T rdfs_subClassOf) (V "a")).

Definition r_scm_eqp1a := Rule "scm-eqp1"
  [TP (V "a") (T owl_equivalentProperty) (V "b")]
  (TP (V "a") (T rdfs_subPropertyOf) (V "b")).

Definition r_scm_eqp1b := Rule "scm-eqp1"
  [TP (V "a") (T owl_equivalentProperty) (V "b")]
  (TP (V "b") (T rdfs_subPropertyOf) (V "a")).

Definition r_cls_svf1 := Rule "cls-svf1"
  [TP (V "r") (T owl_onProperty) (V "p");
   TP (V "r") (T owl_someValuesFrom) (V "c");
   TP (V "x") (V "p") (V "y");
   TP (V "y") (T rdf_type) (V "c")]
  (TP (V "x") (T rdf_type) (V "r")).

Definition r_cls_avf := Rule "cls-avf"
  [TP (V "r") (T owl_onProperty) (V "p");
   TP (V "r") (T owl_allValuesFrom) (V "c");
   TP (V "x") (T rdf_type) (V "r");
   TP (V "x") (V "p") (V "y")]
  (TP (V "y") (T rdf_type) (V "c")).

Definition r_cls_hv1 := Rule "cls-hv1"
  [TP (V "r") (T owl_onProperty) (V "p");
   TP (V "r") (T owl_hasValue) (V "v");
   TP (V "x") (T rdf_type) (V "r")]
  (TP (V "x") (V "p") (V "v")).

Definition r_cls_hv2 := Rule "cls-hv2"
  [TP (V "r") (T owl_onProperty) (V "p");
   TP (V "r") (T owl_hasValue) (V "v");
   TP (V "x") (V "p") (V "v")]
  (TP (V "x") (T rdf_type) (V "r")).

Definition r_scm_svf1 := Rule "scm-svf1"
  [TP (V "c1") (T owl_someValuesFrom) (V "y1");
   TP (V "c1") (T owl_onProperty) (V "p");
   TP (V "c2") (T owl_someValuesFrom) (V "y2");
   TP (V "c2") (T owl_onProperty) (V "p");
   TP (V "y1") (T rdfs_subClassOf) (V "y2")]
  (TP (V "c1") (T rdfs_subClassOf) (V "c2")).

Definition r_scm_svf2 := Rule "scm-svf2"
  [TP (V "c1") (T owl_someValuesFrom) (V "y");
   TP (V "c1") (T owl_onProperty) (V "p1");
   TP (V "c2") (T owl_someValuesFrom) (V "y");
   TP (V "c2") (T owl_onProperty) (V "p2");
   TP (V "p1") (T rdfs_subPropertyOf) (V "p2")]
  (TP (V "c1") (T rdfs_subClassOf) (V "c2")).

Definition r_scm_avf1 := Rule "scm-avf1"
  [TP (V "c1") (T owl_allValuesFrom) (V "y1");
   TP (V "c1") (T owl_onProperty) (V "p");
   TP (V "c2") (T owl_allValuesFrom) (V "y2");
   TP (V "c2") (T owl_onProperty) (V "p");
   TP (V "y1") (T rdfs_subClassOf) (V "y2")]
  (TP (V "c1") (T rdfs_subClassOf) (V "c2")).

(** The reversed head is not a typo. [scm-avf2] concludes [?c2 subClassOf ?c1]
    where [scm-svf2] concludes [?c1 subClassOf ?c2], because a universal
    restriction is ANTITONE in its property while an existential one is
    monotone. The fixture says so and the proof below needs it that way. *)
Definition r_scm_avf2 := Rule "scm-avf2"
  [TP (V "c1") (T owl_allValuesFrom) (V "y");
   TP (V "c1") (T owl_onProperty) (V "p1");
   TP (V "c2") (T owl_allValuesFrom) (V "y");
   TP (V "c2") (T owl_onProperty) (V "p2");
   TP (V "p1") (T rdfs_subPropertyOf) (V "p2")]
  (TP (V "c2") (T rdfs_subClassOf) (V "c1")).

Definition r_scm_dom1 := Rule "scm-dom1"
  [TP (V "p") (T rdfs_domain) (V "c1"); TP (V "c1") (T rdfs_subClassOf) (V "c2")]
  (TP (V "p") (T rdfs_domain) (V "c2")).

Definition r_scm_dom2 := Rule "scm-dom2"
  [TP (V "p2") (T rdfs_domain) (V "c"); TP (V "p1") (T rdfs_subPropertyOf) (V "p2")]
  (TP (V "p1") (T rdfs_domain) (V "c")).

Definition r_scm_rng1 := Rule "scm-rng1"
  [TP (V "p") (T rdfs_range) (V "c1"); TP (V "c1") (T rdfs_subClassOf) (V "c2")]
  (TP (V "p") (T rdfs_range) (V "c2")).

Definition r_scm_rng2 := Rule "scm-rng2"
  [TP (V "p2") (T rdfs_range) (V "c"); TP (V "p1") (T rdfs_subPropertyOf) (V "p2")]
  (TP (V "p1") (T rdfs_range) (V "c")).

Definition builtin : list rule :=
  [r_rdfs2; r_rdfs3; r_rdfs5; r_rdfs7; r_rdfs9; r_rdfs11;
   r_prp_trp; r_prp_symp; r_prp_inv1; r_prp_inv2; r_eq_sym;
   r_scm_eqc1a; r_scm_eqc1b; r_scm_eqp1a; r_scm_eqp1b;
   r_cls_svf1; r_cls_avf; r_cls_hv1; r_cls_hv2;
   r_scm_svf1; r_scm_svf2; r_scm_avf1; r_scm_avf2;
   r_scm_dom1; r_scm_dom2; r_scm_rng1; r_scm_rng2].

(** ** Proof plumbing *)

Lemma sat_rule_intro : forall (I : world) nm body hd,
  (forall s, (forall a, In a body -> I (inst s a)) -> I (inst s hd)) ->
  sat_rule I (Rule nm body hd).
Proof. intros I nm body hd H s Hb. exact (H s Hb). Qed.

Ltac unf := cbv [itrue rel icext inst inst_pat psubj ppred pobj tsubj tpred tobj] in *.

Ltac arm := apply sat_rule_intro; intros s Hb; unf.

Ltac prem1 B H := pose proof (B _ (or_introl eq_refl)) as H; unf.
Ltac prem2 B H := pose proof (B _ (or_intror (or_introl eq_refl))) as H; unf.
Ltac prem3 B H := pose proof (B _ (or_intror (or_intror (or_introl eq_refl)))) as H; unf.
Ltac prem4 B H := pose proof (B _ (or_intror (or_intror (or_intror (or_introl eq_refl))))) as H; unf.
Ltac prem5 B H :=
  pose proof (B _ (or_intror (or_intror (or_intror (or_intror (or_introl eq_refl)))))) as H; unf.

(** ** The arms

    Forward only. Thirteen rows. *)

Lemma arm_rdfs2 : forall I, C_dom_fwd I -> sat_rule (itrue I) r_rdfs2.
Proof.
  intros I Hd. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hd _ _ H2) as [_ [_ Hsub]]. exact (Hsub _ _ H1).
Qed.

Lemma arm_rdfs3 : forall I, C_rng_fwd I -> sat_rule (itrue I) r_rdfs3.
Proof.
  intros I Hr. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hr _ _ H2) as [_ [_ Hsub]]. exact (Hsub _ _ H1).
Qed.

Lemma arm_rdfs7 : forall I, C_sp_fwd I -> sat_rule (itrue I) r_rdfs7.
Proof.
  intros I Hp. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hp _ _ H2) as [_ [_ Hsub]]. exact (Hsub _ _ H1).
Qed.

Lemma arm_rdfs9 : forall I, C_sc_fwd I -> sat_rule (itrue I) r_rdfs9.
Proof.
  intros I Hc. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hc _ _ H2) as [_ [_ Hsub]]. exact (Hsub _ H1).
Qed.

Lemma arm_prp_trp : forall I, C_trp I -> sat_rule (itrue I) r_prp_trp.
Proof.
  intros I Ht. arm. prem1 Hb H1. prem2 Hb H2. prem3 Hb H3.
  exact (Ht _ H1 _ _ _ H2 H3).
Qed.

Lemma arm_prp_symp : forall I, C_symp I -> sat_rule (itrue I) r_prp_symp.
Proof.
  intros I Hs. arm. prem1 Hb H1. prem2 Hb H2. exact (Hs _ H1 _ _ H2).
Qed.

Lemma arm_prp_inv1 : forall I, C_inv I -> sat_rule (itrue I) r_prp_inv1.
Proof.
  intros I Hi. arm. prem1 Hb H1. prem2 Hb H2.
  exact (proj1 (Hi _ _ H1 _ _) H2).
Qed.

Lemma arm_prp_inv2 : forall I, C_inv I -> sat_rule (itrue I) r_prp_inv2.
Proof.
  intros I Hi. arm. prem1 Hb H1. prem2 Hb H2.
  exact (proj2 (Hi _ _ H1 _ _) H2).
Qed.

Lemma arm_eq_sym : forall I, C_sameAs I -> sat_rule (itrue I) r_eq_sym.
Proof.
  intros I Hs. arm. prem1 Hb H1.
  apply (proj1 (Hs _ _)) in H1.
  apply (proj2 (Hs _ _)). now symmetry.
Qed.

Lemma arm_cls_svf1 : forall I, C_svf I -> sat_rule (itrue I) r_cls_svf1.
Proof.
  intros I Hv. arm. prem1 Hb H1. prem2 Hb H2. prem3 Hb H3. prem4 Hb H4.
  destruct (Hv _ _ _ H1 H2) as [_ Hiff].
  apply (proj2 (Hiff _)). exists (iden I (s "y")). split; assumption.
Qed.

Lemma arm_cls_avf : forall I, C_avf I -> sat_rule (itrue I) r_cls_avf.
Proof.
  intros I Hv. arm. prem1 Hb H1. prem2 Hb H2. prem3 Hb H3. prem4 Hb H4.
  destruct (Hv _ _ _ H1 H2) as [_ Hiff].
  exact (proj1 (Hiff _) H3 _ H4).
Qed.

Lemma arm_cls_hv1 : forall I, C_hv I -> sat_rule (itrue I) r_cls_hv1.
Proof.
  intros I Hv. arm. prem1 Hb H1. prem2 Hb H2. prem3 Hb H3.
  destruct (Hv _ _ _ H1 H2) as [_ Hiff].
  exact (proj1 (Hiff _) H3).
Qed.

Lemma arm_cls_hv2 : forall I, C_hv I -> sat_rule (itrue I) r_cls_hv2.
Proof.
  intros I Hv. arm. prem1 Hb H1. prem2 Hb H2. prem3 Hb H3.
  destruct (Hv _ _ _ H1 H2) as [_ Hiff].
  exact (proj2 (Hiff _) H3).
Qed.

(** *** The fourteen that need a backward condition

    Each one concludes a [subClassOf], [subPropertyOf], [domain] or [range]
    triple. Nothing that only runs forwards can conclude one, so each of these
    statements carries a [_bwd] hypothesis and that is the machine-checked form
    of R-INT-5's warning. *)

Lemma arm_rdfs5 : forall I, C_sp_fwd I -> C_sp_bwd I -> sat_rule (itrue I) r_rdfs5.
Proof.
  intros I Hf Hb'. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hf _ _ H1) as [Hia [_ Hab]].
  destruct (Hf _ _ H2) as [_ [Hic Hbc]].
  apply Hb'; [exact Hia | exact Hic | intros u v Huv; exact (Hbc _ _ (Hab _ _ Huv))].
Qed.

Lemma arm_rdfs11 : forall I, C_sc_fwd I -> C_sc_bwd I -> sat_rule (itrue I) r_rdfs11.
Proof.
  intros I Hf Hb'. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hf _ _ H1) as [Hia [_ Hab]].
  destruct (Hf _ _ H2) as [_ [Hic Hbc]].
  apply Hb'; [exact Hia | exact Hic | intros u Hu; exact (Hbc _ (Hab _ Hu))].
Qed.

Lemma arm_scm_eqc1a : forall I, C_eqc I -> C_sc_bwd I -> sat_rule (itrue I) r_scm_eqc1a.
Proof.
  intros I He Hb'. arm. prem1 Hb H1.
  destruct (proj1 (He _ _) H1) as [Hia [Hib Hiff]].
  apply Hb'; [exact Hia | exact Hib | intros u Hu; exact (proj1 (Hiff u) Hu)].
Qed.

Lemma arm_scm_eqc1b : forall I, C_eqc I -> C_sc_bwd I -> sat_rule (itrue I) r_scm_eqc1b.
Proof.
  intros I He Hb'. arm. prem1 Hb H1.
  destruct (proj1 (He _ _) H1) as [Hia [Hib Hiff]].
  apply Hb'; [exact Hib | exact Hia | intros u Hu; exact (proj2 (Hiff u) Hu)].
Qed.

Lemma arm_scm_eqp1a : forall I, C_eqp I -> C_sp_bwd I -> sat_rule (itrue I) r_scm_eqp1a.
Proof.
  intros I He Hb'. arm. prem1 Hb H1.
  destruct (proj1 (He _ _) H1) as [Hia [Hib Hiff]].
  apply Hb'; [exact Hia | exact Hib | intros u v Huv; exact (proj1 (Hiff u v) Huv)].
Qed.

Lemma arm_scm_eqp1b : forall I, C_eqp I -> C_sp_bwd I -> sat_rule (itrue I) r_scm_eqp1b.
Proof.
  intros I He Hb'. arm. prem1 Hb H1.
  destruct (proj1 (He _ _) H1) as [Hia [Hib Hiff]].
  apply Hb'; [exact Hib | exact Hia | intros u v Huv; exact (proj2 (Hiff u v) Huv)].
Qed.

Lemma arm_scm_svf1 : forall I,
  C_svf I -> C_sc_fwd I -> C_sc_bwd I -> sat_rule (itrue I) r_scm_svf1.
Proof.
  intros I Hv Hf Hb'. arm.
  prem1 Hb H1. prem2 Hb H2. prem3 Hb H3. prem4 Hb H4. prem5 Hb H5.
  destruct (Hv _ _ _ H2 H1) as [Hic1 Hiff1].
  destruct (Hv _ _ _ H4 H3) as [Hic2 Hiff2].
  destruct (Hf _ _ H5) as [_ [_ Hsub]].
  apply Hb'; [exact Hic1 | exact Hic2 |].
  intros u Hu. apply (proj2 (Hiff2 u)).
  destruct (proj1 (Hiff1 u) Hu) as [w [Hw1 Hw2]].
  exists w. split; [exact Hw1 | exact (Hsub _ Hw2)].
Qed.

Lemma arm_scm_svf2 : forall I,
  C_svf I -> C_sp_fwd I -> C_sc_bwd I -> sat_rule (itrue I) r_scm_svf2.
Proof.
  intros I Hv Hf Hb'. arm.
  prem1 Hb H1. prem2 Hb H2. prem3 Hb H3. prem4 Hb H4. prem5 Hb H5.
  destruct (Hv _ _ _ H2 H1) as [Hic1 Hiff1].
  destruct (Hv _ _ _ H4 H3) as [Hic2 Hiff2].
  destruct (Hf _ _ H5) as [_ [_ Hsub]].
  apply Hb'; [exact Hic1 | exact Hic2 |].
  intros u Hu. apply (proj2 (Hiff2 u)).
  destruct (proj1 (Hiff1 u) Hu) as [w [Hw1 Hw2]].
  exists w. split; [exact (Hsub _ _ Hw1) | exact Hw2].
Qed.

Lemma arm_scm_avf1 : forall I,
  C_avf I -> C_sc_fwd I -> C_sc_bwd I -> sat_rule (itrue I) r_scm_avf1.
Proof.
  intros I Hv Hf Hb'. arm.
  prem1 Hb H1. prem2 Hb H2. prem3 Hb H3. prem4 Hb H4. prem5 Hb H5.
  destruct (Hv _ _ _ H2 H1) as [Hic1 Hiff1].
  destruct (Hv _ _ _ H4 H3) as [Hic2 Hiff2].
  destruct (Hf _ _ H5) as [_ [_ Hsub]].
  apply Hb'; [exact Hic1 | exact Hic2 |].
  intros u Hu. apply (proj2 (Hiff2 u)).
  intros w Hw. exact (Hsub _ (proj1 (Hiff1 u) Hu w Hw)).
Qed.

Lemma arm_scm_avf2 : forall I,
  C_avf I -> C_sp_fwd I -> C_sc_bwd I -> sat_rule (itrue I) r_scm_avf2.
Proof.
  intros I Hv Hf Hb'. arm.
  prem1 Hb H1. prem2 Hb H2. prem3 Hb H3. prem4 Hb H4. prem5 Hb H5.
  destruct (Hv _ _ _ H2 H1) as [Hic1 Hiff1].
  destruct (Hv _ _ _ H4 H3) as [Hic2 Hiff2].
  destruct (Hf _ _ H5) as [_ [_ Hsub]].
  apply Hb'; [exact Hic2 | exact Hic1 |].
  intros u Hu. apply (proj2 (Hiff1 u)).
  intros w Hw. exact (proj1 (Hiff2 u) Hu w (Hsub _ _ Hw)).
Qed.

Lemma arm_scm_dom1 : forall I,
  C_dom_fwd I -> C_dom_bwd I -> C_sc_fwd I -> sat_rule (itrue I) r_scm_dom1.
Proof.
  intros I Hf Hb' Hc. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hf _ _ H1) as [Hipp [_ Hsub1]].
  destruct (Hc _ _ H2) as [_ [Hicc2 Hsub2]].
  apply Hb'; [exact Hipp | exact Hicc2 |].
  intros u v Huv. exact (Hsub2 _ (Hsub1 _ _ Huv)).
Qed.

Lemma arm_scm_dom2 : forall I,
  C_dom_fwd I -> C_dom_bwd I -> C_sp_fwd I -> sat_rule (itrue I) r_scm_dom2.
Proof.
  intros I Hf Hb' Hp. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hf _ _ H1) as [_ [Hicc Hsub1]].
  destruct (Hp _ _ H2) as [Hipp1 [_ Hsub2]].
  apply Hb'; [exact Hipp1 | exact Hicc |].
  intros u v Huv. exact (Hsub1 _ _ (Hsub2 _ _ Huv)).
Qed.

Lemma arm_scm_rng1 : forall I,
  C_rng_fwd I -> C_rng_bwd I -> C_sc_fwd I -> sat_rule (itrue I) r_scm_rng1.
Proof.
  intros I Hf Hb' Hc. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hf _ _ H1) as [Hipp [_ Hsub1]].
  destruct (Hc _ _ H2) as [_ [Hicc2 Hsub2]].
  apply Hb'; [exact Hipp | exact Hicc2 |].
  intros u v Huv. exact (Hsub2 _ (Hsub1 _ _ Huv)).
Qed.

Lemma arm_scm_rng2 : forall I,
  C_rng_fwd I -> C_rng_bwd I -> C_sp_fwd I -> sat_rule (itrue I) r_scm_rng2.
Proof.
  intros I Hf Hb' Hp. arm. prem1 Hb H1. prem2 Hb H2.
  destruct (Hf _ _ H1) as [_ [Hicc Hsub1]].
  destruct (Hp _ _ H2) as [Hipp1 [_ Hsub2]].
  apply Hb'; [exact Hipp1 | exact Hicc |].
  intros u v Huv. exact (Hsub1 _ _ (Hsub2 _ _ Huv)).
Qed.

(** ** The table, discharged *)

Theorem builtin_sound : forall I, RL I -> sat_rules (itrue I) builtin.
Proof.
  intros I HRL r Hr.
  destruct HRL as [Hdf Hdb Hrf Hrb Hpf Hpb Hcf Hcb Htrp Hsymp Hinv Hsame Heqc Heqp Hsvf Havf Hhv].
  cbv [builtin In] in Hr.
  destruct Hr as [<-|Hr]; [now apply arm_rdfs2|].
  destruct Hr as [<-|Hr]; [now apply arm_rdfs3|].
  destruct Hr as [<-|Hr]; [now apply arm_rdfs5|].
  destruct Hr as [<-|Hr]; [now apply arm_rdfs7|].
  destruct Hr as [<-|Hr]; [now apply arm_rdfs9|].
  destruct Hr as [<-|Hr]; [now apply arm_rdfs11|].
  destruct Hr as [<-|Hr]; [now apply arm_prp_trp|].
  destruct Hr as [<-|Hr]; [now apply arm_prp_symp|].
  destruct Hr as [<-|Hr]; [now apply arm_prp_inv1|].
  destruct Hr as [<-|Hr]; [now apply arm_prp_inv2|].
  destruct Hr as [<-|Hr]; [now apply arm_eq_sym|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_eqc1a|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_eqc1b|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_eqp1a|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_eqp1b|].
  destruct Hr as [<-|Hr]; [now apply arm_cls_svf1|].
  destruct Hr as [<-|Hr]; [now apply arm_cls_avf|].
  destruct Hr as [<-|Hr]; [now apply arm_cls_hv1|].
  destruct Hr as [<-|Hr]; [now apply arm_cls_hv2|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_svf1|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_svf2|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_avf1|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_avf2|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_dom1|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_dom2|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_rng1|].
  destruct Hr as [<-|Hr]; [now apply arm_scm_rng2|].
  contradiction.
Qed.

(** ** THE ABSOLUTE THEOREM

    No rule is assumed. A certificate over exactly this table has conclusions
    true in every model of the asserted graph, where "model" is an interpretation
    satisfying the conditions of [Interp.v] and nothing else.

    The word this earns is not the word a user table earns. Decision 0003 makes
    that separation normative and the driver prints two different verdicts
    naming two different theorems, because a checker that collapses them is a
    machine for turning an assumption into a fact with a proof attached. *)

Theorem entails_of_builtin_horn : forall G ss,
  check_cert G builtin ss = true ->
  forall t, In t (map sconcl ss) -> entails_abs G t.
Proof.
  intros G ss Hchk t Hin I HRL HG.
  exact (horn_certificate_sound G builtin ss Hchk t Hin (itrue I) HG (builtin_sound I HRL)).
Qed.
