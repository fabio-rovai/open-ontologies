# Tier Transition Guide: Enhanced → Aspirational

You have a NAPH Enhanced-compliant collection. This guide walks through what's needed to reach Aspirational tier.

## What Aspirational adds

Three semantic enrichment layers:

1. **Subject classification** (`naph:depicts`) — what each photograph shows (places, events, themes)
2. **Place authority links** — connection to GeoNames / Wikidata / equivalent gazetteers
3. **Cross-collection links** (`naph:linkedRecord`) — connection to peer institutions' related records

## Effort at scale

For a 100,000-record collection:

| Activity | Per-record effort | Total at 100k | Automation potential |
|---|---|---|---|
| Subject classification (vision-LM) | 0.0002/record | 20 FTE-days for inference + 30 FTE-days human validation | High first-pass, medium QC |
| Place authority linkage | 0.0001/record | 10 FTE-days | Full automation, residual conflict resolution |
| Cross-collection record matching | 0.0003/record | 30 FTE-days | High via embedding similarity |
| Event linkage (Wikidata) | 0.00005/record | 5 FTE-days | Full once authority lists curated |
| QA + drift monitoring | 12 FTE-days one-time + ongoing | | Medium |
| Total | | ~107 FTE-days | mostly automatable |

At ~£700/day, Aspirational tier costs roughly **£75,000** for 100k records — but most of this is automation + QA, scaling sub-linearly with collection size.

## Sequencing

### Phase 1 — Place authority bootstrap (week 1-2)

For each unique geographic location in your collection:

1. Identify candidate Wikidata QID via search
2. Verify the QID actually points to the right entity (the red-team failure mode)
3. Generate `naph:Place` records linking to QIDs

Most aerial collections cover a small number of distinct places per geographic region. The places list is shorter than the records list — typically a few hundred unique places for tens of thousands of records.

Tools:

- [OpenRefine](https://openrefine.org/) with Wikidata reconciliation service
- Wikidata SPARQL endpoint for batch lookup
- [Pelias](https://pelias.io/) or [Nominatim](https://nominatim.org/) for OpenStreetMap-based geocoding

### Phase 2 — Subject classification (week 3-12)

For each record:

1. Run vision-language model (Claude Vision, GPT-4 Vision, Gemini, open-source LLaVA)
2. Get subject suggestions with confidence scores
3. Validate sample (~5-10%) by human reviewer
4. Apply classifications to records

Reference implementation: [`pipeline/vision-classify.py`](../../../pipeline/vision-classify.py) (planned v0.4 — spec in [`vlm-pipeline-spec.md`](../../06-knowledge-transfer/vlm-pipeline-spec.md)).

Required structure for each AI-derived classification:

```turtle
ex:photo-X naph:depicts ex:place-Y ;
    naph:placeDerivedBy ex:vlm-classifier-2024 ;
    naph:placeConfidence 0.87 .

ex:vlm-classifier-2024 a prov:SoftwareAgent ;
    rdfs:label "Vision-language model classifier" ;
    naph:modelName "claude-3.5-sonnet-vision" ;
    naph:modelVersion "20241022" .
```

Confidence threshold guidance:

- ≥ 0.95: accept without further review (still document it's AI-derived)
- 0.80-0.94: accept with sampling-based review
- 0.50-0.79: human-validate before accepting
- < 0.50: reject; reclassify or leave un-classified

### Phase 3 — Event linkage (week 13-14)

For records connected to historic events:

1. Identify the relevant Wikidata event QIDs
2. Verify accuracy
3. Apply via `naph:relatedToEvent`

Common categories for aerial reconnaissance:

- WW2 operations and battles (e.g. Operation Hydra → Q3399170)
- Atomic bombings (Hiroshima → Q703203)
- Specific reconnaissance missions where named (rare)

### Phase 4 — Cross-collection matching (week 15-18)

For each record, identify peer-institution records that:

- Cover the same location at the same time
- Document the same event from a different angle
- Are part of an authoritative monument or event record (Canmore, Pastscape, IWM catalogue)

Tooling for scale:

- Embedding-based similarity (sentence-transformers, multimodal embeddings)
- Federated SPARQL queries against peer institutions
- Manual review for high-research-value subsets

Apply via `naph:linkedRecord` URIs.

### Phase 5 — QA + drift monitoring (ongoing)

Aspirational tier requires:

- 10% stratified sample human review per audit cycle
- External link health monitoring (Wikidata QIDs do get deprecated)
- Computational reuse tests passing as part of validation
- Test fixture set of 50+ records with known expected outcomes

Set up scheduled jobs for these checks.

## The Wikidata QID minefield

The single biggest red-team finding was fabricated Wikidata QIDs — IDs that pointed to wrong entities (Q11461 = "Sound", not the atomic bombing). **Verify every QID before using it.**

Verification process:

1. Visit https://www.wikidata.org/wiki/Q[NUMBER]
2. Confirm the entity matches your intent
3. Check the entity's labels, descriptions, instance-of types
4. For Aspirational tier, run a verification SPARQL query on Wikidata to confirm

For high-volume verification, use the Wikidata API:

```python
import requests

def verify_qid(qid: str, expected_label_substring: str) -> bool:
    r = requests.get(
        f"https://www.wikidata.org/wiki/Special:EntityData/{qid}.json"
    )
    data = r.json()
    labels = data.get("entities", {}).get(qid, {}).get("labels", {})
    en = labels.get("en", {}).get("value", "")
    return expected_label_substring.lower() in en.lower()
```

## Common challenges

### Confidence scores aren't well-calibrated

Vision-language model confidences are not always reliable. A "0.87 confidence" for "this is Edinburgh Castle" may be more or less reliable than the same score for "this is a residential area" — calibration varies by class.

Mitigation:

- Validate sample across confidence buckets
- Track per-class precision/recall in validation
- Adjust per-class confidence thresholds

### Place granularity is variable

A photograph of central Edinburgh might be:

- "Edinburgh" (city level)
- "Edinburgh Old Town" (district level)
- "Edinburgh Castle and Royal Mile" (specific location)

NAPH allows multiple `naph:depicts` links per record at different granularities. Use `skos:broader` to express containment between places.

### AI gets places wrong

Even high-confidence classifications can be wrong. The 5-10% human validation rate is the baseline; for high-stakes records (research-significant subsets), validate at higher rates.

If a class of records is repeatedly mis-classified, that's a signal to:

- Re-train / re-prompt the classifier
- Skip that class for AI classification
- Hand-curate that subset

### Cross-collection links break

URLs at peer institutions move. Aspirational tier requires `naph:linkedRecord` URIs to resolve.

Set up automated link-health monitoring:

```bash
# Scheduled task — weekly link health check
python3 pipeline/check-link-health.py data/sample-photographs.ttl > reports/link-health.json
```

Broken links should be either repaired (find the new URL) or marked deprecated.

## Validation checklist

Before claiming Aspirational tier for any record:

- [ ] At least one `naph:depicts` link
- [ ] Each `naph:Place` has `naph:placeAuthorityURI` (verified to resolve to the correct entity)
- [ ] At least one `naph:linkedRecord` cross-reference (verified to resolve)
- [ ] AI-derived fields have `naph:placeDerivedBy` + `naph:placeConfidence`
- [ ] Where AI confidence < 0.95, human validation event recorded
- [ ] SHACL validation passes for Aspirational shape

## Pitfalls

### Inventing Wikidata QIDs

Worst-case credibility failure. Always verify.

### Over-reliance on AI without validation

A collection where 95% of records are AI-classified without any human validation is technically Aspirational but practically suspect. Researchers will distrust the data.

Maintain a meaningful human validation rate even at scale.

### Place authority drift

GeoNames, Wikidata, and other authorities update their data. A QID's referent can change subtly (renaming, splitting). Periodic re-verification is necessary.

## Next steps

After Aspirational tier:

1. Update [compliance registry submission](../../../registry/compliance-declaration-template.ttl) — your collection now claims `tierAspirationalCount > 0`
2. Set up SPARQL endpoint federation participation (see [federation playbook](../../06-knowledge-transfer/federation-playbook/README.md))
3. Publish IIIF Image API service for content-based retrieval
4. Consider research partnerships using your Aspirational tier subset as a foundation

## Cross-references

- [Tutorial 3 — Reaching Aspirational tier](../tutorials/03-aspirational-tier.md) — single-record walkthrough
- [Module B §B.2.3 — Aspirational](../../01-standard/modules/B-metadata-data-structures.md#b23-aspirational-b-aspirational) — specification
- [Module E §E.6 — AI-derived fields](../../01-standard/modules/E-paradata-workflow.md#e6-ai-derived-fields) — provenance for AI outputs
- [Federation playbook](../../06-knowledge-transfer/federation-playbook/README.md)
- [Cost & effort analysis](../../docs/cost-effort-analysis.md)
