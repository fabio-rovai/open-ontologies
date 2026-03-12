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
