# BUILD_REPORT: assure-health

Honest record of data, method, and the limits of the claim. Every number in `results/` is produced
by `src/assure.py` on a fixed seed.

## What was used (real, public)

| Source | Use |
|---|---|
| UCI Diabetes 130-US-hospitals for years 1999-2008 (id 296, via `ucimlrepo`) | 101,766 hospital encounters; predict 30-day readmission; race as the protected attribute |

Fetched 2026-07-21. Features: eight numeric clinical counts (time in hospital, lab procedures,
procedures, medications, outpatient/emergency/inpatient visits, diagnoses) plus an ordinal age band.
Rows with missing race are dropped. Features are standardised and each row is scaled to unit L2 norm
(required for the DP sensitivity bound). Target: readmitted within 30 days.

## Method

- **Frozen model.** L2-regularised logistic regression (`sklearn`, `fit_intercept=False`), trained
  once on 8,000 encounters. lambda = 0.01, so `sklearn` C = 1/(lambda*n).
- **Differential privacy by output perturbation** (Chaudhuri, Monteleoni, Sarwate 2011): the trained
  weight vector has L2 sensitivity Delta = 2/(n*lambda) for 1-Lipschitz logistic loss on unit-norm
  rows. We release w + N(0, sigma^2 I) with sigma = Delta * sqrt(2 ln(1.25/delta)) / epsilon, a
  rigorous (epsilon, delta)-DP Gaussian mechanism (delta = 1e-5). epsilon in {inf, 4, 1, 0.5, 0.25}.
- **Averaging.** DP is randomised, so every metric is averaged over 40 independent noise draws; a
  single draw is far too noisy to read a trend from.
- **Privacy plane.** Loss-based membership inference (Yeom et al. 2018): members (train) tend to have
  lower loss than non-members (held-out). Per subgroup, the attack advantage is 2*AUC - 1 of the loss
  separating members from non-members.
- **Equity plane.** Per-subgroup balanced accuracy on 12,000 held-out encounters, and the accuracy
  DROP relative to the non-private model, the actual disparate-impact quantity.
- **Subgroups.** Analysis is restricted to the three subgroups with stable statistics (Caucasian
  6,150 / African American 1,520 / Hispanic 153 in train; larger in test). Asian (41) and Other (136)
  are too small for a reliable per-group accuracy estimate and are excluded from the comparison,
  stated rather than silently included.

## The result, precisely

- Disparate impact of DP at epsilon = 0.25: accuracy drop 2.5 points (Caucasian), 4.4 (African
  American), 8.5 (Hispanic), monotone with decreasing subgroup size; minority-to-majority drop ratio
  2.58x. The trend is monotone across the whole epsilon sweep.
- Membership-inference advantage on this linear model is near zero (about 0.004 non-private), so the
  privacy the DP buys here is marginal.

## Limits of the claim (do not overstate)

1. **Linear model leaks little by design.** A regularised linear classifier does not memorise
   individual records, so its membership-inference advantage is near zero; the "privacy gained" side
   of the trade is therefore small HERE. On higher-capacity or overfit models (deep nets, boosted
   trees) membership inference is substantial (Shokri 2017, Yeom 2018), and the same report card
   would show a larger privacy benefit against the same equity cost. The contribution is the coupled
   card, not a claim that all models leak equally.
2. **Membership-inference numbers are model/dataset specific.** Reported as measured, not inflated;
   the small absolute values are honest, and the slight non-monotonicity at near-zero advantage is
   noise, not signal.
3. **One DP mechanism.** Output perturbation is a rigorous closed-form (epsilon, delta)-DP mechanism
   for convex ERM; DP-SGD is the analogue for non-convex/deep models and would be the extension there.
4. **One dataset, one protected attribute.** Real but specific (US hospital readmission, race).
   MIMIC-IV and MIMIC-IV-Note (both credentialed under a PhysioNet DUA) are the clinical scale-up,
   including the mental-health arm; this public dataset avoids the DUA while making the method and the
   coupling concrete.
5. **Engine.** This reference computes the report card directly; wiring it through the etiq assurance
   primitives and the open-governance audit trail (for a provenance-signed card) is the productisation.

## Reproducibility

`./run-demo.sh` re-fetches the dataset and reruns the analysis. numpy/pandas/scikit-learn/ucimlrepo
at build time; fixed seed, 40 DP draws, so results are stable across machines.
