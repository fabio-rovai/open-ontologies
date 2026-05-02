# Vision-Language Model Classification Pipeline — Specification

A specification for the VLM-based subject classification pipeline that supports Aspirational-tier adoption at scale. The reference implementation `pipeline/vision-classify.py` is planned for NAPH v0.4.

## Purpose

Aspirational tier requires subject classification (`naph:depicts`) for every record. Manual tagging at the scale of NCAP (30M records) is impossible. The VLM pipeline:

1. Runs vision-language model classification on aerial photograph access copies
2. Produces structured subject suggestions with confidence scores
3. Records full provenance per AI-derived field
4. Outputs NAPH-compliant Turtle ready for human validation

The pipeline does NOT replace human curation. It produces drafts that human curators validate. This is what Module E.A.1 requires.

## Pipeline flow

```
Access surrogate (JPEG/JP2)
    ↓
[1] VLM inference — generate subject suggestions
    ↓
[2] Confidence scoring — per-suggestion
    ↓
[3] Authority resolution — map suggestions to Wikidata/GeoNames QIDs
    ↓
[4] Verification — confirm QIDs point to correct entities
    ↓
[5] Output Turtle — with provenance + confidence
    ↓
[6] Human validation queue — sampled records for review
    ↓
[7] Validation events — recorded as provenance
```

## Step 1 — VLM inference

### Input

- Path to access surrogate file (JPEG/JP2)
- Sortie + capture context (helps disambiguate — a photograph from RAF/Berlin/1944 has different likely subjects than one from RAF/Edinburgh/1947)

### Process

Send the image plus a structured prompt to the VLM:

```text
You are analysing a historic aerial photograph for archival classification.
The photograph was captured by [aircraft] over [region] on [date].

Please identify:

1. Primary visible places (urban centres, named landmarks, geographic features)
2. Visible structures (buildings, infrastructure, military facilities)
3. Land-use categories (urban, agricultural, industrial, military)
4. Approximate scale (can you see individual buildings, roads, fields?)
5. Any unusual or research-significant features

For each identified subject, provide:

- A short label (e.g. "Edinburgh Castle", "industrial district", "harbour")
- A confidence (0.0-1.0)
- A brief rationale for the identification
- Where applicable, a candidate Wikidata QID (verify before use)

Respond in JSON.
```

### Models supported

- **Claude 3.5 Sonnet** (Anthropic) — strong visual reasoning, accessible API
- **GPT-4 Vision** (OpenAI) — established baseline
- **Gemini Pro Vision** (Google) — particularly strong for georeferenced material
- **LLaVA / open-source models** — for institutions requiring on-premises processing

### Output (per record)

```json
{
  "model": "claude-3.5-sonnet-vision",
  "modelVersion": "20241022",
  "inferenceTimestamp": "2024-04-15T14:23:00Z",
  "subjects": [
    {
      "label": "Edinburgh Castle",
      "category": "specific-landmark",
      "confidence": 0.92,
      "rationale": "Clearly visible fortress on volcanic rock; characteristic shape; matches expected location given sortie metadata",
      "candidateQid": "Q212065"
    },
    {
      "label": "Edinburgh Old Town",
      "category": "place",
      "confidence": 0.85,
      "rationale": "Dense pre-modern urban fabric extending east from castle",
      "candidateQid": "Q591133"
    }
  ]
}
```

## Step 2 — Confidence scoring

The VLM provides a confidence score, but these are not reliably calibrated across:

- Models (different VLMs have different scoring characteristics)
- Subjects (place identification is typically more reliable than activity inference)
- Image quality (low-resolution scans get less reliable classifications)

The pipeline normalises confidences into NAPH-relevant buckets:

| Bucket | Threshold | Action |
|---|---|---|
| **High** | ≥ 0.95 | Accept; still document as AI-derived |
| **Medium-high** | 0.85 - 0.94 | Accept with sampling validation |
| **Medium** | 0.70 - 0.84 | Queue for human validation |
| **Low** | 0.50 - 0.69 | Queue for human review; mark as preliminary |
| **Reject** | < 0.50 | Discard; do not include in output |

Bucket thresholds are configurable per institution.

## Step 3 — Authority resolution

For each subject suggested by the VLM:

1. Take the candidate QID (if provided)
2. Verify it resolves and matches expected entity type
3. If candidate QID is invalid, search Wikidata via API for matching label
4. Confirm match by checking entity labels and types
5. Resolve to canonical NAPH place / event / subject IRIs

### Wikidata search query

```python
import requests

def search_wikidata(label: str, expected_type: str = None) -> str | None:
    """Search Wikidata for a place/event matching the label."""
    r = requests.get(
        "https://www.wikidata.org/w/api.php",
        params={
            "action": "wbsearchentities",
            "language": "en",
            "format": "json",
            "search": label,
        },
    )
    results = r.json().get("search", [])
    for result in results:
        qid = result["id"]
        # Verify type if specified
        if expected_type:
            if not check_qid_is_of_type(qid, expected_type):
                continue
        return qid
    return None
```

## Step 4 — Verification

Before applying any QID to a record, verify:

- The QID resolves (200 response from Wikidata)
- The English label matches what the VLM claimed
- The QID's instance-of types are appropriate (e.g. "place" for `naph:Place`, "event" for `naph:HistoricEvent`)
- The QID isn't deprecated (`schema:DeprecatedSchemaItem`)

This verification step catches the red-team failure case (Q11461 was provided as "atomic bombing" but actually refers to Sound).

## Step 5 — Output Turtle

For each record, emit:

```turtle
ex:photo-X naph:depicts ex:place-Y ;
    naph:placeDerivedBy ex:vlm-classifier-2024-10-22 ;
    naph:placeConfidence 0.92 .

ex:place-Y a naph:Place, skos:Concept ;
    rdfs:label "Edinburgh Castle" ;
    naph:placeAuthorityURI <https://www.geonames.org/2650225/edinburgh-castle.html> ;
    skos:exactMatch <https://www.wikidata.org/wiki/Q212065> ;
    naph:placeDerivedBy ex:vlm-classifier-2024-10-22 .

ex:vlm-classifier-2024-10-22 a prov:SoftwareAgent, prov:Activity ;
    rdfs:label "Vision-language classifier" ;
    naph:modelName "claude-3.5-sonnet-vision" ;
    naph:modelVersion "20241022" ;
    naph:promptVersion "v1.0" ;
    prov:atTime "2024-04-15T14:23:00Z"^^xsd:dateTime ;
    prov:used ex:photo-X-thumbnail .
```

## Step 6 — Human validation queue

Records flagged for validation are added to the institution's review queue. Recommended interface:

- Show the access surrogate
- Show the AI-suggested classifications with confidence
- Allow validator to: accept all, reject all, or edit individual suggestions
- Record the validator's identity and timestamp
- Optionally allow free-text notes

Processing: ~30-60 records/hour for a competent reviewer.

## Step 7 — Validation events

Each human validation produces a `prov:Activity`:

```turtle
ex:photo-X-validation-001 a prov:Activity ;
    rdfs:label "Human validation of AI-derived place classification" ;
    prov:atTime "2024-04-22T10:11:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:reviewer-A ;
    prov:used ex:photo-X ;
    naph:validationOutcome "accepted" ;
    naph:validationNote "Edinburgh Castle confirmed; Old Town extent slightly broader than VLM suggested but accepted." .
```

The validation event is what makes the record fully Aspirational-compliant. Records with AI classifications but no validation events are at "preliminary Aspirational" status.

## Performance considerations

### Throughput

- Per-record VLM inference: ~3-15 seconds (depending on model and image size)
- Per-record authority resolution: ~1-3 seconds (Wikidata API calls)
- Per-record verification: ~0.5 seconds
- Total: ~5-20 seconds per record

For a 100,000-record collection: roughly 6-30 days of continuous processing on a single machine.

### Cost

For commercial VLM APIs (April 2026 pricing):

- Claude 3.5 Sonnet: ~$0.003 per image
- GPT-4 Vision: ~$0.01 per image
- Gemini: ~$0.0005 per image

For 100k records: $50-$1,000 depending on model.

### Caching

- VLM results: cache permanently (re-running on the same image with the same prompt produces the same output)
- Wikidata lookups: cache for 7 days
- Verification: re-verify monthly (Wikidata QIDs do get deprecated)

### Privacy / on-premises

For institutions where sending images to third-party APIs is prohibited:

- Use open-source models (LLaVA, Qwen-VL)
- Run on local GPU
- Throughput is lower but data never leaves the institution

## Quality assurance

### Cross-validation

Run two different VLMs (e.g. Claude + Gemini) on the same sample. Where they agree → high confidence. Where they disagree → human review priority.

### Sampling

Track per-VLM, per-class precision and recall against the human-validated subset. Adjust confidence thresholds based on calibration.

### Drift monitoring

Run periodic re-classification of a fixed sample (e.g. 100 records) over time. Track:

- Classification consistency (does the model give the same answer over time?)
- Authority drift (do the QIDs still resolve correctly?)
- New classifications (does an updated model identify subjects the original missed?)

## When NOT to use this pipeline

- **High-stakes legal / forensic records** — human curation should be primary, AI strictly assistive
- **Records with restricted access** — privacy / data-protection rules may prohibit third-party API processing
- **Research-significant subsets** — for the most-cited records, hand-curate; AI is for the long tail

## Future extensions (v0.5+)

- Multi-modal classification combining image + caption text where present
- Audio classification for related oral history (where institution holds both)
- Time-series change detection across overlapping coverage
- Active learning: use validator feedback to improve future classifications

## Cross-references

- [Module B §B.2.3 — Aspirational subject classification](../01-standard/modules/B-metadata-data-structures.md#b23-aspirational-b-aspirational)
- [Module E §E.6 — AI-derived fields](../01-standard/modules/E-paradata-workflow.md#e6-ai-derived-fields)
- [Tier Transition Guide: Enhanced → Aspirational](../04-adoption-guidance/transition-guides/enhanced-to-aspirational.md)
- [SPARQL library — provenance audit Q-PROV-6, Q-PROV-7](../04-adoption-guidance/sparql-library/provenance.sparql)
