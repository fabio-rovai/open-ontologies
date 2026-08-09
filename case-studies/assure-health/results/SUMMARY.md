# Assurance report card: privacy AND fairness on one model

Dataset: **UCI Diabetes 130-US-hospitals readmission (real, public)**, task = predict 30-day readmission, protected attribute = race. Frozen L2 logistic model; differential privacy by output perturbation, (epsilon, delta)-DP Gaussian mechanism (CMS 2011); metrics averaged over 40 DP noise draws. Subgroup train sizes: Caucasian 6150, AfricanAmerican 1520, Hispanic 153.

| epsilon (privacy) | membership-inference advantage | accuracy drop, majority (Caucasian) | accuracy drop, minority (AfrAm+Hisp) |
|---|--:|--:|--:|
| inf | 0.004 | +0.000 | +0.000 |
| 4.0 | 0.004 | +0.002 | +0.004 |
| 1.0 | 0.006 | +0.005 | +0.011 |
| 0.5 | 0.006 | +0.009 | +0.028 |
| 0.25 | 0.013 | +0.025 | +0.065 |

**Headline.** At the tightest privacy (epsilon = 0.25), differential privacy costs the minority subgroups **2.58x** more accuracy than the majority (per subgroup, ordered by size: Caucasian 6150 -> 2.5 points, AfricanAmerican 1520 -> 4.4 points, Hispanic 153 -> 8.5 points). This is the disparate impact of differential privacy, and it grows monotonically as privacy tightens.

**The coupling.** Meanwhile the membership-inference advantage this DP is meant to reduce is near zero on this regularised linear model (0.004), so the privacy it buys here is marginal. A privacy-only audit would sign off 'DP applied at epsilon=0.25' as a win; a fairness audit at a fixed epsilon would see a subgroup gap but not its cause. Only both planes on one card show the trade for what it is: on this model, almost all equity cost for almost no privacy gain. Audit them together.