# NAPH Commerce Profile

**Machine-readable licensing for rights-reserved aerial photography collections.**

Built and validated against 292 real records from the National Collection of
Aerial Photography's public catalogue. Offered to NCAP as a gift, with no
strings.

## The problem it solves

To a machine, restrictive rights metadata and absent rights metadata are
identical: both cause the record to be dropped. A collection marked "In
Copyright" with no further information is excluded by every aggregator, crawler
and research pipeline that meets it. For an archive that funds its digitisation
by licensing reproductions, that is a revenue leak, not an openness debate.

The NAPH Baseline rights module (Module C) is a *status* vocabulary. It records
what the copyright position is. It has no way to say:

    rights reserved, AND licensable, AND here is the route to acquire it.

This profile adds the second and third clauses.

## What is here

| Path | What it is |
|---|---|
| `ontology/naph-commerce.ttl` | OWL extension: availability scheme, ODRL-aligned offers, discovery surface |
| `ontology/naph-commerce-shapes.ttl` | SHACL shapes; one rule carries the profile |
| `build_commerce.py` | Lifts a NAPH Baseline graph into the commerce profile and emits STAC |
| `brief-for-ncap.md` | The one-page case, written for the archive rather than for a standards body |
| `build/` | Generated output, reproducible from the command below; `build/report.json` records the verified counts |

## Design decisions worth knowing

**Availability is separate from copyright status.** Every existing rights
vocabulary conflates them. A public-domain frame may be unobtainable because it
is unscanned; an in-copyright frame may be trivially licensable. The scheme has
five concepts: Licensable, Openly available, Not yet digitised, Access
restricted, Rights unresolved. The last exists so an institution can state
honestly that a determination is incomplete, which is more useful downstream
than silence.

**Offers are ODRL 2.2** (W3C Recommendation, 15 February 2018). Each licensable
record carries an Offer naming the assigner, a `reproduce` permission, and a
`compensate` duty. No price is asserted anywhere. Price belongs on the order
page and stays there.

**No claim is invented.** The per-item order route is the catalogue URL each
harvested record already carried as `prov:hadPrimarySource`. The profile makes
an existing fact machine-actionable.

**The discovery surface is explicit.** Footprint, date and a pixel-capped
preview may be indexed by third parties; the deliverable raster is the product.
Stating this per record is what lets an aggregator carry the collection instead
of guessing conservatively.

## Reproduce it

```bash
python3 build_commerce.py \
    --source ../../data/real-ncap-sample.ttl
```

## Verified results

```
records                              292
offers with per-item order route     292  (100%)
offers needing a fallback              0
triples                           11,382
SHACL violations                       0
STAC items                           292
```

The shapes were also run against deliberately broken input, because a validator
that never fails proves nothing:

| Case | Expected | Result |
|---|---|---|
| Licensable record, no offer | fail | fails, with the reason |
| Offer with assigner, target and permission but no route | fail | fails, with the reason |
| Same offer, order endpoint added | pass | passes |

## Status

Proposed extension to NAPH, offered for comment. Rights in the underlying
imagery and catalogue remain with NCAP and Historic Environment Scotland.
Nothing in this repository redistributes catalogue records; the build runs
against a locally held Baseline sample.

## Negative controls

`controls/` holds three deliberately broken fixtures and a runner
(`controls/run_controls.py`) that asserts the expected outcome of each:
a Licensable record with no offer FAILS, an offer with no route FAILS,
and the same offer with an order endpoint PASSES.
