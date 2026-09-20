#!/usr/bin/env python3
"""Draw what a well-formedness check finds in HQDM that a reasoner cannot.

The companion figure to `knowledge-graph.py`. That one shows a file being
reasoned over and proved; this one shows a file that passes the check everyone
runs and fails three checks nobody runs.

The input is `hqdmTop/hqdmFramework/rdf/hqdm-0.0.1-alpha.ttl`, which MagmaCore
vendors byte for byte. It is NOT the `hqdm.owl` that OM-2026 measured: that one
carries 14 disjointness axioms and 39 natively unsatisfiable classes, and this
one carries no `owl:` term at all. With no disjointness, no named class can be
unsatisfiable, so a coherence check on this file must return zero. The defects
below are the ones that survive that clean bill.

Every count here is computed from the rows. Nothing is typed twice.
"""
import math
import random
import sys

W, H = 1100, 720
SEED = 11

R = "http://www.w3.org/2000/01/rdf-schema#"
RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
SUBCLASS = f"<{R}subClassOf>"
TYPE = f"<{RDF}type>"
DOMAIN = f"<{R}domain>"
RANGE = f"<{R}range>"
CLASS = f"<{R}Class>"
LINKING = (SUBCLASS, TYPE, DOMAIN, RANGE)

C_OK = "#7dd3fc"
C_EDGE = "#334155"
C_SUB = "#475569"
C_BAD = "#fb3b53"
C_WARN = "#fbbf24"
CYCLE = 15.0


def short(iri):
    body = iri.strip("<>")
    h = body.rfind("#")
    return body[h + 1:] if h >= 0 else body[body.rfind("/") + 1:]


def read(path):
    rows = []
    with open(path) as f:
        for line in f:
            p = line.rstrip("\n").split("\t")
            if len(p) >= 3:
                rows.append((p[0], p[1], p[2]))
    return rows


def findings(rows):
    """The three defects, each computed, each with the rows behind it."""
    declared = {s for s, p, o in rows if p == TYPE and o == CLASS}
    used = {}
    for s, p, o in rows:
        if p == SUBCLASS:
            used.setdefault(s, set()).add("subClassOf")
            used.setdefault(o, set()).add("subClassOf")
        elif p in (DOMAIN, RANGE):
            used.setdefault(o, set()).add(p)
    undeclared = sorted(t for t in used if t not in declared and t.startswith("<"))

    # A range that names a RELATION.
    #
    # `rdfs:range` must name a class. These name terms that are never declared
    # a class AND never appear anywhere in the subclass hierarchy, in either
    # position: they are not classes that were merely left undeclared, they are
    # relation names standing where a class belongs. In this file that is
    # exactly `hqdm:part_of` and `hqdm:participant_in`.
    in_hierarchy = {x for s, p, o in rows if p == SUBCLASS for x in (s, o)}
    not_a_class = {t for t in undeclared if t not in in_hierarchy}
    bad_range = [(s, o) for s, p, o in rows if p == RANGE and o in not_a_class]

    # Names separated by one trailing underscore, which carries no meaning.
    subjects = {s for s, _, _ in rows}
    pairs = []
    for a in sorted(subjects):
        b = a[:-1] + "_>" if a.endswith(">") else a + "_"
        if b in subjects:
            fa = {(p, o) for s, p, o in rows if s == a}
            fb = {(p, o) for s, p, o in rows if s == b}
            pairs.append((a, b, fa == fb))
    return declared, undeclared, bad_range, pairs


def main(path, out_path):
    rows = read(path)
    declared, undeclared, bad_range, pairs = findings(rows)
    identical = [p for p in pairs if p[2]]

    edges = [(s, o, p) for s, p, o in rows if p in LINKING and o.startswith("<")]
    names = sorted({x for s, o, _ in edges for x in (s, o)})
    idx = {n: i for i, n in enumerate(names)}

    # One graph, or say so and stop. Same invariant the companion figure holds.
    parent = list(range(len(names)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for s, o, _ in edges:
        a, b = find(idx[s]), find(idx[o])
        if a != b:
            parent[a] = b
    pieces = len({find(i) for i in range(len(names))})
    if pieces != 1:
        raise SystemExit(f"HQDM is {pieces} pieces under {LINKING}; the figure says one graph")

    # ── Layout: one force run, one camera ───────────────────────────────
    rng = random.Random(SEED)
    K = (W * H * 240.0 / max(1, len(names))) ** (1.0 / 3.0) * 1.15
    pos = [[rng.uniform(-K, K) for _ in range(3)] for _ in names]
    eidx = [(idx[s], idx[o]) for s, o, _ in edges]
    for step in range(300):
        t = 1.0 - step / 300.0
        disp = [[0.0, 0.0, 0.0] for _ in names]
        for a in range(len(names)):
            for b in range(a + 1, len(names)):
                d = [pos[a][c] - pos[b][c] for c in range(3)]
                d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
                if d2 < 1e-6:
                    d = [rng.uniform(-1, 1) for _ in range(3)]
                    d2 = 1.0
                f = (K * K) / d2
                for c in range(3):
                    disp[a][c] += d[c] * f
                    disp[b][c] -= d[c] * f
        for a, b in eidx:
            d = [pos[a][c] - pos[b][c] for c in range(3)]
            dist = math.sqrt(sum(x * x for x in d)) or 1.0
            f = (dist * dist) / K / 14.0
            for c in range(3):
                disp[a][c] -= d[c] / dist * f
                disp[b][c] += d[c] / dist * f
        for a in range(len(names)):
            for c in range(3):
                disp[a][c] += (0.0 - pos[a][c]) * 0.02
            mag = math.sqrt(sum(x * x for x in disp[a])) or 1.0
            for c in range(3):
                pos[a][c] += disp[a][c] / mag * min(mag, 20 * t)

    YAW, PITCH = 0.6, 0.28
    cy_, sy_ = math.cos(YAW), math.sin(YAW)
    cp_, sp_ = math.cos(PITCH), math.sin(PITCH)
    cam = []
    for x, y, z in pos:
        x1, z1 = x * cy_ + z * sy_, -x * sy_ + z * cy_
        y2, z2 = y * cp_ - z1 * sp_, y * sp_ + z1 * cp_
        cam.append([x1, y2, z2])
    zs = [c[2] for c in cam]
    zmin, zmax = min(zs), max(zs)
    DIST = (zmax - zmin) * 0.8 + 200.0
    proj = []
    for x, y, z in cam:
        f = DIST / (DIST + (z - zmin))
        proj.append([x * f, y * f, (z - zmin) / max(1e-6, zmax - zmin)])

    L, Rr, T, B = 34, W - 34, 132, H - 196
    xs = sorted(p[0] for p in proj)
    ys = sorted(p[1] for p in proj)

    def band(v):
        lo = v[max(0, int(len(v) * 0.02))]
        hi = v[min(len(v) - 1, int(len(v) * 0.98))]
        return lo, (hi if hi > lo else lo + 1.0)

    x0, x1 = band(xs)
    y0, y1 = band(ys)
    sc = min((Rr - L) / (x1 - x0), (B - T) / (y1 - y0))
    ox = L + ((Rr - L) - (x1 - x0) * sc) / 2.0
    oy = T + ((B - T) - (y1 - y0) * sc) / 2.0
    pt = [[min(max(ox + (p[0] - x0) * sc, L), Rr),
           min(max(oy + (p[1] - y0) * sc, T), B), p[2]] for p in proj]

    deg = [0] * len(names)
    for a, b in eidx:
        deg[a] += 1
        deg[b] += 1

    out = []
    A = out.append
    A(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" '
      f'height="{H}" font-family="ui-sans-serif,-apple-system,Segoe UI,Roboto,Helvetica,'
      f'Arial,sans-serif" role="img" aria-label="HQDM 0.0.1-alpha as one connected graph '
      f'of {len(names)} terms, with {len(undeclared)} terms used as a class but never '
      f'declared, {len(bad_range)} ranges naming a relation, and {len(pairs)} names '
      f'separated only by a trailing underscore.">')
    A('<defs><radialGradient id="n"><stop offset="0" stop-color="#bae6fd"/>'
      '<stop offset="1" stop-color="#0ea5e9"/></radialGradient>'
      '<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">'
      '<stop offset="0" stop-color="#020617"/><stop offset="1" stop-color="#070d20"/>'
      '</linearGradient></defs>')
    A(f'<rect width="{W}" height="{H}" rx="14" fill="url(#bg)"/>')

    def anim(attr, values, times):
        return (f'<animate attributeName="{attr}" dur="{CYCLE}s" repeatCount="indefinite" '
                f'values="{values}" keyTimes="{times}" calcMode="linear"/>')

    def kt(*secs):
        return ";".join(f"{x / CYCLE:.4f}" for x in secs)

    undeclared_set = set(undeclared)
    bad_pairs = {(s, o) for s, o in bad_range}
    underscore = {b for _, b, _ in pairs}

    depth = [p[2] for p in pt]

    def rad(i):
        return (2.0 + min(5.0, deg[i] * 0.26)) * (0.62 + 0.62 * depth[i])

    # Edges, far to near, with the bad ranges drawn last and loud.
    order = sorted(range(len(edges)), key=lambda e: (depth[idx[edges[e][0]]] + depth[idx[edges[e][1]]]) / 2)
    plain, flagged = [], []
    for e in order:
        s, o, p = edges[e]
        i, j = idx[s], idx[o]
        d = (depth[i] + depth[j]) / 2
        bad = (s, o) in bad_pairs
        col = C_BAD if bad else (C_SUB if p == SUBCLASS else C_EDGE)
        wdt = (2.2 if bad else (0.5 if p == SUBCLASS else 0.35)) * (0.7 + 0.8 * d)
        op = (0.75 + 0.25 * d) if bad else (0.14 + 0.34 * d)
        line = (f'<line x1="{pt[i][0]:.1f}" y1="{pt[i][1]:.1f}" x2="{pt[j][0]:.1f}" '
                f'y2="{pt[j][1]:.1f}" stroke="{col}" stroke-width="{wdt:.2f}" '
                f'opacity="{op:.2f}"/>')
        (flagged if bad else plain).append(line)
    A("".join(plain))

    for i in sorted(range(len(names)), key=lambda i: depth[i]):
        r = rad(i)
        bad = names[i] in undeclared_set
        A(f'<circle cx="{pt[i][0]:.1f}" cy="{pt[i][1]:.1f}" r="{r:.1f}" '
          f'fill="{"none" if bad else "url(#n)"}" '
          + (f'stroke="{C_BAD}" stroke-width="1.6" ' if bad else "")
          + f'opacity="{0.45 + 0.5 * depth[i]:.2f}"/>')

    A(f'<g opacity="0.55">{"".join(flagged)}'
      + anim("opacity", "0.55;0.55;1;1;0.55;0.55", kt(0, 5.0, 5.5, 9.4, 10.0)) + '</g>')

    # The three findings, said in order.
    BEATS = [
        (C_BAD, f"1 · {len(undeclared)} terms are used as a class and never declared", 0.5, 5.0),
        (C_BAD, f"2 · {len(bad_range)} rdfs:range declarations name a relation, not a class", 5.0, 10.0),
        (C_WARN, f"3 · {len(pairs)} names differ only by a trailing underscore, {len(identical)} of them identical", 10.0, 15.0),
    ]
    A(f'<text x="34" y="44" font-size="17" font-weight="800" fill="#f8fafc">'
      f'hqdm-0.0.1-alpha.ttl: coherent, and not well formed</text>')
    for n_, (col, text, t0, t1) in enumerate(BEATS):
        first = "1" if n_ == 0 else "0"
        A(f'<text x="34" y="68" font-size="14" font-weight="700" fill="{col}" '
          f'opacity="{first}">'
          + anim("opacity", f"{first};0;1;1;0;0", kt(0, max(0.0, t0 - 0.3), t0 + 0.2, t1 - 0.3, t1))
          + f'{text}</text>')
    A(f'<text x="34" y="88" font-size="10.5" fill="#64748b">'
      f'{len(rows)} triples, {len(declared)} declared classes, {len(names)} terms in one '
      f'connected graph. No owl: term and no disjointness axiom, so no named class can be '
      f'unsatisfiable and a coherence check returns zero.</text>')

    # Legend.
    ly = H - 128
    A(f'<rect x="28" y="{ly}" width="{W - 56}" height="110" rx="12" fill="#030a1c" '
      f'opacity="0.94" stroke="#1e3a5f"/>')
    A(f'<text x="46" y="{ly + 24}" font-size="12" font-weight="800" fill="{C_BAD}" '
      f'letter-spacing="1.4">WHAT A REASONER CANNOT SEE HERE</text>')
    A(f'<line x1="40" y1="{ly + 33}" x2="{W - 40}" y2="{ly + 33}" stroke="#1e3a5f"/>')
    rows_ = [
        (C_BAD, "UNDECLARED", len(undeclared),
         "used as a class in subClassOf, domain or range; never rdf:type rdfs:Class"),
        (C_BAD, "RANGE IS A RELATION", len(bad_range),
         "rdfs:range must name a class. these name part_of and participant_in"),
        (C_WARN, "UNDERSCORE TWINS", len(pairs),
         f"one trailing underscore apart, carrying no meaning. {len(identical)} share domain AND range"),
    ]
    for m, (col, label, count, means) in enumerate(rows_):
        yy = ly + 51 + m * 18
        A(f'<rect x="46" y="{yy - 4}" width="22" height="3" fill="{col}"/>')
        A(f'<text x="78" y="{yy}" font-size="10.5" font-weight="700" fill="{col}" '
          f'letter-spacing="0.6">{label}</text>')
        A(f'<text x="266" y="{yy}" font-size="11" font-weight="700" fill="#e2e8f0" '
          f'text-anchor="end">{count}</text>')
        A(f'<text x="278" y="{yy}" font-size="10.5" fill="#94a3b8">{means}</text>')
    A(f'<line x1="40" y1="{ly + 92}" x2="{W - 40}" y2="{ly + 92}" stroke="#1e3a5f"/>')
    A(f'<text x="46" y="{ly + 104}" font-size="10" fill="#64748b">'
      f'This is not the file OM-2026 measured. That one, hqdm.owl, carries 14 disjointness '
      f'axioms and 39 natively unsatisfiable classes. Same ontology, two shipped renderings, '
      f'and the defect you find depends on which you fetched.</text>')
    A('</svg>')

    with open(out_path, "w") as f:
        f.write("".join(out))
    print(f"wrote {out_path}: {len(names)} terms, {len(edges)} edges, "
          f"{len(undeclared)} undeclared, {len(bad_range)} bad ranges, "
          f"{len(pairs)} underscore twins ({len(identical)} identical)")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
