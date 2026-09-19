# Where `asserted.tsv` came from

`hqdmTop/hqdmFramework`, path `rdf/hqdm-0.0.1-alpha.ttl`, 79,623 bytes.

MagmaCore vendors the same file at
`examples/src/test/resources/hqdm-0.0.1-alpha.ttl`. The two are **byte
identical**, verified with `cmp`, so the audit is of upstream HQDM and not of
something MagmaCore changed.

`asserted.tsv` is that file with its three prefixes expanded and one triple per
line, subject, predicate, object, tab separated. Nothing is added, removed or
reordered: 1,094 triples in, 1,094 out. It parses as a flat `S P O .` grammar
because the source uses no blank nodes, no lists and no literals.

## This is not the file OM-2026 measured

That paper measures `hqdm.owl` as shipped by GCHQ: 3,127 triples, 229 named
classes, 14 `owl:disjointWith` axioms and 39 natively unsatisfiable classes
under HermiT.

This file contains **no `owl:` term at all** and **no disjointness axiom**. Its
whole predicate inventory is:

    rdf:type            210
    rdfs:subClassOf     372
    rdfs:domain         257
    rdfs:range          255

With no disjointness, no named class can be unsatisfiable, so a coherence check
on this file must return zero however carefully it is run. The two renderings
of HQDM disagree about what HQDM is, and the defect a consumer finds depends on
which one they fetched. Neither file says which is canonical.

## What the audit finds instead

Three things a coherence check cannot see, all recomputed from these rows by
`tests/hqdm_audit_asset_test.rs`:

1. **23 terms used as a class and never declared** `rdf:type rdfs:Class` —
   as a subject or object of `rdfs:subClassOf`, or as the object of
   `rdfs:domain` or `rdfs:range`. `hqdm:thing` *is* declared, at line 1067 of
   the source, so this is not a parsing artefact.
2. **12 `rdfs:range` declarations naming a relation** — 9 at `hqdm:part_of`
   and 3 at `hqdm:participant_in`. Neither is declared a class, and neither
   appears anywhere in the subclass hierarchy in either position, so they are
   not classes left undeclared: they are relation names standing where RDFS
   requires a class.
3. **13 pairs of names one trailing underscore apart.** Ten share a domain and
   differ in range, so they are distinct relations separated by a character
   that carries no meaning and is documented nowhere. Three —
   `contract_process_consists_of`, `offer_and_acceptance_for_goods_consists_of`
   and `sale_of_goods_consists_of` — have identical domain **and** identical
   range, so they are the same relation under two names.
