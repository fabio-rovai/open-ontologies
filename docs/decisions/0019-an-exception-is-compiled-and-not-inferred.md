# 0019 · An exception is compiled, and not inferred

- **Status**: implemented · `src/exceptions.rs`, `src/defects.rs` · CLI `compile-exceptions`,
  `batch` `compile-exceptions`, MCP `onto_compile_exceptions` ·
  `tests/disjointness_exceptions_test.rs` · issue #347
- **Written**: 2026-10-30
- **Related**: decision 0002 (an inference carries a certificate), decision 0016 (a conclusion
  names the axioms responsible for it)

## The problem

Issue #347 asked for inherited disjointness to become DEFEASIBLE. A taxonomy built from defaults
says `Vehicle owl:disjointWith Building`, files `Houseboat` under both, and means "disjoint unless
stated otherwise". Classically `Houseboat` is unsatisfiable, `defects` reports it as
`inherited_disjoint`, and the tableau agrees. The issue proposed an `oo:defeasible true`
annotation on the disjointness and a non-monotonic semantics (rational closure, or overriding as
in DL^N) under which `defects` would report an `overridden_default` instead of an error.

That is not built. What is built is an explicit exception, compiled into classical axioms before
any tool reads the ontology.

## Why the defeasible design is not built

1. **In a machine-extracted taxonomy most `inherited_disjoint` hits are extraction errors.** The
   usual cause of a class under two disjoint parents is a wrong parent, not an intended exception.
   A defeasible disjointness relabels every such error as an exception and switches the check off
   for exactly the users whose data most needs it. An exception has to be named, class by class,
   or it is not distinguishable from a mistake.

2. **A defeasible version of a symmetric disjointness is useless.** Read defeasibly,
   `Vehicle ⊓ Building ⊑ ⊥` becomes `Vehicle ⊓ Building |~ ⊥`. In the preferential semantics of
   Kraus, Lehmann and Magidor (Artificial Intelligence 44, 1990), and in the DL semantics built on
   it, `C |~ D` holds when the most preferred instances of C are in D. A satisfiable C has most
   preferred instances, so `C |~ ⊥` holds exactly when C is classically unsatisfiable, and the
   defeasible axiom collapses into the strict one. To be defeasible at all it has to be split into
   two directional defaults, `Vehicle |~ ¬Building` and `Building |~ ¬Vehicle`, which is a
   different ontology from the one the user wrote.

3. **Rational closure drowns.** Rational closure for description logics (Casini and Straccia,
   "Rational Closure for Defeasible Description Logics", JELIA 2010, LNCS 6341) ranks classes by
   exceptionality, and an exceptional class loses EVERY default it would have inherited, not only
   the one that conflicts. A `Houseboat` exceptional with respect to `Vehicle |~ ¬Building` would
   also lose every other default about vehicles. DL^N (Bonatti, Faella, Petrova and Sauro, "A new
   semantics for overriding in description logics", Artificial Intelligence 222, 2015) overrides a
   default only where it conflicts, so of the two semantics the issue cites, only DL^N fits the
   intent, and it is the more expensive one to implement.

4. **Every other tool would disagree.** If only `defects` read the annotation, `reason`, `shacl`,
   the owl-dl tableau and the Lean certificates (`oo-cert`) would still find `Houseboat`
   unsatisfiable, and the engine would contradict itself across tools. Teaching each of them a
   non-monotonic semantics is not on offer for the certificates in any case: the Lean checkers
   prove monotone Horn derivations sound (decision 0002), and a non-monotonic conclusion has no
   certificate in this repository and no route to one.

## Decisions

1. **An exception is an annotation on the reified disjointness, in this engine's namespace.**

   ```turtle
   @prefix exc: <https://open-ontologies.org/exceptions#> .
   ex:Vehicle owl:disjointWith ex:Building .
   [] a owl:Axiom ; owl:annotatedSource ex:Vehicle ; owl:annotatedProperty owl:disjointWith ;
      owl:annotatedTarget ex:Building ; exc:exceptFor ex:Houseboat .
   ```

   The namespace follows the existing pattern of one namespace per feature under
   `open-ontologies.org` (`shacl#`, `temporal#`). The annotation is accepted whichever direction
   the disjointness triple is written in, because `owl:disjointWith` is symmetric. Several
   reifications of one pair contribute the union of their exceptions, as OWL 2 reads several
   annotations on one axiom.

2. **The compile is classical, monotone in its input, and reads only the ASSERTED hierarchy of
   named classes.** That is `rdfs:subClassOf` between named classes, `owl:equivalentClass`
   between named classes both ways, and the named conjuncts of an `owl:intersectionOf` a class is
   declared under or equivalent to. Nothing is inferred and no reasoner runs.

   The reading is closed over that hierarchy. For an annotated `X ⊥ Y` with honoured exceptions
   E, let C be the exceptions and every class below one. A class `a` under X and a class `b` under
   Y stay compatible exactly when some member of C is told-below both, and are disjoint otherwise.
   This is not the GCI `X ⊓ Y ⊑ E1 ⊔ … ⊔ En`: under that GCI `Car ⊓ Building` would be
   satisfiable, and here it is not. The reading is written out as classical axioms:

   - Z, the exempt set, is C and every ancestor of a member of C. X and Y are in Z, since every
     honoured exception is under both.
   - `X owl:disjointWith Y` is removed.
   - `S owl:disjointWith Y` is emitted for every named S under X not in Z, and
     `T owl:disjointWith X` for every named T under Y not in Z. Nothing licensed is below such a
     class, so it meets nothing on the other side, and one pair against that side says so.
   - For `a` in Z under X and `b` in Z under Y, `a owl:disjointWith b` is emitted when no member
     of C is below both. Only the most general such pairs are written, since a pair below another
     is entailed by it, and a class on both sides is paired once with each partner.

   With one exception the last rule adds nothing, because every member of Z is then above or
   below that exception. With two exceptions under different intermediate classes it is what
   keeps the intermediates apart. The first version of this compile stopped at the first three
   rules, and on `X ⊥ Y` with `E1 ⊑ A1 ⊓ B1` and `E2 ⊑ A2 ⊓ B2` (each `Ai` under X, each `Bi`
   under Y) it emitted nothing at all, so `A1 ⊓ B2` became satisfiable and the axiom vanished. It
   now emits `A1 ⊥ B2` and `A2 ⊥ B1`. Z also used to hold only the ancestors of the exceptions,
   not of the classes below them, so a subclass of an exception filed under a second class on one
   side (`HouseboatDeluxe ⊑ Luxury ⊑ Vehicle`) gave `Luxury ⊥ Building` and made the subclass
   unsatisfiable. Under the reading `Luxury` and `Building` are compatible, and both cases are
   tests.

   A class under both X and Y that is not covered by an exception (`FloatingRestaurant` in the
   tests) gets both pairs and stays unsatisfiable. It moves from `inherited_disjoint` to
   `disjoint_with_ancestor` in `defects`, still an error, and the compile report lists it under
   `still_under_both`.

3. **An exception that is not under both sides is rejected by name.** It is reported as
   `exception_not_under_both`, with which side it is under, and contributes nothing. Honouring it
   would exempt its ancestors and descendants for no conflict, and ignoring it would leave a
   declared intention with no trace. When no exception on an axiom is honoured, the axiom is left
   as written.

4. **The compiled axiom leaves a record, not a dangling reification.** After the compile
   `X owl:disjointWith Y` is gone, and a reification still naming `owl:disjointWith` would describe
   an axiom the document no longer holds. A consumer that rebuilds an axiom from its reification
   would put the disjointness back. So the main triple becomes `X exc:compiledDisjointWith Y`,
   declared an `owl:AnnotationProperty`, and the reification's `owl:annotatedProperty` names that
   property: a well-formed annotated annotation assertion, inert to every reasoner. A second run
   reads the record and emits pairs for classes filed under X or Y since the first. A run over an
   unchanged graph changes nothing, and the tests compare the triple sets after RDFC-1.0
   canonicalisation to show it.

5. **A record whose every exception is now rejected restores the disjointness.** If the hierarchy
   changes so that no exception is under both sides any more, the record no longer justifies
   anything and `X owl:disjointWith Y` is asserted again (`status: restored`), rather than leaving
   the disjointness switched off with nothing to say why.

6. **`defects` on an uncompiled store still reports the exception as an error.** The disjointness
   is still asserted and every reasoner here still finds the class unsatisfiable, so `defects`
   must too, or the tools disagree, which is reason 4 above. The row gains
   `declared_exception: true`, the `exception` responsible (the class itself or an ancestor), and
   a `hint` naming the compile. The report counts such rows under `declared_exceptions`. Only an
   exception the compile would honour marks a row, so the hint never sends a reader to a command
   that would reject their annotation. The decoration is on the listing and not on `Finding`, so
   `plan`, which compares findings, does not read an added annotation as a new defect.

7. **Pairs are written where the axiom was.** A disjointness held in a named graph is compiled in
   that graph, and a pair asserted anywhere in the store already is never written twice.

## Cost, stated in the report rather than discovered later

- **Compatibility is per class, not per individual.** Two classes that stay compatible are
  compatible for every individual. With `Houseboat` excepted, `Vehicle` and `Building` stay
  compatible, so an individual typed only as `Vehicle` and `Building`, with no more specific
  class, is no longer a contradiction. `exempted_classes` is Z restricted to the classes under X
  or Y, the classes that keep some compatibility with the other side, and
  `exempted_beyond_the_exceptions` names those that are not exceptions or below one, which is
  that cost listed class by class. A test pins `Vehicle ⊓ Building` as satisfiable after the
  compile so the cost cannot change silently.
- **Two exceptions are disjoint from each other unless a class sits below both.** That follows
  from the reading: nothing licensed is below `Houseboat ⊓ MobileHome`, so the compile emits
  `Houseboat ⊥ MobileHome`. A class meant to be both is declared by filing it under both, which
  puts it below an exception.
- **New classes need a re-run.** A class filed under X or Y after the compile gets no pair until
  `compile-exceptions` runs again. A re-run adds pairs and never retracts one, so a pair emitted
  for a class that has since moved stays until removed by hand; `defects` after a re-run shows
  any such leftover as an `inherited_disjoint` or `disjoint_with_ancestor` row.
- **The compiled file grows linearly outside Z.** One pair per class under either side outside
  Z, plus pairs inside Z, a number quadratic only in the size of Z, which is the exceptions, what
  is below them and their ancestors. Measured on a synthetic ontology with 10,000 classes under
  each side and 10 exceptions, each directly under both sides (debug build, on a machine at a
  load average between 40 and 130): 20,045 pairs emitted, 20,000 against the other side and 45
  between the ten exceptions, 20,048 triples inserted, 3.4 s of CPU for the compile and 3.6 s for
  a re-run that changes nothing, and `defects` on the compiled file reports no finding. The first
  version wrote one `INSERT DATA` per pair and had not finished the same file after five minutes,
  because each update counts the store before and after; the writes are now batched.
- **Only named classes and `owl:disjointWith` are compiled.** An exception on an
  `owl:AllDisjointClasses` member pair, on a disjointness between class expressions, or written
  outside an axiom annotation is reported in `unsupported_annotations` and not acted on.

## What this does not claim

The compile does not decide which classes SHOULD be exceptions. It does not detect the extraction
errors of reason 1; `defects` still does that, before the compile and after it. It does not make
`exceptFor` mean anything to any other OWL tool: a third-party reasoner given the uncompiled file
reads the disjointness strictly, and given the compiled file reads the same classical axioms this
engine does.
