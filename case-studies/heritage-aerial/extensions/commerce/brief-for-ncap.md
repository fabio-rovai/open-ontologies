# Open metadata, licensed pixels

## A machine-readable licensing layer for NCAP, built and validated on NCAP's own public catalogue

Fabio Rovai, The Tesseract Academy. 21 August 2026.

---

## The short version

Earlier work of mine measured how close the NCAP catalogue already is to being
computation-ready, and found that one field was missing: a machine-readable
rights statement. Stated that way it reads like a criticism, and it is the wrong
way round.

Looking again at the same 292 records, every single one already carries a
per-frame catalogue URL in the API payload, of the form
`airphotofinder.ncap.org/image/797810`. That is the page from which a licence
for that exact frame can be bought. So NCAP does not lack the commercial
substrate. It has **292 out of 292, one hundred per cent**, of the thing that
actually matters commercially. What is missing is one field saying so in a form
a machine can act on.

I have built that field, wired it to your live catalogue URLs, and validated it.
It is yours, with no strings and no expectation.

## Why this is a revenue question rather than an openness question

Most people who approach a national collection about open data are asking it to
give away the thing it sells. This is the opposite request, and it rests on one
observation:

> To a machine, restrictive rights metadata and absent rights metadata are
> identical. Both cause the record to be dropped.

An aggregator, a search crawler or a research pipeline that encounters a record
marked "In Copyright" with no further information does the only safe thing and
excludes it. The collection becomes invisible to precisely the automated
discovery that would have generated its licensing enquiries. The current state
is not neutral; it costs enquiries that were never made because the researcher
never saw the frame existed.

The fix is not to open the imagery. It is to say, in machine-readable form:

    rights reserved, AND licensable, AND here is the route to acquire it.

The first clause is what you already publish. The second and third clauses are
what turn a dead end into a sales channel. This is what the profile adds.

## What has been built

**A NAPH Commerce Profile.** An OWL extension with a SKOS availability scheme
that separates two things every existing rights vocabulary conflates: copyright
*status* and reproduction *availability*. A public-domain frame may be
unobtainable because it is not scanned; an in-copyright frame may be trivially
licensable. Current vocabularies cannot tell those apart, which is why "In
Copyright" reads as "go away".

The availability concepts are Licensable, Openly available, Not yet digitised,
Access restricted, and Rights unresolved. That last one matters for a collection
of declassified material of mixed origin: being able to state honestly that a
determination has not been completed is more useful to a downstream consumer
than silence, and it commits you to nothing.

**Offers expressed in ODRL.** Each licensable record carries an ODRL 2.2 Offer
(W3C Recommendation) naming the assigner, the permitted action, and a
compensation duty. No price is asserted anywhere; the amount stays where it
belongs, on your order page. It also carries `scanOnDemand`, which converts an
apparent absence into an offer for the large majority of the collection that is
not yet digitised.

**A discovery surface.** A per-record statement of what a third party may index
and display without a licence: the footprint, the date, and a preview capped at
a stated pixel size. This is the clause that makes it safe for an aggregator to
carry your catalogue, because right now they are guessing and guessing
conservatively.

**SHACL shapes that enforce the point.** One rule carries the profile: a record
declared licensable must carry a resolvable route to that licence. An offer with
no route is not an offer.

## What was run, and what it produced

Input was the 292 real frames harvested from the public Air Photo Finder API on
2 July 2026, metadata only, already lifted to the NAPH Baseline tier.

| Measure | Result |
|---|---|
| Records processed | 292 |
| Offers with a **per-item** order route | **292 (100%)** |
| Offers needing a collection-level fallback | 0 |
| Triples in the output graph | 11,382 |
| SHACL violations against the commerce shapes | **0** |
| STAC items emitted, licence and order links included | 292 |

The order routes were not invented. Each one is the catalogue URL the record
already carries as its primary source. The profile makes an existing fact
actionable; it does not add a claim.

The shapes were also tested against deliberately broken input, because a
validator that never fails proves nothing. A record declared licensable with no
offer fails. An offer carrying an assigner, a target and a permission but no
order route fails. Adding the order route makes it pass.

## The STAC surface, and who it reaches

The build also emits a STAC catalogue in which the licence is a first-class
field and the order route is a typed link. STAC is the standard the entire
modern geospatial toolchain reads, so a STAC-published NCAP becomes visible in
QGIS, in stac-browser and in any pipeline built on pystac, without a single
pixel being given away. Footprint, date and a capped preview generate the
demand; the deliverable raster remains the product.

The audience for this is not an abstraction. It is the community represented by
the economists on your own scanning paper. A researcher planning a study needs
to know, before writing a proposal, whether coverage exists for a place and a
decade and whether it can be obtained on a timetable. That is a footprint query,
a date filter and a turnaround figure. Today that question is answerable only by
a human reading a search interface, which means for most study designs it is not
asked at all and the collection is quietly excluded.

## What this is not

It is not a request for NCAP to open its imagery, adopt a Creative Commons
licence, or change its business model. Every design decision here assumes the
opposite: that reproduction rights are reserved, that a fee is payable, and that
the fee is set by NCAP per order.

It is not a product pitch. The profile, the shapes and the build script are
offered as they are, for NCAP to use, ignore, fork or hand to someone else.

It is not a criticism of your cataloguing. The finding underneath all of this is
that the substrate is already present and always was. The earlier gap analysis
I wrote by reading your public website was wrong on two of three points, and
reading the actual API payload corrected it. That correction is in the paper.

## What would make it real

Three things, and only the first needs anyone's permission.

1. **A decision on the metadata licence.** The profile carries a placeholder of
   CC BY 4.0 for catalogue metadata, distinct from the imagery. Only NCAP can
   set that. Open metadata with reserved pixels is a coherent and well-precedented
   position, but it is only actionable by third parties if it is stated.
2. **A turnaround figure and a scan-on-demand policy** per collection. Two
   values, and they unblock research planning.
3. **Emitting the profile from the existing API.** The mapping is mechanical
   and the build script demonstrates it end to end on live data.

If any of it is useful, take it. If the modelling is wrong somewhere, I would
rather be told than be polite about it.
