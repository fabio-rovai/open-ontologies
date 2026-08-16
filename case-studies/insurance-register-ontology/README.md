# Case study: the Insurance Register Ontology's governance rules, executed as written

**Ontology under test:** the [Insurance Register Ontology (IRO)](https://github.com/fabio-rovai/insurance-register-ontology), an open OWL 2 ontology + SKOS registries + SHACL governance layer for the authorization, cross-border operation, and identifier fabric of EEA insurance and reinsurance undertakings, built against the full EIOPA Register of Insurance Undertakings joined with same-day GLEIF API records: a 276,683-triple graph (14 Aug 2026 build). The four Turtle files here are vendored from IRO at that state (same author, CC BY 4.0). Background article: [gov.tesseract.academy/research/insurance-register-ontology](https://gov.tesseract.academy/research/insurance-register-ontology).

**The question this case study answers:** IRO ships its six layer-3 business rules (R1-R6) as standard SHACL-SPARQL "for platforms that execute it natively", but its reference pipeline runs them set-based in Python over the source CSVs. The sibling [IFO case study](../investment-fund-ontology/) established that rdflib times out on multi-way self-joins at 1.29M triples. The insurance register is 4.7 times smaller. So two questions: do the six rules run *as written* on Open Ontologies' Oxigraph-backed engine and reproduce the pipeline's published findings, and at register scale is the set-based workaround still forced, or merely faster?

**Answer: all six rules run as written and the counts agree, and the scale finding is honest rather than dramatic.** At 276,683 triples rdflib completes every rule; nothing times out. What remains is a two-orders-of-magnitude gap on the worst rule (97 seconds versus well under a second for R1's double anti-join) and roughly 133 seconds versus single-digit seconds for the whole suite. Register-scale graphs sit inside the zone where rdflib survives; IFO's 1.29M-triple fund universe is where it stops. The workaround is about scale, not about SPARQL dialect: the identical query text produced identical counts on both engines.

## The A/B, measured on the same graph, same machine, same SPARQL text

Full 276,683-triple `iro_graph.ttl` (IRO build of 14 Aug 2026), Apple silicon laptop carrying unrelated concurrent load (which is why engine wall clock is quoted as a range). rdflib 7.6.0 (the library IRO's pipeline uses) vs this repo's `open-ontologies` binary (Oxigraph 0.5). Every number below was measured for this case study; nothing is copied from the IFO table.

| Step | rdflib 7.6.0 | Open Ontologies |
|---|---|---|
| Load 276,683 triples | 5.6s | 0.8s |
| R1: active undertaking with no LEI (double `FILTER NOT EXISTS`) | 97.0s, 643 findings | 643 findings |
| R2: LEI fails ISO 7064 check digits | 0.0s, 4 | 4 |
| R3: active undertaking, LEI LAPSED in GLEIF | 7.3s, 118 | 118 |
| R4: active undertaking, GLEIF entity INACTIVE | 22.4s, 42 | 42 |
| R5: LEI shared by more than one entity node (self-join) | 0.2s, 20 | 20 |
| R6: open operation for an ended registration (zombie passport) | 0.8s, 291 | 291 |
| **Load + all six rules, one process** | **133.2s** | **2s to 9s across repeated runs** |

Three things worth being precise about:

1. **R1-R4 now have three independent executors agreeing exactly.** IRO's set-based Python pipeline published 643 / 4 / 118 / 42 in its governance report for this build; rdflib's SPARQL engine and Oxigraph both reproduce those counts from the graph. Same constraints, three executors, identical results. These figures are pinned to the 14 Aug 2026 EIOPA export and same-day GLEIF harvest; both are living systems, so a rebuilt graph will differ.
2. **R5 and R6 differ from the published CSV-level numbers, and the difference is entity resolution, not error.** The pipeline's R5 counted 227 LEIs filed for more than one register key; the graph rule finds 20 LEI nodes identifying more than one entity node, because the build deliberately joins branch rows onto their home undertaking (by register key first, then by unambiguous LEI), absorbing the 207 legitimate branch shares before the rule ever runs. All 3 hard entity collapses the report singled out (including the SCOR France / SCOR Ireland pair) are among the 20; this was checked by listing the 20 values. R6 runs the other way: the graph finds 291 zombie passports against the report's 283, and replicating both join disciplines set-based over the same CSV confirms the 8 extra are open operations whose own register key has no domestic row at all, so only the LEI join can attribute them to their ended home registration. The same rule text measures a sharper universe after entity resolution: fewer false shares, more true zombies.
3. **rdflib's pain point here is the anti-join stack, not the self-join.** R5's self-join over 3,630 LEI nodes takes 0.2s; what costs 97s is R1's double `FILTER NOT EXISTS` over undertakings and registrations, with R4 (22.4s) and R3 (7.3s) close behind. At IFO's 1.29M triples the killer was the multi-way self-join. The lesson is that rdflib's failure mode depends on where the graph is big, and the only executor that was indifferent to the question in both case studies is the Oxigraph engine.

## What the demo runs

```
./run-demo.sh
```

**Part 1, self-contained** (vendored files plus a synthetic subgraph, no data download, no EIOPA or GLEIF data): loads `iro-core.ttl` + `operation-modes.ttl` + `iro-example-synthetic.ttl`, then

- reads the SKOS registries as data: the five operation modes grounded in Directive 2009/138/EC with the register's own notations, and the identifier schemes grouped by declared scope (the LEI entity-scoped, the NCA code authority-scoped), which is IRO's load-bearing design idea;
- executes all six layer-3 rules as plain SPARQL against engineered defects with known counts (1 / 2 / 1 / 1 / 1 / 1), each defect a miniature of a defect class the real build surfaced: the letter-O-for-zero LEI transposition, the 19-character truncated LEI, two reinsurers filed under one LEI, the passport outliving its authorization, and the active undertaking with no LEI at all;
- counts identifier assertions per source system (provenance as a query);
- validates the layer-1/2 shapes: 5 findings, 4 Violation and 1 Warning, exercising `sh:pattern`, `sh:hasValue`, `sh:minCount`, `sh:severity`, and the `sh:inversePath` registration shape.

The synthetic subgraph is exactly that and says so in its header: every undertaking, authority, and identifier value in `iro-example-synthetic.ttl` is invented. The "valid" LEI values use a deliberately fake `9999` prefix but carry real ISO 7064 MOD 97-10 check digits, generated and verified with the source repository's own `pipeline/checksums.py`; the two "impossible" values fail that arithmetic on purpose. The letter-O value passes the 20-character syntax pattern (a letter O is alphanumeric) and fails only the checksum, which is why IRO keeps arithmetic in code and policy in shapes.

**Part 2, full EEA register** (runs when `iro_graph.ttl` is present, else prints how to build it): loads the 276,683-triple graph and executes the six rules as plain SPARQL, printing counts next to the pipeline's published figures, then closes with the passporting-fabric query: top exporters of open freedom-of-services operations DE 2,794, IE 1,658, FR 1,280, LU 1,166, NL 1,140, MT 915, matching the governance report exactly.

To build the full graph: clone IRO, run `pipeline/fetch_eiopa.py`, `pipeline/harvest_gleif.py`, and `pipeline/build_graph.py`, and point `IRO_GRAPH` at `data/build/iro_graph.ttl` (default location is `~/projects/insurance-register-ontology/data/build/iro_graph.ttl`). **No EIOPA register data is redistributed by this case study or this repository**; you fetch the export from EIOPA yourself, exactly as the source repository does. GLEIF records are CC0, but none are vendored here either.

## What this case study forced the validator to fix

IRO's layer-2 LEI shape is precisely a `sh:pattern` + `sh:hasValue` pair: 20-character ISO 17442 syntax, and the recorded checksum state must be `true`. This repo's SHACL validator executed neither constraint before this case study; both property shapes were silently skipped, so the shapes file would have reported a clean pass over data containing a 19-character LEI. Both are now implemented as SPARQL translations (`!REGEX` over the string form; `FILTER NOT EXISTS` for the required term), with regression tests in `tests/shacl_test.rs` covering the truncated-LEI and false-checksum cases. The demo's Part 1 SHACL run is the acceptance test: 5 findings with the right constraints and severities, including the Warning-graded unregistered undertaking via `sh:inversePath`.

## Files

| File | Vendored from IRO | Role |
|---|---|---|
| `iro-core.ttl` | `ontology/iro-core.ttl` | Core OWL: `InsuranceUndertaking`, reified `Registration` and `CrossBorderOperation`, `RegisteredPresence`, reified identifier assertions with GLEIF reconciliation facets |
| `operation-modes.ttl` | `skos/operation-modes.ttl` | SKOS registries: 5 operation modes grounded in Directive 2009/138/EC, identifier schemes with scope levels, source systems |
| `iro-shapes.ttl` | `shapes/iro-shapes.ttl` | Layer-1/2 SHACL: syntax, checksum policy, structure |
| `iro-rules.ttl` | `shapes/iro-rules.ttl` | Layer-3 SHACL-SPARQL business rules R1-R6 (the `sh:select` bodies Part 2 executes) |
| `iro-example-synthetic.ttl` | written for this case study | Synthetic example subgraph, engineered defects with known counts; contains no EIOPA or GLEIF data |

Vendored files are CC BY 4.0 (per IRO's LICENSE; ontology, SKOS registries, and shapes carry that licence, pipeline code is MIT). Attribution: [github.com/fabio-rovai/insurance-register-ontology](https://github.com/fabio-rovai/insurance-register-ontology) and the write-up at [gov.tesseract.academy/research/insurance-register-ontology](https://gov.tesseract.academy/research/insurance-register-ontology).
