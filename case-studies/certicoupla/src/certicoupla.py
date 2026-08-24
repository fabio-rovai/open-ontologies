"""
certicoupla — copula-coupled, certificate-emitting uncertainty on a frozen scientific model.

The claim (#105, #112): independent, per-target conformal prediction gives correct MARGINAL
coverage but the wrong JOINT coverage, because scientific targets are correlated. A certificate
that must hold jointly (a box or region the realized outcome vector falls inside at the claimed
rate) needs the coupling. We show this on real DFT materials data.

Frozen model: a gradient-boosted regressor trained once on a train split and frozen; the
uncertainty methods wrap it without touching it. Targets: formation energy, band gap and
stability from the OQMD (real, keyless). Features: composition descriptors from the formula.

Methods compared, all calibrated to a nominal 90% JOINT coverage:
  - independent  : per-target split-conformal at 90% each (marginal-correct, joint-blind)
  - Bonferroni   : per-target at 1 - 0.1/d (joint-valid but conservative/wide)
  - coupled      : global max-score split-conformal (finite-sample joint guarantee, box)
  - copula       : Gaussian copula on the residuals -> Mahalanobis region (dependence-aware)

Every number is computed on a held-out test split; the certificate is the emitted region and
its soundness is the measured joint coverage against the claimed 90%.
"""
import json, os, re
import numpy as np
from scipy.stats import norm
from sklearn.ensemble import GradientBoostingRegressor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA, RES = os.path.join(ROOT, "data"), os.path.join(ROOT, "results")
os.makedirs(RES, exist_ok=True)
RNG = np.random.default_rng(7)
ALPHA = 0.10                      # nominal miscoverage; 90% joint target
TARGETS = ["delta_e", "band_gap", "stability"]

# Pauling electronegativity for the common elements (missing -> median at use).
EN = {"H":2.20,"He":0,"Li":0.98,"Be":1.57,"B":2.04,"C":2.55,"N":3.04,"O":3.44,"F":3.98,"Ne":0,
"Na":0.93,"Mg":1.31,"Al":1.61,"Si":1.90,"P":2.19,"S":2.58,"Cl":3.16,"Ar":0,"K":0.82,"Ca":1.00,
"Sc":1.36,"Ti":1.54,"V":1.63,"Cr":1.66,"Mn":1.55,"Fe":1.83,"Co":1.88,"Ni":1.91,"Cu":1.90,"Zn":1.65,
"Ga":1.81,"Ge":2.01,"As":2.18,"Se":2.55,"Br":2.96,"Kr":3.00,"Rb":0.82,"Sr":0.95,"Y":1.22,"Zr":1.33,
"Nb":1.6,"Mo":2.16,"Tc":1.9,"Ru":2.2,"Rh":2.28,"Pd":2.20,"Ag":1.93,"Cd":1.69,"In":1.78,"Sn":1.96,
"Sb":2.05,"Te":2.1,"I":2.66,"Xe":2.60,"Cs":0.79,"Ba":0.89,"La":1.10,"Ce":1.12,"Pr":1.13,"Nd":1.14,
"Pm":1.13,"Sm":1.17,"Eu":1.2,"Gd":1.20,"Tb":1.1,"Dy":1.22,"Ho":1.23,"Er":1.24,"Tm":1.25,"Yb":1.1,
"Lu":1.27,"Hf":1.3,"Ta":1.5,"W":2.36,"Re":1.9,"Os":2.2,"Ir":2.20,"Pt":2.28,"Au":2.54,"Hg":2.00,
"Tl":1.62,"Pb":2.33,"Bi":2.02,"Po":2.0,"At":2.2,"Th":1.3,"Pa":1.5,"U":1.38,"Np":1.36,"Pu":1.28}
# atomic numbers 1..96
SYM = ("H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar K Ca Sc Ti V Cr Mn Fe Co Ni Cu Zn Ga Ge As "
       "Se Br Kr Rb Sr Y Zr Nb Mo Tc Ru Rh Pd Ag Cd In Sn Sb Te I Xe Cs Ba La Ce Pr Nd Pm Sm Eu Gd "
       "Tb Dy Ho Er Tm Yb Lu Hf Ta W Re Os Ir Pt Au Hg Tl Pb Bi Po At Rn Fr Ra Ac Th Pa U Np Pu Am Cm").split()
Z = {s: i + 1 for i, s in enumerate(SYM)}
EN_MED = np.median([v for v in EN.values() if v > 0])

def featurize(formula):
    toks = re.findall(r"([A-Z][a-z]?)(\d*)", formula or "")
    els = [(s, int(n) if n else 1) for s, n in toks if s in Z]
    if not els:
        return None
    zs = np.array([Z[s] for s, _ in els], float)
    ens = np.array([EN[s] if EN.get(s, 0) > 0 else EN_MED for s, _ in els], float)
    w = np.array([n for _, n in els], float); w = w / w.sum()
    return np.array([
        len(els), sum(n for _, n in els),
        (zs * w).sum(), zs.std(), zs.max() - zs.min(),
        (ens * w).sum(), ens.std(), ens.max() - ens.min(),
    ])

def load():
    rows = json.load(open(os.path.join(DATA, "oqmd.json")))
    X, Y = [], []
    for r in rows:
        f = featurize(r.get("name"))
        if f is None:
            continue
        X.append(f); Y.append([r["delta_e"], r["band_gap"], r["stability"]])
    return np.array(X), np.array(Y)

def cover_box(Y, lo, hi):
    inside = (Y >= lo) & (Y <= hi)               # n x d
    return inside.mean(0), inside.all(1).mean()   # marginal (per target), joint

def main():
    X, Y = load()
    n = len(X); idx = RNG.permutation(n)
    a, b = int(0.5 * n), int(0.75 * n)
    tr, cal, te = idx[:a], idx[a:b], idx[b:]
    d = Y.shape[1]
    print(f"[*] {n} materials  |  train {len(tr)}  cal {len(cal)}  test {len(te)}  |  {d} targets")

    # frozen model: one GBR per target, trained on train split only
    models = [GradientBoostingRegressor(n_estimators=200, max_depth=3, random_state=0).fit(X[tr], Y[tr, j]) for j in range(d)]
    def predict(Xs): return np.column_stack([m.predict(Xs) for m in models])
    Pcal, Pte = predict(X[cal]), predict(X[te])
    Rcal, Rte = Y[cal] - Pcal, Y[te] - Pte          # residual vectors
    s = np.abs(Rcal).std(0) + 1e-9                    # per-target scale

    corr = np.corrcoef(Rcal.T)
    ncal = len(cal)
    def q(v, level): return np.quantile(v, min(1.0, level * (ncal + 1) / ncal), method="higher")

    results = {}

    # 1) independent: per-target 90%
    qj = np.array([q(np.abs(Rcal[:, j]), 1 - ALPHA) for j in range(d)])
    lo, hi = Pte - qj, Pte + qj
    marg, joint = cover_box(Y[te], lo, hi)
    vol_ind = np.prod(2 * qj)
    results["independent"] = {"marginal_mean": float(marg.mean()), "joint": float(joint), "rel_volume": 1.0}

    # 2) Bonferroni: per-target 1 - alpha/d
    qb = np.array([q(np.abs(Rcal[:, j]), 1 - ALPHA / d) for j in range(d)])
    lo, hi = Pte - qb, Pte + qb
    margB, jointB = cover_box(Y[te], lo, hi)
    vol_bon = np.prod(2 * qb)
    results["bonferroni"] = {"marginal_mean": float(margB.mean()), "joint": float(jointB), "rel_volume": float(vol_bon / vol_ind)}

    # 3) coupled: global max-score conformal (box)
    score = np.max(np.abs(Rcal) / s, axis=1)
    qc = q(score, 1 - ALPHA)
    half = qc * s
    lo, hi = Pte - half, Pte + half
    margC, jointC = cover_box(Y[te], lo, hi)
    vol_cpl = np.prod(2 * half)
    results["coupled_conformal"] = {"marginal_mean": float(margC.mean()), "joint": float(jointC), "rel_volume": float(vol_cpl / vol_ind)}

    # 4) copula: Gaussian copula on residual ranks -> Mahalanobis region
    u = (np.argsort(np.argsort(Rcal, axis=0), axis=0) + 1) / (ncal + 1)   # empirical CDF ranks
    zc = norm.ppf(u)
    Sig = np.cov(zc.T); Si = np.linalg.inv(Sig)
    m_cal = np.einsum("ij,jk,ik->i", zc, Si, zc) ** 0.5
    qm = q(m_cal, 1 - ALPHA)
    # map test residuals into the same Gaussian-copula space using cal marginals
    def to_z(col, j):
        ranks = np.searchsorted(np.sort(Rcal[:, j]), col, side="right")
        uu = np.clip((ranks + 0.5) / (ncal + 1), 1e-4, 1 - 1e-4)
        return norm.ppf(uu)
    zte = np.column_stack([to_z(Rte[:, j], j) for j in range(d)])
    m_te = np.einsum("ij,jk,ik->i", zte, Si, zte) ** 0.5
    joint_cop = float((m_te <= qm).mean())
    # region volume in copula space ~ ellipsoid volume proportional to qm^d * sqrt(det Sig)
    vol_cop_space = (qm ** d) * np.sqrt(np.linalg.det(Sig))
    vol_cpl_space = (qc ** d) * 1.0  # coupled box in standardized space, comparable proxy
    results["copula"] = {"marginal_mean": None, "joint": joint_cop,
                         "rel_region_vs_coupled_in_copula_space": float(vol_cop_space / vol_cpl_space)}

    out = {
        "source": "OQMD (Open Quantum Materials Database), real DFT properties",
        "n_materials": int(n), "targets": TARGETS, "nominal_joint_coverage": 1 - ALPHA,
        "residual_correlation": np.round(corr, 3).tolist(),
        "methods": results,
        "headline": {
            "independent_marginal_coverage": round(results["independent"]["marginal_mean"], 3),
            "independent_joint_coverage": round(results["independent"]["joint"], 3),
            "coupled_joint_coverage": round(results["coupled_conformal"]["joint"], 3),
            "copula_joint_coverage": round(results["copula"]["joint"], 3),
            "bonferroni_joint_coverage": round(results["bonferroni"]["joint"], 3),
            "coupled_rel_volume_vs_bonferroni": round(results["coupled_conformal"]["rel_volume"] / results["bonferroni"]["rel_volume"], 3),
        },
    }
    json.dump(out, open(os.path.join(RES, "results.json"), "w"), indent=2)
    print(json.dumps(out["headline"], indent=2))
    print("residual corr:\n", np.round(corr, 3))
    for k, v in results.items():
        print(f"  {k}: {v}")

if __name__ == "__main__":
    main()
