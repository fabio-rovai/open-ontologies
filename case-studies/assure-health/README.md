# assure-health

**One report card for privacy AND fairness: differential privacy applied to a clinical model
costs the smallest patient subgroups 2.6x more accuracy than the majority, while the membership
leakage it targets is near zero. You only see the trade when both planes are on one card.**

Privacy and fairness are audited on separate benches, by separate people, with separate
incentives. But they interact: the standard privacy defence, differential privacy, is known to
degrade minority-subgroup accuracy fastest (the "disparate impact of differential privacy",
Bagdasaryan et al., NeurIPS 2019). So a model can pass a privacy audit and a fairness audit run
independently, while the act of making it private is what broke equity for the exact groups
clinical safety cares about. This is one assurance report card that runs both on the same model
and shows the coupling.

Built for the Encode / ARIA Challengescape items on rigorous privacy guarantees for sensitive-data
ML and on equitable, clinically-safe health AI.

## The result

Deterministic run of [`src/assure.py`](src/assure.py) on the real, public **UCI Diabetes
130-US-hospitals** readmission dataset (predict 30-day readmission; protected attribute: race),
metrics averaged over 40 DP noise draws (full card in [`results/report_card.json`](results/report_card.json)):

| epsilon (privacy) | membership-inference advantage | accuracy drop, majority | accuracy drop, minority |
|---|--:|--:|--:|
| inf (non-private) | 0.004 | +0.000 | +0.000 |
| 4.0 | 0.004 | +0.002 | +0.004 |
| 1.0 | 0.006 | +0.005 | +0.011 |
| 0.5 | 0.006 | +0.009 | +0.028 |
| 0.25 | 0.013 | **+0.025** | **+0.065** |

At the tightest privacy (epsilon = 0.25), the accuracy differential privacy costs is not shared
evenly. Ordered by subgroup size, the drop is **2.5 points for Caucasians (n=6,150), 4.4 for
African Americans (n=1,520), and 8.5 for Hispanics (n=153)**, a **2.6x disparate impact** on the
minority groups, growing monotonically as privacy tightens. That is the disparate impact of
differential privacy, measured on a real clinical task.

## The coupling (the ownable primitive)

The membership-inference advantage this DP is meant to reduce is **near zero** on this regularised
linear model (about 0.004, essentially no leakage). So the privacy the DP buys here is marginal,
and the equity it costs is real and concentrated on the smallest subgroups. Now look at how three
different audits read the same situation:

- A **privacy-only** audit signs off: "differential privacy applied at epsilon = 0.25." A win.
- A **fairness-only** audit at a fixed epsilon sees a subgroup accuracy gap, but not that the DP
  knob caused it.
- The **coupled report card** shows the trade for what it is: almost all equity cost, for almost no
  privacy gain, borne by the patients with the least data.

The ownable contribution is not a new attack or a new fairness metric. It is putting both planes on
one model, across a privacy sweep, so the interaction is visible. The engine is designed to reuse
our [etiq](https://github.com/fabio-rovai) assurance primitives and the
[open-governance](https://github.com/fabio-rovai/open-governance) audit trail so the card is
provenance-signed.

## Reproduce

```bash
./run-demo.sh     # fetches the UCI dataset, trains, runs both planes under the DP sweep
```

Requires Python 3.10+ and network access (the dataset is fetched via `ucimlrepo`).

## Honest scope

See [`BUILD_REPORT.md`](BUILD_REPORT.md). The model is a regularised linear classifier, which by
nature leaks little to membership inference; higher-capacity or overfit models leak substantially
more (Shokri et al. 2017, Yeom et al. 2018), and the same report card applies to them, where the
privacy side of the trade would be larger. The DP mechanism is output perturbation (a rigorous
closed-form (epsilon, delta)-DP), the convex analogue of DP-SGD for deep models. The data is a real
but specific US dataset with race as the protected attribute; MIMIC-IV (credentialed) is the
clinical scale-up. The disparate-impact finding is robust and monotone; the absolute membership
leakage is dataset- and model-specific and reported as measured, not inflated.

---

### Built by Tesseract Academy

We build the assurance layer for AI on sensitive data. If you are deploying a model on patient or
personal data and need privacy and fairness assured together, on one auditable report, we can help.

[gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com
Part of [Open Ontologies](../../README.md) · MIT · real data, real numbers.
