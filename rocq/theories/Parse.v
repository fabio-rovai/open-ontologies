(** * From the bytes on disk to the objects in [Syntax.v].

    The parser is inside the development rather than in the untrusted driver,
    because the driver's job should be reading a file and printing a number and
    not deciding what a certificate says. Every decision below is one the format
    description does not settle, which is exactly why they are written down: a
    second reading of an unsettled question is how two verified checkers come to
    disagree about one file.

    R-PARSE-1. LINES ARE SEPARATED BY LF, AND A TRAILING LF AT END OF FILE DOES
    NOT MAKE A FINAL EMPTY LINE. A file of zero bytes has no lines, which makes
    an empty asserted graph and an empty certificate legal. An empty certificate
    is ACCEPTED, vacuously, and that is correct: it claims nothing.

    R-PARSE-2. AN EMPTY LINE ANYWHERE ELSE IS A PARSE ERROR. It splits into a
    single empty field, which is neither a rule nor a triple nor a step. Skipping
    it would be a repair, and a parser that repairs its input decides what the
    input meant.

    R-PARSE-3. CR IS AN ORDINARY BYTE. A file with CRLF line endings therefore
    has a trailing CR inside its last field on every line, and its terms differ
    from the LF version's terms. This is the decision most likely to produce a
    difference from another implementation, and it is made this way on purpose:
    stripping CR is a repair, the repository pins LF on these formats in
    [.gitattributes], and a checker that silently accepts two spellings of one
    term has two readings of one certificate.

    R-PARSE-4. FIELDS ARE SEPARATED BY TAB AND AN EMPTY FIELD IS A LEGAL TERM,
    namely the empty string. No field is trimmed. A term is its bytes (R-TERM-1),
    and the empty term is a term.

    R-PARSE-5. A COUNT FIELD IS DECIMAL, NON-EMPTY, AND ALL DIGITS. No sign, no
    whitespace, no leading plus. Leading zeros are allowed, so [04] is 4: the
    field is a number and nothing in the format says it has a canonical
    spelling.

    R-PARSE-8. COUNT FIELDS ARE PARSED INTO BINARY NATURALS AND THE CONSUMERS
    NEVER RECURSE ON THEM. Nothing bounds the digits, so a certificate may cite
    rule 99999999999999999999 or declare a binding of that many pairs, and a
    checker has to answer in the time it takes to read the line. Rocq's [nat] is
    unary, so building such a number is not an answer; the first version of this
    parser did build it, and the extracted checker died of a stack overflow that
    OCaml reports with exit code 2, which is this tool's code for a parse error.
    A crash was wearing a verdict's clothes. [take_tpats] and [take_pairs] now
    recurse on the FIELD LIST and decrement the count in binary, so a count
    larger than the fields available fails as fast as one that is one too big.
    See R-STEP-3 and decision 0015.

    R-PARSE-6. THE FIELD COUNT MUST BE EXACT. A rule line carries 2 + 3n + 3
    fields for a body of n atoms and no more; a triple line carries exactly 3. A
    trailing extra field is a parse error rather than something to ignore. A
    certificate step is the one place where a count is computed rather than
    declared: after the index, the binding count, the bindings and the
    conclusion, everything left is premises, and it must divide by three. *)

From Stdlib Require Import String List Bool Arith Ascii NArith.
From OOCertRocq Require Import Syntax.
Import ListNotations.
Open Scope string_scope.

Definition LF : ascii := ascii_of_nat 10.
Definition TAB : ascii := ascii_of_nat 9.
Definition QMARK : ascii := ascii_of_nat 63.

Fixpoint srev_app (s acc : string) : string :=
  match s with
  | EmptyString => acc
  | String c r => srev_app r (String c acc)
  end.

Definition srev (s : string) : string := srev_app s EmptyString.

Fixpoint split_go (sep : ascii) (s cur : string) (acc : list string) : list string :=
  match s with
  | EmptyString => List.rev (srev cur :: acc)
  | String c rest =>
      if Ascii.eqb c sep
      then split_go sep rest EmptyString (srev cur :: acc)
      else split_go sep rest (String c cur) acc
  end.

Definition split (sep : ascii) (s : string) : list string :=
  split_go sep s EmptyString [].

(** R-PARSE-1 in code: one trailing empty element, and only one, is dropped. *)
Definition lines (s : string) : list string :=
  let ls := split LF s in
  match List.rev ls with
  | EmptyString :: rest => List.rev rest
  | _ => ls
  end.

Definition fields (s : string) : list string := split TAB s.

(** R-PARSE-5 *)
Definition digit_val (c : ascii) : option nat :=
  let n := nat_of_ascii c in
  if andb (Nat.leb 48 n) (Nat.leb n 57) then Some (n - 48) else None.

Fixpoint parse_N_go (s : string) (acc : N) : option N :=
  match s with
  | EmptyString => Some acc
  | String c r =>
      match digit_val c with
      | Some d => parse_N_go r (N.add (N.mul acc 10) (N.of_nat d))
      | None => None
      end
  end.

Definition parse_N (s : string) : option N :=
  match s with
  | EmptyString => None
  | _ => parse_N_go s 0
  end.

(** R-PAT-1 in code. A field whose first byte is [?] is a variable whose name is
    the remaining bytes, so the field [?] alone names the variable whose name is
    the empty string. See the note in [Syntax.v]: this is a real corner of the
    format and it is left to the checker rather than patched here. *)
Definition parse_pat (f : string) : pat :=
  match f with
  | String c rest => if Ascii.eqb c QMARK then PVar rest else PTerm f
  | EmptyString => PTerm f
  end.

Fixpoint take_tpats (fs : list string) (n : N) {struct fs}
  : option (list tpat * list string) :=
  if N.eqb n 0 then Some ([], fs) else
  match fs with
  | a :: b :: c :: r =>
      match take_tpats r (N.pred n) with
      | Some (ps, rest) => Some (TP (parse_pat a) (parse_pat b) (parse_pat c) :: ps, rest)
      | None => None
      end
  | _ => None
  end.

Fixpoint take_pairs (fs : list string) (n : N) {struct fs}
  : option (list (string * term) * list string) :=
  if N.eqb n 0 then Some ([], fs) else
  match fs with
  | v :: t :: r =>
      match take_pairs r (N.pred n) with
      | Some (ps, rest) => Some ((v, t) :: ps, rest)
      | None => None
      end
  | _ => None
  end.

Fixpoint triples_of (fs : list string) : option (list triple) :=
  match fs with
  | [] => Some []
  | a :: b :: c :: r =>
      match triples_of r with
      | Some ts => Some (Tri a b c :: ts)
      | None => None
      end
  | _ => None
  end.

(** ** Lines *)

Definition parse_rule_line (fs : list string) : option rule :=
  match fs with
  | nm :: ns :: rest =>
      match parse_N ns with
      | None => None
      | Some n =>
          match take_tpats rest n with
          | Some (body, [h1; h2; h3]) =>
              Some (Rule nm body (TP (parse_pat h1) (parse_pat h2) (parse_pat h3)))
          | _ => None
          end
      end
  | _ => None
  end.

Definition parse_triple_line (fs : list string) : option triple :=
  match fs with
  | [a; b; c] => Some (Tri a b c)
  | _ => None
  end.

Definition parse_step_line (fs : list string) : option step :=
  match fs with
  | is_ :: ks :: rest =>
      match parse_N is_, parse_N ks with
      | Some i, Some k =>
          match take_pairs rest k with
          | Some (b, c1 :: c2 :: c3 :: prems) =>
              match triples_of prems with
              | Some ps => Some (Step i b (Tri c1 c2 c3) ps)
              | None => None
              end
          | _ => None
          end
      | _, _ => None
      end
  | _ => None
  end.

Fixpoint map_opt {A B : Type} (f : A -> option B) (l : list A) : option (list B) :=
  match l with
  | [] => Some []
  | x :: r =>
      match f x, map_opt f r with
      | Some y, Some ys => Some (y :: ys)
      | _, _ => None
      end
  end.

Definition parse_rules (txt : string) : option (list rule) :=
  map_opt (fun l => parse_rule_line (fields l)) (lines txt).

Definition parse_triples (txt : string) : option (list triple) :=
  map_opt (fun l => parse_triple_line (fields l)) (lines txt).

Definition parse_steps (txt : string) : option (list step) :=
  map_opt (fun l => parse_step_line (fields l)) (lines txt).

(** ** Comparing a supplied table against the built-in one

    R-PARSE-7. THE TABLE IS COMPARED AS PARSED DATA, NOT AS BYTES. Two files
    that parse to the same rules are the same table, so a table with different
    rule NAMES is a different table, because the name is a field of a rule. That
    is the conservative direction: the absolute verdict is awarded to fewer
    files rather than more. *)

Definition pat_eqb (p q : pat) : bool :=
  match p, q with
  | PVar a, PVar b => String.eqb a b
  | PTerm a, PTerm b => String.eqb a b
  | _, _ => false
  end.

Definition tpat_eqb (a b : tpat) : bool :=
  pat_eqb (psubj a) (psubj b) && pat_eqb (ppred a) (ppred b) && pat_eqb (pobj a) (pobj b).

Fixpoint tpats_eqb (a b : list tpat) : bool :=
  match a, b with
  | [], [] => true
  | x :: xs, y :: ys => tpat_eqb x y && tpats_eqb xs ys
  | _, _ => false
  end.

Definition rule_eqb (r q : rule) : bool :=
  String.eqb (rname r) (rname q)
  && tpats_eqb (rbody r) (rbody q)
  && tpat_eqb (rhead r) (rhead q).

Fixpoint rules_eqb (a b : list rule) : bool :=
  match a, b with
  | [], [] => true
  | x :: xs, y :: ys => rule_eqb x y && rules_eqb xs ys
  | _, _ => false
  end.
