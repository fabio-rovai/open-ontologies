# certicoupla

**Copula-coupled, certificate-emitting uncertainty for scientific models: independent conformal
prediction gives the right per-target coverage and the wrong joint coverage. Measured on real DFT
data.**

Foundation and scientific models are deployed with uncertainty that is either uncalibrated or
calibrated one output at a time. But scientific predictions come in correlated vectors (a
material's formation energy, band gap and stability are not independent), and a certificate that
must hold *jointly*, the realized outcome vector falling inside an emitted region at the claimed
rate, needs the coupling. This case study shows, on **real Open Quantum Materials Database (OQMD)
data**, that per-target conformal prediction silently under-covers the joint region, and that a
coupled certificate fixes it.

This is the reliability layer Encode / ARIA Challengescape items on foundation-model uncertainty
and on validating scientific AI for deployment are asking for.

## The result

Deterministic run of [`src/certicoupla.py`](src/certicoupla.py) on 1,600 real materials, three
targets, a frozen gradient-boosted regressor, all certificates calibrated to **90% joint coverage**
on a held-out test split (full table in [`results/SUMMARY.md`](results/SUMMARY.md)):

| Method | Marginal coverage | **Joint coverage** | Region size |
|---|--:|--:|--:|
| Independent conformal (90%/target) | 0.902 | **0.792** | 1.0x |
| Bonferroni conformal | 0.960 | **0.907** | 6.18x |
| Coupled conformal (max-score box) | 0.954 | **0.897** | 4.83x |
| Gaussian-copula region | joint by design | **0.892** | 0.43x of coupled |

**Independent conformal is marginally correct (90.2% per target) but its joint coverage is only
79.2%, an 11-point shortfall against the 90% it claims.** The coupled certificate restores joint
coverage to 89.7%; the Gaussian-copula certificate matches it (89.2%) in a region 57% smaller. The
coupled box is also 22% tighter than the Bonferroni box while still holding nominal coverage.

The reason is in the data: the residuals are genuinely correlated (formation energy and stability
+0.50, formation energy and band gap -0.28). Independent intervals ignore that structure, so the
joint region they imply is the wrong shape and the wrong size.

## Why this is the ownable gap

- **Marginal coverage is not joint coverage.** A per-output 90% interval, stacked across d
  correlated outputs, does not give a 90% guarantee on the vector. Reporting the marginal number as
  if it were the deployment guarantee is the quiet failure.
- **Bonferroni is valid but wasteful.** Tightening each margin to 1 - alpha/d restores joint
  validity at the cost of intervals so wide they stop being useful (6.18x the volume here).
- **Coupling recovers both.** A global max-score conformal certificate has a finite-sample joint
  guarantee and is 22% tighter than Bonferroni; a Gaussian copula on the residuals uses the
  dependence to shrink the region much further, at the same coverage.
- **It emits a certificate.** Each prediction carries a machine-checkable region; soundness is the
  measured joint coverage against the claimed 90%, reported here on held-out data.

This composes with the frozen model rather than replacing it, and it is the uncertainty companion to
the [WorldKernel](https://github.com/fabio-rovai/worldkernel) coupling work.

## Reproduce

```bash
./run-demo.sh     # fetches real OQMD materials, trains + freezes a regressor, runs all certificates
```

Requires Python 3.10+ and network access (OQMD is queried live via its OPTIMADE endpoint).

## Honest scope

See [`BUILD_REPORT.md`](BUILD_REPORT.md). The frozen model is a gradient-boosted regressor trained
once and frozen; the point is the uncertainty wrapper, not the model, and the conformal guarantees
are distribution-free so they hold regardless of model quality. The copula is Gaussian (a first,
tractable dependence model); heavier-tailed vine copulas are the natural extension. The dataset is a
1,600-material slice of OQMD taken in id order, enough to show the effect, not a curated benchmark.
The result is a coverage phenomenon demonstrated on real data, not a claim of a new theorem.

---

### Built by Tesseract Academy

We build the calibration and assurance layer for scientific and foundation models. If you deploy a
model whose outputs are correlated and whose uncertainty has to hold jointly, we can wrap it in a
certificate that actually does.

[gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com
Part of [Open Ontologies](../../README.md) · MIT · real data, real numbers.
