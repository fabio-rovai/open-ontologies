"""
verifiabench (multi-domain): the closed-world oracle across two authorities and three task
families, single-hop and multi-hop.

Authorities:
  - Biolink Model (classes + slots), https://w3id.org/biolink/vocab/
  - Gene Ontology (GO term ids), http://purl.obolibrary.org/obo/GO_...

Task families (each a real biomedical fact the model must express in RDF using only REAL terms):
  1. biolink_gene_disease : gene associated with a disease (Biolink only).
  2. go_annotation        : gene involved in a biological process (Biolink gene + a real GO term).
  3. multihop             : gene, its disease, and its biological process at once (Biolink + GO).

The oracle is closed-world across BOTH authorities: every biolink: term must be a declared Biolink
term and every GO id must be a real GO id. Correctness is set-membership plus structure, computed
deterministically. Fluency cannot buy a point, and a multi-hop task cannot be gamed by getting one
authority right and inventing the other.
"""
import json, os, re, sys, glob
from concurrent.futures import ThreadPoolExecutor
import verifiabench as vb   # reuse query(), slug(), display(), the Biolink helpers

ROOT = vb.ROOT; DATA, RES = vb.DATA, vb.RES
BL = vb.BL
GO_SET = set(json.load(open(os.path.join(DATA, "go_terms.json"))))   # {'GO_0006915', ...}

# ---- term extraction across authorities --------------------------------------
def go_terms(text):
    ids = set()
    for m in re.findall(r"GO[:_](\d{7})", text):
        ids.add("GO_" + m)
    for m in re.findall(r"obo/GO_(\d{7})", text):
        ids.add("GO_" + m)
    return ids

def extract(output):
    bl = vb.extract(output)[0]                      # Biolink terms (prefix-robust, from verifiabench)
    go = go_terms(vb.strip_fence(output))
    return bl, go

def check(output, family):
    bl, go = extract(output)
    bl_real, bl_fake = {t for t in bl if t in vb.DECLARED}, {t for t in bl if t not in vb.DECLARED}
    go_real, go_fake = {t for t in go if t in GO_SET}, {t for t in go if t not in GO_SET}
    n_terms = len(bl) + len(go)
    n_real = len(bl_real) + len(go_real)
    n_fake = len(bl_fake) + len(go_fake)
    has_gene = vb.GENE in bl_real
    has_disease = vb.DISEASE in bl_real
    has_assoc = len(bl_real & vb.ASSOC_SLOTS) > 0
    has_go = len(go_real) > 0
    raw_ok = n_terms > 0
    if family == "biolink_gene_disease":
        verified = has_gene and has_disease and has_assoc and n_fake == 0
    elif family == "go_annotation":
        verified = has_gene and has_go and n_fake == 0
    else:  # multihop: needs both authorities and the full structure, all real
        verified = has_gene and has_disease and has_go and n_fake == 0 and len(bl_real) >= 2
    return {
        "family": family, "n_terms": n_terms, "n_real": n_real, "n_fake": n_fake,
        "term_existence_rate": (n_real / n_terms) if n_terms else 0.0,
        "raw_ok": raw_ok, "verified_ok": verified,
        "fake_terms": sorted([t.split("/")[-1] for t in bl_fake] + sorted(go_fake))[:6],
        "output": output[:500],
    }

# ---- tasks (real facts) ------------------------------------------------------
GENE_DISEASE = vb.FACTS   # reuse the 30 gene-disease facts

# gene -> real biological process (name); the model must emit a real GO term for the process
GENE_PROCESS = [
    ("TP53", "apoptotic process"), ("EGFR", "signal transduction"), ("BRCA1", "DNA repair"),
    ("KRAS", "Ras protein signal transduction"), ("MYC", "cell population proliferation"),
    ("CFTR", "chloride transport"), ("MLH1", "mismatch repair"), ("ATM", "DNA damage response"),
    ("VEGFA", "angiogenesis"), ("TNF", "inflammatory response"), ("INS", "glucose homeostasis"),
    ("SOD1", "response to oxidative stress"), ("HMGCR", "cholesterol biosynthetic process"),
    ("CDK1", "cell cycle"), ("CASP3", "apoptotic process"), ("IL6", "inflammatory response"),
    ("MTOR", "regulation of cell growth"), ("PARP1", "DNA repair"), ("ESR1", "signal transduction"),
    ("HIF1A", "response to hypoxia"),
]
# gene, disease, process (all real) for the cross-ontology multi-hop
MULTIHOP = [
    ("TP53", "Li-Fraumeni syndrome", "apoptotic process"),
    ("BRCA1", "breast cancer", "DNA repair"),
    ("EGFR", "non-small cell lung carcinoma", "signal transduction"),
    ("KRAS", "pancreatic carcinoma", "Ras protein signal transduction"),
    ("CFTR", "cystic fibrosis", "chloride transport"),
    ("MLH1", "Lynch syndrome", "mismatch repair"),
    ("ATM", "ataxia telangiectasia", "DNA damage response"),
    ("HTT", "Huntington disease", "protein aggregation"),
    ("MYC", "Burkitt lymphoma", "cell population proliferation"),
    ("VHL", "von Hippel-Lindau disease", "response to hypoxia"),
    ("PTEN", "Cowden syndrome", "regulation of cell growth"),
    ("SOD1", "amyotrophic lateral sclerosis", "response to oxidative stress"),
    ("APC", "familial adenomatous polyposis", "Wnt signaling pathway"),
    ("RB1", "retinoblastoma", "cell cycle"),
    ("CASP3", "cancer", "apoptotic process"),
]

P_GD = ("Using the Biolink Model, write RDF in Turtle asserting that the gene {g} is associated with "
        "the disease {d}. Use only real Biolink Model classes and predicates with the `biolink:` prefix "
        "(biolink: = <{bl}>). Output only the Turtle.")
P_GO = ("Write RDF in Turtle asserting that the gene {g} is involved in the biological process '{p}'. "
        "Type the gene with the Biolink Model (`biolink:` = <{bl}>) and represent the process with a real "
        "Gene Ontology term (GO: prefix, e.g. GO:0008150). Use only real terms. Output only the Turtle.")
P_MH = ("Write RDF in Turtle capturing all of: the gene {g} is associated with the disease {d}, and the "
        "gene {g} is involved in the biological process '{p}'. Use the Biolink Model (`biolink:` = <{bl}>) "
        "for the gene, disease and association, and a real Gene Ontology term (GO: prefix) for the process. "
        "Use only real terms. Output only the Turtle.")

def build_tasks():
    t = []
    for g, d in GENE_DISEASE:
        t.append(("biolink_gene_disease", P_GD.format(g=g, d=d, bl=BL)))
    for g, p in GENE_PROCESS:
        t.append(("go_annotation", P_GO.format(g=g, p=p, bl=BL)))
    for g, d, p in MULTIHOP:
        t.append(("multihop", P_MH.format(g=g, d=d, p=p, bl=BL)))
    return t

FAMILIES = ["biolink_gene_disease", "go_annotation", "multihop"]

def _one(model, fam, prompt):
    try:
        out = vb.query(model, prompt)
    except Exception as e:
        out = f"__ERROR__ {e}"
    return check(out, fam)

def run_model(model, tasks):
    # `claude -p` calls each cold-start the CLI (~9s), so run them concurrently; the local MLX
    # server is single-model, so keep those sequential.
    if model.startswith("claude:"):
        with ThreadPoolExecutor(max_workers=8) as ex:
            per = list(ex.map(lambda t: _one(model, t[0], t[1]), tasks))
    else:
        per = [_one(model, fam, prompt) for fam, prompt in tasks]
    done = sum(x["verified_ok"] for x in per)
    print(f"  {len(tasks)} tasks done, verified={done}", flush=True)
    return per

def aggregate(model, per):
    def rate(sub, key):
        return round(sum(x[key] for x in sub) / len(sub), 3) if sub else None
    byfam = {}
    for fam in FAMILIES:
        sub = [x for x in per if x["family"] == fam]
        byfam[fam] = {"n": len(sub), "raw": rate(sub, "raw_ok"), "verified": rate(sub, "verified_ok"),
                      "mean_term_existence": rate(sub, "term_existence_rate")}
    return {"model": vb.display(model), "n_tasks": len(per),
            "raw_capability": rate(per, "raw_ok"),
            "verified_capability": rate(per, "verified_ok"),
            "mean_term_existence": rate(per, "term_existence_rate"),
            "total_fabricated_terms": sum(x["n_fake"] for x in per),
            "by_family": byfam}

def rebuild_leaderboard():
    board = []
    for fn in sorted(glob.glob(os.path.join(RES, "multi_per_task_*.json"))):
        per = json.load(open(fn))
        model = os.path.basename(fn)[len("multi_per_task_"):-len(".json")]
        board.append(aggregate(model, per))
    board.sort(key=lambda a: (a["verified_capability"], a["mean_term_existence"]), reverse=True)
    out = {"authorities": ["Biolink Model", "Gene Ontology"], "task_families": FAMILIES,
           "n_tasks": len(build_tasks()),
           "oracle": "closed-world term existence across both authorities + structure (deterministic, no LLM judge)",
           "leaderboard": board}
    json.dump(out, open(os.path.join(RES, "results_multi.json"), "w"), indent=2)
    return out

def main():
    tasks = build_tasks()
    models = sys.argv[1:] or ["mlx-community/Qwen2.5-3B-Instruct-4bit"]
    for m in models:
        print(f"[*] {m} ({len(tasks)} tasks) ...", flush=True)
        per = run_model(m, tasks)
        json.dump(per, open(os.path.join(RES, "multi_per_task_" + vb.slug(m) + ".json"), "w"), indent=2)
        print("   ", json.dumps(aggregate(m, per)), flush=True)
    out = rebuild_leaderboard()
    print(json.dumps(out, indent=2))

if __name__ == "__main__":
    main()
