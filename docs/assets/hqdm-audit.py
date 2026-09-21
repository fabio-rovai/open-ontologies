#!/usr/bin/env python3
"""Draw HQDM as it is shipped: two files, two different defect classes.

The companion figure to `knowledge-graph.py`. That one shows a file being
reasoned over and checked; this one shows the same ontology name published as
two files that disagree about what HQDM is, side by side, under the same
checks.

Left: `hqdmTop/hqdmFramework/rdf/hqdm-0.0.1-alpha.ttl`, which MagmaCore vendors
byte for byte. RDFS only, no `owl:` term, no disjointness axiom, so no named
class in it can be unsatisfiable and a coherence check returns zero however
carefully it is run. It fails three well-formedness checks nobody runs.

Right: `gchq/HQDM/hqdm.owl`, the OWL rendering. Well formed by the same three
checks, and not coherent: this repository's SHIQ tableaux finds most of its
named classes satisfiable and cannot decide the rest, and an external reasoner
(HermiT, an opinion in this repository's vocabulary) calls exactly those
remaining classes unsatisfiable. The figure COMPUTES that intersection from the
two committed lists; it does not state it.

Every count here is computed from the inputs. Nothing is typed twice.
"""
import json
import math
import random
import sys

W, H = 1100, 760
SEED = 11

R = "http://www.w3.org/2000/01/rdf-schema#"
RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
OWL = "http://www.w3.org/2002/07/owl#"
XSD = "http://www.w3.org/2001/XMLSchema#"
HQDM = "http://www.semanticweb.org/magma-core/ontologies/hqdm#"
SUBCLASS = f"<{R}subClassOf>"
SUBPROP = f"<{R}subPropertyOf>"
TYPE = f"<{RDF}type>"
DOMAIN = f"<{R}domain>"
RANGE = f"<{R}range>"
DISJOINT = f"<{OWL}disjointWith>"
CLASSES = {f"<{R}Class>", f"<{OWL}Class>"}
# Objects of rdf:type that are vocabulary, not terms of the ontology. Drawing
# them would hang every class off one owl:Class hub and say nothing.
META = {f"<{OWL}Class>", f"<{OWL}ObjectProperty>", f"<{OWL}DatatypeProperty>",
        f"<{OWL}Ontology>", f"<{OWL}NamedIndividual>", f"<{OWL}Restriction>",
        f"<{R}Class>", f"<{OWL}TransitiveProperty>", f"<{OWL}FunctionalProperty>"}
LINKING = (SUBCLASS, TYPE, DOMAIN, RANGE, DISJOINT, SUBPROP)

C_OK = "#7dd3fc"
C_EDGE = "#334155"
C_SUB = "#475569"
C_BAD = "#fb3b53"
C_WARN = "#fbbf24"
C_UND = "#f59e0b"
CYCLE = 20.0


# Every word on the drawing, in each language it is published in. The NUMBERS
# are never in here: they are computed and formatted into these strings, so a
# translation cannot state a count the data does not support.
TEXT = {
    "en": {
        "headline": "HQDM, two shipped renderings: one is not well formed, the other is not coherent",
        "beat1": "1 · left: {n} terms are used as a class and never declared",
        "beat2": "2 · left: {n} rdfs:range declarations name a relation, not a class",
        "beat3": "3 · right: this engine finds {sat} named classes satisfiable and cannot decide {und}",
        "beat4": "4 · right: HermiT, an opinion here, calls {n} unsatisfiable, and they are the same {both}",
        "beat5": "5 · both: {a} and {b} names differ only by a trailing underscore",
        "ttl_sub": "· hqdmTop/hqdmFramework · MagmaCore · RDFS",
        "owl_sub": "· gchq/HQDM · OWL, {n} disjointness axioms",
        "facts1": "{t} triples, {d} declared classes, {n} terms in one connected graph.",
        "ttl_facts2": "No owl: term and no disjointness axiom, so no named class can be unsatisfiable.",
        "owl_facts2": "{u} undeclared terms, {b} ranges naming a relation: well formed by the same checks.",
        "left_head": "WHAT A REASONER CANNOT SEE",
        "right_head": "WHAT ONLY A REASONER CAN SEE",
        "undeclared": ("UNDECLARED", "used as a class, never typed as one"),
        "badrange": ("RANGE IS A RELATION", "rdfs:range naming part_of or participant_in"),
        "twins": ("UNDERSCORE TWINS", "one underscore apart; {n} share domain and range"),
        "sat": ("SATISFIABLE", "named classes this engine's tableaux found a model for"),
        "und": ("UNDECIDED", "budget ran out before a verdict either way"),
        "oracle": ("ORACLE: UNSATISFIABLE", "HermiT, OM 2026 run; {n} are the undecided ones"),
        "foot1": "Same ontology name, two files, and the defect you find depends on which you fetched; "
                 "neither file says which is canonical.",
        "foot2": "{o} of the {t} underscore twins survive into the OWL rendering. "
                 "Every count here is recomputed from the rows by a test.",
    },
    "zh": {
        "headline": "HQDM 的两个发布文件：一个不是良构的，另一个不是融贯的",
        "beat1": "1 · 左：{n} 个术语被当作类使用，却从未声明",
        "beat2": "2 · 左：{n} 条 rdfs:range 声明指向关系，而不是类",
        "beat3": "3 · 右：本引擎判定 {sat} 个具名类可满足，另有 {und} 个无法判定",
        "beat4": "4 · 右：HermiT（在此只是一种意见）判定 {n} 个不可满足，恰好就是同样的 {both} 个",
        "beat5": "5 · 两者：{a} 对与 {b} 对名称仅相差一个尾部下划线",
        "ttl_sub": "· hqdmTop/hqdmFramework · MagmaCore · RDFS",
        "owl_sub": "· gchq/HQDM · OWL，{n} 条不相交公理",
        "facts1": "{t} 条三元组，{d} 个已声明的类，{n} 个术语构成一个连通图。",
        "ttl_facts2": "没有 owl: 术语，也没有不相交公理，因此没有具名类可能不可满足。",
        "owl_facts2": "{u} 个未声明术语，{b} 条指向关系的值域：按同样的检查是良构的。",
        "left_head": "推理机看不到的问题",
        "right_head": "只有推理机才能看到的问题",
        "undeclared": ("未声明", "被当作类使用，却从未被声明为类"),
        "badrange": ("值域是关系", "rdfs:range 指向 part_of 或 participant_in"),
        "twins": ("下划线孪生名", "仅相差一个尾部下划线；其中 {n} 对定义域与值域完全相同"),
        "sat": ("可满足", "本引擎的 tableaux 为其找到了模型的具名类"),
        "und": ("无法判定", "预算用尽，两个方向都没有结论"),
        "oracle": ("外部意见：不可满足", "HermiT，OM 2026 运行；其中 {n} 个正是无法判定的那些"),
        "foot1": "同一个本体名称，两个文件；你发现的缺陷取决于你取到了哪一个，两个文件都没有说明哪一个是规范版本。",
        "foot2": "{t} 对下划线孪生名中有 {o} 对延续到了 OWL 版本。此处每个数字都由测试从数据行重新计算。",
    },
}


def text_width(s, size):
    """Conservative width of a string at `size`. CJK is one em, Latin about
    half. The companion figure carries the same function and the same reason:
    a line that fits in English can run past the panel in Chinese."""
    w = 0.0
    for ch in s:
        o = ord(ch)
        if 0x2E80 <= o <= 0x9FFF or 0xAC00 <= o <= 0xD7AF or 0xFF00 <= o <= 0xFF60:
            w += size
        elif o < 0x2000 and ch.islower():
            w += size * 0.50
        else:
            w += size * 0.60
    return w


def must_fit(s, size, budget, where):
    """Refuse to draw a line that will not fit where it is drawn."""
    w = text_width(s, size)
    if w > budget:
        raise SystemExit(
            f"{where}: {w:.0f} units of text at font-size {size} in {budget:.0f} units of "
            f"space. Shorten it or widen the column.\n  {s}"
        )
    return s


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


def is_datatype(t):
    """A datatype in range position is correct RDFS, not a missing class.

    The RDFS rendering has no literals and no datatypes, so this changes
    nothing there. The OWL rendering ranges its datatype properties over
    xsd:string, xsd:dateTime, rdfs:Literal; those are not terms it forgot to
    declare.
    """
    b = t.strip("<>")
    return b.startswith(XSD) or b in (f"{R}Literal", f"{RDF}PlainLiteral",
                                      f"{RDF}langString", f"{OWL}rational", f"{OWL}real")


def findings(rows):
    """The three defects, each computed, each with the rows behind it."""
    declared = {s for s, p, o in rows if p == TYPE and o in CLASSES}
    used = {}
    for s, p, o in rows:
        if p == SUBCLASS:
            used.setdefault(s, set()).add("subClassOf")
            used.setdefault(o, set()).add("subClassOf")
        elif p in (DOMAIN, RANGE):
            used.setdefault(o, set()).add(p)
    undeclared = sorted(t for t in used
                        if t not in declared and t.startswith("<") and not is_datatype(t))

    # A range that names a RELATION.
    #
    # `rdfs:range` must name a class. These name terms that are never declared
    # a class AND never appear anywhere in the subclass hierarchy, in either
    # position: they are not classes that were merely left undeclared, they are
    # relation names standing where a class belongs. In the RDFS file that is
    # exactly `hqdm:part_of` and `hqdm:participant_in`.
    in_hierarchy = {x for s, p, o in rows if p == SUBCLASS for x in (s, o)}
    not_a_class = {t for t in undeclared if t not in in_hierarchy}
    bad_range = [(s, o) for s, p, o in rows if p == RANGE and o in not_a_class]

    # Names separated by one trailing underscore, which carries no meaning.
    subjects = {s for s, _, _ in rows if s.startswith("<")}
    pairs = []
    for a in sorted(subjects):
        b = a[:-1] + "_>"
        if b in subjects:
            fa = {(p, o) for s, p, o in rows if s == a}
            fb = {(p, o) for s, p, o in rows if s == b}
            pairs.append((a, b, fa == fb))
    return declared, undeclared, bad_range, pairs


def one_graph(names, edges, label):
    """One connected graph, or say so and stop. Same invariant as the companion figure."""
    idx = {n: i for i, n in enumerate(names)}
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
        raise SystemExit(f"{label} is {pieces} pieces under {LINKING}; the figure says one graph")
    return idx


def layout(names, edges, idx, box):
    """One force run in 3-D, one camera, fitted into `box` = (L, R, T, B)."""
    L, Rr, T, B = box
    rng = random.Random(SEED)
    area = (Rr - L) * (B - T)
    K = (area * 240.0 / max(1, len(names))) ** (1.0 / 3.0) * 1.15
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
    return pt, deg, eidx


def anim(attr, values, times):
    return (f'<animate attributeName="{attr}" dur="{CYCLE}s" repeatCount="indefinite" '
            f'values="{values}" keyTimes="{times}" calcMode="linear"/>')


def kt(*secs):
    return ";".join(f"{x / CYCLE:.4f}" for x in secs)


def window(t0, t1, low, high):
    """Opacity `low` outside [t0, t1] and `high` inside, as one animate element."""
    first = f"{high}" if t0 <= 0.0 else f"{low}"
    return anim("opacity",
                f"{first};{low};{high};{high};{low};{low}",
                kt(0, max(0.0, t0 - 0.3), t0 + 0.2, t1 - 0.3, t1, CYCLE))


def panel(A, rows, box, label, undeclared, bad_range, undecided, hermit, disjoint_on):
    """Draw one rendering into `box`; returns (names, edges) for the captions."""
    L, Rr, T, B = box
    edges = [(s, o, p) for s, p, o in rows
             if p in LINKING and o.startswith("<") and s.startswith("<") and o not in META]
    names = sorted({x for s, o, _ in edges for x in (s, o)})
    idx = one_graph(names, edges, label)
    pt, deg, eidx = layout(names, edges, idx, box)
    depth = [p[2] for p in pt]

    def rad(i):
        return (1.9 + min(4.6, deg[i] * 0.24)) * (0.62 + 0.62 * depth[i])

    undeclared_set = set(undeclared)
    bad_pairs = {(s, o) for s, o in bad_range}
    order = sorted(range(len(edges)), key=lambda e: (depth[idx[edges[e][0]]] + depth[idx[edges[e][1]]]) / 2)
    plain, flagged, disjoint = [], [], []
    for e in order:
        s, o, p = edges[e]
        i, j = idx[s], idx[o]
        d = (depth[i] + depth[j]) / 2
        bad = (s, o) in bad_pairs
        dis = p == DISJOINT
        col = C_BAD if bad else (C_UND if dis else (C_SUB if p == SUBCLASS else C_EDGE))
        wdt = (2.2 if bad else (1.4 if dis else (0.5 if p == SUBCLASS else 0.35))) * (0.7 + 0.8 * d)
        op = (0.75 + 0.25 * d) if (bad or dis) else (0.14 + 0.34 * d)
        line = (f'<line x1="{pt[i][0]:.1f}" y1="{pt[i][1]:.1f}" x2="{pt[j][0]:.1f}" '
                f'y2="{pt[j][1]:.1f}" stroke="{col}" stroke-width="{wdt:.2f}" '
                f'opacity="{op:.2f}"' + (' stroke-dasharray="4 3"' if dis else "") + '/>')
        (flagged if bad else (disjoint if dis else plain)).append(line)
    A("".join(plain))

    for i in sorted(range(len(names)), key=lambda i: depth[i]):
        r = rad(i)
        n = names[i]
        bad = n in undeclared_set
        und = n in undecided
        if und:
            # An undecided class: amber fill. If the oracle calls it
            # unsatisfiable as well, a red ring appears over it in beat 4.
            A(f'<circle cx="{pt[i][0]:.1f}" cy="{pt[i][1]:.1f}" r="{r + 0.8:.1f}" fill="{C_UND}" '
              f'opacity="{0.55 + 0.45 * depth[i]:.2f}"/>')
            if n in hermit:
                A(f'<circle cx="{pt[i][0]:.1f}" cy="{pt[i][1]:.1f}" r="{r + 3.2:.1f}" fill="none" '
                  f'stroke="{C_BAD}" stroke-width="1.8" opacity="0.15">'
                  + window(12.0, 16.0, 0.15, 1.0) + '</circle>')
        else:
            A(f'<circle cx="{pt[i][0]:.1f}" cy="{pt[i][1]:.1f}" r="{r:.1f}" '
              f'fill="{"none" if bad else "url(#n)"}" '
              + (f'stroke="{C_BAD}" stroke-width="1.6" ' if bad else "")
              + f'opacity="{0.45 + 0.5 * depth[i]:.2f}"/>')

    if flagged:
        A(f'<g opacity="0.55">{"".join(flagged)}' + window(4.0, 8.0, 0.55, 1.0) + '</g>')
    if disjoint:
        A(f'<g opacity="0.5">{"".join(disjoint)}' + window(disjoint_on[0], disjoint_on[1], 0.5, 1.0) + '</g>')
    return names, edges


def main(ttl_path, owl_path, dl_path, hermit_path, out_path, lang="en"):
    T = TEXT[lang]
    ttl = read(ttl_path)
    owl = read(owl_path)
    dl = json.load(open(dl_path))
    hermit = {f"<{HQDM}{line.strip()}>" for line in open(hermit_path) if line.strip()}

    t_declared, t_undeclared, t_bad, t_pairs = findings(ttl)
    o_declared, o_undeclared, o_bad, o_pairs = findings(owl)
    t_identical = [p for p in t_pairs if p[2]]
    o_identical = [p for p in o_pairs if p[2]]

    undecided = set(dl["undetermined_classes"])
    refuted = set(dl["unsatisfiable_classes"])
    sat_found = dl["agents"]["satisfiability_agent"]["satisfiable_found"]
    checked = dl["agents"]["satisfiability_agent"]["classes_checked"]
    if sat_found + len(undecided) + len(refuted) != checked:
        raise SystemExit("owl-dl.json does not add up: satisfiable + undecided + unsatisfiable != checked")
    both = undecided & hermit
    disjoint_n = sum(1 for s, p, o in owl if p == DISJOINT)
    owl_term = sum(1 for s, p, o in owl if p.startswith(f"<{OWL}") or o.startswith(f"<{OWL}"))
    ttl_owl_term = sum(1 for s, p, o in ttl if p.startswith(f"<{OWL}") or o.startswith(f"<{OWL}"))
    if ttl_owl_term:
        raise SystemExit("the RDFS rendering now carries owl: terms; the caption below is wrong")

    out = []
    A = out.append
    A(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" '
      f'height="{H}" font-family="ui-sans-serif,-apple-system,Segoe UI,Roboto,Helvetica,'
      f'Arial,sans-serif" role="img" aria-label="HQDM as two shipped files side by side. '
      f'Left, the RDFS rendering hqdm-0.0.1-alpha.ttl: {len(t_undeclared)} terms used as a '
      f'class but never declared drawn as hollow red rings, {len(t_bad)} rdfs:range '
      f'declarations naming a relation drawn as red edges, {len(t_pairs)} underscore twins. '
      f'Right, the OWL rendering hqdm.owl: {sat_found} named classes this engine found '
      f'satisfiable, {len(undecided)} it could not decide drawn amber, and a red ring on '
      f'the {len(both)} of those an external reasoner calls unsatisfiable.">')
    A('<defs><radialGradient id="n"><stop offset="0" stop-color="#bae6fd"/>'
      '<stop offset="1" stop-color="#0ea5e9"/></radialGradient>'
      '<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">'
      '<stop offset="0" stop-color="#020617"/><stop offset="1" stop-color="#070d20"/>'
      '</linearGradient></defs>')
    A(f'<rect width="{W}" height="{H}" rx="14" fill="url(#bg)"/>')

    # Two panels, one camera each, the same checks.
    GT, GB = 150, 580
    MID = W // 2
    A(f'<line x1="{MID}" y1="{GT - 34}" x2="{MID}" y2="{GB + 8}" stroke="#1e3a5f" stroke-dasharray="3 5"/>')
    t_names, t_edges = panel(A, ttl, (34, MID - 22, GT, GB), "hqdm-0.0.1-alpha.ttl",
                             t_undeclared, t_bad, set(), set(), (0, 0))
    o_names, o_edges = panel(A, owl, (MID + 22, W - 34, GT, GB), "hqdm.owl",
                             o_undeclared, o_bad, undecided, hermit, (8.0, 16.0))

    # Headline, then the beats in order.
    A(f'<text x="34" y="44" font-size="17" font-weight="800" fill="#f8fafc">'
      f'{T["headline"]}</text>')
    BEATS = [
        (C_BAD, T["beat1"].format(n=len(t_undeclared)), 0.0, 4.0),
        (C_BAD, T["beat2"].format(n=len(t_bad)), 4.0, 8.0),
        (C_UND, T["beat3"].format(sat=sat_found, und=len(undecided)), 8.0, 12.0),
        (C_BAD, T["beat4"].format(n=len(hermit), both=len(both)), 12.0, 16.0),
        (C_WARN, T["beat5"].format(a=len(t_pairs), b=len(o_pairs)), 16.0, 20.0),
    ]
    # One caption at a time, fades included. Two defects here, both measured in
    # a browser rather than reasoned about: the ramps overlapped, so every
    # transition drew two sentences through each other (8 such moments), and
    # the animation carried SIX values against FIVE keyTimes, which is not
    # valid SMIL at all. A caption now fades out over [t1-0.3, t1] and the next
    # fades in over [t0, t0+0.3], and consecutive beats share t1 == t0.
    for n_, (col, text, t0, t1) in enumerate(BEATS):
        first = "1" if n_ == 0 else "0"
        values = "1;1;1;1;0;0" if n_ == 0 else "0;0;1;1;0;0"
        times = (kt(0, 0, 0, t1 - 0.3, t1, CYCLE) if n_ == 0
                 else kt(0, t0, t0 + 0.3, t1 - 0.3, t1, CYCLE))
        A(f'<text x="34" y="70" font-size="14" font-weight="700" fill="{col}" opacity="{first}">'
          + anim("opacity", values, times)
          + f'{text}</text>')

    # Per-panel titles and facts.
    #
    # THESE WERE NEVER MEASURED. `must_fit` guarded the legend rows and the
    # footer and not the two column headings, so the left one grew until it
    # ended 50 units short of the right column in Chrome and overlapped it in
    # a browser whose fonts render a little wider. Nothing caught that because
    # nothing was looking.
    #
    # GUTTER is the headroom the estimator does not have. `text_width` is an
    # estimate, measured against Chrome at 6 per cent OVER the truth for this
    # string, which is the safe direction; the gutter covers the other
    # direction, a font that renders wider than the estimate.
    GUTTER = 30
    left_budget = (MID + 22) - 34 - GUTTER
    right_budget = (W - 34) - (MID + 22) - GUTTER
    must_fit(f'hqdm-0.0.1-alpha.ttl {T["ttl_sub"]}', 13, left_budget, "left panel heading")
    must_fit(T["facts1"].format(t=len(ttl), d=len(t_declared), n=len(t_names)),
             10.5, left_budget, "left panel facts")
    must_fit(T["ttl_facts2"], 10.5, left_budget, "left panel second fact")
    must_fit(f'hqdm.owl {T["owl_sub"].format(n=disjoint_n)}', 13, right_budget,
             "right panel heading")
    must_fit(T["facts1"].format(t=len(owl), d=len(o_declared), n=len(o_names)),
             10.5, right_budget, "right panel facts")
    must_fit(T["owl_facts2"].format(u=len(o_undeclared), b=len(o_bad)),
             10.5, right_budget, "right panel second fact")
    A(f'<text x="34" y="{GT - 42}" font-size="13" font-weight="800" fill="#e2e8f0">'
      f'hqdm-0.0.1-alpha.ttl <tspan fill="#64748b" font-weight="500">{T["ttl_sub"]}</tspan></text>')
    A(f'<text x="34" y="{GT - 26}" font-size="10.5" fill="#64748b">'
      f'{T["facts1"].format(t=len(ttl), d=len(t_declared), n=len(t_names))}</text>')
    A(f'<text x="34" y="{GT - 12}" font-size="10.5" fill="#64748b">'
      f'{T["ttl_facts2"]}</text>')
    A(f'<text x="{MID + 22}" y="{GT - 42}" font-size="13" font-weight="800" fill="#e2e8f0">'
      f'hqdm.owl <tspan fill="#64748b" font-weight="500">{T["owl_sub"].format(n=disjoint_n)}</tspan></text>')
    A(f'<text x="{MID + 22}" y="{GT - 26}" font-size="10.5" fill="#64748b">'
      f'{T["facts1"].format(t=len(owl), d=len(o_declared), n=len(o_names))}</text>')
    A(f'<text x="{MID + 22}" y="{GT - 12}" font-size="10.5" fill="#64748b">'
      f'{T["owl_facts2"].format(u=len(o_undeclared), b=len(o_bad))}</text>')

    # Legend: the two columns.
    ly = GB + 22
    A(f'<rect x="28" y="{ly}" width="{W - 56}" height="{H - ly - 18}" rx="12" fill="#030a1c" '
      f'opacity="0.94" stroke="#1e3a5f"/>')
    A(f'<text x="46" y="{ly + 24}" font-size="12" font-weight="800" fill="{C_BAD}" '
      f'letter-spacing="1.4">{T["left_head"]}</text>')
    A(f'<text x="{MID + 18}" y="{ly + 24}" font-size="12" font-weight="800" fill="{C_UND}" '
      f'letter-spacing="1.4">{T["right_head"]}</text>')
    A(f'<line x1="40" y1="{ly + 33}" x2="{W - 40}" y2="{ly + 33}" stroke="#1e3a5f"/>')
    left = [
        (C_BAD, T["undeclared"][0], len(t_undeclared), T["undeclared"][1]),
        (C_BAD, T["badrange"][0], len(t_bad), T["badrange"][1]),
        (C_WARN, T["twins"][0], len(t_pairs), T["twins"][1].format(n=len(t_identical))),
    ]
    right = [
        (C_OK, T["sat"][0], sat_found, T["sat"][1]),
        (C_UND, T["und"][0], len(undecided), T["und"][1]),
        (C_BAD, T["oracle"][0], len(hermit), T["oracle"][1].format(n=len(both))),
    ]
    for col_x, rows_ in ((46, left), (MID + 18, right)):
        for m, (col, label, count, means) in enumerate(rows_):
            yy = ly + 51 + m * 18
            A(f'<rect x="{col_x}" y="{yy - 4}" width="22" height="3" fill="{col}"/>')
            A(f'<text x="{col_x + 32}" y="{yy}" font-size="10.5" font-weight="700" fill="{col}" '
              f'letter-spacing="0.6">{label}</text>')
            A(f'<text x="{col_x + 214}" y="{yy}" font-size="11" font-weight="700" fill="#e2e8f0" '
              f'text-anchor="end">{count}</text>')
            # Each column ends where the other begins, or at the panel edge.
        limit = (MID - 10) if col_x < MID else (W - 28 - 6)
        must_fit(means, 9.4, limit - (col_x + 224), f"legend row {label!r}")
        A(f'<text x="{col_x + 224}" y="{yy}" font-size="9.4" fill="#94a3b8">{means}</text>')
    A(f'<line x1="40" y1="{ly + 108}" x2="{W - 40}" y2="{ly + 108}" stroke="#1e3a5f"/>')
    # Two lines, because one ran to x=1095 while the panel ends at 1072: the
    # sentence was printed outside the box that frames it.
    must_fit(T["foot1"], 10, (W - 28) - 46 - 6, "footer line 1")
    must_fit(T["foot2"].format(o=len(o_pairs), t=len(t_pairs)), 10, (W - 28) - 46 - 6, "footer line 2")
    A(f'<text x="46" y="{ly + 120}" font-size="10" fill="#64748b">'
      f'{T["foot1"]}</text>')
    A(f'<text x="46" y="{ly + 132}" font-size="10" fill="#64748b">'
      f'{T["foot2"].format(o=len(o_pairs), t=len(t_pairs))}</text>')
    A('</svg>')

    with open(out_path, "w") as f:
        f.write("".join(out))
    print(f"wrote {out_path}: ttl {len(t_names)} terms/{len(t_edges)} edges, "
          f"{len(t_undeclared)} undeclared, {len(t_bad)} bad ranges, {len(t_pairs)} twins "
          f"({len(t_identical)} identical); owl {len(o_names)} terms/{len(o_edges)} edges, "
          f"{len(o_undeclared)} undeclared, {len(o_bad)} bad ranges, {len(o_pairs)} twins "
          f"({len(o_identical)} identical), {sat_found} satisfiable, {len(undecided)} undecided, "
          f"hermit {len(hermit)}, both {len(both)}")


if __name__ == "__main__":
    main(*sys.argv[1:7])
