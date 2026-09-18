(** * Soundness of the Horn certificate checker.

    THE THEOREM. If [check_cert G R ss] returns [true] then every conclusion of
    every step is true in every world that satisfies [G] and satisfies every
    rule of [R].

    The rules are assumed and never checked, so this is the RELATIVE warrant and
    it is the only one a certificate over a table somebody supplied can earn.
    See [Builtin.v] for how much of the absolute one this development reaches,
    and [rocq/README.md] for how much it does not.
*)

From Stdlib Require Import String List Bool Arith NArith.
From OOCertRocq Require Import Syntax Semantics Checker.
Import ListNotations.
Open Scope list_scope.

(** ** From the checker's partial instantiation to a total substitution

    The checker never builds a total function ([Checker.v], R-CHK-1). The
    semantics quantifies over nothing else ([Semantics.v], R-SEM-2). One
    substitution bridges them, and the bridge is needed only on the patterns
    that instantiated, so the value it takes elsewhere is irrelevant and is
    visibly arbitrary below. *)

Definition subst_of (b : list (string * term)) : subst :=
  fun v => match blookup b v with
           | Some t => t
           | None => EmptyString   (* never consulted; see [pinst_agrees] *)
           end.

Lemma pinst_agrees : forall b p t,
  pinst b p = Some t -> inst_pat (subst_of b) p = t.
Proof.
  intros b [v|u] t H; simpl in *.
  - unfold subst_of. now rewrite H.
  - now injection H.
Qed.

Lemma ainst_agrees : forall b a t,
  ainst b a = Some t -> inst (subst_of b) a = t.
Proof.
  intros b a t H. unfold ainst in H. unfold inst.
  destruct (pinst b (psubj a)) eqn:Hs; try discriminate.
  destruct (pinst b (ppred a)) eqn:Hp; try discriminate.
  destruct (pinst b (pobj a)) eqn:Ho; try discriminate.
  apply pinst_agrees in Hs, Hp, Ho.
  rewrite Hs, Hp, Ho. now injection H.
Qed.

Lemma ainsts_agrees : forall b l ts,
  ainsts b l = Some ts -> map (inst (subst_of b)) l = ts.
Proof.
  induction l as [|a rest IH]; intros ts H; simpl in *.
  - now injection H.
  - destruct (ainst b a) eqn:Ha; try discriminate.
    destruct (ainsts b rest) eqn:Hr; try discriminate.
    injection H as <-. simpl.
    now rewrite (ainst_agrees _ _ _ Ha), (IH l eq_refl).
Qed.

(** ** Membership *)

Lemma mem_triple_In : forall t l, mem_triple t l = true -> In t l.
Proof.
  induction l as [|u rest IH]; simpl; intro H.
  - discriminate.
  - apply orb_prop in H as [H|H].
    + left. symmetry. now apply triple_eqb_true.
    + right. now apply IH.
Qed.

(** ** The checker with its well-formedness conjuncts removed

    R-CHK-2 claims that distinct keys and coverage carry NO soundness content.
    A claim like that is exactly the kind a formalisation should be made to
    check rather than allowed to assert, so here is the checker without them,
    and the soundness theorem is proved of THIS one. The real checker is shown
    to imply it, so the real checker inherits the theorem and the claim is
    machine-checked in the direction that matters: adding the two conjuncts
    makes the hypothesis stronger, so it cannot have weakened anything. *)

Lemma nth_error_N_In : forall {A : Type} (l : list A) n x,
  nth_error_N l n = Some x -> In x l.
Proof.
  induction l as [|y r IH]; simpl; intros n x H; [discriminate|].
  destruct (N.eqb n 0).
  - injection H as <-. now left.
  - right. exact (IH _ _ H).
Qed.

Definition check_step_weak (known : list triple) (R : list rule) (s : step) : bool :=
  match nth_error_N R (sidx s) with
  | None => false
  | Some r =>
      match ainsts (sbind s) (rbody r), ainst (sbind s) (rhead r) with
      | Some prems, Some hd =>
          triple_eqb (sconcl s) hd
          && forallb (fun p => mem_triple p known) prems
      | _, _ => false
      end
  end.

Lemma check_step_implies_weak : forall known R s,
  check_step known R s = true -> check_step_weak known R s = true.
Proof.
  intros known R s H. unfold check_step, check_step_weak in *.
  destruct (nth_error_N R (sidx s)) as [r|]; [|discriminate].
  apply andb_prop in H as [_ H].
  destruct (ainsts (sbind s) (rbody r)); [|discriminate].
  destruct (ainst (sbind s) (rhead r)); [|discriminate].
  apply andb_prop in H as [H1 H2].
  apply andb_prop in H1 as [_ H1].
  now rewrite H1, H2.
Qed.

(** ** One step

    Everything the step needs is already entailed, so the rule carries it. *)

Lemma check_step_sound : forall G R known s,
  (forall t, In t known -> entails_rel G R t) ->
  check_step_weak known R s = true ->
  entails_rel G R (sconcl s).
Proof.
  intros G R known s Hknown Hchk.
  unfold check_step_weak in Hchk.
  destruct (nth_error_N R (sidx s)) as [r|] eqn:Hr; [|discriminate].
  destruct (ainsts (sbind s) (rbody r)) as [prems|] eqn:Hprems; [|discriminate].
  destruct (ainst (sbind s) (rhead r)) as [hd|] eqn:Hhd; [|discriminate].
  apply andb_prop in Hchk as [Hconcl Hmem].
  apply triple_eqb_true in Hconcl. subst hd.
  apply ainsts_agrees in Hprems.
  apply ainst_agrees in Hhd.
  intros I HG HR.
  rewrite <- Hhd.
  apply (HR r (nth_error_N_In _ _ _ Hr)).
  intros a Ha.
  (* the instantiated body atom is one of the written premises, and every
     written premise is known, and everything known is entailed *)
  assert (Hin : In (inst (subst_of (sbind s)) a) prems).
  { rewrite <- Hprems. now apply in_map. }
  rewrite forallb_forall in Hmem.
  specialize (Hmem _ Hin).
  apply mem_triple_In in Hmem.
  exact (Hknown _ Hmem I HG HR).
Qed.

(** ** The whole certificate

    Induction on the step list, with the accumulator generalised. The invariant
    is that everything in [known] is already entailed, which R-CHK-5 keeps true
    because the accumulator only ever grows by a conclusion that has just been
    proved entailed. *)

Lemma check_from_sound : forall R ss G known,
  (forall t, In t known -> entails_rel G R t) ->
  check_from known R ss = true ->
  forall t, In t (map sconcl ss) -> entails_rel G R t.
Proof.
  intros R ss. induction ss as [|s rest IH]; intros G known Hknown Hchk t Hin.
  - simpl in Hin. contradiction.
  - simpl in Hchk. apply andb_prop in Hchk as [Hstep Hrest].
    apply check_step_implies_weak in Hstep.
    assert (Hs : entails_rel G R (sconcl s))
      by exact (check_step_sound _ _ _ _ Hknown Hstep).
    simpl in Hin. destruct Hin as [<-|Hin]; [exact Hs|].
    apply (IH G (known ++ [sconcl s])); [|exact Hrest|exact Hin].
    intros u Hu. apply in_app_or in Hu as [Hu|Hu].
    + now apply Hknown.
    + destruct Hu as [<-|[]]. exact Hs.
Qed.

(** ** THE THEOREM *)

Theorem horn_certificate_sound : forall G R ss,
  check_cert G R ss = true ->
  forall t, In t (map sconcl ss) -> entails_rel G R t.
Proof.
  intros G R ss Hchk t Hin.
  apply (check_from_sound R ss G G); [|exact Hchk|exact Hin].
  intros u Hu. now apply entails_rel_asserted.
Qed.

(** The same statement over the checker with the two well-formedness conjuncts
    deleted. This is the machine-checked form of R-CHK-2's claim that neither
    conjunct carries soundness content: the theorem holds without them, so
    adding them cannot have bought any of it. What they buy is in
    [Determinacy.v]. *)

Fixpoint check_from_weak (known : list triple) (R : list rule) (ss : list step) : bool :=
  match ss with
  | [] => true
  | s :: rest => check_step_weak known R s && check_from_weak (known ++ [sconcl s]) R rest
  end.

Lemma check_from_weak_sound : forall R ss G known,
  (forall t, In t known -> entails_rel G R t) ->
  check_from_weak known R ss = true ->
  forall t, In t (map sconcl ss) -> entails_rel G R t.
Proof.
  intros R ss. induction ss as [|s rest IH]; intros G known Hknown Hchk t Hin.
  - simpl in Hin. contradiction.
  - simpl in Hchk. apply andb_prop in Hchk as [Hstep Hrest].
    assert (Hs : entails_rel G R (sconcl s))
      by exact (check_step_sound _ _ _ _ Hknown Hstep).
    simpl in Hin. destruct Hin as [<-|Hin]; [exact Hs|].
    apply (IH G (known ++ [sconcl s])); [|exact Hrest|exact Hin].
    intros u Hu. apply in_app_or in Hu as [Hu|Hu].
    + now apply Hknown.
    + destruct Hu as [<-|[]]. exact Hs.
Qed.

Theorem horn_certificate_sound_without_wellformedness : forall G R ss,
  check_from_weak G R ss = true ->
  forall t, In t (map sconcl ss) -> entails_rel G R t.
Proof.
  intros G R ss Hchk t Hin.
  apply (check_from_weak_sound R ss G G); [|exact Hchk|exact Hin].
  intros u Hu. now apply entails_rel_asserted.
Qed.
