# Schema Alignment

Detect `owl:equivalentClass`, `skos:exactMatch`, `rdfs:subClassOf` candidates between two ontologies using 7 weighted signals:

| Signal | Weight | What it measures |
| ------ | ------ | ---------------- |
| Label similarity | 0.20 | Jaro-Winkler on normalized labels (camelCase split, lowercased) |
| Property overlap | 0.15 | Jaccard on domain property + range signatures |
| Parent overlap | 0.12 | Jaccard on rdfs:subClassOf parent local names |
| Instance overlap | 0.12 | Jaccard on shared individuals by local name |
| Restriction similarity | 0.12 | Jaccard on OWL restriction signatures (property->filler) |
| Neighborhood similarity | 0.09 | Jaccard on 2-hop property neighborhood |
| Embedding similarity | 0.20 | Cosine similarity on text embeddings (requires `onto_embed`) |

When compiled without the `embeddings` feature, alignment uses the first 6 signals with the original weights (0.25, 0.20, 0.15, 0.15, 0.15, 0.10).

Candidates above the confidence threshold are auto-applied to the main graph. Use `onto_align_feedback` to accept/reject candidates — feedback is stored in SQLite and used to self-calibrate signal weights over time.

## CLI Usage

```bash
# Compare two ontology files (dry run)
open-ontologies align source.ttl target.ttl --min-confidence 0.7 --dry-run

# Accept a candidate
open-ontologies align-feedback --source http://ex.org/Dog --target http://other.org/Canine --accept
```
