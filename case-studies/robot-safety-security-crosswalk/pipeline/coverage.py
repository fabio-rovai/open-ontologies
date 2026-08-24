#!/usr/bin/env python3
"""Count what the RSSC crosswalk covers, and say plainly what it does not.

Reads the same graph validate.py reads (every .ttl in ontology/, crosswalk/,
data/ and the case-study root) and writes pipeline/coverage-report.md plus
pipeline/coverage-metrics.json.

The report answers four questions with counts rather than claims:
  1. How many declared ISO 10218:2025 clauses carry at least one mapping.
  2. Which declared ISO 10218:2025 clauses carry none. Listed in full, every
     one of them, never sampled and never truncated.
  3. How the assertions distribute across the three autonomy bands, including
     the bands that come out empty.
  4. Which security-side anchors (the seven foundational requirements, the
     cited security requirements) are never reached.

Two honesty rules are built into the arithmetic and are restated in the report.

First, the denominator is the set of clause references DECLARED IN THIS GRAPH,
not the full clause tree of ISO 10218-1:2025 or ISO 10218-2:2025. Those
documents are paywalled; their contents pages are public, their clause bodies
are not. A coverage percentage against the whole standard would be a fabricated
denominator, so none is printed. What is printed is coverage against the
clauses this project could cite, which is a smaller and honest claim.

Second, an empty cell is a result. A band with no assertions, a foundational
requirement nothing maps to, a clause with no mapping: each is reported as a
finding with its own line, because in this artefact the absence of coverage for
the higher autonomy bands is the argument, not an omission to be papered over.

Takes no arguments and reads fixed paths, as the house build pipelines do.
Exit codes: 0 report written; 1 nothing to count; 2 inputs missing or unparsable.
"""

import json
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path

from rdflib import Literal, URIRef

import rssc_graph as R

REPORT = R.ROOT / "pipeline" / "coverage-report.md"
METRICS = R.ROOT / "pipeline" / "coverage-metrics.json"
DEFAULT_ROOT = Path(__file__).resolve().parent.parent

ASSERTION_LABELS = [
    ("crosswalk_assertions", R.CROSSWALK_ASSERTION, "rssc:CrosswalkAssertion"),
    ("safety_impact_assertions", R.SAFETY_IMPACT_ASSERTION, "rssc:SafetyImpactAssertion"),
    ("security_level_claims", R.SECURITY_LEVEL_CLAIM, "rssc:SecurityLevelClaim"),
    ("control_gaps", R.CONTROL_GAP, "rssc:ControlGap"),
]


def cell(value):
    """Make a value safe to drop into a markdown table cell."""
    text = "" if value is None else str(value)
    return text.replace("|", "\\|").replace("\n", " ").strip()


def clause_sort_key(identifier):
    """Sort '5.1.16' before '5.2.1' before '7.5.11', annexes after clauses."""
    text = str(identifier)
    if not text or not text[0].isdigit():
        return (1, text, ())
    parts = []
    for chunk in text.replace(" ", ".").split("."):
        parts.append(int(chunk) if chunk.isdigit() else 0)
    return (0, "", tuple(parts))


def main():
    data_load = R.load_data()

    if data_load.errors or not data_load.files:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        lines = [
            "# RSSC coverage report",
            "",
            "**No report could be produced.** The pipeline found nothing to count.",
            "",
        ]
        for d in data_load.missing_dirs:
            lines.append(f"- missing directory: `{d}`")
        for d in data_load.empty_dirs:
            lines.append(f"- no `.ttl` files in: `{d}`")
        for path, err in data_load.errors:
            lines.append(f"- parse failure in `{path}`: {err}")
        lines += [
            "",
            "Nothing was estimated, inferred or filled in. The absence of input is",
            "reported as an absence of input.",
            "",
        ]
        REPORT.write_text("\n".join(lines), encoding="utf-8")
        print("coverage: cannot count, inputs missing or unparsable")
        for d in data_load.missing_dirs:
            print(f"  missing directory: {d}")
        for d in data_load.empty_dirs:
            print(f"  no .ttl files in:  {d}")
        for path, err in data_load.errors:
            print(f"  parse failure:     {path}: {err}")
        print(f"  wrote {REPORT}")
        return 2

    g = data_load.graph

    # ------------------------------------------------------------- inventories
    assertions_by_key = {}
    all_assertions = set()
    for key, cls, _label in ASSERTION_LABELS:
        members = R.instances_of(g, R.subclass_closure(g, cls))
        assertions_by_key[key] = members
        all_assertions |= members
    all_assertions |= R.instances_of(g, R.subclass_closure(g, R.EVIDENCED_ASSERTION))

    standards = sorted(
        R.instances_of(g, R.subclass_closure(g, R.STANDARD)), key=str
    )
    clauses = sorted(
        R.instances_of(g, R.subclass_closure(g, R.CLAUSE_REFERENCE)), key=str
    )
    sec_reqs = sorted(
        R.instances_of(g, R.subclass_closure(g, R.SECURITY_REQUIREMENT)), key=str
    )
    frs = sorted(
        R.instances_of(g, R.subclass_closure(g, R.FOUNDATIONAL_REQUIREMENT)), key=str
    )
    bands = sorted(
        R.instances_of(g, R.subclass_closure(g, R.AUTONOMY_BAND)), key=str
    )
    sources = sorted(R.instances_of(g, R.subclass_closure(g, R.SOURCE)), key=str)
    safety_functions = sorted(
        R.instances_of(g, R.subclass_closure(g, R.SAFETY_FUNCTION)), key=str
    )

    if not all_assertions:
        note = (
            "The graph parsed, but it contains no instance of "
            "rssc:EvidencedAssertion. There is nothing to count yet."
        )
    else:
        note = None

    # --------------------------------------------------- clause to assertions
    mappings_for_clause = defaultdict(set)
    for node in assertions_by_key["crosswalk_assertions"]:
        for clause in g.objects(node, R.P_SUBJECT_CLAUSE):
            mappings_for_clause[clause].add(node)

    gaps_for_clause = defaultdict(set)
    for node in assertions_by_key["control_gaps"]:
        for clause in g.objects(node, R.P_UNADDRESSED_BY):
            gaps_for_clause[clause].add(node)

    # A safety impact assertion reaches a clause through the safety function it
    # degrades, because every safety function carries rssc:definedByClause.
    impacts_for_clause = defaultdict(set)
    for node in assertions_by_key["safety_impact_assertions"]:
        for func in g.objects(node, R.P_IMPACTED_SAFETY_FUNCTION):
            for clause in g.objects(func, R.P_DEFINED_BY_CLAUSE):
                impacts_for_clause[clause].add(node)

    def bands_of(nodes):
        out = set()
        for node in nodes:
            for band in g.objects(node, R.P_APPLIES_TO_BAND):
                out.add(band)
        return out

    # ------------------------------------------------------- standards in view
    iso10218 = []
    for std in standards:
        designation = str(R.one(g, std, R.P_DESIGNATION, Literal("")))
        if "10218" in designation:
            iso10218.append(std)
    iso10218.sort(key=lambda s: str(R.one(g, s, R.P_DESIGNATION, Literal(""))))

    clauses_by_standard = defaultdict(list)
    orphan_clauses = []
    for clause in clauses:
        parents = list(g.objects(clause, R.P_IN_STANDARD))
        if not parents:
            orphan_clauses.append(clause)
        for parent in parents:
            clauses_by_standard[parent].append(clause)

    def clause_rows(clause_list):
        rows = []
        for clause in clause_list:
            identifier = str(R.one(g, clause, R.P_CLAUSE_IDENTIFIER, Literal("")))
            rows.append(
                {
                    "node": clause,
                    "id": identifier,
                    "title": str(R.one(g, clause, R.P_CLAUSE_TITLE, Literal(""))),
                    "status": str(R.one(g, clause, R.P_NORMATIVE_STATUS, Literal(""))),
                    "mappings": len(mappings_for_clause.get(clause, ())),
                    "gaps": len(gaps_for_clause.get(clause, ())),
                    "impacts": len(impacts_for_clause.get(clause, ())),
                    "bands": sorted(
                        R.notation(g, b)
                        for b in bands_of(
                            mappings_for_clause.get(clause, set())
                            | impacts_for_clause.get(clause, set())
                            | gaps_for_clause.get(clause, set())
                        )
                    ),
                }
            )
        rows.sort(key=lambda r: clause_sort_key(r["id"]))
        return rows

    iso_rows = []
    for std in iso10218:
        for row in clause_rows(clauses_by_standard.get(std, [])):
            row["standard"] = str(R.one(g, std, R.P_DESIGNATION, Literal("")))
            iso_rows.append(row)

    iso_total = len(iso_rows)
    iso_mapped = [r for r in iso_rows if r["mappings"] > 0]
    iso_unmapped = [r for r in iso_rows if r["mappings"] == 0]
    iso_touched = [
        r for r in iso_rows if r["mappings"] or r["gaps"] or r["impacts"]
    ]
    iso_untouched = [
        r for r in iso_rows if not (r["mappings"] or r["gaps"] or r["impacts"])
    ]
    mapping_rate = round(len(iso_mapped) / iso_total, 4) if iso_total else None
    touch_rate = round(len(iso_touched) / iso_total, 4) if iso_total else None

    # ---------------------------------------------------------- band breakdown
    band_matrix = defaultdict(Counter)   # band notation -> assertion kind -> n
    unbanded = Counter()
    for key, _cls, label in ASSERTION_LABELS:
        for node in assertions_by_key[key]:
            values = list(g.objects(node, R.P_APPLIES_TO_BAND))
            if not values:
                unbanded[label] += 1
            for value in values:
                band_matrix[R.notation(g, value)][label] += 1

    band_names = [R.notation(g, b) for b in bands]
    band_clause_cover = {}
    for band in bands:
        name = R.notation(g, band)
        covered = set()
        for row in iso_rows:
            if name in row["bands"]:
                covered.add(row["id"])
        band_clause_cover[name] = sorted(covered, key=clause_sort_key)

    # ------------------------------------------------- security side reachback
    fr_hits = defaultdict(set)
    for node in all_assertions:
        for prop in R.SECURITY_ANCHOR_PROPERTIES:
            for target in g.objects(node, prop):
                fr_hits[target].add(node)
    # A security requirement resolves up to its foundational requirement.
    for req in sec_reqs:
        if req in fr_hits:
            for parent in g.objects(req, R.P_DERIVED_FROM_FR):
                fr_hits[parent] |= fr_hits[req]

    fr_rows = []
    for fr in sorted(frs, key=lambda n: R.notation(g, n)):
        fr_rows.append(
            {
                "id": R.notation(g, fr),
                "label": str(R.one(g, fr, URIRef(
                    "http://www.w3.org/2004/02/skos/core#prefLabel"), Literal(""))),
                "hits": len(fr_hits.get(fr, ())),
            }
        )
    fr_unreached = [r for r in fr_rows if r["hits"] == 0]

    req_rows = []
    for req in sec_reqs:
        req_rows.append(
            {
                "id": str(R.one(g, req, R.P_REQUIREMENT_IDENTIFIER, Literal(""))),
                "title": str(R.one(g, req, R.P_REQUIREMENT_TITLE, Literal(""))),
                "hits": len(fr_hits.get(req, ())),
            }
        )
    req_rows.sort(key=lambda r: r["id"])
    req_unreached = [r for r in req_rows if r["hits"] == 0]

    # ------------------------------------------------ evidence and confidence
    ev_counter = Counter()
    conf_counter = Counter()
    for node in all_assertions:
        values = list(g.objects(node, R.P_EVIDENCE_TYPE))
        if not values:
            ev_counter["(missing)"] += 1
        for value in values:
            ev_counter[R.notation(g, value)] += 1
        values = list(g.objects(node, R.P_CONFIDENCE))
        if not values:
            conf_counter["(missing)"] += 1
        for value in values:
            conf_counter[R.notation(g, value)] += 1

    uncited = sorted(
        R.short(n) for n in all_assertions
        if not list(g.objects(n, R.P_CITATION))
    )
    unsourced = sorted(
        R.short(n) for n in all_assertions
        if not list(g.objects(n, URIRef("http://www.w3.org/ns/prov#wasDerivedFrom")))
    )
    unused_sources = sorted(
        R.short(s) for s in sources
        if not list(g.subjects(
            URIRef("http://www.w3.org/ns/prov#wasDerivedFrom"), s))
    )
    standards_without_clauses = [
        str(R.one(g, s, R.P_DESIGNATION, Literal(R.short(s))))
        for s in standards
        if not clauses_by_standard.get(s)
    ]

    # -------------------------------------------------------------- the report
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    L = []
    add = L.append

    add("# RSSC coverage report")
    add("")
    add("Counted output of `pipeline/coverage.py`. Every number below is produced")
    add("by that script from the committed Turtle; none is hand-entered. Re-run the")
    add("script and this file is rewritten.")
    add("")
    add(f"- generated: {now}")
    add(f"- case study root: `{R.ROOT}`")
    add(f"- graph: {data_load.triples} triples from {len(data_load.files)} file(s)")
    add("")
    add("| File | Triples | sha256 (first 16) |")
    add("|---|---:|---|")
    for path, count, digest in data_load.files:
        add(f"| `{path.relative_to(R.ROOT)}` | {count} | `{digest}` |")
    add("")
    if data_load.missing_dirs or data_load.empty_dirs:
        add("Data locations that contributed nothing, listed so that a thin graph")
        add("cannot be mistaken for a complete one:")
        add("")
        for d in data_load.missing_dirs:
            add(f"- `{d.name}/` does not exist")
        for d in data_load.empty_dirs:
            name = d.name if d != R.ROOT else "(case study root)"
            add(f"- `{name}` holds no `.ttl` files")
        add("")
    if R.ROOT != DEFAULT_ROOT:
        add("> **This report was generated against a non-default `RSSC_ROOT`.**")
        add("> It is a self-test fixture result, not the coverage of the published")
        add("> crosswalk. Fixture nodes are minted under a `SELFTEST-FIXTURE` base.")
        add("")
    if note:
        add(f"> {note}")
        add("")

    add("## 1. What was counted, and against what denominator")
    add("")
    add("The denominator for clause coverage is the set of `rssc:ClauseReference`")
    add("nodes **declared in this graph** and bound with `rssc:inStandard` to an")
    add("ISO 10218:2025 part. It is not the clause tree of the published standard.")
    add("ISO 10218-1:2025 and ISO 10218-2:2025 are paywalled: their contents pages")
    add("are public, their clause bodies are not. Dividing by a clause count taken")
    add("from a document nobody in this project has read in full would be a")
    add("fabricated denominator, so no percentage against the whole standard is")
    add("printed anywhere in this report.")
    add("")
    add("What is printed instead: of the clauses this project could cite and did")
    add("declare, how many carry at least one mapping. That is a smaller claim and")
    add("a checkable one.")
    add("")

    add("## 2. Inventory")
    add("")
    add("| Node kind | Count |")
    add("|---|---:|")
    for key, _cls, label in ASSERTION_LABELS:
        add(f"| `{label}` | **{len(assertions_by_key[key])}** |")
    add(f"| all `rssc:EvidencedAssertion` | **{len(all_assertions)}** |")
    add(f"| `rssc:Standard` | **{len(standards)}** |")
    add(f"| `rssc:ClauseReference` | **{len(clauses)}** |")
    add(f"| `rssc:SecurityRequirement` | **{len(sec_reqs)}** |")
    add(f"| `rssc:FoundationalRequirement` | **{len(frs)}** |")
    add(f"| `rssc:SafetyFunction` | **{len(safety_functions)}** |")
    add(f"| `rssc:AutonomyBand` | **{len(bands)}** |")
    add(f"| `rssc:Source` | **{len(sources)}** |")
    add("")

    add("## 3. ISO 10218:2025 clause coverage")
    add("")
    if not iso10218:
        add("**No `rssc:Standard` with a designation containing `10218` is declared**")
        add("in the graph, so no ISO 10218 clause coverage can be computed. This is")
        add("reported as a zero, not skipped.")
        add("")
    else:
        add("| Standard | Clauses declared | With >=1 mapping | With no mapping | Reached by any assertion |")
        add("|---|---:|---:|---:|---:|")
        for std in iso10218:
            designation = str(R.one(g, std, R.P_DESIGNATION, Literal(R.short(std))))
            rows = [r for r in iso_rows if r["standard"] == designation]
            add(
                f"| {cell(designation)} | **{len(rows)}** | "
                f"**{len([r for r in rows if r['mappings'] > 0])}** | "
                f"**{len([r for r in rows if r['mappings'] == 0])}** | "
                f"**{len([r for r in rows if r['mappings'] or r['gaps'] or r['impacts']])}** |"
            )
        add(
            f"| **total** | **{iso_total}** | **{len(iso_mapped)}** | "
            f"**{len(iso_unmapped)}** | **{len(iso_touched)}** |"
        )
        add("")
        if mapping_rate is not None:
            add(f"Mapping rate over declared clauses: **{mapping_rate}** "
                f"({len(iso_mapped)} of {iso_total}).")
            add(f"Reached rate, counting mappings, gaps and safety impacts: "
                f"**{touch_rate}** ({len(iso_touched)} of {iso_total}).")
            add("")

        add("### 3.1 Every declared ISO 10218:2025 clause")
        add("")
        add("The full list. Not sampled, not truncated, not sorted to flatter.")
        add("")
        add("| Standard | Clause | Title | Status | Mappings | Gaps | Safety impacts | Bands |")
        add("|---|---|---|---|---:|---:|---:|---|")
        for row in iso_rows:
            add(
                f"| {cell(row['standard'])} | `{cell(row['id'])}` | {cell(row['title'])} "
                f"| {cell(row['status'])} | {row['mappings']} | {row['gaps']} "
                f"| {row['impacts']} | {', '.join(row['bands']) or 'none'} |"
            )
        add("")

        add("### 3.2 Declared ISO 10218:2025 clauses with NO mapping")
        add("")
        if not iso_unmapped:
            add("None. Every declared clause carries at least one mapping.")
        else:
            add(f"{len(iso_unmapped)} of {iso_total} declared clauses have no")
            add("`rssc:CrosswalkAssertion`. All of them are listed here.")
            add("")
            add("| Standard | Clause | Title | Reached by a gap or safety impact? |")
            add("|---|---|---|---|")
            for row in iso_unmapped:
                reached = []
                if row["gaps"]:
                    reached.append(f"{row['gaps']} gap(s)")
                if row["impacts"]:
                    reached.append(f"{row['impacts']} safety impact(s)")
                add(
                    f"| {cell(row['standard'])} | `{cell(row['id'])}` "
                    f"| {cell(row['title'])} | {', '.join(reached) or '**no**'} |"
                )
        add("")

    add("## 4. Autonomy band distribution")
    add("")
    if not bands:
        add("No `rssc:AutonomyBand` instances are declared in the graph.")
        add("")
    else:
        add("| Band | " + " | ".join(label for _k, _c, label in ASSERTION_LABELS)
            + " | Total |")
        add("|---|" + "---:|" * (len(ASSERTION_LABELS) + 1))
        for name in band_names:
            counts = [band_matrix[name][label] for _k, _c, label in ASSERTION_LABELS]
            add(f"| **{cell(name)}** | " + " | ".join(str(c) for c in counts)
                + f" | **{sum(counts)}** |")
        counts = [unbanded[label] for _k, _c, label in ASSERTION_LABELS]
        add("| _(no band declared)_ | " + " | ".join(str(c) for c in counts)
            + f" | **{sum(counts)}** |")
        add("")
        empty_bands = [n for n in band_names if sum(band_matrix[n].values()) == 0]
        if empty_bands:
            add("**Bands with zero assertions: " + ", ".join(empty_bands) + ".**")
            add("An empty band is a result of this crosswalk, not a hole in it. Where")
            add("no standard in the corpus reaches a band, forcing a clause onto that")
            add("row would manufacture coverage that does not exist.")
        else:
            add("Every declared band carries at least one assertion.")
        add("")
        add("### 4.1 ISO 10218:2025 clauses reached, per band")
        add("")
        add("| Band | Declared ISO 10218 clauses reached | Clause identifiers |")
        add("|---|---:|---|")
        for name in band_names:
            ids = band_clause_cover.get(name, [])
            add(f"| **{cell(name)}** | {len(ids)} | "
                + (", ".join(f"`{cell(i)}`" for i in ids) or "none") + " |")
        add("")

    add("## 5. Security side: what the crosswalk reaches")
    add("")
    add("### 5.1 Foundational requirements")
    add("")
    if not fr_rows:
        add("No `rssc:FoundationalRequirement` instances are declared in the graph.")
    else:
        add("| FR | Label | Assertions pointing at it |")
        add("|---|---|---:|")
        for row in fr_rows:
            add(f"| `{cell(row['id'])}` | {cell(row['label'])} | {row['hits']} |")
        add("")
        if fr_unreached:
            add("**Foundational requirements no assertion reaches: "
                + ", ".join(f"`{r['id']}`" for r in fr_unreached) + ".**")
        else:
            add("Every foundational requirement is reached by at least one assertion.")
    add("")

    add("### 5.2 Security requirements")
    add("")
    if not req_rows:
        add("No `rssc:SecurityRequirement` instances are declared in the graph.")
    else:
        add(f"{len(req_rows)} declared, of which "
            f"**{len(req_rows) - len(req_unreached)}** are pointed at by at least one")
        add("assertion. Every declared requirement is listed.")
        add("")
        add("| Requirement | Title | Assertions pointing at it |")
        add("|---|---|---:|")
        for row in req_rows:
            add(f"| `{cell(row['id'])}` | {cell(row['title'])} | {row['hits']} |")
    add("")

    add("## 6. Evidence type and confidence")
    add("")
    add("| Evidence type | Assertions |")
    add("|---|---:|")
    for key, count in sorted(ev_counter.items(), key=lambda kv: (-kv[1], kv[0])):
        add(f"| `{cell(key)}` | {count} |")
    add("")
    add("| Confidence | Assertions |")
    add("|---|---:|")
    for key, count in sorted(conf_counter.items(), key=lambda kv: (-kv[1], kv[0])):
        add(f"| `{cell(key)}` | {count} |")
    add("")

    add("## 7. What is NOT covered")
    add("")
    add("This section exists so that the reader does not have to reconstruct the")
    add("negative space from the tables above. Every item is a count taken from the")
    add("same graph.")
    add("")
    add(f"1. **ISO 10218:2025 clauses declared with no mapping: "
        f"{len(iso_unmapped)}** of {iso_total}. Listed in full at 3.2.")
    add(f"2. **ISO 10218:2025 clauses declared that no assertion of any kind "
        f"reaches: {len(iso_untouched)}.**")
    if iso_untouched:
        add("   " + ", ".join(f"`{cell(r['id'])}` ({cell(r['standard'])})"
                              for r in iso_untouched))
    add(f"3. **Autonomy bands with zero assertions: "
        f"{len([n for n in band_names if sum(band_matrix[n].values()) == 0])}** "
        f"of {len(band_names)} declared.")
    add(f"4. **Foundational requirements no assertion reaches: "
        f"{len(fr_unreached)}** of {len(fr_rows)}.")
    add(f"5. **Declared security requirements no assertion reaches: "
        f"{len(req_unreached)}** of {len(req_rows)}.")
    add(f"6. **Assertions with no autonomy band declared: {sum(unbanded.values())}.** "
        "Such an assertion is a claim about the corpus that is not tied to a")
    add("   point on the gradient, so it is excluded from the per-band arithmetic at 4.")
    add(f"7. **Assertions carrying no `rssc:citation`: {len(uncited)}.** "
        "This number must be zero; `validate.py` fails the build if it is not.")
    if uncited:
        add("   " + ", ".join(f"`{u}`" for u in uncited))
    add(f"8. **Assertions with no `prov:wasDerivedFrom` link to a "
        f"`rssc:Source`: {len(unsourced)}.**")
    if unsourced:
        add("   " + ", ".join(f"`{u}`" for u in unsourced))
    add(f"9. **Declared sources nothing is derived from: {len(unused_sources)}.**")
    if unused_sources:
        add("   " + ", ".join(f"`{u}`" for u in unused_sources))
    add(f"10. **Standards declared with no clause reference attached: "
        f"{len(standards_without_clauses)}.**")
    if standards_without_clauses:
        add("   " + ", ".join(f"{cell(s)}" for s in standards_without_clauses))
    add(f"11. **Clause references bound to no standard: {len(orphan_clauses)}.**")
    if orphan_clauses:
        add("   " + ", ".join(f"`{R.short(c)}`" for c in orphan_clauses))
    add("")

    add("## 8. Limits of this count")
    add("")
    add("1. Coverage here means a mapping exists and carries evidence. It does not")
    add("   mean the mapping is correct. Correctness is contestable per assertion,")
    add("   which is why each one is reified and carries its own citation.")
    add("2. The clause denominator is what this project declared, not what ISO")
    add("   published. See section 1.")
    add("3. No normative text from any ISO or IEC standard was read or reproduced")
    add("   to produce these counts. The graph records clause identifiers, clause")
    add("   titles, part numbers and dates, which are published free in the front")
    add("   matter and contents pages, and nothing else.")
    add("4. A clause reached only by a `rssc:ControlGap` is reached by a negative")
    add("   finding: the crosswalk says that clause does not address the exposure.")
    add("   It is counted separately from a mapping throughout, and the two are")
    add("   never added together into a single coverage figure.")
    add("")

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text("\n".join(L) + "\n", encoding="utf-8")

    metrics = {
        "generated": now,
        "graph_triples": data_load.triples,
        "files": [
            {"path": str(p.relative_to(R.ROOT)), "triples": c, "sha256_16": d}
            for p, c, d in data_load.files
        ],
        "evidenced_assertions": len(all_assertions),
        **{key: len(assertions_by_key[key]) for key, _c, _l in ASSERTION_LABELS},
        "standards": len(standards),
        "clause_references": len(clauses),
        "security_requirements": len(sec_reqs),
        "foundational_requirements": len(frs),
        "safety_functions": len(safety_functions),
        "autonomy_bands": len(bands),
        "sources": len(sources),
        "iso10218_clauses_declared": iso_total,
        "iso10218_clauses_mapped": len(iso_mapped),
        "iso10218_clauses_unmapped": len(iso_unmapped),
        "iso10218_clauses_reached_any": len(iso_touched),
        "iso10218_clauses_reached_none": len(iso_untouched),
        "iso10218_mapping_rate": mapping_rate,
        "iso10218_reached_rate": touch_rate,
        "assertions_by_band": {
            name: sum(band_matrix[name].values()) for name in band_names
        },
        "assertions_without_band": sum(unbanded.values()),
        "iso10218_clauses_by_band": {
            name: len(ids) for name, ids in band_clause_cover.items()
        },
        "evidence_types": dict(ev_counter),
        "confidence_levels": dict(conf_counter),
        "foundational_requirements_unreached": [r["id"] for r in fr_unreached],
        "security_requirements_unreached": len(req_unreached),
        "assertions_without_citation": len(uncited),
        "assertions_without_source_link": len(unsourced),
        "sources_unused": len(unused_sources),
        "standards_without_clauses": len(standards_without_clauses),
        "clause_references_without_standard": len(orphan_clauses),
    }
    METRICS.write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")

    # ------------------------------------------------------------ terminal echo
    print("RSSC coverage")
    print(f"  graph                        : {data_load.triples} triples, "
          f"{len(data_load.files)} file(s)")
    print(f"  evidenced assertions         : {len(all_assertions)}")
    for key, _cls, label in ASSERTION_LABELS:
        print(f"    {label:<30} {len(assertions_by_key[key]):>5}")
    print(f"  ISO 10218 clauses declared   : {iso_total}")
    print(f"    with at least one mapping  : {len(iso_mapped)}")
    print(f"    with no mapping            : {len(iso_unmapped)}")
    print(f"    reached by no assertion    : {len(iso_untouched)}")
    print(f"  mapping rate (declared)      : {mapping_rate}")
    print("  assertions by band           : "
          + (", ".join(f"{n}={sum(band_matrix[n].values())}" for n in band_names)
             or "(no bands declared)"))
    print(f"  assertions with no band      : {sum(unbanded.values())}")
    print(f"  FRs unreached                : "
          + (", ".join(r["id"] for r in fr_unreached) or "none"))
    print(f"  assertions with no citation  : {len(uncited)}")
    print(f"  wrote {REPORT}")
    print(f"  wrote {METRICS}")

    return 0 if all_assertions else 1


if __name__ == "__main__":
    sys.exit(main())
