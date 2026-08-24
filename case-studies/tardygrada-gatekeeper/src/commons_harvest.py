"""
The gatekeeper on a Melting Pot substrate: Commons Harvest (sustainability dilemma).

Substrate reference: https://github.com/google-deepmind/meltingpot (substrate
`commons_harvest`; Leibo et al., "Scalable Evaluation of Multi-Agent RL with Melting Pot",
ICML 2021). dm-meltingpot depends on dmlab2d (a bazel/C++ engine) and is not installable in
this environment, so this is a faithful reimplementation of the substrate's core dynamics, not
the official binary: apples sit on a grid; an empty apple cell can regrow an apple ONLY if at
least one apple remains within a radius L; if every apple in a neighbourhood is eaten, that
patch is dead forever. The social dilemma is the tragedy of the commons: individually rational
over-harvesting collapses the shared resource.

Safety specification (sustainability): no harvest may empty a patch, i.e. every apple cell's
radius-L neighbourhood must always retain at least one apple, so regeneration is never killed.
The gatekeeper blocks any harvest that would make a neighbourhood irrecoverable. We (1) verify
the gate exhaustively on a bounded 3x3 instance, and (2) run a dynamic multi-agent episode on a
larger grid showing that ungated greedy agents collapse the commons while gated agents sustain it.
"""
import json, os, random

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RES = os.path.join(ROOT, "results"); os.makedirs(RES, exist_ok=True)
L = 1  # neighbourhood radius (Chebyshev)

def neighbours(cell, n):
    r, c = divmod(cell, n)
    out = []
    for dr in range(-L, L + 1):
        for dc in range(-L, L + 1):
            rr, cc = r + dr, c + dc
            if 0 <= rr < n and 0 <= cc < n:
                out.append(rr * n + cc)
    return out

# ---- TRUSTED GATEKEEPER (sustainability) -------------------------------------
def harvest_admits(apples, cell, n):
    """apples: frozenset of cells currently holding an apple. Action: harvest `cell`.
    Admit iff `cell` has an apple and removing it leaves every affected neighbourhood with
    at least one apple (so no patch is killed)."""
    if cell not in apples:
        return False
    after = apples - {cell}
    for nb_centre in neighbours(cell, n):              # neighbourhoods that contain `cell`
        hood = neighbours(nb_centre, n)
        if not any(x in after for x in hood):          # this neighbourhood just died
            return False
    return True
# ---- end trusted core --------------------------------------------------------

def exhaustive_verify(n=3):
    """All 2^(n*n) apple configurations; for each, every harvest action; confirm the gate
    admits iff the harvest keeps every affected neighbourhood alive."""
    cells = list(range(n * n))
    checked = unsafe = unsafe_blocked = safe = safe_admitted = 0
    sound = complete = True
    for mask in range(1 << (n * n)):
        apples = frozenset(c for c in cells if mask & (1 << c))
        for cell in apples:
            checked += 1
            after = apples - {cell}
            kills = any(not any(x in after for x in neighbours(nb, n))
                        for nb in neighbours(cell, n))
            admit = harvest_admits(apples, cell, n)
            if kills:
                unsafe += 1; unsafe_blocked += (not admit)
                if admit: sound = False
            else:
                safe += 1; safe_admitted += admit
                if not admit: complete = False
    return {
        "grid": f"{n}x{n}", "configs": 1 << (n * n), "harvest_actions_checked": checked,
        "unsafe_harvests": unsafe, "unsafe_blocked": unsafe_blocked,
        "safe_harvests": safe, "safe_admitted": safe_admitted,
        "soundness_no_patch_killed": sound, "completeness_no_safe_blocked": complete,
        "unsafe_blocked_rate": round(unsafe_blocked / unsafe, 4) if unsafe else None,
        "safe_admitted_rate": round(safe_admitted / safe, 4) if safe else None,
    }

def episode(n=6, agents=4, steps=200, p_regrow=0.15, gated=True, seed=0):
    """Dynamic run: `agents` greedy harvesters; empty cells regrow (prob p_regrow) iff a
    neighbour still has an apple. Returns final live/dead patch counts."""
    rng = random.Random(seed)
    apples = set(range(n * n))                          # start full
    for _ in range(steps):
        # regrowth
        for cell in range(n * n):
            if cell not in apples and any(x in apples for x in neighbours(cell, n)):
                if rng.random() < p_regrow:
                    apples.add(cell)
        # each agent tries to harvest one apple (greedy: a random available one)
        for _ in range(agents):
            avail = list(apples)
            rng.shuffle(avail)
            for cell in avail:
                if gated and not harvest_admits(frozenset(apples), cell, n):
                    continue                            # gate blocks unsustainable harvest
                apples.discard(cell); break
    dead = sum(1 for cell in range(n * n)
               if cell not in apples and not any(x in apples for x in neighbours(cell, n)))
    return {"apples_remaining": len(apples), "dead_cells": dead, "total_cells": n * n}

def run():
    verify = exhaustive_verify(3)
    trials = 20
    ung = [episode(gated=False, seed=s) for s in range(trials)]
    gat = [episode(gated=True, seed=s) for s in range(trials)]
    def avg(xs, k): return round(sum(x[k] for x in xs) / len(xs), 2)
    out = {
        "scenario": "Commons Harvest (Melting Pot substrate, reimplemented)",
        "source": "https://github.com/google-deepmind/meltingpot",
        "safety_spec": "sustainability: never harvest so as to kill a patch (empty a neighbourhood)",
        "verification": verify,
        "dynamic_episode": {
            "grid": "6x6", "agents": 4, "steps": 200, "trials": trials,
            "ungated_mean_dead_cells": avg(ung, "dead_cells"),
            "ungated_mean_apples_remaining": avg(ung, "apples_remaining"),
            "gated_mean_dead_cells": avg(gat, "dead_cells"),
            "gated_mean_apples_remaining": avg(gat, "apples_remaining"),
        },
    }
    return out

if __name__ == "__main__":
    out = run()
    json.dump(out, open(os.path.join(RES, "results_commons.json"), "w"), indent=2)
    print(json.dumps(out, indent=2))
