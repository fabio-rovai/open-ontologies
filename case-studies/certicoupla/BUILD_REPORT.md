# BUILD_REPORT: certicoupla

Honest record of data, method and limits. Every number in `results/` is produced by
`src/certicoupla.py` on a fixed seed (numpy default_rng(7), model random_state 0).

## What was fetched (real, public)

| Source | Endpoint | Use |
|---|---|---|
| OQMD (Open Quantum Materials Database) | oqmd.org/optimade/structures (live, keyless) | real DFT formation energy, band gap, stability per material |

Fetched 2026-07-20 via the OPTIMADE endpoint (`src/fetch.py`), 1,600 materials with all three
numeric properties present, taken in id order until the endpoint timed out. Composition features are
derived from the reduced chemical formula with a small embedded periodic table (atomic number and
Pauling electronegativity); missing electronegativities fall back to the median.

## Method

- **Split.** 50% train, 25% calibration, 25% test (random, seeded).
- **Frozen model.** One `GradientBoostingRegressor` per target, trained on the train split only and
  then frozen. The uncertainty methods never touch it.
- **Residuals.** Prediction errors on calibration and test, per target. Their correlation is real
  and non-trivial (formation energy / stability +0.50, formation energy / band gap -0.28).
- **Certificates, all calibrated to 90% joint coverage:**
  - *independent* : per-target split-conformal quantile of `|residual|` at 90%.
  - *Bonferroni* : per-target at `1 - 0.10/d` (d = 3).
  - *coupled* : global nonconformity `max_j |r_j| / s_j`; its 90% quantile scales an axis-aligned
    box. This has the standard finite-sample split-conformal joint coverage guarantee.
  - *copula* : rank-transform each residual to uniform (empirical CDF, the Gaussian-copula margins),
    map to normal scores, estimate the covariance, and use the Mahalanobis radius at 90% as an
    elliptical region. Test residuals are mapped through the calibration marginals before scoring.
- **Metrics.** Marginal coverage (mean over targets), joint coverage (all targets inside at once),
  and region size. Certificate soundness = measured joint coverage against the claimed 90%.

## The result, precisely

- Independent: marginal 0.902, **joint 0.792** (11-point shortfall). Marginally correct, jointly wrong.
- Bonferroni: joint 0.907, region 6.18x the independent box (valid but wide).
- Coupled: joint 0.897, region 4.83x independent, i.e. 22% tighter than Bonferroni at nominal coverage.
- Copula: joint 0.892, region 0.43x of the coupled box measured in the Gaussian-copula space.

## Limits of the claim (do not overstate)

1. **It is a coverage phenomenon, not a theorem.** That independent conformal under-covers a
   correlated joint is known; the contribution is measuring it, and the coupled/copula fix, on real
   DFT data with a clean margin.
2. **Region-size comparisons mix spaces.** Box volumes (independent, Bonferroni, coupled) are in the
   original target units; the copula region is compared to the coupled box in the Gaussian-copula
   space, not the original units, so the 0.43x is a fair like-for-like only there. This is stated,
   not smoothed.
3. **Gaussian copula.** A single, tractable dependence model; heavy tails and non-elliptical
   dependence would need a vine copula, which is the documented next step and the tie to the
   copula-net world-model program.
4. **Frozen model is a proxy for a foundation model.** A gradient-boosted regressor stands in for a
   large frozen scientific model; the conformal guarantees are distribution-free, so the coverage
   result transfers, but running the wrapper on ESM-2 / ESMFold or a Materials Project foundation
   model is the honest scale-up.
5. **Dataset slice.** 1,600 OQMD entries in id order, not a curated or class-balanced benchmark.

## Reproducibility

`./run-demo.sh` re-fetches OQMD and reruns the analysis. Live OQMD content can shift, so exact
numbers may move slightly; the structural result (independent under-covers, coupled/copula restore)
is stable. numpy, scipy, scikit-learn at build time.
