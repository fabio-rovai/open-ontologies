# BUILD_REPORT: bio-kg-triage

Honest record of what was fetched, what was computed, and the limits of the claim. Every number
in `results/` is produced by `src/pipeline.py`; none is hand-entered.

## What was fetched (real, public)

| Source | Endpoint | Use |
|---|---|---|
| Biolink Model | raw.githubusercontent.com/biolink/biolink-model (`biolink_model.yaml`, ~518 KB) | declared vocabulary (classes + slots) = the closed set |
| Open Targets Platform | api.platform.opentargets.org/api/v4/graphql (live) | structured gene-disease associations + scores |
| PubTator3 (NLM) | ncbi.nlm.nih.gov/research/pubtator3-api (live) | literature-extracted gene-disease relations |
| CARD / ARO | purl.obolibrary.org/obo/aro.obo (~76 k lines) | AMR vocabulary + real "confers resistance to" relationships |
| CARD data | card.mcmaster.ca/latest/data (card.json, 4.6 MB tar) | resistance-gene to organism (NCBI taxid) links |
| NCBI taxonomy | ftp.ncbi.nlm.nih.gov/pub/taxonomy/taxdump.tar.gz (75 MB) | current valid taxids (nodes.dmp) + retired ids (merged.dmp) |

Fetched 2026-07-20. Targets queried: EGFR, TP53, KRAS, BRCA1, PTEN, BRAF, ALK, MYC (8 real human
genes by Ensembl id); top 5 associated diseases each -> 40 associations.

## What was computed

1. **Biolink vocabulary extraction.** Classes and slots from the model YAML, mapped to their
   `biolink:` IRIs (`https://w3id.org/biolink/vocab/` + PascalCase for classes, + snake_case for
   slots). 868 declared terms. This IRI derivation is documented here because the closed-world
   check depends on it; the grounded KG uses only terms present in this derived set, and the
   fabricated term is confirmed absent from it.
2. **Grounded KG.** Each association becomes: `<gene> a biolink:Gene ; rdfs:label ... ;
   biolink:gene_associated_with_condition <disease>` and `<disease> a biolink:Disease ; rdfs:label`.
   Gene IRIs are Ensembl (identifiers.org), disease IRIs are MONDO/EFO (OBO / identifiers.org).
   Provenance (Open Targets score + source) is attached in a non-policed `ex:` namespace, so it is
   data, not a Biolink-vocabulary claim.
3. **Ungrounded variant.** Identical, but the real predicate `gene_associated_with_condition` is
   replaced by `associated_with_disease`, a plausible Biolink-looking slot that is NOT in the
   declared set (confirmed programmatically). This is the ungrounded-extraction failure mode.
4. **Validation.** SHACL (pySHACL, a realistic non-closed Gene shape requiring a label) and the
   closed-world gate (every `biolink:` predicate and `rdf:type` value must be declared).

## Literature layer (`src/pubtator.py`)

For each of the 8 target genes we query PubTator3 for PMIDs, export the BioC-JSON, and keep only
its machine-extracted **relations** whose two roles are a Gene and a Disease (types Association,
Positive/Negative_Correlation, Cause). Each becomes a Biolink `gene_associated_with_condition`
edge (gene by NCBI Gene id, disease by MeSH id). Result: 40 PMIDs, 40 annotated documents, 57
unique gene-disease relations, grounded KG 169 triples with **0** closed-world violations; the
ungrounded twin is caught. All 8 targets carry at least one literature edge.

## AMR layer (`src/amr.py`)

We parse `aro.obo`, take the declared ARO terms as the closed set (8,564 terms), and extract the
real `confers_resistance_to_antibiotic` and `confers_resistance_to_drug_class` relationships
(5,053 total; a deterministic sorted slice of 800 is used for a tidy artifact, logged not hidden).
Each becomes a `resistance-determinant -> drug` edge with ARO IRIs. The gate polices the ARO
namespace. Grounded KG 1,283 triples, **0** violations; the ungrounded twin points one edge at a
fabricated `ARO_9999999` and the gate rejects it. This shows the gate generalising to a third
biomedical ontology.

## AMR pathogen linkage (`src/pathogen.py`)

CARD's `card.json` records, for each resistance determinant, the organisms its reference sequences
come from (NCBI taxid). We build 6,404 `gene in_taxon organism` edges over 740 organisms and police
three namespaces at once: ARO (the gene) against ARO's terms, Biolink (the `in_taxon` predicate and
the `Gene`/`OrganismTaxon` types) against Biolink, and NCBITaxon (the organism) against the current
NCBI taxonomy (2,871,791 taxids parsed from `nodes.dmp`). SHACL conforms. The gate catches an
injected fabricated taxid (`NCBITaxon_99999999`). Separately, run against the current taxonomy, it
flags 17 organism ids in CARD as not current; cross-checking `merged.dmp` confirms all 17 are
retired-and-merged taxa (0 unexplained), so this is a data-freshness catch, not fabrication, and it
is a real signal SHACL and a well-formedness check both miss.

## The structured result, precisely

- Grounded KG (284 triples): SHACL `conforms=true`, **0** closed-world violations.
- Ungrounded KG (284 triples): SHACL `conforms=true`, **1** closed-world violation
  (`biolink:associated_with_disease`). SHACL cannot see it; the gate rejects it.
- Triage: the 40 associations ranked by Open Targets score, each row a validated Biolink-typed
  triple with provenance.

## Limits of the claim (do not overstate)

1. **The resistance-to-drug layer uses a slice.** It builds from a deterministic 800-edge slice of
   ARO's 5,053 resistance relationships (logged, not silent); the pathogen layer uses all 6,404
   gene-organism links. The 17 retired taxon ids the pathogen gate flags are real deprecated NCBI
   ids CARD still carries, reported as a finding, not smoothed over.
2. **The triage score is Open Targets'.** We did not build a scoring model. The contribution is
   that ranking on a *validated* graph cannot surface a fabricated edge; the score itself is a
   real external silver-truth signal, presented with provenance.
3. **IRI derivation is ours.** Biolink IRIs are reconstructed from names by a documented rule
   rather than read from a published IRI list, so the closed set is only as correct as that rule.
   The terms actually used (Gene, Disease, gene_associated_with_condition) were spot-checked against
   the model; a fuller run should validate the derivation against Biolink's own JSON-LD context.
4. **Grounded, not harvested.** The KG is built from a structured API, not from raw model output.
   Replaying the shipped `fabsssss/qwen3-coder-30b-a3b-bio` model's generated edges through this
   same gate is the honest next experiment.

## Reproducibility

`./run-demo.sh` re-fetches Biolink and re-queries Open Targets, so exact association scores may
shift as Open Targets updates; the structural result (grounded = 0 violations, ungrounded caught)
is stable. rdflib, pyshacl, pyyaml, requests at build time.
