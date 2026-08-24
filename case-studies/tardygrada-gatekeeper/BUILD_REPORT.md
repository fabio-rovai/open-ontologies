# BUILD_REPORT: tardygrada-gatekeeper

Honest record of the model, the method and the limits. Every number in `results/` is produced by
`src/gatekeeper.py` (Python standard library only, deterministic).

## The model

- **State:** (position of robot A, position of robot B, shared battery budget). Positions are cells
  0..8 on a 3x3 grid; the battery is an integer 0..6.
- **Actions:** each robot picks a move in {stay, N, S, E, W}; a joint action is a pair. Grid-illegal
  moves (off the edge) are ill-formed. Battery drops by the number of non-stay moves.
- **Safety specification (three invariants):** the two robots never occupy the same cell; neither
  robot enters the hazard cell (the centre); the battery budget is never negative.
- **Certificate:** the proposer's claimed successor state plus an implicit "this is safe" claim.

## The trusted core (measured)

`gatekeeper_admits` plus the functions it calls (`successor`, `cell_ok_move`, `satisfies_spec`) are
the only trusted code, 31 non-comment source lines. Everything else, the proposer, the state
enumerator, the metrics harness, is untrusted: a bug there cannot cause an unsafe action to be
dispatched, because the gate re-derives the outcome and re-checks the spec itself.

## The method

1. **Reachability.** From a safe start state, breadth-first over only gatekeeper-admitted actions,
   giving the set of states the runtime can actually reach (96 states).
2. **Exhaustive verification.** For every reachable state and every joint action, compare the
   gatekeeper's admit/block decision (on an honest certificate) against whether the true successor
   satisfies the specification. Soundness = it never admits an action whose successor is unsafe;
   completeness = it never blocks an action whose successor is safe. Both hold across all 1,176
   checked transitions, so this is a complete result over the reachable space, not a sample.
3. **Adversarial proposer.** A proposer that always claims safety: the gate re-checks and blocks
   672/672 of the genuinely-unsafe proposals.
4. **Ungated baseline.** Without the gate, 672/1,176 (57.1%) of transitions land in an unsafe
   state, so the gate is load-bearing, not decorative.
5. **Latency.** Mean wall-clock time of one gatekeeper check over 200,000 calls: ~0.36 microseconds.

## The result, precisely

- Reachable states 96; transitions checked 1,176.
- Unsafe transitions 672, all blocked (100%). Safe transitions 504, all admitted (100%).
- Soundness and completeness both hold over the entire reachable space.
- Adversarial unsafe proposals blocked: 672/672.
- Ungated unsafe rate: 57.1%.
- Checker latency ~0.36 us; trusted core 31 lines.

## Limits of the claim (do not overstate)

1. **Bounded model.** The exhaustive guarantee holds over this bounded system (two robots, 3x3
   grid, battery 0..6). It is a complete result *for that model*, not a proof for unbounded or much
   larger systems. Scaling needs symbolic / SMT model checking; the point this repo makes concrete
   is that the *checker* stays small and fast regardless.
2. **No machine-checked proof of the checker.** The trusted core is 31 lines and simple, but this
   repo verifies the *system* exhaustively, it does not ship a Coq/Lean proof of the checker
   function itself. That is the role of the Tardygrada formally-verified runtime (C core), the
   production vehicle for this gate; this is its reference specification and evidence.
3. **Two real benchmarks, one caveat each.** Beyond the warehouse, the gate runs on Island
   Navigation using the official AI Safety Gridworlds map and safety definition (taken verbatim from
   the repo; the live pycolab engine also loads and steps here, but it cannot teleport to arbitrary
   states, so the exhaustive reachability is done over the same map with a faithful deterministic
   model), and on Commons Harvest, a faithful reimplementation of the Melting Pot substrate's
   regrowth dynamics (dm-meltingpot's dmlab2d/bazel engine does not build in this environment).
   Gating a neural proposer verified with VNN-COMP-style tooling remains the next step.
4. **Deterministic dynamics.** The transition function is deterministic; stochastic dynamics would
   require the certificate and check to range over successor distributions (a documented extension).

## Reproducibility

`./run-demo.sh` reruns everything; pure standard library, so results are identical across machines.
