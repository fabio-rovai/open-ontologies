(** * Non-vacuity, and the gap between the two verdicts.

    A soundness theorem over an empty class of models proves nothing, and a
    soundness theorem over a class in which every triple is true proves nothing
    either. Both failures are invisible from inside the soundness proof, so both
    are ruled out here by exhibiting structures rather than by arguing.

    Three things are established.

    1. THE CONDITIONS OF [Interp.v] ARE SATISFIABLE. If they were contradictory,
       [entails_abs] would hold of everything and [entails_of_builtin_horn]
       would be a theorem about nothing.
    2. [entails_abs] IS NOT THE TOTAL RELATION. Some triple is not entailed by
       the empty graph, so the absolute verdict distinguishes cases.
    3. THE RELATIVE VERDICT IS STRICTLY WEAKER THAN THE ABSOLUTE ONE, and the
       gap is exhibited by a rule anybody could write. This is decision 0003's
       point made into a theorem: a certificate over a user table proves its
       conclusion only in models that satisfy the table, and a one-line table
       makes any triple at all come out relatively entailed. Printing one word
       for both would be a machine for turning an assumption into a fact with a
       proof attached. *)

From Stdlib Require Import String List Bool NArith.
From OOCertRocq Require Import Syntax Semantics Checker Sound Interp Builtin.
Import ListNotations.
Open Scope string_scope.

(** ** A model of the conditions

    Everything is false except that [owl:sameAs] denotes the identity, which is
    the one condition of [Interp.v] stated as an equality and so the one that
    cannot be satisfied by making the extension empty. IC and IP are empty,
    which is what makes every backward condition vacuous: they demand a triple
    only of things already in IC or IP.

    The domain is the set of terms and the denotation is the identity, so two
    different terms denote two different things. That is what lets the
    [owl:sameAs] condition hold without forcing anything else to. *)

Definition Iempty : interp :=
  Interp term
         (fun t => t)
         (fun p x y => String.eqb p owl_sameAs = true /\ x = y)
         (fun _ => False)
         (fun _ => False).

(** Every condition is discharged the same way: the hypothesis says a pair is
    in the extension of some property other than [owl:sameAs], which this
    interpretation makes false, or it says something is in IC or IP, which this
    interpretation makes empty. *)

Ltac no_such_rel H := destruct H as [H _]; vm_compute in H; discriminate.

Theorem the_conditions_are_satisfiable : RL Iempty.
Proof.
  constructor.
  - intros x y H; no_such_rel H.
  - intros x y H; contradiction.
  - intros x y H; no_such_rel H.
  - intros x y H; contradiction.
  - intros x y H; no_such_rel H.
  - intros x y H; contradiction.
  - intros x y H; no_such_rel H.
  - intros x y H; contradiction.
  - intros p H; no_such_rel H.
  - intros p H; no_such_rel H.
  - intros p q H; no_such_rel H.
  - intros x y; split.
    + intros [_ Hxy]; exact Hxy.
    + intros <-; split; [vm_compute; reflexivity | reflexivity].
  - intros x y; split.
    + intros H; no_such_rel H.
    + intros [H _]; contradiction.
  - intros x y; split.
    + intros H; no_such_rel H.
    + intros [H _]; contradiction.
  - intros r p c H _; no_such_rel H.
  - intros r p c H _; no_such_rel H.
  - intros r p v H _; no_such_rel H.
Qed.

(** ** The absolute relation is not the total relation *)

Definition probe : triple :=
  Tri "<http://ex.org/a>" "<http://ex.org/p>" "<http://ex.org/b>".

Theorem not_everything_is_absolutely_entailed : ~ entails_abs [] probe.
Proof.
  intro H.
  specialize (H Iempty the_conditions_are_satisfiable).
  assert (Hg : forall u, In u (@nil triple) -> itrue Iempty u) by (intros u []).
  specialize (H Hg).
  destruct H as [Hp _]. vm_compute in Hp. discriminate.
Qed.

(** ** The relative relation is not the total relation either *)

Theorem not_everything_is_relatively_entailed : ~ entails_rel [] [] probe.
Proof.
  intro H. exact (H (fun _ => False) (fun t Ht => match Ht with end)
                    (fun r Hr => match Hr with end)).
Qed.

(** ** THE GAP BETWEEN THE VERDICTS

    One rule with an empty body and a ground head. Nothing about it is exotic
    and nothing in the checker refuses it: a rule is data, and this is data.
    Under it the probe triple is relatively entailed by the EMPTY graph, and it
    is not absolutely entailed by the empty graph, so the two verdicts are
    genuinely different claims and a checker that prints one word for both is
    reporting a fact it does not have. *)

Definition laundering_table : list rule :=
  [Rule "assume-it" [] (TP (PTerm "<http://ex.org/a>")
                           (PTerm "<http://ex.org/p>")
                           (PTerm "<http://ex.org/b>"))].

Theorem a_user_rule_makes_anything_relatively_entailed :
  entails_rel [] laundering_table probe.
Proof.
  intros I _ HR.
  exact (HR _ (or_introl eq_refl) (fun v => v) (fun a Ha => match Ha with end)).
Qed.

Theorem the_relative_verdict_is_strictly_weaker :
  entails_rel [] laundering_table probe /\ ~ entails_abs [] probe.
Proof.
  split.
  - exact a_user_rule_makes_anything_relatively_entailed.
  - exact not_everything_is_absolutely_entailed.
Qed.

(** ** The checker accepts a certificate over that table

    Otherwise the gap above would be a statement about a table no certificate
    can cite. A one-step certificate citing rule 0 with an empty binding checks,
    so the weaker verdict is reachable from real bytes. *)

Definition laundering_cert : list step :=
  [Step 0%N [] (Tri "<http://ex.org/a>" "<http://ex.org/p>" "<http://ex.org/b>") []].

Theorem the_laundering_certificate_is_accepted :
  check_cert [] laundering_table laundering_cert = true.
Proof. reflexivity. Qed.
