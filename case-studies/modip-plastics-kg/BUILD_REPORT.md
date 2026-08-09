# BUILD_REPORT

A scrupulous account of what was fetched, what was computed, and what could not
be obtained. Every number here is reproduced by the scripts in `src/`.

## Source data — what was fetched

- **Provider:** Museum Data Service (MDS) — a joint service of Art UK, Collections
  Trust and the University of Leicester — via its public `extract` API
  (`https://mds-data-2.ciim.k-int.com/api/v1/extract`).
- **Collection:** Museum of Design in Plastics (MoDiP), Arts University Bournemouth.
- **Records fetched:** **11,865** (the complete MoDiP set in MDS at the time of
  the pull), retrieved by token pagination in `src/fetch_modip.py`, 100 records
  per page, 119 pages, `stats.remaining` reaching 0.
- **Licence:** every record carries `CC BY 4.0` in its `License` / `License Url`
  fields. That per-record licence is retained verbatim in the graph
  (`dct:license`), and MoDiP is recorded as `dct:rightsHolder`.
- **API access:** the MDS API token is obtained free from the declaration form on
  any MDS object-search results page. It is supplied to the fetch script via the
  `MDS_TOKEN` environment variable and is **not** committed to this repository.
- The raw records are stored unmodified at `data/raw/modip_records.json` so every
  downstream step is reproducible without re-hitting the API.

## What was computed

| artefact | script | result |
|---|---|---|
| raw-data profile | `profile_data.py` | 476 distinct material strings, 82 techniques, 955 object names; synonymy + cross-reference evidence (`data/PROFILE.md`) |
| materials taxonomy | `materials_taxonomy.py` → `build_taxonomies.py` | 137 SKOS concepts; resolves **99.9%** of 35,172 material assertions (49 unresolved) |
| process taxonomy | `process_taxonomy.py` | 65 concepts; **100%** of 11,482 technique assertions |
| domain taxonomy | `concept_taxonomy.py` | 29 concepts; **100%** of 13,897 associated-concept assertions |
| Getty AAT alignment | `reconcile_getty.py` | **55** concepts given a verified `skos:exactMatch` to AAT (query-reconciled, exact-label only, 6 over-specific matches pruned) |
| CIDOC-CRM instance graph | `build_graph.py` | **485,013** triples over 11,865 `crm:E22_Human-Made_Object`s |
| variant / same-mould DAG | `build_graph.py` | **289** object-to-object edges over 308 objects, 114 components, largest component 12 |
| validation | `validate.py` | SHACL `conforms=True`; closed-world vocab check **0 dangling** concepts; union graph 487,243 triples |

Instance graph composition (selected): 35,123 material→concept links; 11,482
technique links; 12,947 maker links; 7,072 inscriptions; 20,735 dimensions parsed
to value+unit; 955 controlled object-name types; 18 colour types.

## What could NOT be obtained / limitations

- **49 material assertions (0.1%) are unreconciled** — obscure single-occurrence
  trade names (D3O, Delaron, Stamisol, Diaplex, Guardian, …). They are linked to
  `mat:plastic_unidentified` and listed in `build/build_stats.json`
  (`top_unreconciled_materials`) rather than silently dropped or guessed.
- **Getty AAT alignment is partial (55 / 137 concepts).** Only exact-label matches
  from the Getty SPARQL endpoint were kept; abbreviations, trade names and
  grouping concepts without an exact AAT peer are deliberately left unaligned
  rather than mapped approximately. Wikidata alignment was attempted but the
  public endpoint was unreliable from the build host and is left for a later pass.
- **Dates are noisy.** Only production dates containing a 4-digit year yield a
  machine-readable `xsd:gYear` (`P82_at_some_time_within`); other date strings are
  kept as the time-span label only.
- **Actor roles** (Manufacturer / Designer / etc.) are recorded as a note on the
  production event rather than via CRM property-of-property reification, to keep
  the graph readable; the information is preserved, the modelling is deliberately
  simple.
- **Images are out of scope.** Only the textual catalogue records were fetched;
  no media files are included.
- The variant DAG depends on curators having written accession numbers into
  descriptions; it therefore captures the *documented* variant relationships, not
  every latent one. 323 descriptions contain such references; 289 resolved to a
  target object in the set.

## Reproduce

```bash
pip install -r requirements.txt
MDS_TOKEN="<free token>" python3 src/fetch_modip.py   # or use the committed raw file
python3 src/profile_data.py
python3 src/reconcile_getty.py        # optional; alignment is committed
python3 src/build_taxonomies.py
python3 src/build_graph.py
python3 src/validate.py               # release gate: must print RESULT: PASS
```
