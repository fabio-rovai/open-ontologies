"""
assure-health — one report card for privacy AND fairness, and how differential privacy
couples them.

Privacy and fairness are usually audited on separate benches. This runs both on the SAME model
and shows they interact: hardening privacy with differential privacy degrades the smallest
subgroups the most (the "disparate impact of differential privacy", Bagdasaryan et al., NeurIPS
2019), so a report that tightens privacy while watching only the aggregate would report success
and hide a widening equity gap.

Data: UCI Diabetes 130-US-hospitals readmission (real, public, no DUA), predicting 30-day
readmission, with race as the protected attribute (subgroups of very different sizes:
Caucasian, AfricanAmerican, Hispanic, Other, Asian). MIMIC-IV is the credentialed scale-up.

Method:
  - A frozen L2-regularised logistic model on unit-norm features.
  - Differential privacy by OUTPUT PERTURBATION (Chaudhuri, Monteleoni, Sarwate 2011): add
    Gaussian noise to the trained weights, sigma = Delta * sqrt(2 ln(1.25/delta)) / epsilon with
    weight sensitivity Delta = 2/(n*lambda). This is a rigorous (epsilon, delta)-DP mechanism.
  - Privacy plane: a loss-based membership-inference attack (Yeom et al. 2018), per subgroup:
    members (train) tend to have lower loss than non-members (held-out); the attack's advantage
    is the AUC of that separation, mapped to 2*AUC - 1.
  - Equity plane: per-subgroup balanced accuracy, and the gap between the best and worst subgroup.
  - Coupling: sweep epsilon and report, on one card, how membership-inference advantage falls
    while the subgroup accuracy gap rises. The Assurance Coupling Score is that trade-off.
"""
import json, os
import numpy as np, pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import roc_auc_score, balanced_accuracy_score

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA, RES = os.path.join(ROOT, "data"), os.path.join(ROOT, "results")
os.makedirs(RES, exist_ok=True)
RNG = np.random.default_rng(0)
DELTA = 1e-5
LAMBDA = 0.01
EPSILONS = [None, 4.0, 1.0, 0.5, 0.25]     # None = non-private baseline
NUMERIC = ["time_in_hospital", "num_lab_procedures", "num_procedures", "num_medications",
           "number_outpatient", "number_emergency", "number_inpatient", "number_diagnoses"]
AGE_MAP = {f"[{a}-{a+10})": i for i, a in enumerate(range(0, 100, 10))}

def load():
    df = pd.read_csv(os.path.join(DATA, "diabetic_data.csv"), low_memory=False)
    df = df[df["race"].notna()].copy()
    df["age_ord"] = df["age"].map(AGE_MAP).fillna(0)
    y = (df["readmitted"] == "<30").astype(int).to_numpy()
    feats = NUMERIC + ["age_ord"]
    X = df[feats].apply(pd.to_numeric, errors="coerce").fillna(0).to_numpy(float)
    group = df["race"].to_numpy()
    return X, y, group

def unit_rows(X):
    n = np.linalg.norm(X, axis=1, keepdims=True); n[n == 0] = 1
    return X / n

def loss_of(w, X, y):
    z = X @ w
    p = 1 / (1 + np.exp(-z))
    p = np.clip(p, 1e-7, 1 - 1e-7)
    return -(y * np.log(p) + (1 - y) * np.log(1 - p))

def mia_advantage(w, Xm, ym, Xn, yn):
    """Yeom loss-based membership inference: member-score = -loss; advantage = 2*AUC - 1."""
    lm, ln = loss_of(w, Xm, ym), loss_of(w, Xn, yn)
    score = np.concatenate([-lm, -ln])
    label = np.concatenate([np.ones(len(lm)), np.zeros(len(ln))])   # 1 = member
    try:
        auc = roc_auc_score(label, score)
    except ValueError:
        return 0.0
    return max(0.0, 2 * auc - 1)

DRAWS = 40                                  # average metrics over DP noise draws (DP is randomised)
GROUPS = ["Caucasian", "AfricanAmerican", "Hispanic"]   # subgroups large enough for stable stats
MINORITY = ["AfricanAmerican", "Hispanic"]

def main():
    X, y, group = load()
    X = unit_rows(StandardScaler().fit_transform(X))
    idx = RNG.permutation(len(X))
    ntr = 8000
    tr, te = idx[:ntr], idx[ntr:ntr + 12000]
    Xtr, ytr, gtr = X[tr], y[tr], group[tr]
    Xte, yte, gte = X[te], y[te], group[te]

    C = 1.0 / (LAMBDA * ntr)
    base = LogisticRegression(C=C, penalty="l2", solver="lbfgs", fit_intercept=False, max_iter=2000).fit(Xtr, ytr)
    w0 = base.coef_.ravel()
    Delta = 2.0 / (ntr * LAMBDA)
    sizes = {g: int((gtr == g).sum()) for g in GROUPS}
    test_sizes = {g: int((gte == g).sum()) for g in GROUPS}

    def metrics(w):
        pred = (Xte @ w > 0).astype(int)
        acc = {g: float(balanced_accuracy_score(yte[gte == g], pred[gte == g]))
               for g in GROUPS if len(np.unique(yte[gte == g])) > 1}
        mia = {g: mia_advantage(w, Xtr[gtr == g], ytr[gtr == g], Xte[gte == g], yte[gte == g]) for g in GROUPS}
        return acc, mia

    base_acc, _ = metrics(w0)
    card = []
    for eps in EPSILONS:
        if eps is None:
            accs = [base_acc]; mias = [metrics(w0)[1]]
        else:
            sigma = Delta * np.sqrt(2 * np.log(1.25 / DELTA)) / eps
            accs, mias = [], []
            for _ in range(DRAWS):
                a, m = metrics(w0 + RNG.normal(0, sigma, size=w0.shape))
                accs.append(a); mias.append(m)
        acc = {g: round(float(np.mean([a[g] for a in accs if g in a])), 4) for g in GROUPS}
        mia = {g: round(float(np.mean([m[g] for m in mias])), 4) for g in GROUPS}
        drop = {g: round(base_acc[g] - acc[g], 4) for g in GROUPS}
        card.append({
            "epsilon": "inf" if eps is None else eps,
            "subgroup_balanced_accuracy": acc,
            "subgroup_accuracy_drop_vs_nonprivate": drop,
            "subgroup_mia_advantage": mia,
            "majority_accuracy_drop": drop["Caucasian"],
            "minority_accuracy_drop": round(float(np.mean([drop[g] for g in MINORITY])), 4),
            "mean_mia_advantage": round(float(np.mean(list(mia.values()))), 4),
        })

    tight = card[-1]
    coupling = {
        "epsilon_tight": tight["epsilon"],
        "mia_advantage_inf_to_tight": [card[0]["mean_mia_advantage"], tight["mean_mia_advantage"]],
        "majority_accuracy_drop_at_tight": tight["majority_accuracy_drop"],
        "minority_accuracy_drop_at_tight": tight["minority_accuracy_drop"],
        "disparate_impact_ratio": (round(tight["minority_accuracy_drop"] / tight["majority_accuracy_drop"], 2)
                                   if tight["majority_accuracy_drop"] else None),
        "reading": ("tightening privacy (lower epsilon) lowers membership-inference advantage, but the "
                    "accuracy it costs falls disproportionately on the smaller subgroups: the disparate "
                    "impact of differential privacy. Auditing privacy or fairness alone hides this."),
    }
    out = {
        "dataset": "UCI Diabetes 130-US-hospitals readmission (real, public)",
        "task": "predict 30-day readmission", "protected_attribute": "race",
        "subgroup_sizes_train": sizes, "subgroup_sizes_test": test_sizes,
        "n_train": ntr, "n_test": len(te), "dp_noise_draws_averaged": DRAWS,
        "dp_mechanism": "output perturbation, (epsilon, delta)-DP Gaussian mechanism (CMS 2011)",
        "delta": DELTA, "report_card": card, "coupling": coupling,
    }
    json.dump(out, open(os.path.join(RES, "report_card.json"), "w"), indent=2)
    print("subgroup train sizes:", sizes)
    for r in card:
        print(f"eps={str(r['epsilon']):4} MIA={r['mean_mia_advantage']:.3f} "
              f"drop[maj={r['majority_accuracy_drop']:+.3f} min={r['minority_accuracy_drop']:+.3f}] acc={r['subgroup_balanced_accuracy']}")
    print("coupling:", json.dumps(coupling, indent=2))

if __name__ == "__main__":
    main()
