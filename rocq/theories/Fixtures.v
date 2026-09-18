(** * The checker run on the committed bytes, inside the kernel.

    Two things are gated here and they are different.

    THE TRANSCRIPTION GATE. [Builtin.v] writes the built-in table out as Rocq
    data, and everything the absolute theorem says is about THAT table. If the
    transcription differs from [tests/fixtures/horn/builtin_rules.tsv] in one
    field then the absolute theorem is about a table nothing uses, and no proof
    in this development would notice.
    [the_built_in_table_is_the_committed_fixture] closes that, by parsing the
    real bytes with this development's own parser and requiring the result to be
    equal to the transcription.

    THE BEHAVIOUR GATE. The verdicts below are computed by the kernel, not by
    the extracted binary, so they are what the PROVED checker does. The Rust
    differential runs the extracted binary over the same files, so a divergence
    between these lines and that run is an extraction bug and is visible rather
    than silent. That is the only handle this development has on the extraction
    step, which is trusted and not verified.

    Nothing in [Sound.v], [Determinacy.v] or [Builtin.v] depends on this file.
    It is a gate, and a gate is not a theorem. *)

From Stdlib Require Import String List Bool.
From OOCertRocq Require Import Syntax Semantics Checker Sound Interp Builtin Parse Run FixtureData.
Import ListNotations.
Open Scope string_scope.

(** ** The transcription *)

Theorem the_built_in_table_is_the_committed_fixture :
  parse_rules f_builtin_rules = Some builtin.
Proof. vm_compute. reflexivity. Qed.

(** ** Accepted

    [good.tsv] cites [rdfs9] over [asserted.tsv]: 27 rules, 2 asserted triples,
    1 step, and the table is the built-in one, so this run earns the absolute
    verdict. *)

Theorem good_is_accepted_with_the_absolute_verdict :
  run f_builtin_rules f_asserted f_good = Accepted true 27 2 1.
Proof. vm_compute. reflexivity. Qed.

(** The certificate says something the graph does not. Without this the
    acceptance above would be compatible with a checker that only ever accepts
    certificates repeating their own input. *)

Theorem the_accepted_conclusion_is_not_asserted :
  match parse_triples f_asserted, parse_steps f_good with
  | Some G, Some (s :: _) => mem_triple (sconcl s) G = false
  | _, _ => False
  end.
Proof. vm_compute. reflexivity. Qed.

(** ** Rejected

    Five shipped forgeries, each wrong in one place, and the deep pair. *)

Theorem bad_binding_is_rejected :
  run f_builtin_rules f_asserted f_bad_binding = Rejected 27 2 1.
Proof. vm_compute. reflexivity. Qed.

Theorem bad_conclusion_is_rejected :
  run f_builtin_rules f_asserted f_bad_conclusion = Rejected 27 2 1.
Proof. vm_compute. reflexivity. Qed.

(** R-CHK-4 in action: rule index 99 into a table of 27 is a rejection and not
    a parse error. The file is readable; the claim it makes is unsupported. *)
Theorem bad_index_is_rejected :
  run f_builtin_rules f_asserted f_bad_index = Rejected 27 2 1.
Proof. vm_compute. reflexivity. Qed.

Theorem bad_premise_is_rejected :
  run f_builtin_rules f_asserted f_bad_premise = Rejected 27 2 1.
Proof. vm_compute. reflexivity. Qed.

(** R-CHK-5, the one-step version: a step whose premise is its own conclusion. *)
Theorem bad_self_is_rejected :
  run f_builtin_rules f_asserted f_bad_self = Rejected 27 2 1.
Proof. vm_compute. reflexivity. Qed.

Theorem deep_self_support_is_rejected :
  run f_builtin_rules f_deep_self_support_asserted f_deep_self_support_cert
    = Rejected 27 1 1.
Proof. vm_compute. reflexivity. Qed.

(** R-CHK-5, the two-step version: each step supported only by the other. *)
Theorem deep_mutual_support_is_rejected :
  run f_builtin_rules f_deep_mutual_asserted f_deep_mutual_cert = Rejected 27 2 2.
Proof. vm_compute. reflexivity. Qed.

(** The same two steps over a graph that seeds the cycle are ACCEPTED, which is
    what makes the rejection above about the cycle rather than about the steps.
    A gate that rejected both would prove nothing. *)
Theorem deep_mutual_seeded_is_accepted :
  run f_builtin_rules f_deep_mutual_seeded_asserted f_deep_mutual_cert
    = Accepted true 27 3 2.
Proof. vm_compute. reflexivity. Qed.

(** ** The other two committed tables parse, and neither is the built-in one *)

Theorem short_rules_parse_and_are_not_the_built_ins :
  match parse_rules f_short_rules with
  | Some R => length R = 3 /\ rules_eqb R builtin = false
  | None => False
  end.
Proof. vm_compute. split; reflexivity. Qed.

Theorem user_rules_parse_and_are_not_the_built_ins :
  match parse_rules f_user_rules with
  | Some R => rules_eqb R builtin = false
  | None => False
  end.
Proof. vm_compute. reflexivity. Qed.
