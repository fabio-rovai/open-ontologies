# Results summary: four grounded layers

Every layer builds a KG from a REAL public source, types it with a real ontology, and runs the closed-world vocabulary gate. Grounded KGs validate clean; ungrounded twins (one fabricated term) pass SHACL but are caught by the gate.

| Layer | Source | Ontology policed | Grounded triples | Closed-world result |
|---|---|---|--:|---|
| Structured (target-disease) | Open Targets | Biolink | 284 | 0 violations; twin caught |
| Literature | PubTator3 | Biolink | 169 | 0 violations; twin caught |
| AMR (resistance-to-drug) | CARD / ARO | ARO | 1283 | 0 violations; twin caught |
| AMR pathogen linkage | CARD + NCBI taxonomy | ARO + Biolink + NCBITaxon | 20692 | see below |

- **Structured**: 40 live Open Targets gene-disease associations, Biolink-typed.
- **Literature**: 57 gene-disease relations from PubTator3 across 40 PubMed abstracts; all 8 target genes carry edges.
- **AMR (resistance-to-drug)**: 800 of 5053 real 'confers resistance to' relationships from CARD's ARO (8564 terms).
- **AMR pathogen linkage**: 6404 gene-organism edges over 740 organisms from CARD's card.json, policing THREE namespaces at once (ARO gene, Biolink type/predicate, NCBITaxon organism against 2,871,791 current NCBI taxids). A fabricated taxon id is caught; and, run against the CURRENT taxonomy, the gate flags **17 organism ids in CARD as no longer current** (all 17 confirmed retired-and-merged in NCBI's merged.dmp, 0 unexplained), a real data-freshness signal that open-world SHACL misses.