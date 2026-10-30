# Reasoner Benchmarks

Compare Open Ontologies reasoning against HermiT and Pellet.

## Setup

### Java reasoners

`setup_jars.sh` downloads OWL API 4.5.29, HermiT 1.4.3.456 and Pellet
2.4.0-ignazio into `lib/`. The harness has been run against a `lib/` holding
`owlapi-osgidistribution-4.5.29.jar`, `HermiT-1.4.3.456.jar` and the OWL API's
runtime dependencies (guava, caffeine, rdf4j, jackson, commons-io,
commons-lang3, hppcrt, xz, javax.inject, automaton, slf4j). Pellet is optional:
the Pizza script runs it only when a `pellet-*.jar` or `openllet-*.jar` is
present. Earlier text here asked for OWL API 5.x and `HermiT.jar` from
hermit-reasoner.net; neither matches `JavaReasoner.java`, which is compiled
against OWL API 4.

The scripts compile `JavaReasoner.java` and `Convert.java` themselves into
`target/java/` (ignored by git). No Maven step is needed.

### Python

`run_ore.py` and `compare_results.py` use the standard library only.
`plot_results.py` needs `pip install matplotlib`.

## What the reference is

`JavaReasoner.java` emits, for every named class HermiT finds satisfiable, every
named class it is entailed to be a subclass of: all superclasses with
`direct=false` plus all equivalent classes. `owl:Thing` and `owl:Nothing` never
appear in a pair, and unsatisfiable classes are listed separately under
`unsatisfiable` instead of contributing a pair to every class in the ontology.
The engine's `classification[].superclasses` is also the full set, so the two
sides now count the same thing.

An earlier version emitted `getSuperClasses(cls, true)`, the DIRECT
superclasses. Compared with the engine's full set, every indirect pair the
engine got right looked like an unsound extra. Direct superclasses also never
include equivalent classes: in ORE file `ore_ont_8937`, axioms such as
`SubClassOf(owl:Thing C23)` make `C23` equivalent to `owl:Thing`, so the true
pair `C25 -> C23` was missing from the reference and a correct engine answer
was read as unsound.

`JavaReasoner` reports its times separately: `jvm_start_ms` (JVM boot to
`main`), `load_ms` (parse), `reasoner_create_ms` (HermiT clausifies here),
`classify_ms` (`precomputeInferences` only) and `extract_ms` (reading the
hierarchy back, not reasoning cost). `time_ms` is create plus classify and is
kept for older callers.

## Benchmarks

### Pizza Correctness

Compares the engine's `owl-dl` classification of
`benchmark/reference/pizza-reference.owl` with HermiT's, and with Pellet's when
its jars are present. The file contains six `owl:hasValue` restrictions and one
`owl:oneOf` enumeration of five individuals, so it is a nominal-bearing
ontology. The engine approximates nominals (each `{a}` is read as an atom
standing for it, which is sound and can miss entailments), so a run on this file
reports `complete: false` and names `owl:hasValue` and `owl:oneOf` under
`constructs_not_modelled`, even though its answer here is exactly HermiT's.

```bash
export OO_BIN=./target/release/open-ontologies
bash benchmark/reasoner/run_pizza_correctness.sh
```

The engine runs as `load` then `reason --profile owl-dl` inside one
`batch -` process, because the store is in memory and a second process would
reason over an empty graph. Before anything is compared, the script refuses
the run unless the load line reports `triples_loaded > 0` with no error and the
reason line carries no error and a non-zero `named_classes`. The previous guard
read `classes` or `class_count`, keys the `owl-dl` result does not have, so it
aborted every real run with "reasoned over an empty graph". The guard can be
exercised with `PIZZA_OWL=/path/to/empty.owl`, which it rejects.

`compare_results.py` reads the engine side from `classification` plus
`equivalences` and scores it with the same code as the ORE driver below. It
previously read a `subsumptions` key from the engine result, which the engine
never emits, so it compared HermiT's direct superclasses against an empty set.

`run_benchmark.py` and `results/pizza_comparison.json` predate these fixes.
`run_benchmark.py` still runs `load` and `reason` as separate processes, and
the committed JSON reports `named_classes: 1`, the signature of a reasoner that
saw an empty store. Do not quote either.

### ORE 2015 classification

`run_ore.py` runs the engine and HermiT over files from the ORE 2015
competition corpus and scores the engine against HermiT pair by pair. It
expects the corpus's `pool_sample` directory at `corpora/ore2015/pool_sample`
(ignored by git; pass `--corpus` to point elsewhere) and reads
`<track>/classification/fileorder.txt` for the track (`el`, `dl` or
`pure_dl`).

#### Where the corpus comes from

The page that used to host it is gone: `cs.ox.ac.uk/isg/tools/ORE/ore2015/` and
`owl.cs.manchester.ac.uk/tools/ore2015/` both return 404 and there is no Wayback
snapshot. The corpus itself is published on Zenodo, and until this note was
added nothing in this repository said so, which made the one comparison that
matters unreproducible. The same corpus feeds the `#[ignore]`d
`ore_pool_round_trip` test in `tests/ofn_writer_test.rs` ("needs the ORE 2015
corpus, which is not in the repository"); after the download below, run it with
`OO_ORE_POOL=benchmark/reasoner/corpora/ore2015/pool_sample/files cargo test
--test ofn_writer_test -- --ignored ore_pool_round_trip`.

| | |
|---|---|
| DOI | [`10.5281/zenodo.18578`](https://doi.org/10.5281/zenodo.18578), "ORE 2015 Reasoner Competition Corpus" |
| Download | 725 MB (`ore2015_sample.zip`) |
| Extracted | 21.9 GB, 1,938 files (1,952 zip entries), 1,920 of them ontologies in `pool_sample/files/`; the largest, `ore_ont_12898.owl`, is 591 MB |
| Licence | **CC-BY-NC-ND-4.0**: benchmark use only. No derivatives, no commercial redistribution: the individual ontologies carry their own licences on top |

```bash
mkdir -p benchmark/reasoner/corpora/ore2015
curl -L -o /tmp/ore2015_sample.zip \
  https://zenodo.org/api/records/18578/files/ore2015_sample.zip/content
unzip -q /tmp/ore2015_sample.zip -d benchmark/reasoner/corpora/ore2015/
# the archive contains pool_sample/ at its root
python3 benchmark/reasoner/run_ore.py --selftest
```

`--selftest` needs neither the corpus nor Java: it feeds synthetic engines to the
scoring code and checks 17 cases, including the negative controls (an engine that
lies, a timeout, a failed load). It passes on a machine with no corpus at all, so
a green selftest says the driver is honest and says nothing about the corpus.

#### Known gap: `Convert.java` needs more jars than `setup_jars.sh` fetches

`setup_jars.sh` downloads `owlapi-distribution`, HermiT and Pellet. That is
enough to compile `Convert.java` but not to run it: the OWL API's transitive
dependencies are missing, and the first file of any run dies with
`NoClassDefFoundError`, observed in this order on macOS/arm64 with Java 18:
`org.slf4j.LoggerFactory`, `javax.inject.Inject`, `com.github.benmanes.caffeine.
cache.Caffeine`, `org.eclipse.rdf4j.model.base.AbstractValueFactory`,
`com.carrotsearch.hppcrt.maps.ObjectObjectHashMap` (the last one is `hppcrt`, not
`hppc`). The driver reports each as `conversion_failed` and withholds every
timing, which is the right behaviour but reads like a reasoner failure. Adding
`slf4j-api`, `slf4j-nop`, `javax.inject`, `guice`, `caffeine` 3.x, `rdf4j-model*`
3.7.x, `commons-io` and `hppcrt` to `lib/` clears it. Not fixed here: the correct
fix is to resolve the dependencies from the OWL API POM instead of pinning a list
that goes stale.

```bash
python3 benchmark/reasoner/run_ore.py --selftest
python3 benchmark/reasoner/run_ore.py --track el --sort-by-size --limit 6 \
    --engine ./target/release/open-ontologies
python3 benchmark/reasoner/run_ore.py --track dl --files ore_ont_9695.owl ore_ont_1325.owl
python3 benchmark/reasoner/run_ore.py --track pure_dl --file-list my_files.txt --repeat 5
```

**Same bytes to both reasoners.** ORE files are OWL Functional Syntax, which
the engine does not read yet. Each input is converted once to RDF/XML by the OWL
API (`Convert.java`), cached under `--work-dir`, and the converted file is given
to both the engine and HermiT. `--native` skips the conversion and gives the
original file to both, for when the engine reads Functional Syntax; until then
every `--native` run fails the load gate, which is the intended outcome.

**One engine process per run.** The engine is driven as
`printf 'load <f>\nreason --profile owl-dl\n' | <bin> batch -` with a
wall-clock cap enforced by the driver (`--engine-timeout`, default 120 s), which
kills the process group when it expires. HermiT has its own cap
(`--hermit-timeout`).

**Gates.** No timing is reported for a run whose load line lacks
`triples_loaded` or has it at 0 or carries an error, whose reason line carries
an error or is missing, whose process exits non-zero, or which the cap killed.
Such a file is listed under its failure status with its time withheld.

**Told-closure subtraction.** For every file the driver also asks the OWL API,
with no reasoner, for the TOLD closure: the transitive closure of
`SubClassOf(A, B)` with both sides named, pairwise `EquivalentClasses` among
named members, and the tautological edge from every named class to
`owl:Thing`. Gold minus told is the INFERRED-only set, the pairs that need
reasoning to find. Recall is reported on both, and the headline uses
inferred-only recall. A file whose inferred-only set is empty is flagged
`told_only` and excluded from the headline, because a structural walk of the
asserted axioms scores 100% on it without reasoning at all. This is not a
corner case. An ontology whose logical axioms are all `SubClassOf` with a named
class on the left and a named class or an existential restriction on the right,
the shape of many files in the EL track, entails no named subsumption beyond
the told closure apart from those through unsatisfiable classes, at any size.
Without the subtraction, a told-hierarchy walk and a complete reasoner are
indistinguishable on such files.

**Per file** the driver reports engine wall time (median over repeats, load
plus reason), the engine's own `agents.total_time_ms`, HermiT create and
classify times, gold, told and inferred-only counts, engine pairs, recall on
all and on inferred-only pairs, the WRONG pairs (the engine claims, HermiT
does not entail; listed in full in the JSON), wrong and missed unsatisfiable
classes, the engine's undetermined count and `budget_exhausted_in`, and whether
the engine's answer was identical across `--repeat` runs (default 3). Pairs
whose subclass HermiT finds unsatisfiable are excluded from both sides, since
such a class is a subclass of everything. Engine pairs involving blank nodes
are dropped and counted. A file HermiT finds inconsistent has no hierarchy to
compare; it is reported as `hermit_inconsistent`, together with whether the
engine reported `consistent: true` for it, which counts as a wrong answer. A
file whose engine run fails a gate still shows its gold, told and
inferred-only counts, so the table says what the failure cost, but it carries
no recall and no time. Results go to `results/ore_<track>_<timestamp>.json`
and `.md` (ignored by git).

**Self-test.** `--selftest` drives fake engines through the same code: a
correct one, a structural walk, an empty classification, a fabricated wrong
pair, a false unsatisfiability claim, an empty load, a load error, a missing
`triples_loaded`, a reason error, a crash, a hang past a 1 s cap, an engine
whose answer changes between runs, an engine that calls an inconsistent
ontology consistent, and a told-only file. Each check asserts
the driver reports the defect, and the command exits non-zero if any does not.

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

### Corrected comparison, July

Measured properly on the canonical `pizza.owl` (1,944 triples, 99 classes, 218
individuals), same task, same machine, HermiT 1.4.3.456 on JDK 17:

| | HermiT | Open Ontologies |
| --- | --- | --- |
| Wall time | **170 ms** | 10.9 s (budget expiry, not completion) |
| Inferred subsumptions | **311** | **0** |
| Outcome | complete | incomplete, 143 undetermined |

The pair counts in this table predate the full-hierarchy reference described
above and are not comparable with what the current harness reports.

The claim was not merely unsupported. It was inverted.

### What changed, 13 October

The engine did not finish on Pizza or on LUBM for five reasons, each found by
running it against HermiT and each fixed in `src/tableaux.rs`:

1. Every inclusion with a non-atomic left-hand side was a disjunction on every
   node. **Absorption** now turns them into domains, `∀r⁻` rules and binary
   rules, which is what made the nominals repro below terminate.
2. Search backtracked chronologically. **Dependency-directed backjumping**
   skips any choice a clash does not depend on.
3. Classification tested every ordered pair of classes (19,394 tableaux on
   Pizza). It now **reads subsumers off each class's model** and tests only the
   candidates a nondeterministic model leaves (231 on Pizza).
4. Disjointness was checked only after a branch finished, never fired against
   `owl:Thing`, and n-ary `DisjointClasses`, `DisjointUnion` and
   `owl:equivalentProperty` were not parsed at all.
5. Only an ancestor could block, so long EL chains grew exponentially.
   **Anywhere blocking** expands each pairwise signature once.

Pizza against HermiT 1.4.3, both on one machine under heavy load:

| | HermiT | Open Ontologies |
| --- | --- | --- |
| Subsumptions (with equivalences) | 500 | **500**, none wrong |
| Unsatisfiable classes | 2 | **2**, the same two |
| Wall time | 1.8 s | 2.6 s |
| `complete` | | `false`, because nominals are approximated |

`run_ore.py` over the 40 smallest files of each ORE 2015 track (120 files, up
to 4 MB): **no wrong subsumption and no wrong unsatisfiability claim** on any
file; recall on inferred-only pairs 100% (el), 79% (dl) and 48% (pure_dl).
Every pair the engine misses is either on a file whose run reports
`budget_exhausted_in` or on one that names the construct it approximated
(nominals, datatype restrictions) under `constructs_not_modelled`, so no
`complete: true` run misses an answer. These are correctness figures. **No
speed claim against any Java reasoner should be made from this repository**:
HermiT's 1.8 s includes JVM start-up, the machine was loaded, and the pure_dl
track shows the engine still runs out of budget on files HermiT classifies in
under a second.

For a workload the compiled `claimcheck` module *is* suited to, see the
claim-verification benchmark in [`docs/benchmarks.md`](../../docs/benchmarks.md),
which is task-matched and audited against HermiT for agreement rather than speed
alone. That module is a library benchmark. No MCP tool and no CLI subcommand
calls it, so it is not part of the shipped server or binary.
