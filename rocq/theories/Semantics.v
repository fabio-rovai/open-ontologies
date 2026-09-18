(** * What a Horn certificate's conclusions are supposed to mean.

    Soundness is only as strong as the entailment relation it concludes, so this
    file is the one to argue with. It defines the RULE-RELATIVE relation, which
    is what a certificate over a rule table somebody supplied can possibly earn.
    The ABSOLUTE relation, which needs the built-in table discharged against a
    formal semantics of the RDFS and OWL vocabulary, is in [Interp.v] and
    [Builtin.v] and is a separate and much larger claim.

    R-SEM-1. A model here is a PREDICATE ON TRIPLES, not a structure with a
    domain. Truth of a triple is primitive.

    That is a deliberate choice and the reason for it is worth stating, because
    it is the opposite of what the absolute layer needs. At the Horn layer there
    is no vocabulary: a rule table is data, its rules mention whatever terms
    they mention, and no clause of any specification is available to interpret
    them. The weakest thing that can carry "this triple holds" is a predicate on
    triples, and the weakest semantics gives the LARGEST class of models, which
    gives the STRONGEST entailment claim. Anything with more structure would
    make the model class smaller and the soundness theorem below weaker than it
    looks, which is exactly the failure a second and third formalisation exist
    to catch. [Interp.v] puts structure back in, on purpose, because the
    built-in rules cannot be discharged without it, and pays the price in the
    size of the claim.

    R-SEM-2. A substitution is a TOTAL function from variable names to terms.
    Not a finite map, not an option-valued lookup. A rule says something about
    every way of instantiating it, so the quantifier in [sat_rule] below ranges
    over every total function, and the checker's finite binding is a fact about
    the certificate rather than a restriction on the semantics. [Checker.v]
    instantiates PARTIALLY and the bridge between the two is a lemma
    ([Sound.v], [subst_of_agrees]) rather than an identification. *)

From Stdlib Require Import String List Bool.
From OOCertRocq Require Import Syntax.
Import ListNotations.
Open Scope string_scope.

Definition subst := string -> term.

Definition inst_pat (s : subst) (p : pat) : term :=
  match p with PVar v => s v | PTerm t => t end.

Definition inst (s : subst) (a : tpat) : triple :=
  Tri (inst_pat s (psubj a)) (inst_pat s (ppred a)) (inst_pat s (pobj a)).

(** A world: the triples that hold. *)
Definition world := triple -> Prop.

Definition sat_graph (I : world) (G : list triple) : Prop :=
  forall t, In t G -> I t.

(** R-SEM-3. A world satisfies a rule when EVERY total instantiation whose body
    holds has a head that holds. This is the universal closure of the rule read
    as an implication, which is the only reading a Horn rule has. *)
Definition sat_rule (I : world) (r : rule) : Prop :=
  forall s : subst,
    (forall a, In a (rbody r) -> I (inst s a)) -> I (inst s (rhead r)).

Definition sat_rules (I : world) (R : list rule) : Prop :=
  forall r, In r R -> sat_rule I r.

(** R-SEM-4. THE RULE-RELATIVE ENTAILMENT RELATION, and the one a certificate
    over a user-supplied table earns.

    The rules are ASSUMED. A table containing "every supplier is compliant"
    makes certificates whose conclusions are entailed in this sense for ever,
    because every world that fails to satisfy that rule has been quantified
    away. Decision 0003 names this and requires the two verdicts never to share
    a word, which is why this relation has its own name here and the absolute
    one is defined in a different file over a different notion of model. *)
Definition entails_rel (G : list triple) (R : list rule) (t : triple) : Prop :=
  forall I : world, sat_graph I G -> sat_rules I R -> I t.

(** The relation is reflexive on the asserted graph, which is the base case of
    the certificate induction and nothing more. *)
Lemma entails_rel_asserted :
  forall G R t, In t G -> entails_rel G R t.
Proof. intros G R t Hin I HG _. exact (HG t Hin). Qed.

(** ** Non-vacuity, stated here and proved in [Witness.v]

    A soundness theorem over an empty model class proves nothing, so the claim
    that this relation is not the total relation is a theorem in this
    development rather than a remark. See [Witness.v]. *)
