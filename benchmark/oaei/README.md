# OAEI Alignment Benchmark

Evaluates `onto_align` against the [Ontology Alignment Evaluation Initiative (OAEI)](http://oaei.ontologymatching.org/) standard tracks.

## Tracks

### Anatomy (Mouse-Human)

The OAEI Anatomy track aligns the NCI Thesaurus (mouse anatomy, 2,737 classes) with the Foundational Model of Anatomy (human anatomy, 3,304 classes). The reference alignment contains 1,516 equivalence mappings.

This is the most widely reported OAEI track — nearly all alignment systems publish results on it, making it the best basis for comparison.

### Conference

The OAEI Conference track aligns 7 ontologies describing the conference organisation domain (ekaw, sigkdd, iasted, confOf, edas, cmt, conference). 21 pairwise alignments with reference mappings.

## Running

```bash
# Download OAEI data (one-time)
python3 download_oaei.py

# Run alignment benchmark
python3 run_oaei_benchmark.py

# Results appear in results/
```

## Comparison Systems

Results are compared against published OAEI 2023.5 results:

| System | Anatomy F1 | Conference F1 |
|--------|-----------|--------------|
| LogMap | 0.912 | 0.670 |
| AML | 0.936 | 0.680 |
| BERTMap | 0.924 | 0.710 |
| OLaLa | 0.890 | 0.720 |
| **Open Ontologies** | **TBD** | **TBD** |
