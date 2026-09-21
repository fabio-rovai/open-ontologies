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
TYPE = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
DOMAIN = "<http://www.w3.org/2000/01/rdf-schema#domain>"
RANGE = "<http://www.w3.org/2000/01/rdf-schema#range>"
ONTOLOGY = "<http://www.w3.org/2002/07/owl#Ontology>"
# The four that make ies-core ONE graph. Measured, not guessed:
#   subClassOf                      6 pieces
#   + rdf:type                      5
#   + rdfs:domain, rdfs:range       1, and the file's own header
LINKING = (SUBCLASS, TYPE, DOMAIN, RANGE)
W, H = 1100, 780
SEED = 20260919


def text_width(s, size):
    """A conservative width for a string at `size`, in user units.

    Not exact: an exact answer needs the font, and the font here is whatever the
    reader's browser resolves from a stack. Conservative on purpose, so the
    guard below errs towards refusing a line that would have fitted rather than
    passing one that will not. A CJK ideograph is one em; Latin averages about
    half. A figure published in two languages needs both, because the Chinese
    verdict line ran 95 units further right than the English one and neither
    fitted.
    """
    w = 0.0
    for ch in s:
        o = ord(ch)
        if 0x2E80 <= o <= 0x9FFF or 0xAC00 <= o <= 0xD7AF or 0xFF00 <= o <= 0xFF60:
            w += size            # CJK, full width
        elif o < 0x2000 and ch.islower():
            w += size * 0.50
        elif o < 0x2000:
            w += size * 0.60     # capitals, digits, punctuation run wider
        else:
            w += size * 0.60
    return w


def must_fit(s, size, budget, where):
    """Refuse to draw a line that will not fit where it is being drawn.

    Every other invariant on this drawing is enforced rather than hoped for:
    one connected graph, a mu.json that really is unasked, no two label boxes
    overlapping. Text staying inside the panel that frames it was the one left
    to eyesight, and it escaped three times, in both languages, including off
    the edge of the canvas. Now it fails the build.
    """
    w = text_width(s, size)
    if w > budget:
        raise SystemExit(
            f"{where}: {w:.0f} units of text at font-size {size} in {budget:.0f} units of "
            f"space. Shorten it or widen the column.\n  {s}"
        )
    return s


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



# Every word on the drawing, in each language it is published in. The NUMBERS
# stay out of this table: they are computed from the run and formatted into
# these strings, so a translation cannot state a count the data does not carry.
TEXT = {
    "en": {
        "headline": "ies-core.ttl, reasoned over and proved",
        "sub": "the Studio 3D view: {n} nodes and {e} edges drawn, out of {a} asserted triples and {d} derived by {rules}",
        "cols": ("the file", "what it becomes", "who reads it"),
        "graphhead": "{n} TERMS OF ies-core, ONE CONNECTED GRAPH · subClassOf, rdf:type, domain, range",
        "beat1": "1 · a person asserts",
        "beat2": "2 · the engine derives",
        "beat3": "3 · Lean checks the one certificate covering all {n}, and accepts",
        "beat4_cert": "4 · four provers read the clauses; Vampire's refutation is checked, the rest opine",
        "beat4_plain": "4 · four provers read a different file, and only opine",
        "beat5": "5 · a line is forged, and the same checker refuses",
        "beat6": "6 · a question the file cannot be asked is returned unasked",
        "legend_head": "PROOF-CARRYING INFERENCE",
        "legend_file": "ies-core.ttl · {n:,} triples",
        "asserted": ("ASSERTED", "read from ies-core.ttl. claimed by a person"),
        "certified": ("CERTIFIED", "derived, then PROVED. one certificate, one run"),
        "rejected": ("REJECTED", "forged. the checker exited 1 and named the rule"),
        "unasked": ("UNASKED", "a question outside the file's language. no judge was asked"),
        "worth_head": "WHAT A VERDICT IS WORTH",
        "w_cert": "certificate",
        "w_cert1_fo": "One file, all {n} lines. Lean 4 and Isabelle/HOL read it.",
        "w_cert2_fo": "OOCert.certificate_sound; Vampire's, {t}.",
        "w_cert1": "One file, all {n} lines. Lean 4 and Isabelle/HOL read it.",
        "w_cert2": "OOCert.certificate_sound. Anyone can re-run it.",
        "w_op": "opinion",
        "w_op1_fo": "E, Z3 and Mace4 read the clauses too, and print a word.",
        "w_op1": "Vampire, E, Z3 and Mace4 read a different file.",
        "w_op2": "believe the program, or believe nothing. no object to check.",
        "w_mu": "mu",
        "w_mu1": "{s} \u2291 {o}, returned unasked.",
        "w_mu2": "{s} is {why}. The presupposition fails.",
        "j_cert": "certificate", "j_same": "same bytes", "j_op": "opinion",
        "j_checks": "checks it", "j_noproof": "no proof",
        "disagree": "disagreement · stops the line",
        "forged": "this line is forged",
        "refused": "exit 1, refused",
        "mu_badge": "returned unasked · {s} is {why}",
        "mu_individual": "a {t}, never a class",
        "mu_individual_plain": "an individual, never a class",
        "mu_typed": "typed {t}, never a class",
        "mu_undeclared": "a name the file never uses",
        "mu_other": "outside the file's language",
    },
    "zh": {
        "headline": "ies-core.ttl：经过推理，并且得到证明",
        "sub": "Studio 三维视图：绘制了 {n} 个节点与 {e} 条边，来自 {a} 条断言三元组和由 {rules} 推出的 {d} 条",
        "cols": ("文件", "它变成什么", "谁来阅读"),
        "graphhead": "ies-core 的 {n} 个术语，一个连通图 · subClassOf、rdf:type、domain、range",
        "beat1": "1 · 有人作出断言",
        "beat2": "2 · 引擎进行推导",
        "beat3": "3 · Lean 检查涵盖全部 {n} 条的那一份证书，并且接受",
        "beat4_cert": "4 · 四个证明器读取子句；Vampire 的反驳被检查，其余只给出意见",
        "beat4_plain": "4 · 四个证明器读取另一个文件，只给出意见",
        "beat5": "5 · 有一行被伪造，同一个检查器拒绝了它",
        "beat6": "6 · 一个该文件无法回答的问题，被原样退回",
        "legend_head": "带证明的推理",
        "legend_file": "ies-core.ttl · {n:,} 条三元组",
        "asserted": ("断言", "读自 ies-core.ttl，由人作出的声称"),
        "certified": ("已认证", "推导得出，并且已证明。一份证书，一次运行"),
        "rejected": ("被拒绝", "伪造。检查器以 exit 1 退出，并指出规则"),
        "unasked": ("未提问", "超出该文件语言范围的问题。没有询问任何裁判"),
        "worth_head": "一个结论值多少",
        "w_cert": "证书",
        "w_cert1_fo": "一个文件，涵盖全部 {n} 条。Lean 4 与 Isabelle/HOL 读取它。",
        "w_cert2_fo": "OOCert.certificate_sound；Vampire 的反驳，{t}。",
        "w_cert1": "一个文件，涵盖全部 {n} 条。Lean 4 与 Isabelle/HOL 读取它。",
        "w_cert2": "OOCert.certificate_sound。任何人都可以重新运行。",
        "w_op": "意见",
        "w_op1_fo": "E、Z3 与 Mace4 也读取这些子句，并给出一个词。",
        "w_op1": "Vampire、E、Z3 与 Mace4 读取的是另一个文件。",
        "w_op2": "要么相信这个程序，要么什么都不信。没有可检查的对象。",
        "w_mu": "无",
        "w_mu1": "{s} \u2291 {o}，被原样退回。",
        "w_mu2": "{s} 是{why}。失败的是预设，而不是主张。",
        "j_cert": "证书", "j_same": "同样的字节", "j_op": "意见",
        "j_checks": "检查它", "j_noproof": "没有证明",
        "disagree": "分歧 · 停止这条流水线",
        "forged": "这一行是伪造的",
        "refused": "exit 1，已拒绝",
        "mu_badge": "原样退回 · {s} 是{why}",
        "mu_individual": "一个 {t}，从来不是类",
        "mu_individual_plain": "一个个体，从来不是类",
        "mu_typed": "被标注为 {t}，从来不是类",
        "mu_undeclared": "该文件从未使用过的名称",
        "mu_other": "超出该文件的语言范围",
    },
}


def main(asserted_path, derivations_path, out_path, prove_path=None, mu_path=None, lang="en"):
    # `T` is already a box coordinate in this file, so the text table is `TX`.
    TX = TEXT[lang]
    asserted = read(asserted_path)
    derivations = read(derivations_path)
    # The prover run's report. Which judge earned `certificate` and which only
    # `opinion` is READ from here, never typed: `refutation_certified` is a
    # word only `onto_fol_prove` can mint, from a token only `oo-resolution`'s exit
    # 0 can produce (decision 0005, second addendum).
    import json as _json
    import re
    prove = _json.load(open(prove_path)) if prove_path else None
    vampire_certified = bool(prove and prove.get("verdict") == "refutation_certified"
                             and prove.get("problem_form") == "cnf")
    fo_theorem = (prove or {}).get("theorem", "")
    # The unasked question: one goal `onto_fol_prove` returned with the verdict
    # `mu` before any prover ran, because it puts in class position a term the
    # file never uses as a class. Read from the run, like the certificate.
    mu = _json.load(open(mu_path)) if mu_path else None
    if mu and (mu.get("report") or {}).get("verdict") != "mu":
        raise SystemExit("mu.json is not an unasked question; the sixth beat would be a lie")
    mu_s = mu["goal"][0] if mu else None
    mu_o = mu["goal"][2] if mu else None
    mu_kind = (mu or {}).get("report", {}).get("kind", "")
    mu_typed = ""
    if mu:
        m_ = re.search(r"typed (\S+?)(?:,|\s|$)", mu["report"].get("why", ""))
        mu_typed = short(m_.group(1)) if m_ else ""
    mu_line = {
        "individual": TX["mu_individual"].format(t=mu_typed) if mu_typed else TX["mu_individual_plain"],
        "typed_not_a_class": TX["mu_typed"].format(t=mu_typed),
        "undeclared": TX["mu_undeclared"],
    }.get(mu_kind, TX["mu_other"])
    fo_checker = "oo-resolution"

    # Asserted subclass edges, and the conclusions the fixpoint derived. A
    # derivation line is: rule, conclusion s p o, then its premises.
    # Subclass edges, and the `rdf:type` edges that make the picture ONE graph.
    #
    # Drawing subClassOf alone, ies-core falls into six pieces, and a reader
    # reasonably asks why a single file is six clouds. The answer was that the
    # drawing was throwing away 87% of the file. `rdf:type` is its commonest
    # predicate, 215 triples, and it is what ties the hierarchy together: 131
    # of these classes are an `rdfs:Class`, so that node is a real hub and not
    # a device invented to join things up.
    #
    # Nothing is fabricated to achieve this. `owl:Thing` would have been the
    # textbook way to give the hierarchy one top, and it is NOT used, because
    # this run derives no such edge: adding it would be drawing a claim the
    # engine never made, in a figure whose whole argument is that it does not.
    # The ontology HEADER is not a term. `<.../ies/core/v0/ont>` typed
    # `owl:Ontology` is the file's record of itself, it stands in no hierarchy
    # with anything, and it is the one thing that stays disconnected however
    # many predicates are drawn. It is also already on the canvas: the file is
    # the leftmost node of the pipeline. So it is dropped here rather than left
    # floating as a two-dot island nobody can explain.
    header = {r[0] for r in asserted if r[1] == TYPE and r[2] == ONTOLOGY}
    header |= {r[2] for r in asserted if r[0] in header}

    a_edges = [(r[0], r[2]) for r in asserted
               if r[1] in LINKING and r[2].startswith("<")
               and r[0] not in header and r[2] not in header]
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
    FILES = ["ies-core.ttl", "certificate", "problem.tsv"]
    FORES = "oo-resolution"
    PROGRAMS = [LEAN, "Isabelle/HOL", "Vampire", "E", "Z3", "Mace4", FORES]
    TOOLS = FILES + PROGRAMS
    tool_edges = [
        ("ies-core.ttl", "certificate", "asserted"),
        ("certificate", LEAN, "certified"),
        ("certificate", "Isabelle/HOL", "certified"),
        ("ies-core.ttl", "problem.tsv", "asserted"),
        ("problem.tsv", "Vampire", "asserted"),
        ("problem.tsv", "E", "asserted"),
        ("problem.tsv", "Z3", "asserted"),
        ("problem.tsv", "Mace4", "asserted"),
    ]
    # Vampire's refutation goes to the Fo checker, and the edge is certified
    # ONLY when the run says so. Not to `Lean 4 · oo-cert`: that binary checks
    # derivation certificates, and drawing it as the checker of a resolution
    # proof would be a new wrong claim.
    tool_edges.append(("Vampire", FORES, "certified" if vampire_certified else "asserted"))

    nodes = sorted({n for e in a_edges + d_edges for n in e}) + TOOLS
    idx = {n: i for i, n in enumerate(nodes)}

    # Every edge as (i, j, warrant), which is what the drawing works from and
    # what the legend counts. Ontology edges first, layer edges after.
    edges = [(idx[a], idx[b], "asserted") for a, b in a_edges]
    edges += [(idx[a], idx[b], "certified") for a, b in d_edges]
    edges += [(idx[a], idx[b], w) for a, b, w in tool_edges]
    # Which of the derived edges is the forged one is decided after the layout,
    # in `forge_one`, because it is decided on where the edge LANDS.

    # ── Layout: a COMPOSITION, not one force run over everything ────────
    #
    # The force layout used to lay out the ontology and the verification layer
    # together, and the result was a scatter: the six judges landed wherever
    # repulsion put them, so the picture had no shape and a reader could not see
    # that this is a PIPELINE. A force layout is the right tool for a graph
    # nobody designed and the wrong one for a diagram of a process.
    #
    # So the two halves are laid out differently, because they are different
    # kinds of thing. The ontology is a graph and gets a force layout, in three
    # dimensions, projected through a perspective camera. The pipeline is a
    # process and is PLACED: the file on the left, the two artefacts it becomes
    # in the middle, the six judges that read them on the right, flowing the way
    # the reader already reads.
    ont = [n for n in nodes if n not in TOOLS]
    oidx = {n: i for i, n in enumerate(ont)}
    oedges = [(oidx[a], oidx[b]) for a, b in a_edges + d_edges]

    rng = random.Random(SEED)

    # ── ies-core is six pieces, not one cloud ───────────────────────────
    #
    # The subclass graph is not connected: 87 classes, then 35, then four
    # small pieces of 7, 5, 3 and 2. One force system over all 139 put the two
    # big pieces in opposite corners and, because a disconnected node feels
    # nothing but repulsion, flung the small ones into the gap between the
    # ontology and the pipeline, which is the one place in the picture where
    # stray dots do the most damage.
    #
    # So each component is laid out in its OWN force system and the systems are
    # packed into the region, area by size. The reader gets a fact out of the
    # arrangement rather than a scatter: the file is six disjoint pieces, and
    # two of them carry almost everything.
    parent = list(range(len(ont)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for i, j in oedges:
        ri, rj = find(i), find(j)
        if ri != rj:
            parent[ri] = rj
    groups = {}
    for i in range(len(ont)):
        groups.setdefault(find(i), []).append(i)
    comps = sorted(groups.values(), key=len, reverse=True)
    # ONE graph, or say so and stop.
    #
    # The heading states "ONE CONNECTED GRAPH", and a figure that states its
    # own structure has to be unable to state it falsely. If a future ies-core,
    # or a different file, does not connect under LINKING, this raises instead
    # of drawing several clouds under a caption that promises one.
    if len(comps) != 1:
        raise SystemExit(
            f"the ontology is {len(comps)} pieces of sizes "
            f"{[len(c) for c in comps]}, and the figure says ONE CONNECTED "
            f"GRAPH. Either widen LINKING until it is one, or change the "
            f"heading to say what is true. Do not draw it as it stands."
        )

    # One ideal edge length for every component, so a big piece comes out as a
    # big cloud and a small piece as a small one. Scaling each component to
    # fill its own tile instead would make a 2-node fragment as visually loud
    # as the 87-node core, which is the opposite of true.
    # The ideal separation between two nodes. Raised from 1.25 to 1.55 because
    # the cloud read as one clump: 233 terms in a box this size crowd into the
    # middle, and a picture whose point is that the graph is CONNECTED has to
    # let a reader see the edges that connect it.
    K = (W * H * 260.0 / max(1, len(ont))) ** (1.0 / 3.0) * 1.55

    # The camera. One camera for all six, so they read as pieces of a single
    # space rather than six unrelated drawings.
    # Yaw chosen by MEASUREMENT, not by eye. This cloud is a dense core with a
    # long arm, and at the old +0.62 the arm pointed away from the camera, so
    # the core piled into the left third of the panel and the right third held
    # almost nothing: 59 / 24 / 16 per cent of the 233 terms across the three
    # thirds, with a horizontal interquartile spread of 28 per cent of the
    # panel. Turning the camera to -0.45 presents the arm across the view
    # instead of into it: 33 / 37 / 30, and the interquartile spread rises to
    # 45 per cent. Same layout, same distances, a different place to stand.
    #
    # A camera move was the right lever because it distorts nothing. Gravity
    # was tried first and changed the balance by two points at four times its
    # value, because the skew is the shape of the graph and not a parameter.
    # Stretching the fit to fill the tile would have worked and is refused
    # above, for the reason written there.
    YAW, PITCH = -0.45, 0.30
    cy_, sy_ = math.cos(YAW), math.sin(YAW)
    cp_, sp_ = math.cos(PITCH), math.sin(PITCH)

    def lay(members):
        """Force-lay one component in 3D, then turn it to face the camera."""
        local = {g: m for m, g in enumerate(members)}
        ed = [(local[a], local[b]) for a, b in oedges if a in local and b in local]
        n = len(members)
        p = [[rng.uniform(-K, K) for _ in range(3)] for _ in range(n)]
        for step in range(420):
            t = 1.0 - step / 420.0
            disp = [[0.0, 0.0, 0.0] for _ in range(n)]
            for a in range(n):
                for b in range(a + 1, n):
                    d = [p[a][c] - p[b][c] for c in range(3)]
                    d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
                    if d2 < 1e-6:
                        d = [rng.uniform(-1, 1) for _ in range(3)]
                        d2 = 1.0
                    f = (K * K) / d2
                    for c in range(3):
                        disp[a][c] += d[c] * f
                        disp[b][c] -= d[c] * f
            for a, b in ed:
                d = [p[a][c] - p[b][c] for c in range(3)]
                dist = math.sqrt(d[0] ** 2 + d[1] ** 2 + d[2] ** 2) or 1.0
                f = (dist * dist) / K / 22.0   # weaker pull along an edge, so hubs spread
                for c in range(3):
                    disp[a][c] -= d[c] / dist * f
                    disp[b][c] += d[c] / dist * f
            for a in range(n):
                for c in range(3):
                    disp[a][c] += (0.0 - p[a][c]) * 0.016
                mag = math.sqrt(sum(x * x for x in disp[a])) or 1.0
                for c in range(3):
                    p[a][c] += disp[a][c] / mag * min(mag, 22 * t)
        out = []
        for x, y, z in p:
            x1, z1 = x * cy_ + z * sy_, -x * sy_ + z * cy_
            y2, z2 = y * cp_ - z1 * sp_, y * sp_ + z1 * cp_
            out.append([x1, y2, z2])
        return out

    cams = [lay(c) for c in comps]

    # Depth is normalised ACROSS the six, not within each. Normalising per
    # component would make the nearest node of a two-node fragment as bright
    # as the nearest node of the core, and depth would stop meaning depth.
    allz = [c[2] for cam in cams for c in cam]
    zmin, zmax = min(allz), max(allz)
    DIST = (zmax - zmin) * 0.75 + 190.0

    projs = []
    for cam in cams:
        pr = []
        for x, y, z in cam:
            f = DIST / (DIST + (z - zmin))
            pr.append([x * f, y * f, (z - zmin) / max(1e-6, zmax - zmin)])
        projs.append(pr)

    # ── Packing the six ─────────────────────────────────────────────────
    #
    # Recursive proportional split: cut the list where the weight halves, cut
    # the rectangle the same way along its longer side, recurse. It fills the
    # region with no leftover, which is the point — the earlier layout left the
    # bottom-right of the canvas empty while fragments crowded the middle.
    OL, OR_, T, B = 470, W - 34, 138, H - 246

    def split(items, rect):
        if len(items) == 1:
            return [(items[0][0], rect)]
        x0_, y0_, x1_, y1_ = rect
        tot = sum(w for _, w in items)
        acc, cut = 0.0, 1
        for m in range(1, len(items)):
            acc += items[m - 1][1]
            if acc >= tot / 2.0:
                cut = m
                break
        left, right = items[:cut], items[cut:]
        fr = sum(w for _, w in left) / tot
        if (x1_ - x0_) >= (y1_ - y0_):
            xm = x0_ + (x1_ - x0_) * fr
            return (split(left, (x0_, y0_, xm, y1_))
                    + split(right, (xm, y0_, x1_, y1_)))
        ym = y0_ + (y1_ - y0_) * fr
        return (split(left, (x0_, y0_, x1_, ym))
                + split(right, (x0_, ym, x1_, y1_)))

    # Pieces of fewer than six classes share one tile rather than each taking
    # their own. Given a tile apiece they were scaled up to fill it, so a
    # two-class fragment drew as wide as the thirty-five-class piece beside it
    # and the arrangement stopped saying anything about size.
    BIG = [ci for ci, c in enumerate(comps) if len(c) >= 6]
    SMALL = [ci for ci, c in enumerate(comps) if len(c) < 6]
    items = [(("c", ci), float(len(comps[ci]))) for ci in BIG]
    if SMALL:
        items.append((("s", None), float(sum(len(comps[ci]) for ci in SMALL)) + 6.0))
    rects = dict(split(items, (OL, T, OR_, B)))

    tiles = {}
    for key, rect in rects.items():
        if key[0] == "c":
            tiles[key[1]] = rect
    if SMALL:
        sx0, sy0, sx1, sy1 = rects[("s", None)]
        # Side by side along the longer axis of the shared tile, biggest first.
        horiz = (sx1 - sx0) >= (sy1 - sy0)
        tot = float(sum(len(comps[ci]) for ci in SMALL))
        run = 0.0
        for ci in SMALL:
            fr0, fr1 = run / tot, (run + len(comps[ci])) / tot
            run += len(comps[ci])
            if horiz:
                tiles[ci] = (sx0 + (sx1 - sx0) * fr0, sy0,
                             sx0 + (sx1 - sx0) * fr1, sy1)
            else:
                tiles[ci] = (sx0, sy0 + (sy1 - sy0) * fr0,
                             sx1, sy0 + (sy1 - sy0) * fr1)

    pt = [[0.0, 0.0, 0.0] for _ in nodes]
    for ci, members in enumerate(comps):
        pr = projs[ci]
        tx0, ty0, tx1, ty1 = tiles[ci]
        pad = min(30.0, (tx1 - tx0) * 0.10, (ty1 - ty0) * 0.10)
        tx0, ty0, tx1, ty1 = tx0 + pad, ty0 + pad, tx1 - pad, ty1 - pad
        xs = sorted(p[0] for p in pr)
        ys = sorted(p[1] for p in pr)

        def band(v):
            lo = v[max(0, int(len(v) * 0.03))]
            hi = v[min(len(v) - 1, int(len(v) * 0.97))]
            return lo, (hi if hi > lo else lo + 1.0)

        x0, x1 = band(xs)
        y0, y1 = band(ys)
        # Aspect preserved. Stretching a component to fill its tile would bend
        # the perspective the whole 3D layout exists to show.
        sc = min((tx1 - tx0) / (x1 - x0), (ty1 - ty0) / (y1 - y0))
        ox = tx0 + ((tx1 - tx0) - (x1 - x0) * sc) / 2.0
        oy = ty0 + ((ty1 - ty0) - (y1 - y0) * sc) / 2.0
        # Clamped to the tile. The fit is on the 3rd and 97th percentiles, so by
        # construction a few nodes land outside it; before the ontology had a
        # drawn boundary that was invisible, and afterwards one class sat above
        # the panel line, on the heading, looking like a mistake.
        for m, g in enumerate(members):
            px_ = min(max(ox + (pr[m][0] - x0) * sc, tx0), tx1)
            py_ = min(max(oy + (pr[m][1] - y0) * sc, ty0), ty1)
            pt[idx[ont[g]]] = [px_, py_, pr[m][2]]

    # ── Which claim is the forged one ───────────────────────────────────
    #
    # It used to be a node of its own, `forged line`, parked in the pipeline
    # column with a red edge to Lean. Nothing about it was in the graph the
    # viewer had spent four beats looking at, so the last beat refused a claim
    # about nothing: the red line was the most important thing in the figure
    # and the easiest thing to miss.
    #
    # The forged claim is now one of the DERIVED edges, drawn among the others
    # and indistinguishable from them until the checker names it. That is what
    # a forgery is: a line that looks like the rest.
    #
    # Chosen on where it lands, not on what it says, so it is chosen after the
    # layout: long enough to read as a line, and as near the middle of the
    # largest piece as a long one gets.
    big = comps[0]
    inbig = {idx[ont[g]] for g in big}
    bx0, by0, bx1, by1 = tiles[0]
    cxm, cym = (bx0 + bx1) / 2.0, (by0 + by1) / 2.0
    best, best_score = None, None
    for e, (i, j, w) in enumerate(edges):
        if w != "certified" or i not in inbig or j not in inbig:
            continue
        L = math.hypot(pt[i][0] - pt[j][0], pt[i][1] - pt[j][1])
        if L < 34.0:
            continue
        mx, my = (pt[i][0] + pt[j][0]) / 2.0, (pt[i][1] + pt[j][1]) / 2.0
        score = math.hypot(mx - cxm, my - cym) - L * 0.55
        if best_score is None or score < best_score:
            best, best_score = e, score
    forged = best
    if forged is not None:
        i, j, _ = edges[forged]
        edges[forged] = (i, j, "rejected")

    # ── The pipeline, placed ────────────────────────────────────────────
    #
    # Three columns, left to right: what was read, what it became, who judged
    # it. The two that read the CERTIFICATE sit together and above; the four
    # that read `problem.tsv`, a different artefact, sit together and below.
    # That grouping is the argument, and no force layout would ever produce it.
    PIPE = {
        "ies-core.ttl": (86, 178),
        "certificate": (196, 214),
        "problem.tsv": (196, 396),
        LEAN: (318, 168),
        "Isabelle/HOL": (318, 250),
        "Vampire": (318, 336),
        FORES: (420, 336),
        "E": (318, 384),
        "Z3": (318, 432),
        "Mace4": (318, 480),
    }
    for n, (px, py) in PIPE.items():
        # z = 0 is the NEAR plane: `depth` below is `1 - z`, so zero here means
        # depth 1, which is full size and full opacity and drawn last. Setting
        # it to 1 put the whole pipeline at the far plane and its edges came out
        # at opacity 0.20, which is why they were invisible.
        pt[idx[n]] = [float(px), float(py), 0.0]

    deg = [0] * len(nodes)
    for i, j, _ in edges:
        deg[i] += 1
        deg[j] += 1

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
    # How many edges one certificate covers, computed here because beat 3 says
    # it. A reader could not tell from the drawing whether there was one
    # certificate or 259 of them, and the answer matters: `reason
    # --certificate` writes ONE (asserted.tsv + derivations.tsv, one line per
    # derived triple with its rule and premises), `oo-cert` reads it in ONE
    # run, and what it discharges is ONE theorem over the whole set.
    n_certified_edges = sum(1 for _, _, w in edges if w == "certified")

    CYCLE = 22.0 if mu else 18.0
    # Five steps, and the prover step is its own rather than a footnote inside
    # the Lean one. A reader was told the first-order family exists and never
    # shown it do anything, and the difference between what Lean produces and
    # what they produce is the single most important distinction on the page.
    BEATS = [
        ("#94a3b8", TX["beat1"], 0.6, 3.4),
        ("#6ee7b7", TX["beat2"], 3.4, 6.4),
        ("#34d399", TX["beat3"].format(n=n_certified_edges), 6.4, 9.6),
        ("#f0abfc", (TX["beat4_cert"] if vampire_certified else TX["beat4_plain"]), 9.6, 13.0),
        ("#fb3b53", TX["beat5"], 13.0, 17.0),
    ]
    C_MU = "#fbbf24"
    if mu:
        BEATS.append((C_MU, TX["beat6"], 17.0, 21.0))

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
      f'green edges the engine derived and Lean accepted, one red edge forged and refused, '
      + ('one amber question returned unasked because its subject is a concept and not a class, ' if mu else '')
      + f'with the verification layer drawn as nodes.">')

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

    # A boundary, because a cloud with no edge reads as spillage. The force
    # layout is fitted inside this rectangle, so the line is where the drawing
    # actually ends rather than a frame put round it afterwards.
    A(f'<rect x="{OL - 16:.0f}" y="{T - 22:.0f}" width="{OR_ - OL + 32:.0f}" '
      f'height="{B - T + 40:.0f}" rx="10" fill="#060e20" fill-opacity="0.55" '
      f'stroke="#16283f" stroke-width="1"/>')

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
        # The forged edge is dashed and the dashes march, so it reads as a
        # claim being PUSHED rather than a line that is merely there. Every
        # other edge is a fact and stays still. It runs between two classes
        # like all the others and is not pulled back from either: nothing is
        # stopped HERE. What stops is the certificate carrying it, and that is
        # drawn at the checker, further down.
        ax, ay, bx, by = pt[i][0], pt[i][1], pt[j][0], pt[j][1]
        pipe = nodes[i] in TOOLS and nodes[j] in TOOLS
        if pipe and w != "rejected":
            # Pipeline edges are a PROCESS and a process has a direction. Drawn
            # as bare segments running under the dots at both ends, the three
            # columns could be read right to left as easily as left to right.
            vx, vy = bx - ax, by - ay
            L = math.hypot(vx, vy) or 1.0
            ux_, uy_ = vx / L, vy / L
            ax, ay = ax + ux_ * (rad(i) + 4.0), ay + uy_ * (rad(i) + 4.0)
            bx, by = bx - ux_ * (rad(j) + 8.5), by - uy_ * (rad(j) + 8.5)
        head = (f'<line x1="{ax:.1f}" y1="{ay:.1f}" x2="{bx:.1f}" y2="{by:.1f}" '
                f'stroke="{COLOR[w]}" stroke-width="{WIDTH[w] * (0.7 + 0.8 * d):.2f}" '
                f'opacity="{op:.2f}"')
        if w == "rejected":
            layers[w].append(
                head + ' stroke-dasharray="7 5">'
                '<animate attributeName="stroke-dashoffset" dur="0.9s" '
                'repeatCount="indefinite" values="0;-24"/></line>')
        else:
            layers[w].append(head + '/>')
        if pipe and w != "rejected":
            nx_, ny_ = -uy_, ux_
            layers[w].append(
                f'<path d="M{bx + ux_ * 7.0:.1f} {by + uy_ * 7.0:.1f} '
                f'L{bx + nx_ * 3.4:.1f} {by + ny_ * 3.4:.1f} '
                f'L{bx - nx_ * 3.4:.1f} {by - ny_ * 3.4:.1f} Z" '
                f'fill="{COLOR[w]}" opacity="{min(1.0, op + 0.25):.2f}"/>')

    A(f'<g opacity="{REST_D}">'
      + anim("opacity", f"{REST_D};{REST_D};{LIT_D};{LIT_D};{REST_D};{REST_D}",
             kt(0, 3.4, 4.2, 9.6, 10.4, CYCLE)))
    A("".join(layers["certified"]))
    A('</g>')
    A(f'<g opacity="{REST_A}">'
      + anim("opacity", f"{REST_A};{LIT_A};{LIT_A};{REST_A};{REST_A}",
             kt(0, 0.6, 2.8, 3.4, CYCLE)))
    A("".join(layers["asserted"]))
    A('</g>')
    # The forged edge last of the three, so nothing is drawn over it.
    A(f'<g opacity="{REST_F}">'
      + anim("opacity", f"{REST_F};{REST_F};{LIT_F};{LIT_F};{REST_F};{REST_F}",
             kt(0, 13.0, 13.5, 16.4, 17.0, CYCLE)))
    A("".join(layers["rejected"]))
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
      + anim("opacity", "0.85;0.85;1;1;0.85;0.85", kt(0, 3.4, 4.2, 9.6, 10.4, CYCLE)))
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
              + anim("r", f"{r*3.0:.1f};{r*3.6:.1f};{r*3.0:.1f};{r*4.6:.1f};{r*3.4:.1f};{r*3.0:.1f}",
                     kt(0, 2.0, 4.0, 7.6, 9.6, CYCLE)) + '</circle>')
            A(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.1f}" fill="{C_LEAN}"/>')
        elif nodes[i] in FILES:
            # Four of the ten things in the pipeline are FILES and six are
            # PROGRAMS, and drawing all ten as identical dots hid the one
            # distinction the picture is about: a certificate is a file you can
            # hand to a checker, and a prover is a thing with an opinion. A
            # sheet with a turned corner says file without a word of legend.
            fwd, fht = r * 1.5, r * 1.85
            fold = fwd * 0.42
            A(f'<path d="M{x - fwd:.1f} {y - fht:.1f} '
              f'H{x + fwd - fold:.1f} L{x + fwd:.1f} {y - fht + fold:.1f} '
              f'V{y + fht:.1f} H{x - fwd:.1f} Z" fill="url(#tool)" '
              f'stroke="#0b1428" stroke-width="0.8" '
              f'opacity="{0.72 + 0.28 * depth[i]:.2f}"/>')
            A(f'<path d="M{x + fwd - fold:.1f} {y - fht:.1f} '
              f'V{y - fht + fold:.1f} H{x + fwd:.1f}" fill="none" '
              f'stroke="#0b1428" stroke-width="0.9" opacity="0.9"/>')
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
        # A file is drawn as a sheet, which is wider and taller than the dot it
        # replaced, so a label offset by the dot radius lands ON it.
        r_ = rad(i) * (1.6 if nodes[i] in FILES else 1.0)
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

    # ── The verification layer, working ────────────────────────────────
    #
    # The provers used to be four magenta dots with names. A reader was told the
    # first-order family exists and never shown it do anything, which left the
    # most important distinction on the page — Lean DECIDES, the provers OPINE —
    # as a sentence in the legend rather than as something visible.
    #
    # Each judge now gets a verdict badge that arrives on its own beat, and the
    # badge says which KIND of answer it is. The two that read the certificate
    # say so in green. The four that read `problem.tsv`, a different artefact
    # entirely, say `opinion` in magenta, and they arrive together in beat 4,
    # because that is the point: four independent programs agreeing is still
    # four opinions.
    JUDGES = [
        (LEAN, TX["j_cert"], C_LEAN, 6.8, 9.6),
        ("Isabelle/HOL", TX["j_same"], C_LEAN, 7.4, 9.6),
        ("Vampire", TX["j_cert"] if vampire_certified else TX["j_op"],
         C_LEAN if vampire_certified else C_TOOL, 10.0, 13.0),
        (FORES, TX["j_checks"] if vampire_certified else TX["j_noproof"],
         C_LEAN if vampire_certified else C_TOOL, 10.4, 13.0),
        ("E", TX["j_op"], C_TOOL, 10.3, 13.0),
        ("Z3", TX["j_op"], C_TOOL, 10.6, 13.0),
        ("Mace4", TX["j_op"], C_TOOL, 10.9, 13.0),
    ]
    for jname, verdict, col, t0, t1 in JUDGES:
        ji = idx[jname]
        jr = rad(ji)
        bw = len(verdict) * 5.4 + 12
        # Badges go through the SAME collision list the labels use. Stacked
        # straight above their nodes they piled on top of one another and on
        # the names beside them, because the four provers sit close together:
        # the badge saying which kind of answer this is was the thing hidden.
        cands = [
            (pt[ji][0] - bw / 2, pt[ji][1] - jr - 13),
            (pt[ji][0] - bw / 2, pt[ji][1] + jr + 20),
            (pt[ji][0] + jr + 7, pt[ji][1] - jr - 6),
            (pt[ji][0] - jr - 7 - bw, pt[ji][1] - jr - 6),
            (pt[ji][0] - bw / 2, pt[ji][1] - jr - 30),
        ]
        bx, by = cands[-1]
        for n_, (cx_, cy_) in enumerate(cands):
            box = (cx_ - 2, cy_ - 11, cx_ + bw + 2, cy_ + 4)
            if all(box[2] < o[0] or box[0] > o[2] or box[3] < o[1] or box[1] > o[3]
                   for o in placed) or n_ == len(cands) - 1:
                bx, by = cx_, cy_
                placed.append(box)
                break
        # Rested at 0.4 rather than hidden, like every other layer in this
        # file. A still frame then shows `certificate` on Lean and `opinion` on
        # the four provers, which is the whole argument of the picture and the
        # thing a reader who never watches it would otherwise never see.
        A('<g opacity="0.4">'
          + anim("opacity", "0.4;0.4;1;1;0.4;0.4",
                 kt(0, t0, t0 + 0.35, t1 - 0.3, t1, CYCLE)))
        A(f'<rect x="{bx:.1f}" y="{by - 10:.1f}" width="{bw:.1f}" height="14" '
          f'rx="7" fill="#020617" stroke="{col}" stroke-width="1.1" opacity="0.95"/>')
        A(f'<text x="{bx + bw / 2:.1f}" y="{by:.1f}" text-anchor="middle" font-size="9.5" '
          f'font-weight="700" fill="{col}">{verdict}</text>')
        A('</g>')
        # A ring on the judge itself, so the eye goes to the node and not only
        # to the label beside it.
        A(f'<circle cx="{pt[ji][0]:.1f}" cy="{pt[ji][1]:.1f}" r="{jr:.1f}" fill="none" '
          f'stroke="{col}" stroke-width="2" opacity="0">'
          + anim("r", f"{jr:.1f};{jr:.1f};{jr * 2.8:.1f};{jr * 2.8:.1f}",
                 kt(0, t0, t0 + 0.55, CYCLE))
          + anim("opacity", "0;0;0.85;0;0", kt(0, t0, t0 + 0.18, t0 + 0.95, CYCLE)) +
          '</circle>')

    # ── What an opinion is FOR ──────────────────────────────────────────
    #
    # The four provers hung off `problem.tsv` with nothing leaving them, so the
    # drawing said their answers go nowhere. That is not what the code does.
    # `src/fol_solve.rs` carries a `Disagreement` -- "a disagreement between two
    # things that were supposed to agree; not a verdict about the ontology, and
    # not a footnote either" -- and it is set "when a checker and the thing it
    # checks disagreed. Stop the line."
    #
    # So an opinion never becomes warrant and can still halt the pipeline. That
    # is a different arrow from the certified one and it is drawn differently:
    # it leaves the provers, it arrives at the certificate, and it is dotted,
    # because what travels along it is a question and not a proof.
    pv = [idx[n] for n in ("Vampire", "E", "Z3", "Mace4")]
    ci0 = idx["certificate"]
    fx0 = min(pt[i][0] for i in pv) - 46.0
    fy0 = sum(pt[i][1] for i in pv) / len(pv)
    A(f'<g opacity="0.34">'
      + anim("opacity", "0.34;0.34;0.9;0.9;0.34;0.34",
             kt(0, 11.4, 11.9, 12.7, 13.0, CYCLE)))
    for i in pv:
        A(f'<path d="M{pt[i][0] - rad(i) - 4:.1f} {pt[i][1]:.1f} '
          f'Q{fx0:.1f} {pt[i][1]:.1f} {fx0:.1f} {fy0:.1f}" fill="none" '
          f'stroke="{C_TOOL}" stroke-width="0.9" stroke-dasharray="2 3" opacity="0.8"/>')
    A(f'<path d="M{fx0:.1f} {fy0:.1f} Q{fx0:.1f} {pt[ci0][1] + 30:.1f} '
      f'{pt[ci0][0]:.1f} {pt[ci0][1] + rad(ci0) + 6:.1f}" fill="none" '
      f'stroke="{C_TOOL}" stroke-width="1.5" stroke-dasharray="4 4"/>')
    dtxt = TX["disagree"]
    dw = len(dtxt) * 5.0 + 12
    # Candidates, like every other badge on this drawing. This one used to take
    # a single computed point, and that point sat on top of the `problem.tsv`
    # label: two pieces of writing, both visible at the same instant, in the
    # same place. Nothing else here places blind, and the overlap guard in
    # `tests/knowledge_graph_asset_test.rs` now refuses the whole asset if one
    # does.
    dcands = [
        (fx0 - dw - 8, fy0 + 4),
        (fx0 - dw - 8, fy0 - 22),
        (fx0 - dw - 8, fy0 + 30),
        (fx0 + 10, fy0 + 30),
        (fx0 - dw / 2, fy0 + 52),
    ]
    dx, dy = dcands[-1]
    for n_, (ccx, ccy) in enumerate(dcands):
        box = (ccx - 3, ccy - 12, ccx + dw + 3, ccy + 5)
        if all(box[2] < o[0] or box[0] > o[2] or box[3] < o[1] or box[1] > o[3]
               for o in placed) or n_ == len(dcands) - 1:
            dx, dy = ccx, ccy
            placed.append(box)
            break
    A(f'<rect x="{dx:.1f}" y="{dy - 10:.1f}" width="{dw:.1f}" height="14" rx="7" '
      f'fill="#020617" stroke="{C_TOOL}" stroke-width="1" opacity="0.95"/>')
    A(f'<text x="{dx + dw / 2:.1f}" y="{dy:.1f}" text-anchor="middle" font-size="8.8" '
      f'font-weight="700" fill="{C_TOOL}">{dtxt}</text>')
    A('</g>')

    # ── Beat five: the claim in the graph, and the refusal at the checker ─
    #
    # Two halves of one event, and both have to be visible or the beat says
    # nothing. The forged claim is an edge among the derived ones, so it is
    # marked WHERE IT IS: a halo on it, and the badge beside it, in the cloud
    # the viewer has been watching for four beats.
    #
    # The refusal is at the checker, on the certificate that carried the claim.
    # The same `certificate -> Lean` segment that lit green in beat three now
    # runs red and stops at a bar short of the node, which is the argument:
    # same path, same reader, different content, different answer.
    li = idx[LEAN]
    lr = rad(li)
    ci = idx["certificate"]
    cvx, cvy = pt[li][0] - pt[ci][0], pt[li][1] - pt[ci][1]
    cL = math.hypot(cvx, cvy) or 1.0
    ux, uy = cvx / cL, cvy / cL
    sx = pt[ci][0] + ux * (rad(ci) + 4.0)
    sy = pt[ci][1] + uy * (rad(ci) + 4.0)
    ex = pt[li][0] - ux * (lr + 15.0)
    ey = pt[li][1] - uy * (lr + 15.0)
    # Rested at 0.32 rather than hidden. The green `certificate -> Lean`
    # edge underneath rests too, and the pair at rest is the claim of the whole
    # figure: this path carries both, and the reader is told which is which by
    # which one is lit.
    A('<g opacity="0.32">'
      + anim("opacity", "0.32;0.32;1;1;0.32;0.32", kt(0, 13.2, 13.7, 16.6, 17.0, CYCLE)))
    A(f'<line x1="{sx:.1f}" y1="{sy:.1f}" x2="{ex:.1f}" y2="{ey:.1f}" '
      f'stroke="{C_REJECT}" stroke-width="2.6" stroke-dasharray="7 5">'
      '<animate attributeName="stroke-dashoffset" dur="0.9s" '
      'repeatCount="indefinite" values="0;-24"/></line>')
    nx_, ny_ = -uy, ux
    A(f'<line x1="{ex + nx_ * 11:.1f}" y1="{ey + ny_ * 11:.1f}" '
      f'x2="{ex - nx_ * 11:.1f}" y2="{ey - ny_ * 11:.1f}" stroke="{C_REJECT}" '
      f'stroke-width="3.4" stroke-linecap="round"/>')
    A('</g>')

    if forged is not None:
        fi, fj, _ = edges[forged]
        fmx = (pt[fi][0] + pt[fj][0]) / 2.0
        fmy = (pt[fi][1] + pt[fj][1]) / 2.0
        flen = math.hypot(pt[fi][0] - pt[fj][0], pt[fi][1] - pt[fj][1])
        # A halo along the edge rather than a ring on a node: the forgery is
        # the LINE, and a ring would point at a class that did nothing wrong.
        A('<g opacity="0.3">'
          + anim("opacity", "0.3;0.3;0.85;0.85;0.3;0.3",
                 kt(0, 13.0, 13.5, 16.6, 17.0, CYCLE)))
        A(f'<line x1="{pt[fi][0]:.1f}" y1="{pt[fi][1]:.1f}" '
          f'x2="{pt[fj][0]:.1f}" y2="{pt[fj][1]:.1f}" stroke="{C_REJECT}" '
          f'stroke-width="9" stroke-linecap="round" opacity="0.22"/>')
        A(f'<circle cx="{fmx:.1f}" cy="{fmy:.1f}" r="{max(13.0, flen * 0.34):.1f}" '
          f'fill="none" stroke="{C_REJECT}" stroke-width="1.4" opacity="0.7"/>')
        A('</g>')

        ftxt = TX["forged"]
        fw = len(ftxt) * 5.4 + 12
        cands = []
        for dy_ in (-26.0, 26.0, -44.0, 44.0):
            for dx_ in (0.0, -46.0, 46.0):
                cands.append((fmx + dx_, fmy + dy_))
        fx, fy = cands[0][0] - fw / 2, cands[0][1]
        for n_, (ccx, ccy) in enumerate(cands):
            bx_, by_ = ccx - fw / 2, ccy
            box = (bx_ - 3, by_ - 12, bx_ + fw + 3, by_ + 5)
            if all(box[2] < o[0] or box[0] > o[2] or box[3] < o[1] or box[1] > o[3]
                   for o in placed) or n_ == len(cands) - 1:
                fx, fy = bx_, by_
                placed.append(box)
                break
        A('<g opacity="0.4">'
          + anim("opacity", "0.4;0.4;1;1;0.4;0.4", kt(0, 13.2, 13.7, 16.6, 17.0, CYCLE)))
        A(f'<rect x="{fx:.1f}" y="{fy - 10:.1f}" width="{fw:.1f}" height="14" rx="7" '
          f'fill="#020617" stroke="{C_REJECT}" stroke-width="1.1" opacity="0.95"/>')
        A(f'<text x="{fx + fw / 2:.1f}" y="{fy:.1f}" text-anchor="middle" font-size="9.5" '
          f'font-weight="700" fill="{C_REJECT}">{ftxt}</text>')
        A('</g>')

    # And the verdict, at the checker.
    rtxt = TX["refused"]
    rw = len(rtxt) * 5.4 + 12
    rx, ry = pt[li][0] - rw / 2, pt[li][1] + lr + 22
    for ccx, ccy in [(pt[li][0] - rw / 2, pt[li][1] + lr + 22),
                     (pt[li][0] + lr + 8, pt[li][1] + lr + 12),
                     (pt[li][0] - rw / 2, pt[li][1] + lr + 40)]:
        box = (ccx - 3, ccy - 12, ccx + rw + 3, ccy + 5)
        if all(box[2] < o[0] or box[0] > o[2] or box[3] < o[1] or box[1] > o[3]
               for o in placed):
            rx, ry = ccx, ccy
            placed.append(box)
            break
    A('<g opacity="0.4">'
      + anim("opacity", "0.4;0.4;1;1;0.4;0.4", kt(0, 13.6, 14.0, 16.6, 17.0, CYCLE)))
    A(f'<rect x="{rx:.1f}" y="{ry - 10:.1f}" width="{rw:.1f}" height="14" rx="7" '
      f'fill="#020617" stroke="{C_REJECT}" stroke-width="1.1" opacity="0.95"/>')
    A(f'<text x="{rx + rw / 2:.1f}" y="{ry:.1f}" text-anchor="middle" font-size="9.5" '
      f'font-weight="700" fill="{C_REJECT}">{rtxt}</text>')
    A('</g>')

    # ── One point becoming the graph ────────────────────────────────────
    #
    # Beat one used to brighten every asserted edge at once, which is true but
    # says "here is a graph" rather than "here is a graph being read". A wave
    # out of the busiest class says the second, and says it without hiding
    # anything: the rings are drawn ON TOP of a cloud that is always there, so
    # a renderer sampling t=0 still gets the whole picture. Unfilled, which is
    # what makes them a sweep rather than a layer.
    # ── The unasked question ────────────────────────────────────────────
    #
    # Beat 6. The question `mu_s subClassOf mu_o` is drawn as a dashed amber
    # line between the two nodes, faint at rest because the still frame must
    # show every content layer. When the beat runs it brightens (the question
    # is asked), 無 appears over it (the engine returns it), and it dims again.
    # It never turns red: nothing was refuted. The file said nothing.
    if mu and mu_s in idx and mu_o in idx:
        qi, qj = idx[mu_s], idx[mu_o]
        qmx, qmy = (pt[qi][0] + pt[qj][0]) / 2.0, (pt[qi][1] + pt[qj][1]) / 2.0
        A('<g opacity="0.3">'
          + anim("opacity", "0.3;0.3;0.95;0.95;0.3;0.3", kt(0, 17.0, 17.4, 19.0, 19.8, CYCLE)))
        A(f'<line x1="{pt[qi][0]:.1f}" y1="{pt[qi][1]:.1f}" x2="{pt[qj][0]:.1f}" '
          f'y2="{pt[qj][1]:.1f}" stroke="{C_MU}" stroke-width="2.2" stroke-dasharray="3 4"/>')
        A('</g>')
        A(f'<text x="{qmx:.1f}" y="{qmy + 9:.1f}" text-anchor="middle" font-size="26" '
          f'font-weight="800" fill="{C_MU}" opacity="0">'
          + anim("opacity", "0;0;1;1;0;0", kt(0, 18.6, 19.0, 20.6, 21.0, CYCLE))
          + '無</text>')
        mtxt = TX["mu_badge"].format(s=short(mu_s), why=mu_line)
        mw = len(mtxt) * 5.4 + 12
        mcands = [(qmx - mw / 2, qmy - 24), (qmx - mw / 2, qmy + 34), (qmx + 30, qmy - 6), (qmx - mw - 30, qmy - 6)]

        # INSIDE THE PANEL, always. The candidates used to be filtered on label
        # overlap alone, and the last one was taken unconditionally when none
        # was clear. That was invisible until the camera moved: the badge
        # follows the node it is about, the node moved right, and half the
        # badge ran off the edge of the drawing.
        PX0, PY0, PX1, PY1 = OL - 12, T - 18, OR_ + 12, B + 34

        def _box(cx_, cy_):
            return (cx_ - 3, cy_ - 12, cx_ + mw + 3, cy_ + 5)

        def _inside(b):
            return b[0] >= PX0 and b[2] <= PX1 and b[1] >= PY0 and b[3] <= PY1

        def _clear(b):
            return all(b[2] < o[0] or b[0] > o[2] or b[3] < o[1] or b[1] > o[3] for o in placed)

        # Clear AND inside first; then merely inside; then slid back inside,
        # which is better than drawn off the edge.
        pick = next((c for c in mcands if _inside(_box(*c)) and _clear(_box(*c))), None)
        if pick is None:
            pick = next((c for c in mcands if _inside(_box(*c))), None)
        if pick is None:
            cx_, cy_ = mcands[0]
            pick = (min(max(cx_, PX0 + 3), PX1 - mw - 3), min(max(cy_, PY0 + 12), PY1 - 5))
        mx, my = pick
        final = _box(mx, my)
        if not _inside(final):
            raise SystemExit(
                f"the unasked-question badge lands at {final} and the panel is "
                f"{(PX0, PY0, PX1, PY1)}. It would be drawn off the edge of the figure.")
        placed.append(final)
        A('<g opacity="0.4">'
          + anim("opacity", "0.4;0.4;1;1;0.4;0.4", kt(0, 18.6, 19.0, 20.6, 21.0, CYCLE)))
        A(f'<rect x="{mx:.1f}" y="{my - 10:.1f}" width="{mw:.1f}" height="14" rx="7" '
          f'fill="#020617" stroke="{C_MU}" stroke-width="1.1" opacity="0.95"/>')
        A(f'<text x="{mx + mw / 2:.1f}" y="{my:.1f}" text-anchor="middle" font-size="9.5" '
          f'font-weight="700" fill="{C_MU}">{mtxt}</text>')
        A('</g>')
        # "no judge is asked": the four provers and the checkers stay dark in this beat.
    elif mu:
        raise SystemExit(f"the unasked question names {mu_s} or {mu_o}, which the drawing does not carry")

    seed = max(
        (idx[ont[g]] for g in comps[0]),
        key=lambda i: deg[i],
    )
    sx0, sy0 = pt[seed][0], pt[seed][1]
    reach = max(
        math.hypot(pt[idx[ont[g]]][0] - sx0, pt[idx[ont[g]]][1] - sy0)
        for g in comps[0]
    )
    for n_ in range(3):
        t0 = 0.6 + n_ * 0.62
        A(f'<circle cx="{sx0:.1f}" cy="{sy0:.1f}" r="4" fill="none" '
          f'stroke="{C_ASSERT}" stroke-width="1.5" opacity="0">'
          + anim("r", f"4;4;{reach:.0f};{reach:.0f}", kt(0, t0, t0 + 1.9, CYCLE))
          + anim("opacity", "0;0;0.55;0;0", kt(0, t0, t0 + 0.25, t0 + 1.9, CYCLE))
          + '</circle>')
    # The seed itself, held for the beat, so the eye has somewhere to start.
    A(f'<circle cx="{sx0:.1f}" cy="{sy0:.1f}" r="{rad(seed) + 4:.1f}" fill="none" '
      f'stroke="{C_ASSERT}" stroke-width="1.6" opacity="0">'
      + anim("opacity", "0;0;0.9;0.9;0;0", kt(0, 0.6, 0.9, 3.1, 3.4, CYCLE))
      + '</circle>')

    # ── Each beat lights the nodes that are IN it ───────────────────────
    #
    # The beats used to light EDGE LAYERS, and the asserted and certified
    # layers live almost entirely in the ontology on the right. So during the
    # first two steps the pipeline on the left, which is what the captions are
    # talking about, did not change at all. A reader watching the named actors
    # saw nothing happen and then a red line at the end.
    #
    # A held ring, not a one-off pulse: the ring is up for as long as the step
    # is running, so at any instant the picture answers "who is acting now".
    SPOT = [
        (["ies-core.ttl"], 0),
        (["certificate", "problem.tsv"], 1),
        ([LEAN, "Isabelle/HOL"], 2),
        (["Vampire", "E", "Z3", "Mace4", FORES], 3),
    ]
    if mu and mu_s in idx:
        SPOT.append(([mu_s], 5))
    for names, bn in SPOT:
        col, _, t0, t1 = BEATS[bn]
        for nm in names:
            si = idx[nm]
            sr = rad(si) + 4.0
            A(f'<circle cx="{pt[si][0]:.1f}" cy="{pt[si][1]:.1f}" r="{sr:.1f}" '
              f'fill="none" stroke="{col}" stroke-width="1.6" opacity="0">'
              + anim("opacity", "0;0;0.9;0.9;0;0",
                     kt(0, t0, t0 + 0.3, t1 - 0.35, t1, CYCLE)) +
              '</circle>')

    n_asserted = len(asserted)
    # Counted from the rows, because the heading states it.
    n_sub = sum(1 for r in asserted if r[1] == SUBCLASS)
    n_type = sum(1 for r in asserted if r[1] == TYPE)
    n_derived = len(derivations)
    rules = ", ".join(f"{r} x{c}" for r, c in sorted(by_rule.items(), key=lambda kv: -kv[1]))
    A(f'<text x="34" y="44" font-size="17" font-weight="800" fill="#f8fafc">'
      f'{TX["headline"]}</text>')

    # The running step, said in words, at the top. It used to be said only on
    # the rail at the bottom of the canvas: ten-point type, six hundred pixels
    # below the thing it described, while the reader's eye was on the pipeline.
    # A viewer could watch the whole loop and never learn what the five stages
    # WERE. The rail still shows where you are in the sequence; this says what
    # is happening, and it says it next to the title.
    # The captions all sit at one position, so only one may be visible at a
    # time, INCLUDING while it is fading. The previous ramps overlapped: a
    # caption began fading in 0.35s before the one before it had finished
    # fading out, so every transition drew two sentences through each other for
    # about a third of a second, forever. Measured in a browser by sampling
    # setCurrentTime: 15 such moments, the outgoing text at opacity 0.67 under
    # the incoming one at 0.20.
    #
    # Now a caption fades out over [t1-0.3, t1] and the next fades in over
    # [t0, t0+0.3], and consecutive beats share t1 == t0, so the ramps touch at
    # a point where both are 0 and never overlap. The first one is simply on
    # from t=0, because a still frame samples t=0 and must show a caption.
    for n_, (col, text, t0, t1) in enumerate(BEATS):
        first = "1" if n_ == 0 else "0"
        values = "1;1;1;1;0;0" if n_ == 0 else "0;0;1;1;0;0"
        times = (kt(0, 0, 0, t1 - 0.3, t1, CYCLE) if n_ == 0
                 else kt(0, t0, t0 + 0.3, t1 - 0.3, t1, CYCLE))
        A(f'<text x="34" y="68" font-size="14.5" font-weight="700" fill="{col}" '
          f'opacity="{first}">'
          + anim("opacity", values, times)
          + f'{text}</text>')

    A(f'<text x="34" y="88" font-size="10.5" fill="#64748b">'
      f'{TX["sub"].format(n=len(nodes), e=len(edges), a=n_asserted, d=n_derived, rules=rules)}</text>')

    # ── B. what each column of the pipeline IS ──────────────────────────
    #
    # Three placed columns carry an argument only if the reader can see that
    # they are columns. Without headings the left half read as a scatter of
    # pink dots that happened to line up.
    for hx, htxt in zip((86, 196, 318), TX["cols"]):
        A(f'<text x="{hx}" y="108" text-anchor="middle" font-size="9" '
          f'font-weight="700" fill="#475569" letter-spacing="1.4">{htxt.upper()}</text>')
    # And what the right-hand half is, which nothing said. A reader met a cloud
    # of unnamed dots and had to guess whether it was data, a result or decor.
    #
    # It says SUBCLASS HIERARCHY and it counts TREES, and both words are the
    # correction of a claim this heading used to make. It read "WHAT THE FILE
    # CONTAINS - 139 CLASSES IN 6 DISJOINT PIECES", which is false. The file
    # holds 1,083 triples over 22 predicates and this picture draws exactly one
    # of them, the 144 `rdfs:subClassOf`. On that one predicate ies-core does
    # fall into six pieces; on the file's own other relations three of them
    # join up and it falls into four. The six are the top branches of one class
    # hierarchy, each a tree with a single root except the largest, which has
    # four. Reporting a consequence of the drawing as a property of the data is
    # the mistake this whole figure is supposed to be an argument against.
    A(f'<text x="{(OL + OR_) / 2:.0f}" y="108" text-anchor="middle" font-size="9" '
      f'font-weight="700" fill="#475569" letter-spacing="1.4">'
      f'{TX["graphhead"].format(n=len(ont))}</text>')

    # ── The step rail ──────────────────────────────────────────────────
    #
    # One caption line told a reader WHICH step was running and nothing about
    # where it sat in the sequence: no sense of how many there were, what came
    # before, or what was still to come. The rail shows all five at once with
    # the running one lit, so the picture reads as a process rather than as a
    # caption that keeps changing.
    #
    # Every stage is drawn at a resting opacity that is never zero, for the
    # reason the header gives: a still renderer samples t=0, and a rail whose
    # inactive stages are invisible degrades to one lonely word.
    rail_y = H - 196
    rail_x, rail_w = 34.0, W - 68.0
    seg = rail_w / len(BEATS)
    A(f'<line x1="{rail_x:.1f}" y1="{rail_y - 16:.1f}" x2="{rail_x + rail_w:.1f}" '
      f'y2="{rail_y - 16:.1f}" stroke="#1e3a5f" stroke-width="1.5"/>')
    # The travelling marker, one element, which is what makes the rail read as
    # a clock rather than as five independent lamps.
    marks = ";".join(f"{rail_x + seg * (n_ + 0.5):.1f}" for n_ in range(len(BEATS)))
    times = ";".join(f"{(t0 - 0.3) / CYCLE:.4f}" for _, _, t0, _ in BEATS)
    A(f'<circle cy="{rail_y - 16:.1f}" r="4.5" fill="#e2e8f0" cx="{rail_x + seg * 0.5:.1f}">'
      f'<animate attributeName="cx" dur="{CYCLE}s" repeatCount="indefinite" '
      f'values="{marks}" keyTimes="{times}" calcMode="linear"/></circle>')
    for n_, (col, text, t0, t1) in enumerate(BEATS):
        cx = rail_x + seg * (n_ + 0.5)
        num = text.split(" · ")[0]
        label = text.split(" · ", 1)[1]
        # The tick, lit for its own window.
        A(f'<circle cx="{cx:.1f}" cy="{rail_y - 16:.1f}" r="3" fill="{col}" opacity="0.30">'
          + anim("opacity", "0.30;0.30;1;1;0.30;0.30",
                 kt(0, t0, t0 + 0.3, t1 - 0.3, t1, CYCLE))
          + anim("r", "3;3;5.5;5.5;3;3", kt(0, t0, t0 + 0.3, t1 - 0.3, t1, CYCLE)) +
          '</circle>')
        A(f'<text x="{cx:.1f}" y="{rail_y + 4:.1f}" text-anchor="middle" font-size="11" '
          f'font-weight="800" fill="{col}" opacity="0.38">{num}</text>')
        # The wording, which only the running stage carries, because five full
        # sentences at once is a wall.
        # The first and last labels hug the rail's ends rather than centring
        # on their segment, or a long last caption runs off the right edge.
        lx, anchor = cx, "middle"
        if n_ == 0:
            lx, anchor = rail_x, "start"
        elif n_ == len(BEATS) - 1:
            lx, anchor = rail_x + rail_w, "end"
        A(f'<text x="{lx:.1f}" y="{rail_y + 20:.1f}" text-anchor="{anchor}" font-size="10.5" '
          f'fill="{col}" opacity="0">'
          + anim("opacity", "0;0;1;1;0;0", kt(0, t0, t0 + 0.3, t1 - 0.3, t1, CYCLE))
          + f'{label}</text>')

    # ── The legend, which is the component's Legend ──────────────────────
    #
    # Same title, same three rows, same wording, same footer. Counted from the
    # edges drawn above rather than typed, exactly as the component counts them
    # from the links on screen.
    n_a = sum(1 for _, _, w in edges if w == "asserted")
    n_c = sum(1 for _, _, w in edges if w == "certified")
    n_r = sum(1 for _, _, w in edges if w == "rejected")
    ly = H - 152
    lw = 560.0          # where the counting column ends and the meaning begins
    A(f'<rect x="28" y="{ly}" width="{W - 56}" height="132" rx="12" fill="#030a1c" '
      f'opacity="0.94" stroke="#1e3a5f"/>')
    A(f'<text x="46" y="{ly+24}" font-size="12" font-weight="800" fill="#34d399" '
      f'letter-spacing="1.4">{TX["legend_head"]}</text>')
    A(f'<text x="300" y="{ly+24}" font-size="11.5" fill="#64748b">'
      f'{TX["legend_file"].format(n=n_asserted)}</text>')
    A(f'<line x1="40" y1="{ly+33}" x2="{lw - 12:.0f}" y2="{ly+33}" stroke="#1e3a5f"/>')
    rows = [(C_ASSERT, TX["asserted"][0], n_a, TX["asserted"][1]),
            (C_CERT, TX["certified"][0], n_c, TX["certified"][1]),
            (C_REJECT, TX["rejected"][0], n_r, TX["rejected"][1])]
    if mu:
        rows.append((C_MU, TX["unasked"][0], 1, TX["unasked"][1]))
    for m, (col, label, count, means) in enumerate(rows):
        yy = ly + 51 + m * 18
        A(f'<rect x="46" y="{yy-4}" width="22" height="3" fill="{col}"/>')
        A(f'<text x="78" y="{yy}" font-size="11" font-weight="700" fill="{col}" '
          f'letter-spacing="0.7">{label}</text>')
        A(f'<text x="168" y="{yy}" font-size="11" font-weight="700" fill="#e2e8f0" '
          f'text-anchor="end">{count}</text>')
        must_fit(means, 11, (lw - 12) - 180 - 6, f"legend row {label!r}")
        A(f'<text x="180" y="{yy}" font-size="11" fill="#94a3b8">{means}</text>')
    # The distinction the whole figure exists to make used to be the tail of a
    # single run-on line along the bottom of the card, which is where a reader
    # stops reading. It is the argument, so it gets its own column, and that
    # column also fills the third of the canvas the card was leaving empty.
    A(f'<line x1="{lw + 12:.0f}" y1="{ly+14}" x2="{lw + 12:.0f}" y2="{ly+118}" '
      f'stroke="#1e3a5f"/>')
    A(f'<text x="{lw + 34:.0f}" y="{ly+24}" font-size="12" font-weight="800" '
      f'fill="#94a3b8" letter-spacing="1.4">{TX["worth_head"]}</text>')
    if vampire_certified:
        verdicts = [
            (C_CERT, TX["w_cert"],
             TX["w_cert1_fo"].format(n=n_certified_edges),
             TX["w_cert2_fo"].format(t=fo_theorem)),
            (C_TOOL, TX["w_op"], TX["w_op1_fo"], TX["w_op2"]),
        ]
    else:
        verdicts = [
            (C_CERT, TX["w_cert"], TX["w_cert1"].format(n=n_certified_edges), TX["w_cert2"]),
            (C_TOOL, TX["w_op"], TX["w_op1"], TX["w_op2"]),
        ]
    if mu:
        verdicts.append((C_MU, TX["w_mu"],
                         TX["w_mu1"].format(s=short(mu_s), o=short(mu_o), why=mu_line),
                         TX["w_mu2"].format(s=short(mu_s), why=mu_line)))
    for m, (col, word, l1, l2) in enumerate(verdicts):
        yy = ly + 48 + m * 30
        A(f'<circle cx="{lw + 40:.0f}" cy="{yy - 4:.1f}" r="4" fill="{col}"/>')
        A(f'<text x="{lw + 52:.0f}" y="{yy}" font-size="11" font-weight="700" '
          f'fill="{col}">{word}</text>')
        # The right edge of the panel is the budget, not the edge of the canvas:
        # a sentence that stops at 1090 is inside the drawing and outside the box
        # that frames it, which is what a reader sees.
        budget = (W - 28) - (lw + 130) - 6
        must_fit(l1, 10, budget, f"verdict {word!r} line 1")
        must_fit(l2, 10, budget, f"verdict {word!r} line 2")
        A(f'<text x="{lw + 130:.0f}" y="{yy - 5:.0f}" font-size="10" fill="#94a3b8">{l1}</text>')
        A(f'<text x="{lw + 130:.0f}" y="{yy + 7:.0f}" font-size="10" fill="#64748b">{l2}</text>')
    A('</svg>')

    # No unsubstituted placeholder may reach the drawing. One did: `w_mu2` gained
    # {s} and {why} while its call site still passed no arguments, so the figure
    # published the literal text "{s} is {why}". A translation table makes this
    # easy to do and impossible to see in the source, since the format string and
    # the call are in different places and neither is wrong on its own.
    svg_text = "\n".join(out)
    import re as _re
    stray = _re.findall(r">[^<>]*(\{[a-z_]+\})[^<>]*<", svg_text)
    if stray:
        raise SystemExit(
            "the drawing carries unsubstituted placeholders, so a format string was "
            f"rendered rather than filled: {sorted(set(stray))}"
        )

    with open(out_path, "w") as f:
        f.write(svg_text)
    print(f"wrote {out_path}: {len(nodes)} nodes, {len(a_edges)} asserted and "
          f"{len(d_edges)} derived edges, from {n_asserted} asserted and {n_derived} derived triples")


if __name__ == "__main__":
    main(*sys.argv[1:7])
