(** * Bytes in, verdict out, with the theorem attached.

    [Sound.v] and [Builtin.v] talk about parsed objects. A user hands the tool
    three FILES, so the statement a user is entitled to has to start at the
    bytes. [run_is_sound] is that statement: if [run] accepts, then the
    conclusions of the certificate the third file parses to are entailed by the
    graph the second file parses to, under the table the first file parses to,
    and if the table is the built-in one then entailed outright.

    R-RUN-1. THE TWO VERDICTS NEVER SHARE A WORD, and the distinction is carried
    in the OUTCOME TYPE rather than in the printing. [Accepted] carries a boolean
    saying whether the supplied table is the built-in one, and the driver turns
    that boolean into one of two verdict strings naming two different theorems.
    Decision 0003 requires the separation; putting it in the type means the
    driver cannot lose it by accident, only by editing the one place it is
    decided.

    R-RUN-2. THE TABLE IS COMPARED AS PARSED DATA (R-PARSE-7), which is why the
    boolean is computed here and not in the driver. A driver comparing bytes
    would award the absolute verdict on the strength of a byte-for-byte match
    and withhold it from a file with a different trailing newline. *)

From Stdlib Require Import String List Bool Arith.
From OOCertRocq Require Import Syntax Semantics Checker Sound Interp Builtin Parse.
Import ListNotations.
Open Scope list_scope.

Inductive outcome : Set :=
  | Accepted   : bool -> nat -> nat -> nat -> outcome
  | Rejected   : nat -> nat -> nat -> outcome
  | ParseError : nat -> outcome.   (* 0 rules, 1 asserted graph, 2 certificate *)

Definition run (rtxt gtxt dtxt : string) : outcome :=
  match parse_rules rtxt with
  | None => ParseError 0
  | Some R =>
      match parse_triples gtxt with
      | None => ParseError 1
      | Some G =>
          match parse_steps dtxt with
          | None => ParseError 2
          | Some ss =>
              if check_cert G R ss
              then Accepted (rules_eqb R builtin) (length R) (length G) (length ss)
              else Rejected (length R) (length G) (length ss)
          end
      end
  end.

(** ** The table comparison is an equality, not a heuristic *)

Lemma pat_eqb_true : forall p q, pat_eqb p q = true -> p = q.
Proof.
  intros [a|a] [b|b] H; simpl in H; try discriminate;
    apply String.eqb_eq in H; now subst.
Qed.

Lemma tpat_eqb_true : forall a b, tpat_eqb a b = true -> a = b.
Proof.
  intros [s1 p1 o1] [s2 p2 o2]. unfold tpat_eqb. simpl. intro H.
  apply andb_prop in H as [H1 H3]. apply andb_prop in H1 as [H1 H2].
  apply pat_eqb_true in H1, H2, H3. now subst.
Qed.

Lemma tpats_eqb_true : forall a b, tpats_eqb a b = true -> a = b.
Proof.
  induction a as [|x xs IH]; intros [|y ys] H; simpl in H; try discriminate.
  - reflexivity.
  - apply andb_prop in H as [H1 H2].
    apply tpat_eqb_true in H1. rewrite H1, (IH ys H2). reflexivity.
Qed.

Lemma rule_eqb_true : forall r q, rule_eqb r q = true -> r = q.
Proof.
  intros [n1 b1 h1] [n2 b2 h2]. unfold rule_eqb. simpl. intro H.
  apply andb_prop in H as [H1 H3]. apply andb_prop in H1 as [H1 H2].
  apply String.eqb_eq in H1. apply tpats_eqb_true in H2. apply tpat_eqb_true in H3.
  now subst.
Qed.

Lemma rules_eqb_true : forall a b, rules_eqb a b = true -> a = b.
Proof.
  induction a as [|x xs IH]; intros [|y ys] H; simpl in H; try discriminate.
  - reflexivity.
  - apply andb_prop in H as [H1 H2].
    apply rule_eqb_true in H1. rewrite H1, (IH ys H2). reflexivity.
Qed.

(** ** THE END-TO-END STATEMENT *)

Theorem run_is_sound : forall rtxt gtxt dtxt R G ss b nr ng ns,
  parse_rules rtxt = Some R ->
  parse_triples gtxt = Some G ->
  parse_steps dtxt = Some ss ->
  run rtxt gtxt dtxt = Accepted b nr ng ns ->
  (forall t, In t (map sconcl ss) -> entails_rel G R t)
  /\ (b = true -> forall t, In t (map sconcl ss) -> entails_abs G t).
Proof.
  intros rtxt gtxt dtxt R G ss b nr ng ns HR HG Hss Hrun.
  unfold run in Hrun. rewrite HR, HG, Hss in Hrun.
  destruct (check_cert G R ss) eqn:Hchk; [|discriminate].
  injection Hrun as Hb _ _ _.
  split.
  - intros t Hin. exact (horn_certificate_sound G R ss Hchk t Hin).
  - intros Htrue t Hin.
    subst b. apply rules_eqb_true in Htrue. subst R.
    exact (entails_of_builtin_horn G ss Hchk t Hin).
Qed.

(** A rejection and a parse error claim nothing, which is why there is no
    theorem about them. That asymmetry is the shape of the guarantee: this
    layer certifies an inference and never refutes one. A certificate the
    checker rejects may still have true conclusions, reached by a rule it does
    not have or written down in a way the format does not define. *)
