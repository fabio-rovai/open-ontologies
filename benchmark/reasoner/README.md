# Reasoner Benchmarks

Compare Open Ontologies reasoning against HermiT and Pellet.

## Setup

### Java Reasoners

Download these JARs into `lib/`:

1. **OWL API** (5.x): `owlapi-distribution-5.1.20.jar`
2. **HermiT**: `HermiT.jar` from [hermit-reasoner.net](http://www.hermit-reasoner.net/)
3. **Pellet**: `pellet-cli-2.4.0.jar` and dependencies from [Pellet releases](https://github.com/stardog-union/pellet/releases)

### Python

```bash
pip install matplotlib
```

## Benchmarks

### Pizza Correctness

Compares OWL2-DL classification of the Pizza ontology across all three reasoners:

```bash
export OO_BIN=./target/release/open-ontologies
bash benchmark/reasoner/run_pizza_correctness.sh
```

### LUBM Performance

Generates university ontologies at increasing scale and measures classification time:

```bash
export OO_BIN=./target/release/open-ontologies
bash benchmark/reasoner/run_lubm_performance.sh
```

Results are saved to `benchmark/reasoner/results/`.

## Findings, and one withdrawn claim

### The LUBM speed claim is withdrawn

Earlier versions of this repository, and `arXiv:2605.09184v1`, reported OWL-RL
forward chaining at 14-15 ms against HermiT's 112-24,490 ms on LUBM, a ratio of
**1,633x** at 50,000 axioms. **That claim is withdrawn. It was measured on an
empty store.**

Three faults stacked in `run_lubm_performance.sh`:

1. `generate_lubm.py` wrote Turtle into a file with an `.owl` extension, and
   format detection keyed on the extension and chose an RDF/XML parser, so every
   load failed.
2. `load` and `reason` ran as separate processes. Process setup builds a fresh
   empty store, so `reason` always executed over zero triples.
3. Both failures were routed to `/dev/null` and replaced with a default value.

The published 14-15 ms was process start-up time, and its flatness across a 50x
scale-up was the tell. Fixed by content sniffing in `detect_format_sniffed`, by
running through `batch -` so a single store is shared, and by asserting
`triples > 0` before any timing is trusted.

### Corrected comparison

Measured properly on the canonical `pizza.owl` (1,944 triples, 99 classes, 218
individuals), same task, same machine, HermiT 1.4.3.456 on JDK 17:

| | HermiT | Open Ontologies |
| --- | --- | --- |
| Wall time | **170 ms** | 10.9 s (budget expiry, not completion) |
| Inferred subsumptions | **311** | **0** |
| Outcome | complete | incomplete, 143 undetermined |

The claim was not merely unsupported. It was inverted. **No speed claim against
any Java reasoner should be made from this repository.** The reason our reasoner
does not complete is documented in [`regressions/`](regressions/): it does not
terminate on nominals, and LUBM emits one `owl:hasValue` per department.

For a workload this engine *is* suited to, see the compiled claim-verification
benchmark in [`docs/benchmarks.md`](../../docs/benchmarks.md), which is
task-matched and audited against HermiT for agreement rather than speed alone.
