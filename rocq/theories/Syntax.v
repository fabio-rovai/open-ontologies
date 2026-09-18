(** * Syntax of the Horn certificate format.

    This file fixes the OBJECTS the rest of the development talks about: terms,
    triples, triple patterns, rules, certificate steps. It carries no semantics
    and no checking, so a reader can disagree with the model theory in
    [Semantics.v] or with the discipline in [Checker.v] without disagreeing with
    anything here.

    Every modelling decision is recorded at the point it is made, tagged [R-...],
    with the source it rests on. That list is what makes a later disagreement
    with another formalisation diagnosable rather than merely embarrassing. The
    sources are the format description in [docs/lean-certificates.md], decision
    records 0002, 0003 and 0008, and the committed fixture bytes under
    [tests/fixtures/horn/]. Nothing under [lean/] or [isabelle/] is a source for
    any decision below; see [rocq/README.md] for exactly how far that goes and
    where it stops.
*)

From Stdlib Require Import String List Bool Arith NArith.
Import ListNotations.
Open Scope string_scope.

(** ** Terms

    R-TERM-1. A term is the field's bytes and nothing else. The format is a TSV
    of opaque fields: no field says whether it holds an IRI, a blank node or a
    literal, and no rule in any table this checker reads inspects a term's
    internal structure. So there is no term datatype here with an [Iri]
    constructor and a [Lit] constructor, there is a string.

    This is a real choice and it has consequences a reader should see rather
    than infer. [<http://ex.org/a>] and [http://ex.org/a] are DIFFERENT terms,
    because their bytes differ. ["01"^^xsd:integer] and ["1"^^xsd:integer] are
    different terms, because their bytes differ, and nothing here normalises a
    lexical form. A term that is not writable RDF at all, such as the bare
    string [z], is still a term. Decision 0008 says the last of those explicitly
    and it is the reason this file does not try to be cleverer. *)

Definition term := string.

Record triple : Set := Tri { tsubj : term ; tpred : term ; tobj : term }.

Definition triple_eqb (a b : triple) : bool :=
  String.eqb (tsubj a) (tsubj b)
  && String.eqb (tpred a) (tpred b)
  && String.eqb (tobj a) (tobj b).

Lemma triple_eqb_true : forall a b, triple_eqb a b = true -> a = b.
Proof.
  intros [s1 p1 o1] [s2 p2 o2]. unfold triple_eqb. simpl.
  intro H.
  apply andb_prop in H as [H1 H3]. apply andb_prop in H1 as [H1 H2].
  apply String.eqb_eq in H1, H2, H3. subst. reflexivity.
Qed.

Lemma triple_eqb_refl : forall a, triple_eqb a a = true.
Proof.
  intros [s p o]. unfold triple_eqb. simpl.
  now rewrite !String.eqb_refl.
Qed.

(** ** Patterns

    R-PAT-1. A pattern position is a variable exactly when its first byte is
    [?], and the variable's name is everything after it. That is read off the
    rule fixtures, where [?s] is a variable and
    [<http://www.w3.org/2000/01/rdf-schema#domain>] is not.

    The rule is deliberately stated as "first byte is [?]" rather than "first
    byte is [?] and the rest is non-empty", because the bare field [?] then
    names a variable whose name is the EMPTY STRING. That is not an accident and
    it is not tidied away here. A format whose lexer has an exception has an
    exception somewhere else too, and the empty-named variable is a real corner
    of this format that a fuzzer reached by a one-character typo. It is refused
    or admitted by the CHECKING rules in [Checker.v], where a refusal can be
    stated and proved, rather than by the lexer, where it would be silent. *)

Inductive pat : Set :=
  | PVar  : string -> pat
  | PTerm : term -> pat.

Record tpat : Set := TP { psubj : pat ; ppred : pat ; pobj : pat }.

(** A rule is a name, a body (a list of triple patterns, order significant) and
    a head (one triple pattern).

    R-RULE-1. The name is carried and is NEVER consulted by the checker. It
    exists so a report can say which rule was cited. Two rules with the same
    name and different shapes are two rules, which the built-in table needs:
    [scm-eqc1] appears twice there, licensing two conclusions. *)

Record rule : Set := Rule { rname : string ; rbody : list tpat ; rhead : tpat }.

(** ** Certificate steps

    R-STEP-1. A step cites a rule by INDEX into the supplied table, carries a
    binding as a list of (variable name, term) pairs, states one conclusion, and
    lists the premises it read. The premise list is part of the written
    certificate rather than something the checker recomputes and keeps to
    itself. [docs/lean-certificates.md] says the premises are written "in a
    fixed order per rule", so the order is contractual; [Checker.v] decides what
    that means and records the decision as R-CHK-3.

    R-STEP-3. THE RULE INDEX IS A BINARY NATURAL, not a unary one. Nothing in
    the format bounds it, so a certificate may cite rule 99999999999999999999,
    and a checker has to answer that in the time it takes to read the digits.
    With Rocq's unary [nat] the answer takes longer than the universe has left:
    the extracted checker built the number before it looked at it and died of a
    stack overflow, which OCaml reports with exit code 2, which is this tool's
    code for a parse error. So a crash was impersonating a verdict. The
    differential against the Lean checker found it; see decision 0015.

    R-STEP-2. The binding's keys are BARE variable names with no [?] prefix,
    which is read off [tests/fixtures/horn/good.tsv]: the rule at index 4 uses
    [?x], [?a], [?b] and the certificate binds [x], [a], [b]. *)

Record step : Set := Step {
  sidx   : N ;
  sbind  : list (string * term) ;
  sconcl : triple ;
  sprem  : list triple
}.

(** ** Variables of a rule

    Body and head, in that order, with duplicates left in. Only membership is
    ever asked of this list, so deduplicating it would be work for nothing. *)

Definition pat_vars (p : pat) : list string :=
  match p with PVar v => [v] | PTerm _ => [] end.

Definition tpat_vars (a : tpat) : list string :=
  pat_vars (psubj a) ++ pat_vars (ppred a) ++ pat_vars (pobj a).

Definition rule_vars (r : rule) : list string :=
  flat_map tpat_vars (rbody r) ++ tpat_vars (rhead r).
