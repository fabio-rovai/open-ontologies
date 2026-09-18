#!/usr/bin/env python3
"""Regenerate docs/assets/demo-certify.svg.

Run: python3 docs/assets/demo-certify.py docs/assets/demo-certify.svg

A split pane in the manner of a screen recording, but vector and tiny: the
terminal on the left runs the checker, the graph on the right lights up in
step with it. Every line of terminal text is output this repository's Lean
binary actually printed for the certificate in tests, not a mock-up, and the
timings are computed here rather than typed one animate element at a time.
"""
import re

DUR = 22.0
W, H, SPLIT = 940, 430, 370

def k(t):  # seconds -> keyTime
    return max(0.0, min(1.0, t / DUR))

def anim(t0, t1=None):
    """Opacity keyframes: invisible, appear at t0, gone again at t1."""
    e = 0.004
    if t1 is None:
        t1 = DUR - 1.0
    a, b = k(t0), k(t1)
    vals = "0;0;1;1;0;0"
    keys = f"0;{a:.4f};{min(a+e,1):.4f};{b:.4f};{min(b+e,1):.4f};1"
    return (f'<animate attributeName="opacity" values="{vals}" keyTimes="{keys}" '
            f'dur="{DUR}s" repeatCount="indefinite"/>')

out = []
A = out.append

A(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
  f'font-family="ui-monospace,SFMono-Regular,Menlo,Consolas,monospace" role="img" '
  f'aria-label="A terminal runs the Lean certificate checker while a supplier graph lights up: '
  f'three asserted edges in grey, three derived edges in green accepted by the checker, then one '
  f'conclusion forged and the same checker refusing it in red">')

# ---- chrome ----
A('<defs>'
  '<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">'
  '<stop offset="0" stop-color="#0b1020"/><stop offset="1" stop-color="#0d1526"/></linearGradient>'
  '<marker id="mg" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">'
  '<path d="M0 0 L10 5 L0 10 z" fill="#64748b"/></marker>'
  '<marker id="mgr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">'
  '<path d="M0 0 L10 5 L0 10 z" fill="#34d399"/></marker>'
  '<marker id="mrd" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">'
  '<path d="M0 0 L10 5 L0 10 z" fill="#fb3b53"/></marker>'
  '</defs>')
A(f'<rect x="1" y="1" width="{W-2}" height="{H-2}" rx="14" fill="url(#bg)" stroke="#1f2937" stroke-width="1.5"/>')
A(f'<line x1="{SPLIT}" y1="16" x2="{SPLIT}" y2="{H-16}" stroke="#1c2534" stroke-width="1"/>')
for i, c in enumerate(('#fb3b53', '#fbbf24', '#34d399')):
    A(f'<circle cx="{26+i*18}" cy="28" r="4.5" fill="{c}"/>')
A('<text x="86" y="32" font-size="10.5" fill="#64748b">oo-horn check</text>')
A(f'<text x="{SPLIT-16}" y="32" text-anchor="end" font-size="9.5" fill="#475569">loops every {int(DUR)}s</text>')
A(f'<text x="{SPLIT+22}" y="32" font-size="10.5" fill="#64748b">the graph it is proving about</text>')

# ---- left: terminal ----
X, Y0, LH, FS = 22, 62, 16.5, 10.5
def line(idx, text, t0, t1=None, fill="#cbd5e1", prompt=False):
    y = Y0 + idx * LH
    body = (f'<tspan fill="#34d399">$ </tspan><tspan fill="{fill}">{text}</tspan>'
            if prompt else f'<tspan fill="{fill}">{text}</tspan>')
    A(f'<text x="{X}" y="{y:.1f}" font-size="{FS}" opacity="0">{body}{anim(t0, t1)}</text>')

ACC_OFF, FORGE_ON = 12.0, 12.4
line(0,  'oo-horn check \\',        0.4, ACC_OFF, prompt=True)
line(1,  '&#160;&#160;builtin_rules.tsv \\', 0.7, ACC_OFF)
line(2,  '&#160;&#160;asserted.tsv \\',      1.0, ACC_OFF)
line(3,  '&#160;&#160;derivations.tsv',      1.3, ACC_OFF)
line(5,  '{"ok":true,',                      7.6, ACC_OFF, fill="#a7f3d0")
line(6,  '&#160;"verdict":"entailed",',      7.9, ACC_OFF, fill="#a7f3d0")
line(7,  '&#160;"asserted":3,',              8.2, ACC_OFF, fill="#a7f3d0")
line(8,  '&#160;"derivations":3,',           8.5, ACC_OFF, fill="#a7f3d0")
line(9,  '&#160;"theorem":',                 8.8, ACC_OFF, fill="#a7f3d0")
line(10, '&#160;&#160;"OOCert.entails_of_builtin_horn"}', 9.1, ACC_OFF, fill="#a7f3d0")
line(12, 'exit 0',                           9.6, ACC_OFF, fill="#34d399")

line(0,  '# forge ONE conclusion.',        FORGE_ON, prompt=True, fill="#64748b")
line(1,  '# leave both premises alone.',   12.7,     prompt=True, fill="#64748b")
line(2,  'oo-horn check \\',               13.2,     prompt=True)
line(3,  '&#160;&#160;... forged.tsv',     13.5)
line(5,  '{"ok":false,',                   15.2, fill="#fecaca")
line(6,  '&#160;"asserted":3,',            15.5, fill="#fecaca")
line(7,  '&#160;"derivations":3}',         15.8, fill="#fecaca")
line(9,  'exit 1',                         16.2, fill="#fb3b53")
line(11, 'The premises were untouched.',   17.0, fill="#94a3b8")
line(12, 'The conclusion did not follow.', 17.3, fill="#94a3b8")

# ---- right: graph ----
def chip(x, y, label, col, size=10):
    """Edge labels sit on a chip so a line never reads through the text."""
    w = len(re.sub(r"&#[0-9]+;", "-", label)) * 6.0 + 10
    return (f'<rect x="{x-w/2:.1f}" y="{y-11}" width="{w:.1f}" height="15" rx="4" '
            f'fill="#0b1020" opacity="0.92"/>'
            f'<text x="{x}" y="{y}" text-anchor="middle" font-size="{size}" '
            f'font-weight="700" fill="{col}">{label}</text>')

def box(x, y, w, label, t0, kind="cls", t1=None):
    fill, stroke, tc = {"ind": ("#172554", "#3b82f6", "#dbeafe"),
                        "bad": ("#3b0710", "#fb3b53", "#fecaca")}.get(
                            kind, ("#1e293b", "#475569", "#e2e8f0"))
    A(f'<g opacity="0">{anim(t0, t1)}'
      f'<rect x="{x}" y="{y}" width="{w}" height="32" rx="7" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>'
      f'<text x="{x+w/2}" y="{y+21}" text-anchor="middle" font-size="11" fill="{tc}">{label}</text></g>')

def edge(d, t0, kind, label=None, lx=0, ly=0, t1=None):
    col, mk, dash = {"a": ("#64748b", "mg", ""),
                     "g": ("#34d399", "mgr", ' stroke-dasharray="6 4"'),
                     "r": ("#fb3b53", "mrd", ' stroke-dasharray="6 4"')}[kind]
    wdt = 2.8 if kind == "r" else 2
    A(f'<g opacity="0">{anim(t0, t1)}'
      f'<path d="{d}" fill="none" stroke="{col}" stroke-width="{wdt}"{dash} marker-end="url(#{mk})"/>'
      + (chip(lx, ly, label, col) if label else '') + '</g>')

# One column of classes, uniform width, so the right edge is a clean spine.
CX, CW = 436, 252
box(760, 58, 152, 'ex:Northwind', 1.8, kind="ind")
box(CX, 150, CW, 'ex:SanctionedJurisdiction', 2.0)
box(CX, 240, CW, 'ex:HighRiskSupplier', 2.4)
box(CX, 330, CW, 'ex:NeedsEnhancedDueDiligence', 2.8)

# asserted: the type link, then the two subclass steps down the spine
edge('M 757,74 C 700,80 640,108 606,147', 2.2, "a", 'rdf:type', 676, 100)
edge('M 520,182 L 520,238', 2.6, "a", 'rdfs:subClassOf', 520, 215)
edge('M 520,272 L 520,328', 3.0, "a", 'rdfs:subClassOf', 520, 305)

# derived: two concentric arcs down the right, so neither crosses the other
edge('M 832,90 C 800,140 770,196 692,252', 4.0, "g", 'rdfs9', 800, 168)
edge('M 434,166 C 396,214 396,300 432,344', 5.2, "g", 'rdfs11', 400, 258)
edge('M 890,90 C 924,180 900,300 694,344', 6.4, "g", 'rdfs9', 906, 232, t1=14.2)

# the forgery: same premises, a conclusion that does not follow
box(726, 330, 176, 'ex:LowRiskSupplier', 14.2, kind="bad")
edge('M 880,90 C 916,170 900,260 848,326', 14.2, "r", 'rdfs9 refused', 848, 208)

A(f'<g opacity="0">{anim(9.6, ACC_OFF)}'
  '<rect x="430" y="382" width="300" height="30" rx="8" fill="#052e1a" stroke="#34d399" stroke-width="1.5"/>'
  '<text x="580" y="402" text-anchor="middle" font-size="11.5" font-weight="700" fill="#34d399">'
  '&#10003; certificate accepted by Lean 4</text></g>')
A(f'<g opacity="0">{anim(16.2)}'
  '<rect x="415" y="382" width="330" height="30" rx="8" fill="#3b0710" stroke="#fb3b53" stroke-width="1.5"/>'
  '<text x="580" y="402" text-anchor="middle" font-size="11.5" font-weight="700" fill="#fb3b53">'
  '&#10007; refused, and it named the rule</text></g>')

A('</svg>')
import pathlib, sys
p = pathlib.Path(sys.argv[1]); p.write_text("\n".join(out))
print(f"wrote {p} ({p.stat().st_size} bytes)")
