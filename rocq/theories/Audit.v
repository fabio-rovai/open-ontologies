(** * The axiom footprint.

    A machine-checked proof is only as good as the list of things it took on
    trust, and that list is not visible from reading the proof. [Print
    Assumptions] is, so it is run here on every theorem this development
    advertises, and [rocq/build.sh] fails the build unless every one of them
    reports being closed under the global context.

    THE GATE CAN FAIL, and the way to see it fail is written down rather than
    asserted: add [Axiom cheat : False.] to any file below and rebuild, or
    replace a [Qed] with [Admitted]. The build script greps for the exact
    phrase, so an admitted proof turns the line into a list of constants and
    the grep stops matching. [rocq/build.sh --prove-the-gate-can-fail] does
    exactly that against a scratch copy and requires the build to break.

    [Print Assumptions] prints its verdict and NOT the name it is about, so fifteen
    consecutive "Closed under the global context" lines are unreadable and, worse,
    unattributable: a reader cannot tell which theorem is missing if fourteen of them
    print. Each one below is therefore preceded by a [Check], whose output carries the
    name and the statement, so the report reads as fifteen labelled pairs and
    [build.sh] can require fifteen of them.

    Nothing here is a theorem. This file adds no content and is a report. *)

From Stdlib Require Import String.
Open Scope string_scope.
From OOCertRocq Require Import Syntax Semantics Checker Determinacy Sound Interp Builtin Parse Run Witness Fixtures.

(** The relative warrant: bytes to entailment, both halves. *)
Check run_is_sound.
Print Assumptions run_is_sound.

(** The two soundness theorems it rests on. *)
Check horn_certificate_sound.
Print Assumptions horn_certificate_sound.
Check entails_of_builtin_horn.
Print Assumptions entails_of_builtin_horn.

(** The claim that neither well-formedness conjunct carries soundness content. *)
Check horn_certificate_sound_without_wellformedness.
Print Assumptions horn_certificate_sound_without_wellformedness.

(** What the two conjuncts do buy. *)
Check wellformed_determines_instantiation.
Print Assumptions wellformed_determines_instantiation.
Check no_substitution_extends_a_duplicate_key.
Print Assumptions no_substitution_extends_a_duplicate_key.
Check two_extensions_of_an_uncovered_binding_disagree.
Print Assumptions two_extensions_of_an_uncovered_binding_disagree.

(** The twenty-seven arms, as one statement. *)
Check builtin_sound.
Print Assumptions builtin_sound.

(** Non-vacuity, and the gap between the verdicts. *)
Check the_conditions_are_satisfiable.
Print Assumptions the_conditions_are_satisfiable.
Check not_everything_is_absolutely_entailed.
Print Assumptions not_everything_is_absolutely_entailed.
Check the_relative_verdict_is_strictly_weaker.
Print Assumptions the_relative_verdict_is_strictly_weaker.

(** The gates over the committed bytes. *)
Check the_built_in_table_is_the_committed_fixture.
Print Assumptions the_built_in_table_is_the_committed_fixture.
Check good_is_accepted_with_the_absolute_verdict.
Print Assumptions good_is_accepted_with_the_absolute_verdict.
Check deep_mutual_support_is_rejected.
Print Assumptions deep_mutual_support_is_rejected.
Check deep_mutual_seeded_is_accepted.
Print Assumptions deep_mutual_seeded_is_accepted.
