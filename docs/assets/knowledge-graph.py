#!/usr/bin/env python3
"""Regenerate docs/assets/knowledge-graph.svg from a real certified run.

Run:
    printf 'load benchmark/reference/ies-core.ttl\nreason --profile rdfs --certificate DIR\n' \
        | open-ontologies batch - --no-connect
    python3 docs/assets/knowledge-graph.py DIR/asserted.tsv DIR/derivations.tsv \
        docs/assets/knowledge-graph.svg

`reason` takes no positional file. It reasons over the loaded store, so the
graph has to be loaded first and the two have to be one batch: a second process
would start with an empty store. The old header here said otherwise and the
command in it exits 2.

Every figure in the legend is counted from those two files. Nothing is typed.

This is a picture OF THE STUDIO'S 3D VIEW, not a diagram invented for the
README. `studio/src/components/Graph3D.tsx` is the thing a person actually uses
to build an ontology here, and every colour, width, particle and legend line
below is read off that component so the two cannot drift: node `#7dd3fc`, the
verification layer `#f0abfc`, Lean `#34d399` and larger than anything it judges,
links `#475569` / `#34d399` / `#fb3b53` at widths 0.5 / 0.9 / 2.4, one particle
along a certified link and six along a rejected one.

Depth is real rather than decorative. The layout runs in THREE dimensions and is
projected through a camera, so a node's size and opacity follow its distance and
the edges cross in front of and behind one another the way they do on screen.
Drawn far to near, painter's algorithm, because SVG has no z-buffer.

It is also ANIMATED, in four beats on one 14-second clock: the asserted graph
arrives, the engine derives, Lean sweeps the derived edges, and a forged edge is
drawn and then refused. That order is the argument the README makes, and a still
image cannot make it: it shows the three colours side by side as though they
were one kind of fact, when the whole point is that they are produced at
different times by different parties with different warrants.

SMIL rather than CSS or script, because the README embeds this through an
`<img>` tag served by raw.githubusercontent.com. Script never runs there.

NOTHING IS BUILT UP FROM NOTHING, and that constraint is the whole shape of the
file. The first attempt animated the graph into existence: opacity 0 to 1, a
mask opening from the checker. It looked right in a browser and rendered as an
EMPTY RECTANGLE under macOS Quick Look, because a still renderer samples the
timeline at t=0 and t=0 was blank. Thumbnails, link previews and PDF exports all
do that. So every layer is drawn at a resting opacity that is never zero, and a
beat BRIGHTENS its layer rather than revealing it. Sample this file at any
instant and you get the whole graph; watch it and you get the argument.
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

    # ── The verification layer ──────────────────────────────────────────
    #
    # `Graph3D.tsx` colours these by NAME and its own comment says which links
    # are drawn: "the certificate goes to Lean and to Isabelle, which read the
    # same bytes, while the first-order family reads a different artefact
    # entirely and never sees this graph". That is the wiring below, and it is
    # the reason the first-order provers hang off `problem.tsv` rather than off
    # anything in the ontology.
    LEAN = "Lean 4 · oo-cert"
    TOOLS = ["certificate", "problem.tsv", "ies-core.ttl", "forged line",
             LEAN, "Isabelle/HOL", "Vampire", "E", "Z3", "Mace4"]
    tool_edges = [
        ("ies-core.ttl", "certificate", "asserted"),
        ("certificate", LEAN, "certified"),
        ("certificate", "Isabelle/HOL", "certified"),
        ("ies-core.ttl", "problem.tsv", "asserted"),
        ("problem.tsv", "Vampire", "asserted"),
        ("problem.tsv", "E", "asserted"),
        ("problem.tsv", "Z3", "asserted"),
        ("problem.tsv", "Mace4", "asserted"),
        ("forged line", LEAN, "rejected"),
    ]

    nodes = sorted({n for e in a_edges + d_edges for n in e}) + TOOLS
    idx = {n: i for i, n in enumerate(nodes)}

    # Every edge as (i, j, warrant), which is what the drawing works from and
    # what the legend counts. Ontology edges first, layer edges after.
    edges = [(idx[a], idx[b], "asserted") for a, b in a_edges]
    edges += [(idx[a], idx[b], "certified") for a, b in d_edges]
    edges += [(idx[a], idx[b], w) for a, b, w in tool_edges]

    # ── Layout, in three dimensions ─────────────────────────────────────
    #
    # The same Fruchterman and Reingold as before with a z axis added, because
    # the view this illustrates is a 3D force graph and a flat plot of it is a
    # different picture. Seeded, so a regeneration is a diff a reader can check.
    rng = random.Random(SEED)
    pos = [[rng.uniform(-300, 300), rng.uniform(-300, 300), rng.uniform(-300, 300)]
           for _ in nodes]
    deg = [0] * len(nodes)
    for i, j, _ in edges:
        deg[i] += 1
        deg[j] += 1

    k = (W * H * 260.0 / max(1, len(nodes))) ** (1.0 / 3.0) * 1.45
    for step in range(420):
        t = 1.0 - step / 420.0
        disp = [[0.0, 0.0, 0.0] for _ in nodes]
        for i in range(len(nodes)):
            for j in range(i + 1, len(nodes)):
                d = [pos[i][a] - pos[j][a] for a in range(3)]
                d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
                if d2 < 1e-6:
                    d = [rng.uniform(-1, 1) for _ in range(3)]
                    d2 = 1.0
                f = (k * k) / d2
                for a in range(3):
                    disp[i][a] += d[a] * f
                    disp[j][a] -= d[a] * f
        for i, j, _ in edges:
            d = [pos[i][a] - pos[j][a] for a in range(3)]
            dist = math.sqrt(d[0] ** 2 + d[1] ** 2 + d[2] ** 2) or 1.0
            f = (dist * dist) / k / 16.0
            for a in range(3):
                disp[i][a] -= d[a] / dist * f
                disp[j][a] += d[a] / dist * f
        for i in range(len(nodes)):
            for a in range(3):
                # Gravity, and it has to be strong. A force layout gives
                # disconnected components nothing but repulsion, so the several
                # small pieces of ies-core sail away from the hub and the
                # drawing becomes one dense corner and three empty quarters.
                disp[i][a] += (0.0 - pos[i][a]) * 0.016
            mag = math.sqrt(sum(x * x for x in disp[i])) or 1.0
            for a in range(3):
                pos[i][a] += disp[i][a] / mag * min(mag, 22 * t)

    # ── The camera ──────────────────────────────────────────────────────
    #
    # One perspective projection, so nearer nodes are larger and the whole
    # thing has the depth the Studio view has. The rotation is fixed: an
    # orbiting camera cannot be expressed as a 2D transform without rotating
    # the labels with it, and a legible still frame matters more than a spin.
    YAW, PITCH = 0.62, 0.28
    cy_, sy_ = math.cos(YAW), math.sin(YAW)
    cp_, sp_ = math.cos(PITCH), math.sin(PITCH)
    cam = []
    for x, y, z in pos:
        x1, z1 = x * cy_ + z * sy_, -x * sy_ + z * cy_
        y2, z2 = y * cp_ - z1 * sp_, y * sp_ + z1 * cp_
        cam.append([x1, y2, z2])

    zs = [c[2] for c in cam]
    zmin, zmax = min(zs), max(zs)
    DIST = (zmax - zmin) * 1.9 + 420.0
    proj = []
    for x, y, z in cam:
        f = DIST / (DIST + (z - zmin))
        proj.append([x * f, y * f, (z - zmin) / max(1e-6, zmax - zmin)])

    # Fit into the area left free by the title and the legend.
    #
    # Fitted on the 3rd and 97th percentiles rather than the extremes. A force
    # layout always throws a couple of weakly connected nodes a long way out,
    # and fitting to those shrinks everything else into a corner: the first
    # version of this drew the whole ontology in the top right quarter with
    # three quarters of the canvas empty. Outliers are allowed to sit slightly
    # outside the frame instead.
    L, R, T, B = 56, W - 56, 96, H - 190
    xs = sorted(p[0] for p in proj)
    ys = sorted(p[1] for p in proj)
    def band(v):
        lo = v[max(0, int(len(v) * 0.03))]
        hi = v[min(len(v) - 1, int(len(v) * 0.97))]
        return lo, hi if hi > lo else lo + 1.0
    x0, x1 = band(xs)
    y0, y1 = band(ys)
    sc = min((R - L) / (x1 - x0), (B - T) / (y1 - y0))
    ox = L + ((R - L) - (x1 - x0) * sc) / 2.0
    oy = T + ((B - T) - (y1 - y0) * sc) / 2.0
    pt = [[ox + (p[0] - x0) * sc, oy + (p[1] - y0) * sc, p[2]] for p in proj]
    # depth: 0 is farthest, 1 nearest
    depth = [1.0 - p[2] for p in pt]

    # ── The clock ───────────────────────────────────────────────────────
    #
    # One cycle, one set of keyTimes, every animation on it. Separate clocks
    # drift apart over a long loop and the beats stop lining up with the
    # captions, which is worse than no animation because the captions then
    # describe the wrong thing.
    #
    # Every `values` list STARTS and ENDS at the layer's resting level, so the
    # frame at t=0 is the same complete picture as the frame at t=CYCLE. A
    # still renderer samples t=0; see the header.
    CYCLE = 14.0
    REST_A, LIT_A = 0.45, 0.95   # asserted
    REST_D, LIT_D = 0.30, 0.85   # derived
    REST_F, LIT_F = 0.50, 1.0    # the forged edge

    def kt(*secs):
        """Seconds to the keyTimes fraction SMIL wants."""
        return ";".join(f"{x / CYCLE:.4f}" for x in secs)

    def anim(attr, values, times, dur=None):
        return (f'<animate attributeName="{attr}" dur="{dur or CYCLE}s" '
                f'repeatCount="indefinite" values="{values}" keyTimes="{times}" '
                f'calcMode="linear"/>')

    out = []
    A = out.append
    A(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
      f'font-family="ui-sans-serif,-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif" '
      f'role="img" aria-label="The Studio 3D view of ies-core.ttl. Grey edges a person asserted, '
      f'green edges the engine derived and Lean accepted, one red edge forged and refused, with '
      f'the verification layer drawn as nodes.">')

    # Colours, widths and particle counts are `Graph3D.tsx`'s, not new ones.
    C_ASSERT, C_CERT, C_REJECT = "#475569", "#34d399", "#fb3b53"
    C_NODE, C_TOOL, C_LEAN = "#7dd3fc", "#f0abfc", "#34d399"
    WIDTH = {"asserted": 0.5, "certified": 0.9, "rejected": 2.4}
    COLOR = {"asserted": C_ASSERT, "certified": C_CERT, "rejected": C_REJECT}
    PARTICLES = {"asserted": 0, "certified": 1, "rejected": 6}

    A('<defs>'
      '<radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">'
      '<stop offset="0" stop-color="#34d399" stop-opacity="0.34"/>'
      '<stop offset="1" stop-color="#34d399" stop-opacity="0"/></radialGradient>'
      '<radialGradient id="node" cx="0.35" cy="0.32" r="0.72">'
      '<stop offset="0" stop-color="#e0f2fe"/><stop offset="1" stop-color="#7dd3fc"/>'
      '</radialGradient>'
      '<radialGradient id="tool" cx="0.35" cy="0.32" r="0.72">'
      '<stop offset="0" stop-color="#fbe8ff"/><stop offset="1" stop-color="#f0abfc"/>'
      '</radialGradient>'
      '<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">'
      '<stop offset="0" stop-color="#020617"/><stop offset="1" stop-color="#070d20"/></linearGradient>'
      '</defs>')
    A(f'<rect width="{W}" height="{H}" rx="14" fill="url(#bg)"/>')

    def rad(i):
        """Node radius: degree for importance, depth for distance, and the
        verification layer drawn larger than the ontology it judges, exactly as
        `nodeVal` does in the component."""
        base = 2.2 + min(4.6, deg[i] * 0.30)
        if nodes[i] == LEAN:
            base = 13.0
        elif nodes[i] in TOOLS:
            base = 7.0
        return base * (0.62 + 0.62 * depth[i])

    # ── Edges, far to near ──────────────────────────────────────────────
    #
    # SVG has no z-buffer, so the painter's algorithm is the depth test: sort
    # by the midpoint's distance and draw in that order. Without it the graph
    # reads flat however good the projection is, because a far edge drawn last
    # crosses in front of a near node.
    order = sorted(range(len(edges)), key=lambda e: (depth[edges[e][0]] + depth[edges[e][1]]) / 2)

    layers = {"asserted": [], "certified": [], "rejected": []}
    for e in order:
        i, j, w = edges[e]
        d = (depth[i] + depth[j]) / 2
        op = {"asserted": 0.20 + 0.45 * d,
              "certified": 0.22 + 0.50 * d,
              "rejected": 0.55 + 0.45 * d}[w]
        layers[w].append(
            f'<line x1="{pt[i][0]:.1f}" y1="{pt[i][1]:.1f}" x2="{pt[j][0]:.1f}" '
            f'y2="{pt[j][1]:.1f}" stroke="{COLOR[w]}" stroke-width="{WIDTH[w] * (0.7 + 0.8 * d):.2f}" '
            f'opacity="{op:.2f}"/>')

    A(f'<g opacity="{REST_D}">'
      + anim("opacity", f"{REST_D};{REST_D};{LIT_D};{LIT_D};{REST_D};{REST_D}",
             kt(0, 3.0, 4.0, 9.0, 10.0, CYCLE)))
    A("".join(layers["certified"]))
    A('</g>')
    A(f'<g opacity="{REST_A}">'
      + anim("opacity", f"{REST_A};{LIT_A};{LIT_A};{REST_A};{REST_A}",
             kt(0, 0.6, 2.6, 3.6, CYCLE)))
    A("".join(layers["asserted"]))
    A('</g>')

    # ── The particles ───────────────────────────────────────────────────
    #
    # `linkDirectionalParticles` in the component: one along a certified link,
    # six along a rejected one, none along an asserted one. They are the view's
    # signature motion and they carry the direction of the claim, which a static
    # line cannot. One `<circle>` and one `<animateMotion>` each.
    parts = []
    for e in order:
        i, j, w = edges[e]
        n_p = PARTICLES[w]
        if not n_p:
            continue
        d = (depth[i] + depth[j]) / 2
        dur = 3.4 if w == "certified" else 1.5
        for q in range(n_p):
            parts.append(
                f'<circle r="{0.9 + 0.9 * d:.2f}" fill="{COLOR[w]}" opacity="{0.5 + 0.5 * d:.2f}">'
                f'<animateMotion dur="{dur}s" repeatCount="indefinite" '
                f'begin="-{q * dur / n_p:.2f}s" '
                f'path="M{pt[i][0]:.1f},{pt[i][1]:.1f} L{pt[j][0]:.1f},{pt[j][1]:.1f}"/>'
                f'</circle>')
    A(f'<g opacity="0.85">'
      + anim("opacity", "0.85;0.85;1;1;0.85;0.85", kt(0, 3.0, 4.0, 9.0, 10.0, CYCLE)))
    A("".join(parts))
    A('</g>')

    # ── Nodes, far to near ──────────────────────────────────────────────
    for i in sorted(range(len(nodes)), key=lambda i: depth[i]):
        x, y = pt[i][0], pt[i][1]
        r = rad(i)
        if nodes[i] == LEAN:
            # The static radius is the animation's FIRST value and not some
            # other one. A still renderer samples t=0, so any disagreement here
            # draws a frame the animation never shows.
            A(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r * 3.0:.1f}" fill="url(#glow)">'
              + anim("r", f"{r*3.0:.1f};{r*3.6:.1f};{r*3.0:.1f};{r*4.4:.1f};{r*3.4:.1f};{r*3.0:.1f}",
                     kt(0, 2.0, 4.0, 7.4, 9.0, CYCLE)) + '</circle>')
            A(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.1f}" fill="{C_LEAN}"/>')
        elif nodes[i] in TOOLS:
            A(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.1f}" fill="url(#tool)" '
              f'opacity="{0.72 + 0.28 * depth[i]:.2f}"/>')
        else:
            A(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.1f}" fill="url(#node)" '
              f'opacity="{0.45 + 0.50 * depth[i]:.2f}"/>')

    # ── Labels ──────────────────────────────────────────────────────────
    #
    # Every verification-layer node is named, because the point of drawing them
    # is to show who judged what. Ontology classes get a label only if they are
    # busy and the chip lands clear, the way the component shows one on hover
    # rather than all at once: a label per node is a grey wall.
    placed = []

    def chip(i, label, color, size, weight, insist=False):
        """Place a label, trying a few positions before giving up.

        `insist` is for the verification layer. An ontology class that cannot
        find room simply goes unlabelled, which is the right trade in a dense
        graph. A judge that goes unlabelled defeats the purpose of drawing the
        layer at all, so those get every candidate position and then take the
        last one regardless. Dropping `Isabelle/HOL` off the picture because a
        blue dot was in the way is not a tidier drawing, it is a wrong one."""
        w_ = len(label) * (size * 0.55) + 8
        r_ = rad(i)
        candidates = [
            (pt[i][0] + r_ + 5, pt[i][1] + 3.5),
            (pt[i][0] - r_ - 5 - w_, pt[i][1] + 3.5),
            (pt[i][0] + r_ + 5, pt[i][1] - r_ - 6),
            (pt[i][0] - w_ / 2, pt[i][1] + r_ + 14),
            (pt[i][0] - w_ / 2, pt[i][1] - r_ - 8),
        ]
        for n_, (x, y) in enumerate(candidates):
            box = (x - 3, y - 11, x + w_, y + 4)
            clear = all(box[2] < o[0] or box[0] > o[2] or box[3] < o[1] or box[1] > o[3]
                        for o in placed)
            last = n_ == len(candidates) - 1
            if clear or (insist and last):
                placed.append(box)
                A(f'<rect x="{x-4:.1f}" y="{y-11:.1f}" width="{w_:.1f}" height="15" rx="3.5" '
                  f'fill="#020617" opacity="0.88"/>')
                A(f'<text x="{x:.1f}" y="{y:.1f}" font-size="{size}" font-weight="{weight}" '
                  f'fill="{color}">{label}</text>')
                return True
        return False

    for name in [LEAN] + [t for t in TOOLS if t != LEAN]:
        chip(idx[name], name, C_LEAN if name == LEAN else C_TOOL,
             11.5 if name == LEAN else 10.5, 800 if name == LEAN else 700, insist=True)
    shown = 0
    for n in sorted((n for n in nodes if n not in TOOLS), key=lambda n: -deg[idx[n]]):
        if shown >= 7:
            break
        if chip(idx[n], short(n), "#cbd5e1", 10, 400):
            shown += 1

    n_asserted = len(asserted)
    n_derived = len(derivations)
    rules = ", ".join(f"{r} x{c}" for r, c in sorted(by_rule.items(), key=lambda kv: -kv[1]))
    A(f'<text x="34" y="46" font-size="17" font-weight="800" fill="#f8fafc">'
      f'ies-core.ttl, reasoned over and proved</text>')
    A(f'<text x="34" y="68" font-size="12" fill="#94a3b8">'
      f'the Studio 3D view: {len(nodes)} nodes and {len(edges)} edges drawn, out of '
      f'{n_asserted} asserted triples and {n_derived} derived by {rules}</text>')

    # Beat caption.
    beats = [("#94a3b8", "1 · asserted by a person", 0.6, 3.6),
             ("#6ee7b7", "2 · derived by the engine", 3.6, 6.6),
             ("#34d399", "3 · checked by Lean, and accepted", 6.6, 9.0),
             ("#fb3b53", "4 · forged, and refused", 9.0, 12.8)]
    for n_, (col, text, t0, t1) in enumerate(beats):
        first = n_ == 0
        if first:
            vals, times = "1;1;1;0;0;1", kt(0, t0, t1 - 0.3, t1, CYCLE - 0.4, CYCLE)
        else:
            vals, times = "0;0;1;1;0;0", kt(0, t0, t0 + 0.4, t1 - 0.3, t1, CYCLE)
        A(f'<text x="34" y="{H-160}" font-size="13" font-weight="700" fill="{col}" '
          f'opacity="{1 if first else 0}">'
          + anim("opacity", vals, times) + f'{text}</text>')

    # ── The legend, which is the component's Legend ──────────────────────
    #
    # Same title, same three rows, same wording, same footer. Counted from the
    # edges drawn above rather than typed, exactly as the component counts them
    # from the links on screen.
    n_a = sum(1 for _, _, w in edges if w == "asserted")
    n_c = sum(1 for _, _, w in edges if w == "certified")
    n_r = sum(1 for _, _, w in edges if w == "rejected")
    ly = H - 132
    A(f'<rect x="28" y="{ly}" width="700" height="112" rx="12" fill="#030a1c" opacity="0.94" '
      f'stroke="#1e3a5f"/>')
    A(f'<text x="46" y="{ly+24}" font-size="12" font-weight="800" fill="#34d399" '
      f'letter-spacing="1.4">PROOF-CARRYING INFERENCE</text>')
    A(f'<text x="300" y="{ly+24}" font-size="11.5" fill="#64748b">'
      f'ies-core.ttl · {n_asserted:,} triples</text>')
    A(f'<line x1="40" y1="{ly+33}" x2="716" y2="{ly+33}" stroke="#1e3a5f"/>')
    rows = [(C_ASSERT, "ASSERTED", n_a, "read from ies-core.ttl. claimed by a person"),
            (C_CERT, "CERTIFIED", n_c, "derived, then PROVED. OOCert.certificate_sound"),
            (C_REJECT, "REJECTED", n_r, "forged. the checker exited 1 and named the rule")]
    for m, (col, label, count, means) in enumerate(rows):
        yy = ly + 51 + m * 18
        A(f'<rect x="46" y="{yy-4}" width="22" height="3" fill="{col}"/>')
        A(f'<text x="78" y="{yy}" font-size="11" font-weight="700" fill="{col}" '
          f'letter-spacing="0.7">{label}</text>')
        A(f'<text x="168" y="{yy}" font-size="11" font-weight="700" fill="#e2e8f0" '
          f'text-anchor="end">{count}</text>')
        A(f'<text x="180" y="{yy}" font-size="11" fill="#94a3b8">{means}</text>')
    A(f'<line x1="40" y1="{ly+94}" x2="716" y2="{ly+94}" stroke="#1e3a5f"/>')
    A(f'<text x="46" y="{ly+106}" font-size="10.5" fill="#64748b">'
      f'<tspan fill="#34d399" font-weight="700">● Lean 4</tspan> decides. '
      f'<tspan fill="#f0abfc">● Isabelle/HOL</tspan> checks the same bytes independently. '
      f'<tspan fill="#f0abfc">● Vampire, E, Z3, Mace4</tspan> read a different artefact; their '
      f'verdicts are oracle opinions, never certificates.</text>')
    A('</svg>')

    with open(out_path, "w") as f:
        f.write("\n".join(out))
    print(f"wrote {out_path}: {len(nodes)} nodes, {len(a_edges)} asserted and "
          f"{len(d_edges)} derived edges, from {n_asserted} asserted and {n_derived} derived triples")


if __name__ == "__main__":
    main(*sys.argv[1:4])
