# Results summary

Real DFT data: **1600 materials from the OQMD**, three targets (delta_e, band_gap, stability). Frozen gradient-boosted regressor; all certificates calibrated to **90% joint coverage** on a held-out test split. Residual correlations are real (formation energy and stability +0.50), which is why the joint problem is not the marginal problem.

| Method | Marginal coverage | **Joint coverage** | Region size |
|---|--:|--:|--:|
| Independent conformal (90%/target) | 0.902 | **0.792** | 1.0x (baseline) |
| Bonferroni conformal | 0.960 | **0.907** | 6.18x |
| Coupled conformal (max-score box) | 0.954 | **0.897** | 4.83x |
| Gaussian-copula region | (joint by design) | **0.892** | 0.43x of coupled* |

**Headline.** Independent conformal is marginally correct (0.902 per target) but its **joint coverage is only 0.792**, an 11-point shortfall against the 90% claim. The **coupled certificate restores joint coverage to 0.897** and the **Gaussian-copula certificate matches it (0.892) with a much smaller region**. Coupled is also 22% tighter than Bonferroni while holding nominal coverage.

*region size for the copula is measured in the Gaussian-copula space against the coupled box; see BUILD_REPORT.