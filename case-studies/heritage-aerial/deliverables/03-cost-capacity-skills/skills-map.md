# Skills Map

What capabilities institutions need to adopt NAPH at each tier, where these skills typically live (in-house vs. consultancy vs. shared sector infrastructure), and what the gap looks like for typical UK GLAM organisations.

This document is **paired with** [`cost-effort-analysis.md`](../../docs/cost-effort-analysis.md) — costs are dominated by skill availability, not by raw effort.

## Skill categories

### 1. Data engineering (Baseline core)

**What it is:** building, running, and validating ETL pipelines that transform legacy data into structured RDF/Turtle. Date normalisation, identifier minting, rights mapping, geometry construction.

**Specific tasks:**

- Reading messy CSV/XML/database exports
- Writing transformations in Python or equivalent
- Validating output via SHACL
- Setting up CI/CD for ongoing validation

**Existing in most GLAM institutions?** Partially. Often outsourced to digital-transformation contractors. Many institutions have someone who could learn it but currently doesn't have the role budget.

**Cost to acquire:** £400-700/day for a competent contractor; or 1-3 months training a willing in-house digital officer.

---

### 2. Linked data / semantic web (Enhanced+)

**What it is:** working knowledge of RDF, OWL, SHACL, SPARQL. Understanding subclass alignment, namespace management, the difference between Datasets and Resources.

**Specific tasks:**

- Designing or evaluating ontology extensions
- Writing SPARQL queries (especially federated)
- Reading and writing SHACL shapes
- Maintaining ontology versions

**Existing in most GLAM institutions?** Rarely in-house. Most institutions encountered linked-data projects as occasional consultancy engagements (e.g. one-off Europeana submission). Few maintain it day-to-day.

**Cost to acquire:** £600-900/day for a competent specialist; in-house up-skilling is a 3-6 month investment.

**Sector capacity comment:** This is the single largest bottleneck for sector-wide NAPH Enhanced/Aspirational adoption. The sector needs ~50 more competent linked-data practitioners than it currently has.

---

### 3. Archival / domain expertise (all tiers)

**What it is:** professional understanding of archival arrangement, provenance interpretation, conservation, and collection-specific traditions.

**Specific tasks:**

- Determining accurate provenance chains
- Resolving rights determinations
- Adjudicating sample-review outcomes
- Curating subject linkages (Aspirational)

**Existing in most GLAM institutions?** Yes, this is core institutional capability. Almost every NAPH-adopting institution will have this.

**Cost to acquire:** existing salaried staff; allocation of FTE to NAPH adoption is the cost.

---

### 4. ML / NLP / vision-language (Aspirational)

**What it is:** running classification models, fine-tuning for domain, training entity-linking pipelines, evaluating model outputs.

**Specific tasks:**

- Subject classification (vision-language models)
- HTR / OCR pipelines for manuscripts
- Cross-collection entity matching via embedding similarity
- Bias and error analysis on model outputs

**Existing in most GLAM institutions?** Almost never. Some specialist archives (the more digital-research-heavy ones) have one-off ML projects via academic partnerships.

**Cost to acquire:** £700-1,200/day for a competent ML engineer with heritage domain understanding; very rare combination.

**Sector capacity comment:** This is where shared-service infrastructure has the highest leverage. A central N-RICH service running entity-linking and classification as a service for sector clients would dramatically reduce per-institution cost.

---

### 5. Web / API engineering (Enhanced+)

**What it is:** running production web services — SPARQL endpoints, IIIF Image API, content-negotiated RDF serving.

**Specific tasks:**

- Operating a SPARQL endpoint with appropriate caching, security, query limits
- Running a IIIF Image API server (Cantaloupe, IIPImage)
- Setting up content negotiation in HTTP servers
- Monitoring, alerting, capacity planning

**Existing in most GLAM institutions?** Variable. National institutions usually have it; smaller institutions often don't, and outsource hosting.

**Cost to acquire:** £500-800/day for a generalist; or use a hosted service (NAPH-as-a-service is a viable model — see [shared infrastructure](#shared-infrastructure-options)).

---

### 6. QA / governance (Enhanced+)

**What it is:** running validation cycles, sampling-based reviews, drift detection, reporting.

**Specific tasks:**

- Defining sampling protocols
- Running quarterly reviews
- Tracking compliance metrics over time
- Producing audit reports for funders / aggregators

**Existing in most GLAM institutions?** Yes, for traditional cataloguing QA. NAPH-specific QA is new but builds on existing competence.

**Cost to acquire:** existing staff with 1-2 weeks of training in NAPH-specific tools.

---

## Skill matrix per tier

| Skill | Baseline | Enhanced | Aspirational |
|---|---|---|---|
| Data engineering | ✅ Required | ✅ Required | ✅ Required |
| Linked data / SPARQL | (S) | ✅ Required | ✅ Required |
| Archival / domain expertise | ✅ Required | ✅ Required | ✅ Required |
| ML / NLP / vision | ❌ | ❌ | ✅ Required (or shared service) |
| Web / API engineering | (S) | ✅ Required | ✅ Required |
| QA / governance | ✅ Required | ✅ Required | ✅ Required |

Legend: ✅ Required · (S) Recommended · ❌ Not needed

---

## Where to acquire each skill

### Option A: In-house

**Best for:** large institutions (50+ FTE) with existing digital teams; institutions adopting NAPH as part of broader digital transformation.

**Pros:** institutional knowledge retention; compatibility with existing IT systems; cost-efficient at scale.

**Cons:** recruitment difficulty for specialist skills (LinkedData, ML); ongoing training cost; harder to start small.

### Option B: Sector consultancy

**Best for:** medium institutions (10-50 FTE) doing specific projects; institutions that need NAPH adoption but don't yet have the capability stack.

**Pros:** rapid access to specialised skills; transfer of knowledge to in-house staff; right-sized engagements (5-30 days).

**Cons:** dependency on external relationships; documentation burden for handover.

### Option C: Shared sector infrastructure

**Best for:** small institutions (<10 FTE); institutions where Aspirational tier is desired but in-house ML/NLP isn't viable.

**Pros:** access to advanced capabilities at low marginal cost; standardised quality across the sector; reduces redundant per-institution capital expenditure.

**Cons:** dependency on shared-service operator; less customisation.

### Option D: Academic partnership

**Best for:** institutions with research-active collections; access to ML/NLP / vision capabilities through PhD students and postdocs.

**Pros:** high-skill access at low rates; produces academic papers + practical outputs.

**Cons:** academic timelines (semesters, grant cycles) don't match operational deadlines; turnover is high.

---

## Shared infrastructure options

The N-RICH programme has the opportunity to invest in **shared infrastructure** that addresses the skill bottlenecks at sector level. Recommended targets, in priority order:

1. **A shared subject-classification service** that runs vision-language models against any institution's images and returns structured subject suggestions (drafts requiring human validation)
2. **A shared entity-linking service** that resolves text mentions to GeoNames / Wikidata authorities
3. **A shared SPARQL hosting service** for institutions that can't run their own
4. **A shared HTR / OCR service** for manuscript collections

Each of these would cost £200k-£600k per year to operate sector-wide and would enable Aspirational-tier adoption at institutions that otherwise could not justify the per-institution investment.

These are **not** part of the immediate NAPH Pilot scope but are the natural follow-up for sustainable sector-wide adoption.

---

## What individual institutions can do today

If you're an institution evaluating NAPH adoption:

1. **Audit your existing skills** against the matrix above
2. **Identify your minimum viable capacity** — what skills you already have that cover Baseline?
3. **Plan your gap** — what's needed for Enhanced/Aspirational that you'd need to acquire?
4. **Choose your acquisition strategy** — in-house, consultancy, shared service, partnership — for each gap
5. **Cost it** against the [cost-effort analysis](../../docs/cost-effort-analysis.md) figures

NAPH Baseline is achievable for almost every UK GLAM institution today using existing or rapidly-acquired skills. Higher tiers require deliberate capability investment, and shared infrastructure dramatically lowers the per-institution cost of Aspirational tier.

## Cross-references

- [Cost-effort analysis](../../docs/cost-effort-analysis.md)
- [Investment case](investment-case.md)
- [How to use this standard](../04-adoption-guidance/how-to-use-this-standard.md)
