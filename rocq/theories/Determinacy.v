(** * What the two well-formedness conjuncts buy.

    [Sound.v] proves that neither [keys_distinct] nor [covers] carries any
    soundness content: the theorem holds with both deleted. So a reader is
    entitled to ask what they are for, and "another formalisation refuses these
    shapes" is not an answer a formalisation may give.

    The answer is a sentence a soundness theorem cannot say. A soundness theorem
    quantifies over whatever substitution the checker happened to build, so it
    is satisfied by a checker that picks one reading of an ambiguous file and
    proves something about the reading it picked. The property below is about
    every checker at once, including checkers nobody has written:

    - DISTINCT KEYS buy EXISTENCE. Read the binding as a list of demands, one
      per written pair, "this variable is that term". Distinct keys make the
      demands satisfiable, and a repeated key with two different terms makes
      them unsatisfiable, so such a certificate has no reading at all rather
      than two.
    - COVERAGE buys UNIQUENESS. Every substitution meeting the demands
      instantiates the cited rule identically, so the step's premises and
      conclusion do not depend on which one a checker built.

    Both halves are refuted with their own hypothesis dropped, at the bottom of
    the file, so neither conjunct is decorative and the file says which one is
    load-bearing for which half.

    The property is deliberately stated over [extends], which is a statement
    about MEMBERSHIP IN THE LIST and mentions [blookup] nowhere. That is the
    point: it speaks about a checker carrying a partial map, a checker carrying
    a total function with a silent default, and a checker built on a hash map
    whose later key wins, and it says the same thing to all three. *)

From Stdlib Require Import String List Bool Arith.
From OOCertRocq Require Import Syntax Semantics Checker Sound.
Import ListNotations.
Open Scope list_scope.

(** A substitution MEETS the demands a binding writes down. *)
Definition extends (b : list (string * term)) (s : subst) : Prop :=
  forall v t, In (v, t) b -> s v = t.

(** ** Plumbing *)

Lemma mem_str_false_neq : forall k l v,
  mem_str k l = false -> In v l -> k <> v.
Proof.
  induction l as [|x rest IH]; simpl; intros v H Hin.
  - contradiction.
  - apply orb_false_elim in H as [Hx Hrest].
    destruct Hin as [<-|Hin].
    + intro Heq. rewrite Heq, String.eqb_refl in Hx. discriminate.
    + now apply IH.
Qed.

Lemma blookup_In : forall b v t, blookup b v = Some t -> In (v, t) b.
Proof.
  induction b as [|[k u] rest IH]; simpl; intros v t H.
  - discriminate.
  - destruct (String.eqb k v) eqn:Hk.
    + apply String.eqb_eq in Hk. subst k. injection H as <-. now left.
    + right. now apply IH.
Qed.

(** ** Existence: what distinct keys buy

    [subst_of] is a witness, and the statement is that it is ONE, not that it is
    the only one. Nothing below privileges it. *)

Lemma keys_distinct_lookup : forall b v t,
  keys_distinct b = true -> In (v, t) b -> blookup b v = Some t.
Proof.
  induction b as [|[k u] rest IH]; simpl; intros v t Hd Hin.
  - contradiction.
  - apply andb_prop in Hd as [Hfresh Hd].
    apply negb_true_iff in Hfresh.
    destruct Hin as [Heq|Hin].
    + injection Heq as <- <-. now rewrite String.eqb_refl.
    + assert (Hneq : k <> v).
      { apply (mem_str_false_neq k (map fst rest) v Hfresh).
        change v with (fst (v, t)). now apply in_map. }
      destruct (String.eqb k v) eqn:Hk.
      * apply String.eqb_eq in Hk. contradiction.
      * now apply IH.
Qed.

Theorem a_distinct_binding_has_a_substitution : forall b,
  keys_distinct b = true -> extends b (subst_of b).
Proof.
  intros b Hd v t Hin.
  unfold subst_of. now rewrite (keys_distinct_lookup _ _ _ Hd Hin).
Qed.

(** ** Uniqueness: what coverage buys *)

Lemma inst_pat_ext : forall s1 s2 p,
  (forall v, In v (pat_vars p) -> s1 v = s2 v) ->
  inst_pat s1 p = inst_pat s2 p.
Proof.
  intros s1 s2 [v|u] H; simpl in *; [apply H; now left | reflexivity].
Qed.

Lemma inst_ext : forall s1 s2 a,
  (forall v, In v (tpat_vars a) -> s1 v = s2 v) ->
  inst s1 a = inst s2 a.
Proof.
  intros s1 s2 a H. unfold inst, tpat_vars in *.
  rewrite (inst_pat_ext s1 s2 (psubj a)),
          (inst_pat_ext s1 s2 (ppred a)),
          (inst_pat_ext s1 s2 (pobj a)); try reflexivity.
  - intros v Hv. apply H. apply in_or_app. right. apply in_or_app. now right.
  - intros v Hv. apply H. apply in_or_app. right. apply in_or_app. now left.
  - intros v Hv. apply H. apply in_or_app. now left.
Qed.

Lemma covers_agree : forall b r s1 s2,
  covers b r = true -> extends b s1 -> extends b s2 ->
  forall v, In v (rule_vars r) -> s1 v = s2 v.
Proof.
  intros b r s1 s2 Hc H1 H2 v Hv.
  unfold covers in Hc. rewrite forallb_forall in Hc.
  specialize (Hc v Hv). unfold is_some in Hc.
  destruct (blookup b v) as [t|] eqn:Hl; [|discriminate].
  apply blookup_In in Hl.
  now rewrite (H1 _ _ Hl), (H2 _ _ Hl).
Qed.

Theorem a_covering_binding_fixes_the_instantiation : forall b r s1 s2,
  covers b r = true -> extends b s1 -> extends b s2 ->
  map (inst s1) (rbody r) = map (inst s2) (rbody r)
  /\ inst s1 (rhead r) = inst s2 (rhead r).
Proof.
  intros b r s1 s2 Hc H1 H2.
  assert (Hag := covers_agree b r s1 s2 Hc H1 H2).
  split.
  - apply map_ext_in. intros a Ha. apply inst_ext.
    intros v Hv. apply Hag. unfold rule_vars. apply in_or_app. left.
    apply in_flat_map. now exists a.
  - apply inst_ext. intros v Hv. apply Hag.
    unfold rule_vars. apply in_or_app. now right.
Qed.

(** ** THE STATEMENT

    Existence and uniqueness together: a well-formed binding gives the cited
    rule exactly one instantiation, whatever a checker's internal
    representation of a binding happens to be. *)

Theorem wellformed_determines_instantiation : forall b r,
  keys_distinct b = true -> covers b r = true ->
  (exists s, extends b s)
  /\ (forall s1 s2, extends b s1 -> extends b s2 ->
        map (inst s1) (rbody r) = map (inst s2) (rbody r)
        /\ inst s1 (rhead r) = inst s2 (rhead r)).
Proof.
  intros b r Hd Hc. split.
  - exists (subst_of b). now apply a_distinct_binding_has_a_substitution.
  - intros s1 s2 H1 H2. now apply (a_covering_binding_fixes_the_instantiation b r).
Qed.

(** ** Each conjunct refuted with its own hypothesis dropped

    Without these two, the theorem above is consistent with both conjuncts being
    decoration. *)

Definition dup : list (string * term) :=
  [("x"%string, "<http://ex.org/a>"%string); ("x"%string, "<http://ex.org/zzz>"%string)].

Theorem no_substitution_extends_a_duplicate_key :
  keys_distinct dup = false /\ ~ (exists s, extends dup s).
Proof.
  split; [reflexivity|].
  intros [s Hs].
  assert (H1 : s "x"%string = "<http://ex.org/a>"%string)
    by (apply Hs; simpl; now left).
  assert (H2 : s "x"%string = "<http://ex.org/zzz>"%string)
    by (apply Hs; simpl; right; now left).
  rewrite H1 in H2. discriminate.
Qed.

(** The rule [?s <p> ?o -> ?s <q> ?z], whose head carries a variable the body
    never binds. A binding for [s] and [o] omits [z]. Two substitutions meeting
    exactly the same demands then put two different terms in the conclusion, and
    neither term was written by anybody. *)

Definition unsafe_head_rule : rule :=
  Rule "unsafe"%string
       [TP (PVar "s"%string) (PTerm "<http://ex.org/p>"%string) (PVar "o"%string)]
       (TP (PVar "s"%string) (PTerm "<http://ex.org/q>"%string) (PVar "z"%string)).

Definition partial_binding : list (string * term) :=
  [("s"%string, "<http://ex.org/a>"%string); ("o"%string, "<http://ex.org/b>"%string)].

Definition pick (t : term) : subst :=
  fun v => if String.eqb v "s"%string then "<http://ex.org/a>"%string
           else if String.eqb v "o"%string then "<http://ex.org/b>"%string
           else t.

Theorem two_extensions_of_an_uncovered_binding_disagree :
  covers partial_binding unsafe_head_rule = false
  /\ extends partial_binding (pick "<http://ex.org/one>"%string)
  /\ extends partial_binding (pick "<http://ex.org/two>"%string)
  /\ inst (pick "<http://ex.org/one>"%string) (rhead unsafe_head_rule)
     <> inst (pick "<http://ex.org/two>"%string) (rhead unsafe_head_rule).
Proof.
  refine (conj _ (conj _ (conj _ _))).
  - reflexivity.
  - intros v t Hin. simpl in Hin.
    destruct Hin as [H|[H|[]]]; injection H as <- <-; reflexivity.
  - intros v t Hin. simpl in Hin.
    destruct Hin as [H|[H|[]]]; injection H as <- <-; reflexivity.
  - simpl. intro H. injection H as H. discriminate.
Qed.

(** The repair decision 0008 relies on: a step whose binding omits a variable
    can always be rewritten with that variable bound to the term the conclusion
    already shows, and the rewritten binding covers. So the refusal loses no
    certificate anybody meant. *)

Theorem the_repair_covers :
  covers (partial_binding ++ [("z"%string, "<http://ex.org/one>"%string)])
         unsafe_head_rule = true.
Proof. reflexivity. Qed.
