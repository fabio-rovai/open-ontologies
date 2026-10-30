# Schema Alignment

Detect `owl:equivalentClass`, `skos:exactMatch`, `rdfs:subClassOf` candidates between two ontologies, two SKOS vocabularies, or one of each.

## What is aligned

| Entity | Names | Parents | Shown to the reviewer |
| ------ | ----- | ------- | --------------------- |
| `owl:Class` | `rdfs:label`, `skos:prefLabel`, `skos:altLabel`, OBO exact and related synonyms | `rdfs:subClassOf` to a named class | labels, parents, `skos:definition` |
| `skos:Concept` that is not also an `owl:Class` | `rdfs:label`, `skos:prefLabel`, `skos:altLabel`, `skos:hiddenLabel` | `skos:broader`, `skos:broaderTransitive`, `rdfs:subClassOf` | labels, parents, `skos:definition` |

An entity typed both `owl:Class` and `skos:Concept` is read once, as a class, so an OWL ontology aligns exactly as it did before SKOS support. Definitions are shown to the reviewer in the borderline `context` block and never scored: string similarity over prose measures phrasing, not meaning.

## Signals

| Signal | Weight | What it measures |
| ------ | ------ | ---------------- |
| Label similarity | 0.20 | Best of Jaro-Winkler and token Jaccard on normalized labels (camelCase split, lowercased) |
| Property overlap | 0.15 | Domain properties and their ranges |
| Parent overlap | 0.12 | Parents (see the table above) |
| Instance overlap | 0.12 | Shared individuals by local name |
| Restriction similarity | 0.12 | OWL restriction signatures (property->filler) |
| Neighborhood similarity | 0.09 | 2-hop property neighborhood |
| Embedding similarity | 0.20 | Cosine similarity on text embeddings (requires `onto_embed`) |

When compiled without the `embeddings` feature, alignment uses the first 6 signals with the weights (0.25, 0.20, 0.15, 0.15, 0.15, 0.10). Use `onto_align_feedback` to accept or reject candidates; feedback is stored in SQLite and recalibrates the weights once ten rows carry signal values.

## How the signals become one confidence

`--scoring informative` (the default) takes a weighted mean over the signals that **count** for the pair, and leaves every other signal out of both the numerator and the denominator:

- The name always counts. Every entity has one, if only its IRI's local name.
- The embedding similarity counts when both sides have a vector. A cosine is a measured distance, so a low one is evidence against.
- A structural signal counts only as **support**, at 1.0, when the two sides share at least one feature that the other vocabulary can name. A parent, range or restriction filler is recognised across vocabularies by IRI local name or by a shared normalized label, because two independently built vocabularies almost never share local names (`MA_0000003` against `NCI_C12345`).

A structural feature the other side has no name for cannot be compared at all, and is left out. A shared feature that is missing is not evidence that the terms differ: under the open-world reading the other side did not say it, ontologies differ in granularity, and multiple inheritance gives one term different direct parents in different places. Counting such a zero as disagreement cost 0.076 F1 on OAEI Anatomy at threshold 0.94 (0.757 against 0.833). Counting a partial overlap at its Jaccard value would make one shared parent out of three score lower than none. Binary support is monotone: more agreement can only raise the confidence.

Each candidate says what its confidence rests on:

```json
"confidence": 1.0,
"informative_signals": ["label_similarity"],
"evidence_coverage": 0.25
```

`evidence_coverage` is the share of the total weight held by the signals that counted. A 1.0 that rests on a name alone reads as exactly that. The top level carries `"scoring": "informative_renormalised"` and a `scoring_means` sentence.

One weakness is stated rather than hidden: siblings share parents, so a weak name under a shared parent gains confidence it may not deserve ("Cat" and "Bat" under "Mammal"). The learned weights are the place to correct that. Reject such pairs with `onto_align_feedback` and the parent weight falls.

### The legacy fusion, kept for the published figures

`--scoring legacy` reproduces the fusion every published alignment figure in this repository was measured with: `label_similarity x 0.85` when every structural signal is zero, otherwise the weighted sum over **all** signals, including those with no data on either side. Two consequences are why it is no longer the default:

- A perfect name match between two flat lists cannot score above 0.85.
- Agreement costs confidence. The same name with the same parent scored `0.25 + 0.15 = 0.40`, under half of what the same pair scored with the parent deleted.

The OAEI Anatomy result (P 0.960, R 0.730, F1 0.829, 1,152 correspondences, set hash `469e0e04cb58`) reproduces byte for byte with `--scoring legacy --min-confidence 0.80`.

### Thresholds

The legacy fusion compresses every name-only confidence by 0.85, so the same threshold means different things on the two scales: a legacy threshold `t` corresponds to `t / 0.85` under the default. The default auto-apply threshold follows the scoring so that auto-apply stays as strict as it was: **0.95** under `informative`, **0.85** under `legacy`. Under legacy, 0.85 auto-applied a pair with no structural evidence only for an exact name.

Measured on OAEI Anatomy, one-to-one, no background knowledge:

| Scoring | Threshold | Precision | Recall | F1 | Correspondences |
| ------- | --------: | --------: | -----: | -: | --------------: |
| legacy | 0.80 | 0.960 | 0.730 | **0.829** | 1,152 |
| informative | 0.80 | 0.600 | 0.811 | 0.690 | 2,051 |
| informative | 0.85 | 0.698 | 0.808 | 0.749 | 1,756 |
| informative | 0.90 | 0.850 | 0.791 | 0.819 | 1,411 |
| informative | 0.94 | 0.946 | 0.745 | **0.833** | 1,194 |
| informative | 0.95 (default) | 0.957 | 0.732 | **0.829** | 1,159 |
| informative | 0.97 | 0.969 | 0.708 | 0.819 | 1,108 |

0.94 is `0.80 / 0.85`, the threshold equivalent to the published run, chosen by that arithmetic and not by the sweep.

## One-to-one, many-to-one, and ties

`--cardinality one-to-one` (the default) keeps the best target for each source and the best source for each target. That constraint is what lifted Anatomy precision, and it is wrong whenever the source is **finer** than the target: several source codes that each correctly map to one coarser target code lose all but one row. `--cardinality many-to-one` keeps the best target for every source and lets targets repeat.

When a row is dropped because a kept row scored the same, within 0.001 (one unit of the printed precision), the kept row won on IRI spelling and not on evidence. The tie is never resolved silently:

```json
"ambiguous": true,
"tied_with": [{"source_iri": "...", "target_iri": "...", "confidence": 1.0}],
"requires_review": true
```

An ambiguous row goes to the borderline bucket and is never written into the graph, because the engine did not choose it. The top level carries `ambiguous_count`. Sharing a target under many-to-one is not a tie.

## CLI Usage

```bash
# Compare two vocabularies (dry run); default scoring and threshold
open-ontologies align source.ttl target.ttl --dry-run

# A detailed source list onto a coarser SKOS taxonomy
open-ontologies align source.ttl taxonomy.ttl --cardinality many-to-one --dry-run

# Reproduce a published OAEI figure
open-ontologies align mouse.owl human.owl --scoring legacy --min-confidence 0.80 --dry-run

# Accept a candidate
open-ontologies align-feedback --source http://ex.org/Dog --target http://other.org/Canine --accept
```

Over MCP, `onto_align` takes the same choices as `scoring` and `cardinality`.

## Checking a crosswalk after it is written

Alignment proposes rows. Two batch verbs check rows that already exist, from an SSSOM TSV:

- `crosswalk-certify SOURCE TARGET MAPPINGS.tsv [--sssom OUT.tsv]` compares the position of each subject and object in their hierarchies, through the mapping. For a SKOS scheme the hierarchy is the `skos:broader` closure. A downgrade is a refutation. An upheld row means only that nothing visible refutes it: two siblings under one mapped parent look identical, so the predicate column never strengthens a claim. An ancestor with no counterpart through the mapping is a coverage gap, named on the row and never counted as a disagreement, and a row with nothing to compare is `not_refuted_nothing_compared` and not certified.
- `crosswalk-probe SOURCE TARGET MAPPINGS.tsv` (also a CLI subcommand) carries the source's subsumptions into the target through the mapping and reports any clash the carried claims add, such as a `cax-dw` disjointness violation. One witness individual is typed into each receiving class so that a TBox-only taxonomy can clash at all, and clashes the target has on its own are reported under `target_alone` rather than blamed on the mapping. It exits 1 when the mapping adds a clash.
