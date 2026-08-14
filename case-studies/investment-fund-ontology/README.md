# Case study: the Investment Fund Ontology's governance rules, executed as written

**Ontology under test:** the [Investment Fund Ontology (IFO)](https://github.com/fabio-rovai/investment-fund-ontology), an open OWL 2 ontology + SKOS identifier-scheme registry + SHACL governance layer for registered investment fund data, built against the full public US fund universe: 1,292,541 triples joined from the SEC series/class register, four quarters of Form N-CEN, and GLEIF's 9.1M-pair ISIN-LEI mapping. The five Turtle files here are vendored from IFO at v0.2.0 (same author, MIT/CC BY 4.0).

**The question this case study answers:** IFO ships its layer-3 business rules as standard SHACL-SPARQL (`ifo-rules.ttl`) "for platforms that execute them natively", but its own reference pipeline does not execute them as SPARQL — it re-implements each rule set-based in Python, because rdflib could not be trusted to run them at 1.3M triples. Is that workaround actually necessary, or just an artifact of the store used? Can the rules run *as written* on Open Ontologies' Oxigraph-backed engine?

**Answer: the workaround is rdflib-specific.** The plain SPARQL inside the rules' `sh:select` bodies runs unmodified on this repo's engine, at full graph scale, in single-digit seconds, and reproduces the IFO reference pipeline's published counts exactly.

## The A/B, measured on the same graph, same machine, same SPARQL text

Full 1,292,541-triple `fund_graph.ttl` (IFO build of 14 Aug 2026), Apple silicon laptop. rdflib 7.6.0 (the library IFO's pipeline uses) vs this repo's `open-ontologies` binary (Oxigraph 0.5). Per-query timeout 300s.

| Step | rdflib 7.6.0 | Open Ontologies |
|---|---|---|
| Load 1.29M triples | 18.0s | 1.8s |
| R1 — ETF share class with no listing (`FILTER NOT EXISTS` anti-join) | 0.1s — 73 findings | 73 findings |
| R2 — conflicting LEI values on one fund (multi-way self-join) | **timeout, >300s** | 0 conflicts |
| R3 — fund reports its registrant's LEI (5-pattern join) | **timeout, >300s** | 214 funds |
| R5 — observed classes exceed N-CEN reported count (nested aggregate) | 0.8s — 2,148 funds | 2,148 funds |
| **Load + all four rules, one process** | **>10 minutes, 2 of 4 rules never complete** | **2.1s** |

Two things worth being precise about, because the obvious summary ("rdflib can't do anti-joins at scale") is wrong:

1. **rdflib's failure mode is the multi-way self-join, not the anti-join.** R1's `NOT EXISTS` and R5's nested `GROUP BY` both run fine in rdflib (0.1s, 0.8s). What it cannot finish is R2 and R3 — the rules that join `ifo:identifiedBy` against itself across typed identifier nodes. Those are exactly the cross-source reconciliation rules that motivate IFO's reified-identifier design, so "run the reconciliation rules" is the one thing the reference store cannot do declaratively.
2. **The counts are the verification, not just the speed.** All four counts match the numbers IFO's set-based Python pipeline published in its governance report (73 / 0 / 214 / 2,148) on the same build of the graph. Same constraints, two independent executors, identical results. (One wording nit surfaced by the replication: IFO's R1 "73" is 73 fund-class findings across **71** distinct funds.) These figures are pinned to the 14 Aug 2026 build; IFO fetches two of its three sources as "latest", so a rebuilt graph will produce different totals.

## What the demo runs

```
./run-demo.sh
```

**Part 1 — self-contained** (vendored files only, no data download): loads `ifo-core.ttl` + `identifier-schemes.ttl` + the 1,883-triple Vanguard example subgraph, then

- queries the SKOS registry for identifier schemes grouped by declared scope (entity- / issue- / venue-scoped) — IFO's load-bearing design idea, readable as data;
- reproduces IFO finding #4 in miniature: 49 tickers quoted in public SEC data, 0 ISIN assertions from any open source — including the Vanguard 500 ETF share class (VOO), which has a real ISIN in commercial data that GLEIF's open file simply does not carry;
- counts identifier assertions per source system (cross-system disagreement as a query, not an audit project);
- validates the subgraph against IFO's layer-1/2 shapes: 37 findings, all the Warning-graded "quotation with no resolvable trading venue" gap, correctly reported as `Warning` per the shape's own `sh:severity`.

**Part 2 — full universe** (runs when `fund_graph.ttl` is present, else prints how to build it): loads the 1.29M-triple graph and executes the four layer-3 rules as plain SPARQL, printing counts next to the pipeline's published figures.

To build the full graph: clone IFO, run `scripts/fetch_data.sh` then `pipeline/build_graph.py`, and point `IFO_GRAPH` at `data/build/fund_graph.ttl` (default location is `~/projects/investment-fund-ontology/data/build/fund_graph.ttl`).

## What this case study forced the validator to fix

Running a real, externally-authored shapes file through `onto_shacl` is a better test than any synthetic fixture, and IFO's shapes immediately exposed two defects in this repo's SHACL engine, both fixed as part of this case study:

1. **`sh:inversePath` support.** IFO's registrant shape uses `sh:path [ sh:inversePath ifo:fundOf ]` ("a registrant must have at least one fund series pointing at it"). The validator previously injected the blank node into a generated SPARQL query and aborted the *entire* validation with a parse error. It now translates `sh:inversePath` to SPARQL's `^` operator; any other complex path (sequence, alternative, zero-or-more) is skipped and reported under `skipped_constraints` instead of poisoning the run.
2. **`sh:severity` honored.** The report hardcoded every finding as `Violation`. IFO grades its venue-gap shape `sh:Warning` with a message that explicitly says "not an error in the record" — reporting it as a Violation would misstate the ontology's own semantics. The validator now reads `sh:severity` off the property shape, defaulting to `Violation` per the SHACL spec.

Regression tests for both are in `tests/shacl_test.rs`.

## Files

| File | Vendored from IFO | Role |
|---|---|---|
| `ifo-core.ttl` | `ontology/ifo-core.ttl` | Core OWL: `FundRegistrant > Fund > ShareClass > Listing` + reified identifier model |
| `identifier-schemes.ttl` | `skos/identifier-schemes.ttl` | 20-scheme SKOS registry with scope levels, syntax patterns, check algorithms |
| `ifo-shapes.ttl` | `shapes/ifo-shapes.ttl` | Layer-1/2 SHACL: syntax, checksum policy, hierarchy |
| `ifo-rules.ttl` | `shapes/ifo-rules.ttl` | Layer-3 SHACL-SPARQL business rules (the `sh:select` bodies Part 2 executes) |
| `vanguard-index-funds.ttl` | `examples/vanguard-index-funds.ttl` | 1,883-triple real example subgraph (Vanguard Index Funds registrant) |
