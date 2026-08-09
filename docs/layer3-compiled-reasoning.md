# Compiled claim verification: design, guarantees, measurements

**Module:** `src/claimcheck.rs` · **Compile step:** `benchmark/reasoner/CompileOntology.java`
**Verification harness:** `benchmark/layer3-prototype/` · **Status:** measured 2026-07-27

## The problem shape

A verification gate for machine-generated claims has a workload unlike ontology
engineering: one **fixed** ontology known in advance, a **stream** of small
candidate claims (a handful of type and relation assertions each), and a hard
latency budget per verdict. General-purpose DL reasoning pays its cost per
query; this workload wants the cost paid once.

> Keep the ontology as the **specification**. Run a complete reasoner **once,
> offline**. Compile its conclusions into flat indexed structures. At query
> time, do **no reasoning at all** — only lookups. Pin the two paths together
> with a parity test against an independent reasoner.

## Architecture

### Offline compile — one classification pass

`CompileOntology.java` classifies the ontology (HermiT today; any complete
reasoner) and emits:

- the **inferred** class hierarchy (reflexive-transitive, including
  equivalences),
- the **asserted** disjointness axioms,
- disjointness pairs **derived** by the propagation rules below,
- unsatisfiable classes.

Incompatibility is represented, not materialised pairwise: the runtime derives
it by a two-hop join,

```
A ⊓ B unsatisfiable  if  ∃ A' ⊒ A, B' ⊒ B  with  disjoint(A', B')
```

which keeps the compiled artifact near-linear in the ontology (Pizza: ~1k rows)
rather than quadratic, and costs one classification instead of n²/2
satisfiability tests.

The compile MUST use a real reasoner: the join is only as good as the hierarchy
it climbs, and classes are routinely subsumed by definitions only inference can
see (Margherita ⊑ VegetarianPizza holds by inference, not assertion).

### Propagation rules (compile-time, run to fixpoint)

Each rule is a sound inference specialised to one modelling idiom observed in
real ontologies; each lands derived pairs in the same compiled surface with
zero hot-path cost:

```
R1  A ⊑ ∃R.C,  B ⊑ ∀R.(D1⊔…⊔Dn),  disj*(C, every Di)      ⟹  disj(A,B)
R2  functional(R),  A ⊑ ∃R.C,  B ⊑ ∃R.D,  disj*(C,D)       ⟹  disj(A,B)
RU  B ⊑ (D1 ⊔ … ⊔ Dn),  disj*(A, every Di)                 ⟹  disj(A,B)
RD  A ⊑ hasValue(p,v1), B ⊑ hasValue(p,v2), v1 ≠ v2,
    and functional(p) or an ancestor of A or B carries ≤1 p ⟹  disj(A,B)
```

Soundness guards worth noting: RU requires every union operand named (deriving
from a subset would claim a tighter constraint than the axiom states); RD
claims literal distinctness only for same-datatype string/boolean literals,
because lexically-distinct numerics can be value-equal ("1" vs "01"). The
fixpoint lets rules compose — e.g. R2 derives topping-level pairs from a
functional property that R1 then lifts to pizza level through closure axioms,
and category-level RU pairs are lifted by the runtime join to all subclasses.

### Runtime — token bitsets

IRIs are interned to `u32` at index build. Each disjointness axiom
`disjoint(X_k, Y_k)` gets a token `k`; each class `C` gets two bitsets over
tokens, `L(C) = {k : X_k ⊒ C}` and `R(C) = {k : Y_k ⊒ C}`. The join becomes

```
A ⊓ B unsatisfiable  ⟺  L(A) ∧ R(B) ≠ ∅  ∨  R(A) ∧ L(B) ≠ ∅
```

— two bitwise ANDs over ⌈m/64⌉ words (Pizza: 447 axioms → 7 words), no
allocation, no store access, no string comparison on the hot path. The first
set bit is the **witness axiom**, so every rejection names the exact clashing
disjointness pair.

Batches fan out over rayon against the immutable `Arc`-shared index. Locks are
acquired once per batch, not per claim: at sub-microsecond per-claim work,
per-claim lock acquisition costs more than the work itself (measured: per-claim
locking made the parallel path 3x slower than sequential; per-batch locking
scales 3.6–4.3x).

### The verdict contract

The compiled surface is **sound but deliberately incomplete**, and the API
makes that impossible to ignore:

- `Rejected` — a contradiction was derived; trustworthy, with a witness.
- `Undetermined` — nothing fired, which is NOT evidence of consistency. The
  undecided class pairs are returned as `residual_pairs`.
- `Consistent` — only reachable after the residual is discharged by tier 2
  (`check_with_oracle`) or explicitly confirmed.

Tier 2 (`ResidualOracle`) consults a real reasoner only for pairs the join
leaves open; verdicts are cached back (incompatibilities as disjointness pairs,
so tier 1 settles them thereafter; a tier-1 rejection never spends an oracle
call). An oracle that cannot decide propagates `Undetermined`, never
`Consistent`. Closed-world vocabulary checks (`declared_class`/`declared_prop`
anti-membership) fire before anything else — open-world OWL semantics cannot
flag an invented term, and for LLM-generated claims that is the most common
failure.

## Measurements

All on Apple M3 Max, canonical `pizza.owl` (1,944 triples, 99 classes) unless
stated, verdicts cross-checked against HermiT 1.4.3.456.

### Latency and throughput

| Per-claim check | median | p95 |
| --- | --- | --- |
| HermiT, warm JVM, amortised load | 4,936 µs | — |
| compiled, Oxigraph-probe variant | 35.4 µs | 61.5 µs |
| **compiled, token bitsets** | **0.3 µs** | **0.4 µs** |

Throughput: 3.1M claims/s single-threaded, 11.2M/s batched.

Substrate note: for store-backed point lookups, Oxigraph's `quads_for_pattern`
measured 165x faster than parameterised SQL in DuckDB on identical data —
point lookup is a triple store's home turf, analytic scans are a columnar
engine's. The final hot path uses neither: interned arrays beat both once the
data is static.

### Correctness

Audited against exhaustive HermiT satisfiability matrices (every named class
pair) and adversarial structural claims:

| | result |
| --- | --- |
| unsound rejections, 78,884 pairs, 13 ontologies | **0** |
| agreement, 793 structural claims, 8 ontologies | **100%**, 0 false negatives |
| contradiction recall, pizza.owl | **3,944/3,944 (100%)** |
| contradiction recall, ore_ont_10230 | **232/232 (100%)** |
| tier-2 residual, structural corpus aggregate | 4.9% of claims |

Adversarial claim generation matters: random pair sampling exercises a real
contradiction only ~11% of the time, so `structural_parity.py` derives claims
from the compiled structure — disjoint pairs pushed down to subclasses
(contradictions reachable only by inference), sibling pairs (where
incompleteness would show), and class-plus-own-superclass probes (which must
never be rejected).

### Compile cost

One classification + fixpoint: Pizza ~120 ms. Scales with ontology hardness,
not just size — a 3,539-class ontology with 27k inferred subsumptions took
28.5 s. Build-time cost; budget it per ontology revision.

### Break-even

The speed is amortisation, not faster reasoning: below a few hundred claims
per ontology, calling HermiT directly is cheaper than compiling. A deployment
verifying a claim stream against a fixed ontology crosses break-even in the
first seconds of operation.

## Envelope — stated, not implied

Outside the compiled surface (and correctly routed to `Undetermined`/tier 2):

- contradiction idioms not covered by R1/R2/RU/RD — observed in the wild:
  one 27-axiom style at 92.1% tier-1 coverage; each new idiom is one small
  sound rule away,
- inverse properties, role hierarchies (∃R vs ∀S with R ⊑ S), nominal
  fillers, numeric literal comparison,
- **ontologies that declare no disjointness at all** — a substantial fraction
  of large real-world ontologies. There the contradiction tier is empty by
  construction and the closed-world vocabulary checks carry the verification
  value. Any deployment claim must be conditioned on the target ontology's
  declared (plus derivable) contradiction surface.

Memory scales as n·m bits (classes × disjointness axioms, two bitsets each):
fine to ~10k classes, needs sparse rows at SNOMED scale.

Structural-parity claim samples vary across compile runs (Java set iteration
order), so per-ontology structural numbers are comparable within a run; the
exhaustive-matrix audit is the stable metric.

## Reproduction

```
# compile an ontology
java -cp ".:lib/*" CompileOntology <ont.owl> compiled.json

# exhaustive ground-truth matrix
java -cp ".:lib/*" DisjointnessMatrix <ont.owl> matrix.csv

# parity + adversarial structural claims
python benchmark/layer3-prototype/structural_parity.py <ont.owl> 120
python benchmark/layer3-prototype/verify_join_soundness.py <corpus_dir> 250

# Rust bench (loads compiled.json)
cargo test --release --test claimcheck_pizza_bench -- --nocapture
```

## Counting rules R4/R5 and the assumed-disjointness WARN tier (2026-07-27)

Two further sound compile rules close the counting idiom (found by reading a
92.1%-coverage ontology's failing axioms — dueling `ExactCardinality` and
`allValuesFrom` restrictions):

```
R4  A ⊑ ≤m R.C,  B ⊑ ≥n R.D,  n > m,  D ⊑* C (or C unqualified) ⟹ disj(A,B)
R5  A ⊑ ∀R.(C…), B ⊑ ∀R.(D…), every Ci disjoint every Dj,
    and A or B forces ≥1 R-successor                              ⟹ disj(A,B)
```

plus `DataExactCardinality(1, p, DataOneOf(v))` recognised as a pinned value
for RD. Effect on the 12-ontology exhaustive audit: tier-2 residual fell from
501 to **23 pairs of 78,884** (aggregate recall 99.83%), still **0 unsound**;
the diagnosed ontology went 96.1% → 99.9%. Pizza and ore_ont_10230 hold at
100%.

### The WARN tier: vetted assumptions for zero-disjointness ontologies

Most large real ontologies declare no disjointness, leaving the entailed
contradiction tier empty. The WARN tier reconstructs a candidate surface under
an explicit assumption semantics:

- a proposer suggests pairs (built-in structural sibling heuristic, or an LLM
  over MCP — the server never embeds one);
- `VetDisjointness` (HermiT) gives each proposal a three-way verdict:
  *entailed* (belongs in the hard tier), *inadmissible* (would break the
  ontology), *admissible* (consistent to assume);
- admissible pairs load via `load_assumed_disjoint` into their own token
  bitsets and fire **warnings, never rejections** — `disjointness_assumed`,
  witness named, explicitly labelled not-entailed. Verdicts are untouched;
  the pair still routes to the residual.

Holdout evaluation (`holdout_disjointness.py`): strip ALL disjointness, let
the sibling heuristic + vetting reconstruct it, score the WARN closure
against the original exhaustive matrix:

| Ontology | axioms stripped | recall | precision |
| --- | --- | --- | --- |
| pizza.owl | 14 | 82.2% | 91.4% |
| ore_ont_11064 | 54 | 44.3% | 92.1% |

Precision is stable ~91–92%; recall depends on how much of the true surface
is sibling-shaped. Measured precision is a **lower bound**: many "false"
warnings (American + IceCream) are false only against entailed truth — they
are exactly the intended-but-never-encoded disjointness the tier exists to
surface. LLM proposals are the open slot for pushing recall past the
structural heuristic.
