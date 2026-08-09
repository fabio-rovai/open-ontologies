# Spectrum → CIDOC-CRM mapping (as applied to MoDiP)

UK museums catalogue to the **Spectrum** standard (Collections Trust). The Museum
Data Service exposes those fields as typed units (`spectrum/…`). To publish the
records as a proper linked-data graph they must be mapped to **CIDOC-CRM**
(ISO 21127) — the reference ontology for cultural heritage — rather than a
home-grown schema. This is the crosswalk `src/build_graph.py` implements. It is
Linked Art compatible (same CRM classes and property paths).

The subject `?obj` is a `crm:E22_Human-Made_Object`.

| Spectrum unit (MDS) | CIDOC-CRM path | Range class | Notes |
|---|---|---|---|
| `object_number` | `?obj crm:P1_is_identified_by ?id` | `E42_Identifier` | content on `P190_has_symbolic_content` |
| `title` | `?obj crm:P102_has_title ?t` | `E35_Title` | content on `P190` |
| `brief_description` | `?obj crm:P3_has_note` | literal | also parsed for cross-references (see DAG) |
| `material` | `?obj crm:P45_consists_of ?c` | `E57_Material` (SKOS concept) | string → concept via the materials taxonomy |
| `object_name` | `?obj crm:P2_has_type ?c` | `E55_Type` | controlled object-name list (`objectnames.ttl`) |
| `associated_concept` | `?obj crm:P2_has_type ?c` | `E55_Type` | use-domain taxonomy (`domains.ttl`) |
| `colour` | `?obj crm:P2_has_type ?c` | `E55_Type` | colour list (`colours.ttl`) |
| `dimension` | `?obj crm:P43_has_dimension ?d` | `E54_Dimension` | parsed to `P2_has_type` (kind) + `P90_has_value` + `P91_has_unit` |
| `inscription_content` | `?obj crm:P128_carries ?i` | `E34_Inscription` | content on `P190` |
| `inscription_method` | `?i crm:P32_used_general_technique ?c` | `E55_Type` | process taxonomy |
| `inscription_position` | `?i crm:P3_has_note` | literal | prefixed `position:` |
| `object_production_organisation` | `?prod crm:P14_carried_out_by ?a` | `E74_Group` | via the production event |
| `object_production_person` | `?prod crm:P14_carried_out_by ?a` | `E21_Person` | via the production event |
| `organisations_/persons_association` | `?prod crm:P3_has_note` | literal | role, e.g. `Manufacturer: Hoover` |
| `technique` | `?prod crm:P32_used_general_technique ?c` | `E55_Type` | process taxonomy |
| `object_production_place` | `?prod crm:P7_took_place_at ?pl` | `E53_Place` | |
| `object_production_date` | `?prod crm:P4_has_time-span ?ts` | `E52_Time-Span` | 4-digit year → `P82_at_some_time_within`^^`xsd:gYear` |
| `license_url` | `?obj dct:license ?url` | IRI | per-record rights, retained verbatim |
| (constant) | `?obj crm:P50_has_current_keeper / P52_has_current_owner` | `E74_Group` | MoDiP |

The production event itself is a `crm:E12_Production` reached from the object by
`crm:P108i_was_produced_by`. Grouping all making-related fields (maker, technique,
place, date) under one event is the CRM idiom and is what makes the data queryable
as provenance rather than as flat columns.

### Variant / same-mould DAG

`brief_description` frequently names another object's accession number in prose.
Those references are resolved against the identifier index and emitted as
object-to-object edges:

- "same … different colourway / version / variant" → `crm:P130_shows_features_of`
- other explicit references → `crm:P67_refers_to`

This turns relationships that were readable only by a human reading the caption
into a graph a machine can traverse.

### Namespaces

| prefix | URI |
|---|---|
| `crm:` | `http://www.cidoc-crm.org/cidoc-crm/` |
| `skos:` | `http://www.w3.org/2004/02/skos/core#` |
| `dct:` | `http://purl.org/dc/terms/` |
| `aat:` | `http://vocab.getty.edu/aat/` |
| `mat:` `proc:` `dom:` (this project) | `https://ontology.tesseract.academy/modip/{materials,processes,domains}/` |

Only the concept-scheme and instance URIs are minted locally; every class and
property above is a published open standard.
