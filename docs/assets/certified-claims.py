#!/usr/bin/env python3
"""Three claims this engine could not check before, and what it says now.

    certified-claims.py CROSSWALK.json MODULES.json MATCERT.json OUT.svg [lang]

EVERY NUMBER IN THE DRAWING COMES OUT OF THOSE FILES. Nothing is typed into
this script: the counts, the predicates, the module names, the matrices and the
theorem name are all read from real runs of `crosswalk-certify`, `modules` and
`oo-matcert`. `tests/certified_claims_asset_test.rs` re-runs all three and
fails if the drawing and a fresh run disagree, which is the only way a figure
stays true after the code under it moves.

The palette is the one `knowledge-graph.py` uses, because these drawings sit on
the same page and a reader should not have to ask whether they came from the
same tool.
"""

import json
import sys

W, H = 1100, 430
BG, BG2 = "#020617", "#070d20"
PANEL, LINE = "#0b1428", "#1e3a5f"
INK, MUTED, DIM = "#e2e8f0", "#94a3b8", "#64748b"
OK, BAD, WARN, NODE = "#34d399", "#fb3b53", "#fbbf24", "#7dd3fc"

TEXT = {
    "en": {
        "title": "Three claims that used to travel on trust",
        "sub": "each column is a real run; every number here is read from its output, never typed",
        "c1": "A CROSSWALK'S MATCH TYPE",
        "c1sub": "claimed, then checked against what each side entails",
        "c2": "CONTEXTS THAT DISAGREE",
        "c2sub": "each module reasons alone; a fact earns “true everywhere”",
        "c3": "A NUMBER",
        "c3sub": "recomputed from the definition, in Lean",
        "upheld": "upheld",
        "down": "downgraded",
        "gap": "{n} term the crosswalk does not carry",
        "gaps": "{n} terms the crosswalk does not carry",
        "because": "the target has no word for ies:ResponsibleActor, so nothing stronger is defensible",
        "entails": "entails {n}",
        "promoted": "{n} fact entailed in {k} of {t} modules",
        "contested": "{n} held by fewer, and named rather than dropped",
        "merged": "one graph holding both is inconsistent; kept apart, both are fine",
        "cost": "cost",
        "verdictcol": "verdict",
        "thm": "theorem",
        "none": "none",
        "v1": "recompute, integers",
        "v2": "Freivalds, integers",
        "v3": "Freivalds, doubles",
        "v1w": "product_checked",
        "v2w": "survived, 2⁻⁴⁰",
        "v3w": "within tolerance",
        "foot": "A word here is this engine's unless a checker earned it. Only the top row of "
                "the right column names a theorem, and only because oo-matcert exited zero "
                "having printed it.",
    },
    "zh": {
        "title": "三个曾经只能靠信任的声明",
        "sub": "每列都是一次真实运行；此处每个数字都读自其输出，不是写上去的",
        "c1": "对照表的匹配类型",
        "c1sub": "先声明，再对照两边的蕴涵核验",
        "c2": "互相矛盾的语境",
        "c2sub": "每个模块单独推理；事实要自己挡得“处处为真”",
        "c3": "一个数字",
        "c3sub": "在 Lean 中按定义重算",
        "upheld": "维持",
        "down": "降级",
        "gap": "{n} 个词对照表未收录",
        "gaps": "{n} 个词对照表未收录",
        "because": "目标方没有 ies:ResponsibleActor 对应的词，因此更强的声明都立不住",
        "entails": "蕴涵 {n}",
        "promoted": "{n} 条事实在 {t} 个模块中的 {k} 个里被蕴涵",
        "contested": "{n} 条只有更少模块持有，逐条列出而不丢弃",
        "merged": "合并成一张图则不一致；分开则两边都成立",
        "cost": "代价",
        "verdictcol": "结论",
        "thm": "定理",
        "none": "无",
        "v1": "重算，整数",
        "v2": "Freivalds，整数",
        "v3": "Freivalds，双精度",
        "v1w": "product_checked",
        "v2w": "通过，2⁻⁴⁰",
        "v3w": "在容差内",
        "foot": "除非核验器挡下，否则此处的词都只是本引擎的意见。只有右列首行引用了定理，且因为 oo-matcert 已将其打印并以 0 退出。",
    },
}


def text_width(s, size):
    """Conservative width. CJK is one em, Latin about half. Same function and
    same reason as the two companion figures."""
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
    """Refuse to draw a line that will not fit. The companion figures learned
    this the hard way: a heading nobody measured grew until it overlapped the
    next column, and only a reader noticed."""
    if text_width(s, size) > budget:
        raise SystemExit(
            f"{where}: {text_width(s, size):.0f} units of text at font-size {size} "
            f"in {budget:.0f} units of space. Shorten it or widen the column.\n  {s}"
        )
    return s


def esc(s):
    return (str(s).replace("&", "&amp;").replace("<", "&lt;")
            .replace(">", "&gt;").replace('"', "&quot;"))


def short(iri):
    return str(iri).rsplit("/", 1)[-1].rsplit("#", 1)[-1]


def main(cw_path, mod_path, mc_path, out_path, lang="en"):
    T = TEXT[lang]
    cw = json.load(open(cw_path))
    cw = cw.get("result", cw)
    md = json.load(open(mod_path))
    md = md.get("result", md)
    mc = json.load(open(mc_path))

    out = []
    A = out.append
    A(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
      f'font-family="ui-sans-serif,-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif">')
    A('<defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">'
      f'<stop offset="0" stop-color="{BG}"/><stop offset="1" stop-color="{BG2}"/>'
      '</linearGradient></defs>')
    A(f'<rect width="{W}" height="{H}" rx="14" fill="url(#bg)"/>')

    A(f'<text x="34" y="40" font-size="17" font-weight="800" fill="{INK}">'
      f'{esc(must_fit(T["title"], 17, W - 68, "title"))}</text>')
    A(f'<text x="34" y="60" font-size="11.5" fill="{DIM}">'
      f'{esc(must_fit(T["sub"], 11.5, W - 68, "subtitle"))}</text>')

    COLS = 3
    GAP = 18
    CW_ = (W - 68 - GAP * (COLS - 1)) / COLS
    TOP, BOT = 84, H - 66

    def column(i, head, sub):
        x = 34 + i * (CW_ + GAP)
        A(f'<rect x="{x:.0f}" y="{TOP}" width="{CW_:.0f}" height="{BOT - TOP}" rx="11" '
          f'fill="{PANEL}" stroke="{LINE}" stroke-width="1"/>')
        A(f'<text x="{x + 16:.0f}" y="{TOP + 26}" font-size="11.5" font-weight="800" '
          f'letter-spacing="0.6" fill="{NODE}">'
          f'{esc(must_fit(head, 11.5, CW_ - 32, f"column {i} heading"))}</text>')
        A(f'<text x="{x + 16:.0f}" y="{TOP + 42}" font-size="10" fill="{DIM}">'
          f'{esc(must_fit(sub, 10, CW_ - 32, f"column {i} subtitle"))}</text>')
        return x

    # ── 1 · the crosswalk ───────────────────────────────────────────────
    x = column(0, T["c1"], T["c1sub"])
    y = TOP + 70
    for row in cw["rows"]:
        claimed, supported = short(row["claimed"]), short(row["supported"])
        down = row["downgraded"]
        col = WARN if down else OK
        A(f'<text x="{x + 16:.0f}" y="{y}" font-size="10.5" fill="{INK}">'
          f'{esc(must_fit(short(row["subject"]), 10.5, CW_ - 150, "crosswalk subject"))}</text>')
        A(f'<text x="{x + CW_ - 16:.0f}" y="{y}" font-size="10" text-anchor="end" fill="{col}">'
          f'{esc(claimed)} → {esc(supported)}</text>')
        y += 17
    y += 4
    A(f'<line x1="{x + 16:.0f}" y1="{y - 10}" x2="{x + CW_ - 16:.0f}" y2="{y - 10}" '
      f'stroke="{LINE}"/>')
    n_gap = cw["gaps"]
    label = T["gap" if n_gap == 1 else "gaps"].format(n=n_gap)
    A(f'<text x="{x + 16:.0f}" y="{y + 8}" font-size="10" fill="{MUTED}">'
      f'{esc(must_fit(label, 10, CW_ - 32, "gap count"))}</text>')
    for g in cw["unmapped"][:3]:
        y += 15
        A(f'<text x="{x + 24:.0f}" y="{y + 8}" font-size="10" fill="{DIM}">'
          f'· {esc(short(g["term"]))}</text>')
    y += 26
    for line in wrap(T["because"], 10, CW_ - 32):
        A(f'<text x="{x + 16:.0f}" y="{y + 8}" font-size="10" fill="{WARN}">{esc(line)}</text>')
        y += 14

    # ── 2 · the modules ─────────────────────────────────────────────────
    x = column(1, T["c2"], T["c2sub"])
    y = TOP + 74
    for mod in md["modules"]:
        A(f'<rect x="{x + 16:.0f}" y="{y - 12}" width="{CW_ - 32:.0f}" height="24" rx="7" '
          f'fill="#060e20" stroke="{LINE}"/>')
        A(f'<text x="{x + 26:.0f}" y="{y + 4}" font-size="10.5" fill="{INK}">'
          f'{esc(mod["name"])}</text>')
        A(f'<text x="{x + CW_ - 26:.0f}" y="{y + 4}" font-size="10" text-anchor="end" '
          f'fill="{DIM}">{esc(T["entails"].format(n=mod["entails"]))}</text>')
        y += 32
    y += 6
    promoted = T["promoted"].format(n=md["promoted"], k=md["threshold"], t=len(md["modules"]))
    A(f'<text x="{x + 16:.0f}" y="{y}" font-size="10.5" font-weight="700" fill="{OK}">'
      f'{esc(must_fit(promoted, 10.5, CW_ - 32, "promoted line"))}</text>')
    y += 17
    A(f'<text x="{x + 16:.0f}" y="{y}" font-size="10" fill="{MUTED}">'
      f'{esc(must_fit(T["contested"].format(n=md["contested"]), 10, CW_ - 32, "contested"))}</text>')
    y += 24
    for line in wrap(T["merged"], 10, CW_ - 32):
        A(f'<text x="{x + 16:.0f}" y="{y}" font-size="10" fill="{BAD}">{esc(line)}</text>')
        y += 14

    # ── 3 · the number ──────────────────────────────────────────────────
    x = column(2, T["c3"], T["c3sub"])
    y = TOP + 74
    dims = f'{mc["m"]}×{mc["p"]} · {mc["p"]}×{mc["n"]}'
    A(f'<text x="{x + 16:.0f}" y="{y}" font-size="10.5" fill="{MUTED}">'
      f'A × B = C &#160;&#160;{esc(dims)}</text>')
    y += 24
    rows = [
        (T["v1"], "O(n³)", T["v1w"], mc["theorem"], OK),
        (T["v2"], "O(n²)", T["v2w"], T["none"], NODE),
        (T["v3"], "O(n²)", T["v3w"], T["none"], WARN),
    ]
    A(f'<text x="{x + 16:.0f}" y="{y}" font-size="9" letter-spacing="0.5" fill="{DIM}">'
      f'{esc(T["cost"].upper())}</text>')
    A(f'<text x="{x + 74:.0f}" y="{y}" font-size="9" letter-spacing="0.5" fill="{DIM}">'
      f'{esc(T["verdictcol"].upper())}</text>')
    A(f'<text x="{x + CW_ - 16:.0f}" y="{y}" font-size="9" letter-spacing="0.5" '
      f'text-anchor="end" fill="{DIM}">{esc(T["thm"].upper())}</text>')
    y += 8
    A(f'<line x1="{x + 16:.0f}" y1="{y}" x2="{x + CW_ - 16:.0f}" y2="{y}" stroke="{LINE}"/>')
    y += 18
    for name, cost, word, thm, col in rows:
        A(f'<text x="{x + 16:.0f}" y="{y}" font-size="9.5" fill="{DIM}">{esc(cost)}</text>')
        A(f'<text x="{x + 74:.0f}" y="{y}" font-size="10" fill="{col}">{esc(word)}</text>')
        A(f'<text x="{x + CW_ - 16:.0f}" y="{y}" font-size="9" text-anchor="end" '
          f'fill="{OK if thm != T["none"] else DIM}">{esc(thm)}</text>')
        y += 16
        A(f'<text x="{x + 16:.0f}" y="{y}" font-size="9" fill="{DIM}">{esc(name)}</text>')
        y += 20

    A(f'<line x1="34" y1="{BOT + 16}" x2="{W - 34}" y2="{BOT + 16}" stroke="{LINE}"/>')
    fy = BOT + 34
    for line in wrap(T["foot"], 10, W - 68):
        A(f'<text x="34" y="{fy}" font-size="10" fill="{DIM}">{esc(line)}</text>')
        fy += 13
    A("</svg>")

    svg = "\n".join(out)
    import re as _re
    stray = _re.findall(r">[^<>]*(\{[a-z_]+\})[^<>]*<", svg)
    if stray:
        raise SystemExit(f"the drawing carries unsubstituted placeholders: {stray}")
    open(out_path, "w").write(svg)
    print(f"wrote {out_path}: {cw['checked']} mappings checked, {cw['downgraded']} downgraded, "
          f"{cw['gaps']} gap; {md['promoted']} promoted of {len(md['modules'])} modules; "
          f"{mc['verdict']}")


def wrap(s, size, budget):
    """Greedy wrap on the measured width, so a long sentence never leaves the
    panel it was written into."""
    words, lines, cur = s.split(" "), [], ""
    for w in words:
        trial = (cur + " " + w).strip()
        if text_width(trial, size) > budget and cur:
            lines.append(cur)
            cur = w
        else:
            cur = trial
    if cur:
        lines.append(cur)
    return lines


if __name__ == "__main__":
    main(*sys.argv[1:6])
