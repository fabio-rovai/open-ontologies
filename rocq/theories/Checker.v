(** * The checker.

    Executable, decidable, and consulting no semantics whatever. Checking a Horn
    certificate is purely syntactic: nothing below mentions a world, a
    substitution in the sense of [Semantics.v], or truth. That is why agreement
    between two checkers is evidence about a FILE FORMAT and not about two model
    theories, and it is why this file can be read on its own.

    The checking discipline is where independent formalisations of one format
    part company, so every decision is written down.
*)

From Stdlib Require Import String List Bool Arith NArith.
From OOCertRocq Require Import Syntax.
Import ListNotations.
Open Scope string_scope.

(** ** The binding, read as data

    R-CHK-1. A binding is a LIST OF PAIRS and this checker instantiates through
    it PARTIALLY. [blookup] returns [option term]; there is no default, no
    fallback to the variable's own name, and no total function anywhere in this
    file. A variable with no pair fails the lookup and the step fails with it.

    The reason to prefer the partial reading is that it has no hidden
    convention. A total reading has to answer "what is an unbound variable" and
    every answer is a decision the format never made. [None] is not a decision,
    it is the absence of one, and it is the shape the failure actually has. *)

Fixpoint blookup (b : list (string * term)) (v : string) : option term :=
  match b with
  | [] => None
  | (k, t) :: rest => if String.eqb k v then Some t else blookup rest v
  end.

Definition pinst (b : list (string * term)) (p : pat) : option term :=
  match p with PVar v => blookup b v | PTerm t => Some t end.

Definition ainst (b : list (string * term)) (a : tpat) : option triple :=
  match pinst b (psubj a), pinst b (ppred a), pinst b (pobj a) with
  | Some s, Some p, Some o => Some (Tri s p o)
  | _, _, _ => None
  end.

Fixpoint ainsts (b : list (string * term)) (l : list tpat) : option (list triple) :=
  match l with
  | [] => Some []
  | a :: rest =>
      match ainst b a, ainsts b rest with
      | Some t, Some ts => Some (t :: ts)
      | _, _ => None
      end
  end.

(** R-CHK-2. DISTINCT KEYS AND COVERAGE are required, and both are required
    BEFORE anything is instantiated.

    These are not soundness conditions. The theorem in [Sound.v] does not use
    either of them and would hold without them, which is stated there as
    [horn_certificate_sound_without_wellformedness] so that the claim is checked rather
    than asserted. They are here because decision 0008 made them normative for
    the format, and because of what they buy that a soundness theorem cannot
    say: that the certificate means ONE thing.

    Read a binding as a set of demands, one per written pair, "this variable is
    that term".

    - Distinct keys make those demands SATISFIABLE. A key written twice with two
      different terms demands two values for one variable and no substitution
      meets both, so first-wins and last-wins are not two readings of such a
      certificate, they are two conventions for discarding half of it.
    - Coverage makes every substitution meeting the demands instantiate the
      cited rule the SAME WAY.

    [Determinacy.v] proves both halves from scratch here rather than taking them
    on trust.

    A binding for a variable the cited rule never mentions is ACCEPTED. It is
    never consulted, so it cannot make a step mean two things, and refusing it
    would be tidiness dressed as determinacy. *)

Fixpoint mem_str (v : string) (l : list string) : bool :=
  match l with
  | [] => false
  | k :: rest => String.eqb k v || mem_str v rest
  end.

Fixpoint keys_distinct (b : list (string * term)) : bool :=
  match b with
  | [] => true
  | (k, _) :: rest => negb (mem_str k (map fst rest)) && keys_distinct rest
  end.

Definition is_some {A : Type} (o : option A) : bool :=
  match o with Some _ => true | None => false end.

Definition covers (b : list (string * term)) (r : rule) : bool :=
  forallb (fun v => is_some (blookup b v)) (rule_vars r).

(** ** A step

    R-CHK-3. THE WRITTEN PREMISE LIST MUST BE EXACTLY THE INSTANTIATED BODY,
    POSITION BY POSITION.

    This is the sharpest decision in the file and it is a reading of a sentence
    rather than a deduction from one. [docs/lean-certificates.md] says the
    reasoner writes "the premises the rule read, in a fixed order per rule". If
    the order is fixed by the rule then a premise list in another order is not
    the list the format describes, and the checker that accepts it is accepting
    a file the format does not define. The alternative reading, in which the
    written premises are a hint and the checker recomputes the body and looks
    each atom up, makes the written list carry no information at all: a
    certificate with the premise fields deleted would check identically, and a
    certificate with the premises of a different step in them would check green.
    A field that cannot be wrong is not evidence.

    So: [sprem s] is compared to the instantiated body by list equality, and
    separately every instantiated body atom must be KNOWN. The first is about
    the certificate being well formed; the second is about the inference being
    supported.

    If another formalisation of this format takes the other reading, these two
    checkers disagree on a permuted premise list and on a premise list of the
    wrong length, and that disagreement is a fact about the format rather than
    about either checker. It is reported, not repaired.

    R-CHK-4. An index outside the table is a REJECTION, not a parse error. The
    file parses; it cites a rule that is not there. Which of the three exit
    codes that deserves is a judgement, and the judgement here is that a
    certificate is a claim and an unsupported claim is false rather than
    unreadable. [tests/fixtures/horn/bad_index.tsv] is the fixture. *)

Definition list_triple_eqb (a b : list triple) : bool :=
  (Nat.eqb (length a) (length b))
  && forallb (fun p => triple_eqb (fst p) (snd p)) (combine a b).

Fixpoint mem_triple (t : triple) (l : list triple) : bool :=
  match l with
  | [] => false
  | u :: rest => triple_eqb t u || mem_triple t rest
  end.

(** R-CHK-6. THE INDEX LOOKUP RECURSES ON THE TABLE, NEVER ON THE INDEX.
    [nth_error] in the standard library recurses on a unary [nat], so an index
    of twenty digits has to be BUILT before it can be examined. This walks the
    list and decrements the index in binary, so an out-of-range index costs the
    length of the table whatever its magnitude. See R-STEP-3. *)

Fixpoint nth_error_N {A : Type} (l : list A) (n : N) : option A :=
  match l with
  | [] => None
  | x :: r => if N.eqb n 0 then Some x else nth_error_N r (N.pred n)
  end.

Definition check_step (known : list triple) (R : list rule) (s : step) : bool :=
  match nth_error_N R (sidx s) with
  | None => false
  | Some r =>
      keys_distinct (sbind s)
      && covers (sbind s) r
      && match ainsts (sbind s) (rbody r), ainst (sbind s) (rhead r) with
         | Some prems, Some hd =>
             list_triple_eqb (sprem s) prems
             && triple_eqb (sconcl s) hd
             && forallb (fun p => mem_triple p known) prems
         | _, _ => false
         end
  end.

(** R-CHK-5. A STEP MAY CITE THE ASSERTED GRAPH AND STRICTLY EARLIER STEPS, AND
    NOTHING ELSE.

    The accumulator grows after the step is checked, never before, so a step
    cannot support itself and two steps cannot support each other. That is the
    whole of the well-foundedness argument and it is structural: there is no
    occurs check and no cycle detection anywhere, because a left-to-right fold
    that appends after checking cannot build a cycle.

    [tests/fixtures/horn/bad_self.tsv] is the one-step version and
    [tests/fixtures/horn/deep/mutual_cert.tsv] the two-step version. Both are
    rejected here, and the rejection is a corollary of the shape of this
    function rather than a case in it.

    The conclusions are appended in order, so [known] after [n] steps is
    [G ++ map sconcl (firstn n ss)]. [Sound.v] uses exactly that. *)

Fixpoint check_from (known : list triple) (R : list rule) (ss : list step) : bool :=
  match ss with
  | [] => true
  | s :: rest => check_step known R s && check_from (known ++ [sconcl s]) R rest
  end.

Definition check_cert (G : list triple) (R : list rule) (ss : list step) : bool :=
  check_from G R ss.
