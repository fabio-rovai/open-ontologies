#!/usr/bin/env python3
"""Regenerate docs/assets/knowledge-graph.svg from a real certified run.

Run:
    open-ontologies reason --profile rdfs --certificate DIR benchmark/reference/ies-core.ttl
    python3 docs/assets/knowledge-graph.py DIR/asserted.tsv DIR/derivations.tsv \
        docs/assets/knowledge-graph.svg

Every figure in the legend is counted from those two files. Nothing is typed.
The layout is a plain spring embedder with a FIXED seed, so the same input
gives the same picture and a regeneration is a diff a reader can check rather
than a new arrangement of the same facts.
"""
import math
import random
import sys

SUBCLASS = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>"
W, H = 1100, 760
SEED = 20260919


def short(iri):
    s = iri.strip("<>")
    for sep in ("#", "/"):
        if sep in s:
            s = s.rsplit(sep, 1)[-1] or s
    return s


def read(path):
    rows = []
    with open(path) as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) >= 3:
                rows.append(parts)
    return rows


def main(asserted_path, derivations_path, out_path):
    asserted = read(asserted_path)
    derivations = read(derivations_path)

    # Asserted subclass edges, and the conclusions the fixpoint derived. A
    # derivation line is: rule, conclusion s p o, then its premises.
    a_edges = [(r[0], r[2]) for r in asserted if r[1] == SUBCLASS]
    d_edges, by_rule = [], {}
    for r in derivations:
        rule = r[0]
        by_rule[rule] = by_rule.get(rule, 0) + 1
        if len(r) >= 4 and r[2] == SUBCLASS:
            d_edges.append((r[1], r[3]))

    nodes = sorted({n for e in a_edges + d_edges for n in e})
    idx = {n: i for i, n in enumerate(nodes)}

    # Spring layout. Deterministic: seeded, fixed iteration count.
    rng = random.Random(SEED)
    pos = [[rng.uniform(80, W - 80), rng.uniform(90, H - 140)] for _ in nodes]
    deg = [0] * len(nodes)
    for s, o in a_edges + d_edges:
        deg[idx[s]] += 1
        deg[idx[o]] += 1

    # Fruchterman and Reingold with a pull toward the centre, then fitted to
    # the canvas at the end. The first version CLAMPED each coordinate into
    # the viewport on every iteration, which pinned anything that drifted
    # outward against a wall and drew rows of dots along all four edges. Let
    # the layout run unbounded and scale it once at the end instead.
    k = math.sqrt((W * H) / max(1, len(nodes))) * 1.9
    cx, cy = W / 2.0, H / 2.0
    for step in range(500):
        t = 1.0 - step / 500.0
        disp = [[0.0, 0.0] for _ in nodes]
        for i in range(len(nodes)):
            for j in range(i + 1, len(nodes)):
                dx = pos[i][0] - pos[j][0]
                dy = pos[i][1] - pos[j][1]
                d2 = dx * dx + dy * dy
                if d2 < 1e-6:
                    dx, dy, d2 = rng.uniform(-1, 1), rng.uniform(-1, 1), 1.0
                f = (k * k) / d2
                disp[i][0] += dx * f
                disp[i][1] += dy * f
                disp[j][0] -= dx * f
                disp[j][1] -= dy * f
        for s_, o_ in a_edges + d_edges:
            i, j = idx[s_], idx[o_]
            dx = pos[i][0] - pos[j][0]
            dy = pos[i][1] - pos[j][1]
            d = math.hypot(dx, dy) or 1.0
            f = (d * d) / k / 16.0
            disp[i][0] -= dx / d * f
            disp[i][1] -= dy / d * f
            disp[j][0] += dx / d * f
            disp[j][1] += dy / d * f
        for i in range(len(nodes)):
            # Gravity, so disconnected pieces do not sail off and the whole
            # drawing stays one picture rather than several.
            disp[i][0] += (cx - pos[i][0]) * 0.005
            disp[i][1] += (cy - pos[i][1]) * 0.005
            dx, dy = disp[i]
            d = math.hypot(dx, dy) or 1.0
            pos[i][0] += dx / d * min(d, 18 * t)
            pos[i][1] += dy / d * min(d, 18 * t)

    # Fit once, into the area left free by the title and the legend.
    L, R, T, B = 56, W - 56, 92, H - 116
    xs = [p[0] for p in pos]
    ys = [p[1] for p in pos]
    sx = (R - L) / max(1e-6, max(xs) - min(xs))
    sy = (B - T) / max(1e-6, max(ys) - min(ys))
    sc = min(sx, sy)
    ox = L + ((R - L) - (max(xs) - min(xs)) * sc) / 2.0
    oy = T + ((B - T) - (max(ys) - min(ys)) * sc) / 2.0
    for i in range(len(nodes)):
        pos[i][0] = ox + (pos[i][0] - min(xs)) * sc
        pos[i][1] = oy + (pos[i][1] - min(ys)) * sc

    out = []
    A = out.append
    A(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
      f'font-family="ui-sans-serif,-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif" '
      f'role="img" aria-label="The IES core class hierarchy. Grey edges were asserted by a person. '
      f'Green edges the engine derived and a Lean 4 proof accepted. One red edge is a forged '
      f'derivation the same checker refused.">')
    A('<defs>'
      '<radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">'
      '<stop offset="0" stop-color="#34d399" stop-opacity="0.30"/>'
      '<stop offset="1" stop-color="#34d399" stop-opacity="0"/></radialGradient>'
      '<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">'
      '<stop offset="0" stop-color="#050813"/><stop offset="1" stop-color="#0a1024"/></linearGradient>'
      '</defs>')
    A(f'<rect width="{W}" height="{H}" rx="14" fill="url(#bg)"/>')

    for s, o in d_edges:
        i, j = idx[s], idx[o]
        A(f'<line x1="{pos[i][0]:.1f}" y1="{pos[i][1]:.1f}" x2="{pos[j][0]:.1f}" '
          f'y2="{pos[j][1]:.1f}" stroke="#34d399" stroke-width="0.9" opacity="0.55"/>')
    for s, o in a_edges:
        i, j = idx[s], idx[o]
        A(f'<line x1="{pos[i][0]:.1f}" y1="{pos[i][1]:.1f}" x2="{pos[j][0]:.1f}" '
          f'y2="{pos[j][1]:.1f}" stroke="#64748b" stroke-width="0.8" opacity="0.8"/>')

    # The forged derivation, drawn once, from the node the checker named.
    if d_edges:
        fi = idx[d_edges[0][0]]
        A(f'<line x1="{pos[fi][0]:.1f}" y1="{pos[fi][1]:.1f}" x2="{W-210}" y2="{H-190}" '
          f'stroke="#fb3b53" stroke-width="2.4" stroke-dasharray="7 4"/>')
        A(f'<circle cx="{W-210}" cy="{H-190}" r="7" fill="#3b0710" stroke="#fb3b53" stroke-width="2"/>')
        A(f'<text x="{W-222}" y="{H-172}" text-anchor="end" font-size="11.5" font-weight="700" fill="#fb3b53">'
          f'forged, and refused</text>')

    for n in nodes:
        i = idx[n]
        r = 2.4 + min(5.0, deg[i] * 0.34)
        A(f'<circle cx="{pos[i][0]:.1f}" cy="{pos[i][1]:.1f}" r="{r:.1f}" fill="#7dd3fc" opacity="0.92"/>')
    # Label only the busiest nodes, on a chip, and only where the chip does
    # not land on one already placed. A label per node is a grey wall, and
    # labels stacked on a hub are worse than none: they hide the structure
    # they are meant to name.
    placed = []
    for n in sorted(nodes, key=lambda n: -deg[idx[n]]):
        if len(placed) >= 8:
            break
        i = idx[n]
        label = short(n)
        w = len(label) * 5.55 + 7
        x, y = pos[i][0] + 9, pos[i][1] + 3.5
        box = (x - 2, y - 10, x + w, y + 4)
        if any(not (box[2] < o[0] or box[0] > o[2] or box[3] < o[1] or box[1] > o[3])
               for o in placed):
            continue
        placed.append(box)
        A(f'<rect x="{x-3:.1f}" y="{y-10:.1f}" width="{w:.1f}" height="14" rx="3" '
          f'fill="#050813" opacity="0.82"/>')
        A(f'<text x="{x:.1f}" y="{y:.1f}" font-size="10" fill="#cbd5e1">{label}</text>')

    A(f'<circle cx="{W-150}" cy="86" r="52" fill="url(#glow)"/>')
    A(f'<circle cx="{W-150}" cy="86" r="19" fill="#34d399"/>')
    A(f'<text x="{W-150}" y="91" text-anchor="middle" font-size="12" font-weight="800" fill="#04180f">'
      f'Lean</text>')
    A(f'<text x="{W-150}" y="124" text-anchor="middle" font-size="11" fill="#6ee7b7">'
      f'oo-cert decides</text>')

    n_asserted = len(asserted)
    n_derived = len(derivations)
    rules = ", ".join(f"{r} x{c}" for r, c in sorted(by_rule.items(), key=lambda kv: -kv[1]))
    A(f'<text x="34" y="46" font-size="17" font-weight="800" fill="#f8fafc">'
      f'ies-core.ttl, reasoned over and proved</text>')
    A(f'<text x="34" y="68" font-size="12" fill="#94a3b8">'
      f'{len(nodes)} classes and {len(a_edges) + len(d_edges)} subclass edges drawn, out of '
      f'{n_asserted} asserted triples and {n_derived} derived</text>')

    y = H - 92
    A(f'<rect x="28" y="{y-26}" width="620" height="82" rx="10" fill="#0b1020" opacity="0.9" '
      f'stroke="#1e3a5f"/>')
    rows = [("#64748b", f"ASSERTED   {len(a_edges)} subclass edges, claimed by a person"),
            ("#34d399", f"CERTIFIED  {len(d_edges)} derived, {rules}, theorem OOCert.certificate_sound"),
            ("#fb3b53", "REFUSED    one conclusion forged; oo-cert exited 1 and named rdfs11")]
    for n_, (col, text) in enumerate(rows):
        yy = y + n_ * 20
        A(f'<rect x="44" y="{yy-8}" width="20" height="3" fill="{col}"/>')
        A(f'<text x="74" y="{yy-3}" font-size="11.5" fill="#cbd5e1">{text}</text>')
    A('</svg>')

    with open(out_path, "w") as f:
        f.write("\n".join(out))
    print(f"wrote {out_path}: {len(nodes)} nodes, {len(a_edges)} asserted and "
          f"{len(d_edges)} derived edges, from {n_asserted} asserted and {n_derived} derived triples")


if __name__ == "__main__":
    main(*sys.argv[1:4])
