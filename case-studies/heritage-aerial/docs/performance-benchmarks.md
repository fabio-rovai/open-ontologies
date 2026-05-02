# Performance Benchmarks

Real performance numbers measured against the NAPH v1.0 toolchain on a single M-series Mac (no parallelisation, no caching).

All measurements via `time.perf_counter()` median of 3-5 iterations. Tested on 2026-05-01.

## Reference dataset (10 records)

| Operation | Median time | Notes |
|---|---|---|
| Validate ontology syntax | **24.2 ms** | 198 triples |
| Validate shapes syntax | **24.1 ms** | 83 triples |
| Validate sample data | **24.4 ms** | 313 triples |
| Lint ontology | **25.4 ms** | 0 issues |
| Full batch (clear+load+SHACL+2Q) | **26.7 ms** | end-to-end pipeline |
| RDFS reasoning | **26.4 ms** | produces 69 inferred triples |
| All 7 competency queries | **25.7 ms** | tier counts, WWII, rights, etc |
| CSV ingest (10 rows) | **41.7 ms** | input → valid TTL |
| IIIF bridge (10 manifests) | **126.6 ms** | NAPH → IIIF Presentation 3.0 |
| Self-assessment full report | **60.3 ms** | runs full pipeline + summary |
| Validation HTML report | **59.1 ms** | full pipeline + render |
| Footprint-from-flight (1 calc) | **39.8 ms** | mostly Python startup |
| Stereo-pair-detector (10 records) | **53.2 ms** | full collection scan |

**Headline:** every operation against the reference dataset completes in under 130 ms. The slowest is IIIF bridge generation, which makes 11 SPARQL calls (one per record + setup).

## Scalability — synthetic dataset 100 → 100,000 records

| Records | Ingest | Validate | SHACL | Triples | File |
|---|---|---|---|---|---|
| 100 | 57 ms | 52 ms | 49 ms | 2,614 | 136 KB |
| 1,000 | 102 ms | 42 ms | 70 ms | 26,014 | 1.3 MB |
| 10,000 | 651 ms | 231 ms | 526 ms | 260,014 | 13 MB |
| 100,000 | 6,480 ms | 1,742 ms | 5,532 ms | 2,600,014 | 135 MB |

### Throughput at 100k records

- **CSV ingest:** ~15,400 records/sec (~65 µs/record)
- **Turtle validation:** ~57,400 records/sec (~17 µs/record)
- **SHACL validation:** ~18,000 records/sec (~55 µs/record)

### At 100k records, single full-pipeline run:

- Load ontology + load 100k records + stats + 2 SPARQL queries: **4.2 seconds**
- Peak memory: **1.17 GB**

Memory scales linearly with triple count — each loaded triple uses ~480 bytes in the in-memory Oxigraph store.

## Projection to NCAP-realistic scale

NCAP holds ~30M records. Linear projection from 100k benchmarks:

| Operation | Projected time at 30M records | Projected memory |
|---|---|---|
| CSV ingest | ~32 min | streaming, low RAM |
| Validate TTL syntax | ~9 min | streaming, low RAM |
| Load into triple store | ~17 min | ~350 GB (won't fit single machine) |
| SHACL validation (full collection) | ~28 min | ~350 GB (won't fit single machine) |

**Implication:** at NCAP-full-collection scale, validation needs to be partitioned. Realistic strategies:

- **Partition by sortie or collection** — validate each collection separately
- **Stream-validate** — process records in batches without holding the whole collection in memory
- **Distributed validation** — multiple workers each handle a partition
- **Incremental validation** — only validate what changed since last run (CI-driven)

For institutional-scale (100k records typical of a single sub-collection), the toolchain runs comfortably on a single laptop or server.

## Comparison with comparable workflows

For context, comparable processing of 100k aerial-photography records:

| Workflow | Approximate time |
|---|---|
| **NAPH ingest + validate + SHACL** | ~14 seconds |
| Manual catalogue-system ingestion (typical CMS bulk import) | hours-to-days |
| Bespoke ETL pipeline development time (one-off) | weeks |
| Adobe Bridge metadata ingestion | hours |
| Cataloguing one record manually (mid-volume) | 5-30 minutes per record |

The NAPH toolchain is fast because it's purpose-built for the operation: pure RDF transformations, no UI, no general-purpose CMS overhead, no DBMS round-trip.

## Bottlenecks

### Where time is spent

For small datasets (<1k records):

- Python interpreter startup: ~30 ms (dominates self-assessment, IIIF bridge, footprint script)
- Open Ontologies CLI startup: ~15-20 ms
- Actual work: <10 ms

For larger datasets (>10k records):

- Turtle parsing: linear with file size
- Triple insertion into Oxigraph: linear with triple count
- SHACL constraint evaluation: linear with shape × target node count

### Optimisation potential

Easy wins (not yet implemented):

- **Pre-loaded persistent server** — avoid 30ms startup by running `open-ontologies serve` and using HTTP API instead of CLI. Savings: ~25-30 ms per operation.
- **Parallel SHACL evaluation** — independent shapes can validate concurrently. Savings: 2-4× speedup at large scale.
- **Incremental validation** — only re-validate records that changed. Critical for production CI.
- **Query plan caching** — repeated competency questions could be planned once. Savings: ~10-30 ms per query.
- **Streaming Turtle parser** — for ingest of multi-GB files. Avoids holding the full TTL in memory.

Hard wins (would require Oxigraph internal changes):

- GeoSPARQL spatial function support (currently missing in Oxigraph)
- Optimised joins for federated queries

## Memory profile

For the in-memory triple store backing Open Ontologies (Oxigraph):

| Triples | Resident memory | Bytes/triple |
|---|---|---|
| 511 (sample) | ~50 MB | ~95 KB (dominated by base interpreter) |
| 26k (1000 records) | ~80 MB | ~3 KB (still includes base) |
| 260k (10k records) | ~270 MB | ~1 KB |
| 2.6M (100k records) | 1.17 GB | ~480 bytes |

The asymptote is around 480 bytes per triple, dominated by string interning of IRIs.

For very large collections, persistent storage backends (RocksDB, Sled) trade memory for disk I/O — not currently used by Open Ontologies but available in the underlying Oxigraph library.

## Reproducing these benchmarks

```bash
cd case-studies/heritage-aerial
bash /tmp/naph-bench.sh             # Full benchmark suite (5 minutes)
python3 /tmp/gen-large-csv.py 1000 > test.csv  # Generate synthetic data
python3 pipeline/ingest.py test.csv > test.ttl
```

Hardware tested on:

- Apple Silicon M-series Mac (laptop)
- Single thread / no parallelisation
- macOS 14.x
- Python 3.11
- Open Ontologies CLI (Rust, native binary)

## Operational implications

### For a single-collection institution (100k records)

- Full validation runs in <10 seconds — can be part of every commit's CI
- Full HTML report generation in <2 seconds
- Memory footprint fits on a developer laptop
- No special infrastructure required

### For a multi-collection institution (1M+ records)

- Partition validation by collection or sub-collection
- Run full collection validation as a scheduled job (weekly/monthly)
- Per-PR validation only validates changed records
- Single server with 16-32 GB RAM sufficient

### For a national-archive scale (10M-30M records)

- Distributed validation infrastructure required
- Streaming ingestion pipeline (avoid loading full collection)
- SHACL partitioning (validate per-sortie chunks)
- Fault-tolerant: a single chunk failure should not block other chunks

This is the natural infrastructure level where N-RICH shared services would add the most value.

## Cross-references

- [Cost & effort analysis](cost-effort-analysis.md) — costs are in FTE-days, not compute time
- [`pipeline/`](../pipeline/) — all benchmarked tools
- [Federation playbook](../deliverables/06-knowledge-transfer/federation-playbook/README.md) — distributed query model
