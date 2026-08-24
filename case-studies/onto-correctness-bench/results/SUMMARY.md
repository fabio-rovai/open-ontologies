# Results summary

Deterministic run of `src/bench.py` (fixed seed). 100 clean + 100 hallucinated record graphs per vocabulary.

| Vocabulary | Ontology triples | Declared classes | Declared props | Fabricated terms injected | **SHACL false-pass rate** | **Closed-world catch rate** | CW false-positive (clean) | CW term recall | SHACL term recall |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| schema.org | 17949 | 933 | 1521 | 136 | 100% | 100% | 0% | 100% | 0% |
| IES4 | 3976 | 510 | 204 | 134 | 100% | 100% | 0% | 100% | 0% |
| OBO (PATO+RO) | 270126 | 2889 | 759 | 148 | 100% | 100% | 0% | 100% | 0% |

**Aggregate:** across 3 real vocabularies, 300 hallucinated graphs carrying 418 fabricated terms — 
open-world SHACL reported `conforms=true` on **300/300** (100%); the closed-world gate flagged a fabricated term in **300/300** (100%) with **0** false positives on clean graphs.