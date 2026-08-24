# tardygrada-gatekeeper

**A proof-carrying-action gatekeeper for multi-agent systems, exhaustively model-checked:
every unsafe action blocked, every safe action admitted, verified over the whole reachable
state space, with a 31-line trusted core and sub-microsecond checks.**

The ARIA Safeguarded-AI design centres on a "gatekeeper": an action is permitted only if it
carries a certificate the gatekeeper can check against a safety specification. The hard part is
not writing another verifier, it is having a *runtime* where every action is gated by a check
small enough to trust and fast enough to run in the loop. This is a small, runnable reference
gatekeeper for a bounded multi-agent system, and because the system is bounded we do not sample
its safety, we verify it completely.

This is the certificate-carrying-runtime layer the Encode / ARIA Challengescape items on safe
multi-agent systems, formal verification at scale, accessible formal methods, and continuous
assurance are asking for.

## The result

Deterministic run of [`src/gatekeeper.py`](src/gatekeeper.py) (pure Python standard library,
raw data in [`results/results.json`](results/results.json)):

| Property | Result |
|---|---|
| Verification | **exhaustive** over the full reachable state space (bounded model checking) |
| Reachable states / transitions checked | 96 / 1,176 |
| Unsafe actions blocked | **672 / 672 (100%)** |
| Safe actions admitted | **504 / 504 (100%)** |
| Soundness (never admits an unsafe action) | **proven over the reachable space** |
| Completeness (never blocks a safe action) | **proven over the reachable space** |
| Adversarial (lying) proposer, unsafe blocked | **672 / 672 (100%)** |
| Ungated baseline, unsafe transitions | **57.1%** (the gate is load-bearing) |
| Checker latency | **0.36 microseconds per check** |
| Trusted-computing-base | **31 source lines** |

The scenario is a small warehouse: two robots on a 3x3 grid, a shared integer battery budget,
and one irreversible hazard cell. The safety specification is three invariants that must hold in
every state: the robots never share a cell (collision-free), neither enters the hazard cell (no
irreversible side effect), and the battery budget never goes negative.

## How it works (proof-carrying actions)

- A **proposer** (which may be untrusted, or an ML policy) emits a joint action *and* a
  certificate: the state it claims results, plus its claim that the result is safe.
- The **gatekeeper is the only trusted component**. It (a) recomputes the successor from the
  action and checks it equals the certificate (integrity: the proposer cannot lie about the
  outcome), and (b) checks that successor against the three invariants. It dispatches if and only
  if both pass.
- Generating a safe action may require search; **checking one is O(number of invariants)**. That
  asymmetry is the whole point: you can put an arbitrarily capable, untrusted proposer behind a
  tiny, trustworthy gate.

Because the model is bounded, [`src/gatekeeper.py`](src/gatekeeper.py) enumerates the entire
reachable state space and, for every state and every action, confirms the gatekeeper admits the
action *if and only if* the resulting state satisfies the specification. That is a complete
soundness-and-completeness result over the reachable space, not a sampled test. A separate
adversarial run confirms a proposer that lies ("this is safe") is blocked on 100% of its unsafe
proposals, and an ungated baseline shows 57% of transitions would land in an unsafe state without
the gate.

## The same gate on two real benchmarks

The gatekeeper is not tied to the warehouse. We run it, unchanged, on two published multi-agent
safety benchmarks, using their real maps and safety definitions.

**Island Navigation** ([DeepMind AI Safety Gridworlds](https://github.com/google-deepmind/ai-safety-gridworlds),
Leike et al. 2017). We take the official map and its safety definition verbatim from the repo (the
agent must reach the goal without ever entering water; `WATER_REWARD = -50`, an irreversible
drowning). The gate permits a move only if its certified destination is not water. Exhaustive over
the 21 reachable land cells and 84 moves: **10/10 water-entering moves blocked, 74/74 safe moves
admitted**, sound and complete. Ungated, 11.9% of moves would drown the agent.

**Commons Harvest** ([Melting Pot](https://github.com/google-deepmind/meltingpot), Leibo et al.
2021). A faithful reimplementation of the substrate's dynamics: apples regrow only while an apple
remains within a radius, so eating the last apple in a patch kills it forever, the tragedy of the
commons. The safety specification is sustainability: never harvest so as to empty a patch. The gate
is verified exhaustively on a bounded 3x3 instance (**447/447 patch-killing harvests blocked,
1,857/1,857 safe harvests admitted**, sound and complete), and a dynamic 6x6 episode with four
greedy agents shows the difference the gate makes:

| Commons after 200 steps (mean of 20 runs) | Ungated | Gated |
|---|--:|--:|
| Dead cells (irrecoverable) | **36 / 36** | **0 / 36** |
| Apples remaining | **0** | **~10** |

Ungated greedy agents collapse the commons completely; the same agents behind the sustainability
gate never kill a patch, and the resource persists.

## Reproduce

```bash
./run-demo.sh     # all three scenarios, pure Python standard library, no dependencies
```

## Honest scope

See [`BUILD_REPORT.md`](BUILD_REPORT.md). Every guarantee here is exhaustive over a *bounded* model
(the warehouse, the 21-cell island, a 3x3 commons); scaling to large or unbounded systems needs
symbolic / SMT model checking, where the trusted checker stays small while the reachable space is
handled abstractly, and this reference makes that checker explicit and measures it. The trusted
core is small (31 lines) but this repo verifies the *systems* exhaustively, it does not ship a
machine-checked proof of the *checker* itself; that is the role of the
[Tardygrada](https://github.com/fabio-rovai) formally-verified agent runtime, whose C core is the
production vehicle for a gate like this. The Island Navigation map and safety definition are taken
verbatim from the official AI Safety Gridworlds repo (the live pycolab engine also loads and steps,
verified during the build); Commons Harvest is a faithful reimplementation of the Melting Pot
substrate's dynamics, since dm-meltingpot's dmlab2d engine does not build in this environment.

---

### Built by Tesseract Academy

We build the assurance layer for autonomous and multi-agent systems: the small, checkable gate
that lets you deploy a capable but untrusted agent inside guardrails you can actually verify. If
you need actions gated by a certificate rather than trusted on faith, we can help.

[gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com
Part of [Open Ontologies](../../README.md) · MIT · verified, reproducible.
