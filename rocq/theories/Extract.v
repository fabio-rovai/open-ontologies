(** * Extraction to OCaml.

    R-EXT-1. NO [Extract Inductive] AND NO [Extract Constant]. Every one of
    those is a mapping the Rocq kernel does not check and the OCaml compiler
    cannot check either, so each is a piece of trusted glue between a proof and
    a program. The usual conveniences, [ExtrOcamlString] and [ExtrOcamlNatInt],
    each add one. They are not used: [string] and [nat] come out as the
    inductive types they are, the driver converts at the boundary in about
    twenty lines, and the whole of what is trusted about the translation is the
    extraction mechanism itself.

    The cost is that the extracted [string] is a linked list of eight-boolean
    characters and [nat] is unary, so the binary is slow. On this format, whose
    largest committed input is a few kilobytes, that is not a cost worth buying
    trust back from.

    R-EXT-2. THE DRIVER IS UNTRUSTED AND IS EXPECTED TO BE. It opens files,
    splits nothing, decides nothing, prints JSON and returns an exit code. The
    exact boundary is in [rocq/README.md]. *)

From OOCertRocq Require Import Run Builtin Parse.
From Stdlib Require Import Extraction.

Extraction Language OCaml.
Set Extraction Output Directory "driver".

(** [run] is the whole checker. [builtin] and [ruleStr]-free printing are not
    extracted: the driver never needs the table, because the comparison against
    it happens inside [run] (R-RUN-2). *)
Extraction "rocq_horn_core.ml" run.
