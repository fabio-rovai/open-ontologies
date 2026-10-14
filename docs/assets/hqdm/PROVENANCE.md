# Where the HQDM inputs came from

HQDM is published as two files that disagree about what HQDM is. The figure
draws both, under the same checks, and every count in it is recomputed from
these inputs by `tests/hqdm_audit_asset_test.rs`.

## `asserted.tsv`: the RDFS rendering

`hqdmTop/hqdmFramework`, path `rdf/hqdm-0.0.1-alpha.ttl`, 79,623 bytes.

MagmaCore vendors the same file at
`examples/src/test/resources/hqdm-0.0.1-alpha.ttl`. The two are **byte
identical**, verified with `cmp`, so the audit is of upstream HQDM and not of
something MagmaCore changed.

`asserted.tsv` is that file with its three prefixes expanded and one triple per
line, subject, predicate, object, tab separated. Nothing is added, removed or
reordered: 1,094 triples in, 1,094 out. It parses as a flat `S P O .` grammar
because the source uses no blank nodes, no lists and no literals. Its whole
predicate inventory is `rdf:type` 210, `rdfs:subClassOf` 372, `rdfs:domain` 257,
`rdfs:range` 255. It contains no `owl:` term and no disjointness axiom, so no
named class in it can be unsatisfiable and a coherence check on it returns zero
however carefully it is run.

## `hqdm.owl` and `owl-rows.tsv`: the OWL rendering

`gchq/HQDM`, file `hqdm.owl`, RDF/XML, 366,581 bytes, Apache-2.0, Crown
Copyright. sha256 `be27ee353e2fefe7…47caff8`, the same file the OM 2026 paper
measured. It is vendored here unchanged so the test below can reason over it.

`owl-rows.tsv` is that file read by rdflib 7.6.0 and written one triple per
line in the same tab-separated form: 3,127 triples in, 3,127 out. Blank nodes
are written `_:label` and literals in double quotes; the well-formedness checks
only read rows whose subject and object are IRIs, so neither matters to them.
It carries 234 `owl:Class` declarations (229 named, the rest anonymous), 14
`owl:disjointWith` axioms and 268 qualified cardinality restrictions.

## `owl-dl.json`: this engine's run over `hqdm.owl`

    open-ontologies batch --data-dir <dir> <<'B'
    load docs/assets/hqdm/hqdm.owl
    reason owl-dl
    B

with `<dir>/config.toml` raising the per-phase budget:

    [reasoner]
    tableaux_test_timeout_ms = 120000
    classify_timeout_ms = 900000

The SHIQ tableaux checks 229 named classes, finds **190 satisfiable** and
**39 unsatisfiable**, and leaves **none undetermined**, in 91 ms. `complete` is
`false` and `search_finished` is `true`: the file has 23 restrictions on datatype
properties, which this engine reads only approximately, and the run names them
under `constructs_not_modelled`. The file keeps the fields the figure reads and
drops the inferred subsumptions. `the_reasoner_still_gives_the_committed_verdicts`
in the test recomputes this run in-process and requires the same two sets.

Regenerated on 2026-10-13. The run committed before that checked 234 classes,
found 195 satisfiable, refuted none and left these 39 undetermined on the
tableaux's expansion cap. Two changes account for the difference. Five of the
234 were anonymous `owl:Class` nodes taken for named classes, which is why the
count is now 229 and the satisfiable count 190. And absorption, backjumping and
the absorption of `owl:disjointWith` as `A ⊓ B ⊑ ⊥` let the search finish on the
39.

## `hermit-unsatisfiable.txt`: the oracle's list

The 39 classes HermiT reports as unsatisfiable in `hqdm.owl`, as recorded by
the OM 2026 crosswalk's reasoning run (`hqdm_prerepair_results.json`,
`deleted_classes`). HermiT is an external reasoner and, in this repository's
vocabulary, an opinion: nothing checks its proof. The figure does not repeat
the paper's 39; it intersects this list with the engine's undetermined set and
prints the size of the intersection with the engine's own unsatisfiable set,
which is all 39: the engine and the oracle name the same classes.

## What the checks find

Three things a coherence check cannot see, recomputed from the rows:

1. **Terms used as a class and never declared** `rdf:type rdfs:Class` or
   `owl:Class`, as a subject or object of `rdfs:subClassOf`, or as the object
   of `rdfs:domain` or `rdfs:range`. A datatype in range position (`xsd:*`,
   `rdfs:Literal`) is correct RDFS and is not counted. RDFS rendering: 23
   (`hqdm:thing` *is* declared, at line 1067 of the source, so this is not a
   parsing artefact). OWL rendering: 0.
2. **`rdfs:range` declarations naming a relation**: terms that are never
   declared a class and never appear in the subclass hierarchy in either
   position, so they are relation names standing where RDFS requires a class.
   RDFS rendering: 12, at `hqdm:part_of` (9) and `hqdm:participant_in` (3).
   OWL rendering: 0.
3. **Pairs of names one trailing underscore apart.** RDFS rendering: 13, ten
   sharing a domain and differing in range, and three,
   `contract_process_consists_of`, `offer_and_acceptance_for_goods_consists_of`
   and `sale_of_goods_consists_of`, identical in domain **and** range. OWL
   rendering: 6, none identical.

And one thing only a reasoner can see, on the file that has enough axioms to
carry it: the 39 classes above.
