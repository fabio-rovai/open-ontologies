# LUBM results

Measured on the official LUBM data generator (UBA 1.7, Lehigh University),
single node, Apple M-series laptop, release build, in-memory store.

These numbers exist so that claims about this engine can be checked rather
than believed, including the ones that do not flatter it. The reproduction
steps are at the bottom; disagreement is welcome and cheap to settle.

## Load and materialisation

| Dataset | Files | Triples | Load | Rate | `owl-rl-ext` | Inferred |
|---|---|---|---|---|---|---|
| LUBM(1) | 15 | 100,866 | 0.2 s | ~649k triples/s | 0.3 s | +48,583 |
| LUBM(10) | 189 | 1,273,246 | 2.0 s | ~625k triples/s | 2.9 s | +472,155 |
| LUBM(100) | 2,007 | **13,409,688** | 25.0 s | ~537k triples/s | 95.0 s | +6,400,920 |

Loading stays close to linear across two orders of magnitude, from 649k to
537k triples per second, so parsing and insertion are not where this engine
falls over.

Materialisation is where the cost lives, and it is superlinear: 0.3 s at
100k triples, 2.9 s at 1.3M, 95 s at 13.4M. The fixpoint is recomputed from
scratch over a growing triple set, which is exactly the case for incremental
reasoning, and the honest reading of this table is that full
re-materialisation is comfortable to a few million triples and painful
beyond it.

A store holding 19.8M triples after inference, single node, no tuning, is
not a toy. It is also not a claim about billions, which remains unmeasured.

## The 14 queries, LUBM(1), after `owl-rl-ext` materialisation

`expected` is the published answer count for LUBM(1) under complete OWL
inference.

| Query | Expected | Ours | |
|---|---|---|---|
| Q1 | 4 | 4 | correct |
| Q2 | 0 | 0 | correct (genuinely empty at LUBM(1), one university) |
| Q3 | 6 | 6 | correct |
| Q4 | 34 | 34 | correct |
| Q5 | 719 | 719 | correct |
| Q6 | 7,790 | 7,790 | correct |
| Q7 | 67 | 67 | correct |
| Q8 | 7,790 | 7,790 | correct |
| Q9 | 208 | 208 | correct |
| Q10 | 4 | 4 | correct |
| Q11 | 224 | 224 | correct |
| Q12 | 15 | 15 | correct |
| Q13 | 1 | 1 | correct |
| Q14 | 5,916 | 5,916 | correct |

**14 of 14 complete**, in 0.3 s of materialisation over 100,866 triples
(+48,583 inferred).

### The profile matters, and choosing it wrongly looks like a reasoner gap

The first run of this suite used `owl-rl` and returned **8 of 14**. Six
queries came back incomplete, and they shared a cause: LUBM defines two
classes by equivalence to an existential restriction,

```
Student ≡ Person ⊓ ∃takesCourse.Course
Chair   ≡ Person ⊓ ∃headOf.Department
```

and recognising a `GraduateStudent` as a `Student` therefore needs
reasoning over `someValuesFrom`, which is outside the OWL-RL profile.

That is a real limit of `owl-rl`, and it is not a limit of the engine:
`owl-rl-ext` implements `someValuesFrom`, `allValuesFrom`, `hasValue`,
intersection and union, and returns all 14 exactly. The lesson worth
recording is about measurement rather than reasoning. An incomplete answer
set looks identical whether the reasoner cannot do the work or was simply
not asked to, and the first version of this file published the wrong
conclusion for exactly that reason.

| Profile | Q6 | Q12 | Inferred | Time |
|---|---|---|---|---|
| `owl-rl` | 5,916 | 0 | +37,942 | 0.2 s |
| `owl-rl-ext` | 7,790 | 15 | +48,583 | 0.3 s |

Use `owl-rl-ext` when the ontology defines classes by restriction, which
any ontology with genuine OWL semantics will.

## Incremental reasoning

The superlinear materialisation above is what `onto_reason_incremental`
answers. Against the LUBM(10) store (1,881,501 triples after full
materialisation), adding facts and deriving their consequences:

| Added facts | Incremental | Full re-materialisation |
|---|---|---|
| 1 | < 10 ms | 2.8 s |
| 50 | < 10 ms | 2.8 s |
| 500 | < 10 ms | 2.8 s |

The cost tracks the delta rather than the store, which is the whole point:
the same 500 facts against LUBM(100) would still be milliseconds where full
re-materialisation is 95 s.

Correctness was checked rather than assumed. Adding twenty typed
individuals and reasoning incrementally gives the same answers as adding
them and re-materialising from scratch: `AssistantProfessor` yields
`Professor`, `Faculty` and `Employee` by either route.

The first implementation of this was slower than full materialisation, 3.6 s
against 2.7 s, because it read the whole store into memory to build its
indexes. Reading 1.9M triples dwarfs the work a delta implies. The rewrite
fetches only the schema, which is a few thousand triples, and joins
everything else on demand. Worth recording: an incremental algorithm that
touches everything is not incremental.

## Query latency, warm store

`query_latency.py` loads once over HTTP, materialises, warms each query,
then times 25 runs. Median and p95 are reported rather than a mean, because
the tail is what a caller feels. LUBM(1), 149,449 triples after inference.

| Query | Results | Median | p95 |
|---|---|---|---|
| Q1 | 4 | 1.09 ms | 1.15 ms |
| Q2 | 0 | 13.97 ms | 14.93 ms |
| Q3 | 6 | 2.02 ms | 2.06 ms |
| Q4 | 34 | 0.85 ms | 0.91 ms |
| Q5 | 719 | 8.82 ms | 11.75 ms |
| Q6 | 7,790 | 8.04 ms | 9.02 ms |
| Q7 | 67 | 0.44 ms | 0.50 ms |
| Q8 | 7,790 | 56.25 ms | 60.95 ms |
| Q9 | 208 | 19.95 ms | 29.71 ms |
| Q10 | 4 | 2.84 ms | 3.10 ms |
| Q11 | 224 | 0.55 ms | 0.60 ms |
| Q12 | 15 | 0.36 ms | 0.42 ms |
| Q13 | 1 | 0.23 ms | 0.24 ms |
| Q14 | 5,916 | 5.71 ms | 6.46 ms |

Ten of the fourteen answer in under 10 ms and half in under a millisecond.
The two expensive ones are the shape to note: Q8 (56 ms) and Q9 (20 ms) are
multi-join queries over large intermediate results, and they are where a
mature query planner earns its keep. That is a fair place to expect a
commercial store to beat this one.

### Throughput

Eight concurrent clients cycling the full 14-query mix, five seconds:
**124 queries/s**. The mix includes Q8 and Q9, so this is a deliberately
unflattering number: a realistic workload weighted toward the cheap queries
would be far higher. Reported as measured rather than as a best case.

## What benchmarking already fixed

Running this suite immediately found a defect that no unit test had:
`load` set no base IRI, so any RDF/XML document using relative IRIs failed
to parse at all. LUBM's generated data is exactly that shape, and loaded
zero triples before the fix. A document's own location is its base per RFC
3986; it is now set from the file path.

That is the argument for benchmarking in one paragraph: the suite paid for
itself before producing a single performance number.

## Reproducing

```bash
# 1. generator (Java), from the LUBM project
curl -sLO https://swat.cse.lehigh.edu/projects/lubm/uba1.7.zip
unzip -q uba1.7.zip
curl -sLO https://swat.cse.lehigh.edu/onto/univ-bench.owl

# 2. data. The generator writes names containing a literal backslash on
#    non-Windows platforms; move them into place afterwards.
java -cp classes edu.lehigh.swat.bench.uba.Generator \
     -univ 1 -index 0 -seed 0 \
     -onto http://swat.cse.lehigh.edu/onto/univ-bench.owl

# 3. run
python3 run_lubm.py --data data1 --reason --profile owl-rl-ext --runs 3 --out results-lubm1.json

# 4. warm-store latency and throughput
python3 query_latency.py --data data1 --runs 25 --clients 8 --seconds 5
```

## Not yet measured

- LUBM(1000) and beyond: 13.4M triples is measured, billions are not
- BSBM, SP2Bench, WatDiv: query optimisation under other shapes
- Any comparison against another store on the same hardware

Until those exist, this file says what it says and no more.
