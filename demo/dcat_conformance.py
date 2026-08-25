#!/usr/bin/env python3
"""Recompute the DCAT-US validator finding end to end, from vendored inputs only.

DCAT-US 3.0's README describes the profile as an implementation of the W3C
DCAT standard. Its published JSON Schema carries no JSON-LD `@context`, so
none of its own examples expand into DCAT (or any other) RDF vocabulary as
published. Pull request 120 (commit 99ef81c9,
https://github.com/GSA/dcat-us/pull/120) deleted two files, not one:
`shacl/dcat-us_3.0_shacl_shapes.ttl` and `context/dcat-us-3.0.jsonld`, the
profile's only published JSON-LD `@context`. The schema's `_oldDocs` blocks
are what is left of the second: every bound property still carries the RDF
term it stood for. This script uses both: `_oldDocs` to derive a
reconstruction, and the real deleted context, recovered separately from the
pull request's base commit (04acd5c25579dd70a86a436b30a33e0784965cc4) and
vendored unmodified as `recovered-context.jsonld`, to check the
reconstruction against what the profile actually published rather than only
against itself.

This script does the following, against files committed under
`demo/corpus/dcat-us/`, with no network access and no model call:

  1. Reads `jsonschema/definitions/*.json` and derives a JSON-LD context from
     the `_oldDocs` blocks: which class each definition binds to, and which
     RDF term and coercion (resource vs. literal, singular vs. set) each of
     its properties binds to. Namespace prefixes are read from the recovered
     shapes file's own `@prefix` block (`recovered-shapes.ttl`), so the
     context and the shapes cannot disagree about what a prefix expands to.
  2. Expands every `jsonschema/examples/<Class>/good/*.json` file several
     ways: exactly as published (no context at all), with the derived
     context injected, and against `recovered-context.jsonld` (the real
     context, both verbatim and with each example's bare `@type` rewritten
     to the CURIE the schema's own `_oldDocs.rdfClass` already names for it,
     since the real context's class bindings are keyed by CURIE and every
     example's `@type` is the bare title). Counts triples, distinct
     predicates and DCAT namespace predicates each way.
  3. Merges the expanded corpus and runs it, every way, against
     `recovered-shapes.ttl` with pySHACL, counting violations and how many
     focus nodes the shapes actually reach.

Every number this script prints is measured here; none is quoted from
anywhere else. Run it with `python3 demo/dcat_conformance.py`.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

import rdflib
from pyshacl import validate as pyshacl_validate
from rdflib import RDF, Graph, URIRef

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "demo" / "corpus" / "dcat-us"
DEFINITIONS = CORPUS / "jsonschema" / "definitions"
EXAMPLES = CORPUS / "jsonschema" / "examples"
SHAPES_PATH = CORPUS / "recovered-shapes.ttl"
REAL_CONTEXT_PATH = CORPUS / "recovered-context.jsonld"
BASE = "https://example.gov/"
DCAT_NS = "http://www.w3.org/ns/dcat#"
SH = rdflib.Namespace("http://www.w3.org/ns/shacl#")

# --------------------------------------------------------------------------
# 1. Read the schema's own binding
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class PropBinding:
    owner: str
    iri: str
    coercion: str | None  # '@id', an xsd curie, or None (plain literal)
    is_list: bool

    def key(self) -> tuple:
        return (self.iri, self.coercion, self.is_list)


@dataclass
class ClassDef:
    name: str
    rdf_class: str | None
    properties: dict[str, PropBinding] = field(default_factory=dict)


def _collect_properties(node: dict) -> tuple[dict, list]:
    """Properties declared at this level, plus those nested in anyOf/oneOf/allOf.

    Concept and Identifier accept a bare string or an object; their real
    properties live one level down, inside `anyOf`. Reading only the top
    level silently drops the two most frequently instantiated classes in the
    example corpus.
    """
    props = dict(node.get("properties") or {})
    required = list(node.get("required") or ())
    for key in ("anyOf", "oneOf", "allOf"):
        for branch in node.get(key, []) or []:
            if not isinstance(branch, dict):
                continue
            for pname, pnode in (branch.get("properties") or {}).items():
                props.setdefault(pname, pnode)
            required.extend(branch.get("required") or ())
    return props, required


def _is_array(node: dict) -> bool:
    t = node.get("type")
    if t == "array" or (isinstance(t, list) and "array" in t):
        return True
    for key in ("anyOf", "oneOf"):
        for branch in node.get(key, []) or []:
            if isinstance(branch, dict) and _is_array(branch):
                return True
    return False


def _single_xsd_token(range_text: str) -> str | None:
    """A single xsd: datatype named in a range string, or None if zero or several."""
    hits = sorted(set(re.findall(r"xsd:[A-Za-z]+", range_text)))
    return hits[0] if len(hits) == 1 else None


def _coercion_for(range_text: str | None) -> str | None:
    if not range_text:
        return None
    if range_text.startswith("xsd:"):
        return _single_xsd_token(range_text)
    if range_text.startswith("rdfs:Literal"):
        # e.g. "rdfs:Literal (typed as xsd:date, xsd:dateTime, xsd:gYear or
        # xsd:gYearMonth)" names four types; left uncoerced deliberately.
        return _single_xsd_token(range_text)
    # Anything else the schema names as a range is a class, i.e. a resource.
    return "@id"


def load_classes(definitions_dir: Path) -> dict[str, ClassDef]:
    classes: dict[str, ClassDef] = {}
    for path in sorted(definitions_dir.glob("*.json")):
        raw = json.loads(path.read_text(encoding="utf-8"))
        name = raw["title"]  # the schema's own title is the bare @type word examples use
        old = raw.get("_oldDocs") or {}
        declared, _required = _collect_properties(raw)
        properties: dict[str, PropBinding] = {}
        for pname, pnode in declared.items():
            if pname.startswith("@") or not isinstance(pnode, dict):
                continue
            pold = pnode.get("_oldDocs") or {}
            iri = pold.get("uri")
            if not iri:
                continue  # the schema names this property but assigns it no RDF term
            properties[pname] = PropBinding(
                owner=name,
                iri=iri,
                coercion=_coercion_for(pold.get("range")),
                is_list=_is_array(pnode),
            )
        classes[name] = ClassDef(name=name, rdf_class=old.get("rdfClass"), properties=properties)
    return classes


# --------------------------------------------------------------------------
# 2. Build the JSON-LD context the schema implies
# --------------------------------------------------------------------------


def load_shapes_namespaces(shapes_path: Path) -> dict[str, str]:
    """The prefix table declared at the top of the shapes file itself.

    Using the shapes file's own prefixes, rather than reconstructing one
    independently, guarantees the generated context and the shapes being
    validated against cannot disagree about what a prefix like `dcat-us:`
    expands to. `bind_namespaces="none"` matters here: rdflib otherwise
    pre-binds its own ~20 default prefixes (brick, csvw, dcam, ...) before
    parsing, which would leak unrelated bindings into a context meant to
    record only what this file declares.
    """
    g = Graph(bind_namespaces="none")
    g.parse(str(shapes_path), format="turtle")
    return {prefix: str(ns) for prefix, ns in g.namespaces() if prefix}


def load_real_context(path: Path) -> dict:
    """The profile's own, actually published JSON-LD context (recovered-context.jsonld).

    Retrieved verbatim from the base commit of pull request 120
    (04acd5c25579dd70a86a436b30a33e0784965cc4), the commit immediately before
    the same pull request that deleted the shapes file also deleted this
    file. Nothing here derives or reconstructs it; it is read as published.
    """
    return json.loads(path.read_text(encoding="utf-8"))


def title_to_rdf_class(classes: dict[str, ClassDef]) -> dict[str, str]:
    """Schema title -> _oldDocs.rdfClass, exactly as load_classes already read it.

    Used only to rewrite a '@type' value from the schema's bare title (what
    every example actually publishes) to the CURIE the real context's
    type-scoped bindings are keyed by. This is a reformatting of a class name
    the schema itself states in _oldDocs.rdfClass, not a new binding: if a
    title has no rdfClass, or an example's '@type' does not literally equal a
    title (one case in this corpus: examples say 'Catalog', the schema's own
    title is 'DCAT-US 3 Catalog'), the value is left as published rather than
    guessed at.
    """
    return {name: cls.rdf_class for name, cls in classes.items() if cls.rdf_class}


ABSOLUTE_IRI = re.compile(r"^[A-Za-z][A-Za-z0-9+.\-]*:")


def scan_bare_values(files: list[Path], classes: dict[str, ClassDef]) -> set[str]:
    """Terms bound to '@id' whose published values are not actually IRI-shaped.

    22 keys the schema declares with a resource range are published as prose
    in the examples (an `accessRights` sentence, a `provenance` paragraph).
    Under a strict binding those become manufactured relative IRIs built out
    of a label. This scan finds which terms that happens to, so a second,
    lenient context can be built that leaves those specific terms as plain
    literals instead.
    """
    bound_ids = {
        pname: b for cls in classes.values() for pname, b in cls.properties.items() if b.coercion == "@id"
    }

    def walk(node) -> set[str]:
        found: set[str] = set()
        if isinstance(node, list):
            for item in node:
                found |= walk(item)
        elif isinstance(node, dict):
            for key, value in node.items():
                if key.startswith("@") or key not in bound_ids:
                    if isinstance(value, (dict, list)):
                        found |= walk(value)
                    continue
                values = value if isinstance(value, list) else [value]
                for item in values:
                    if isinstance(item, (dict, list)):
                        found |= walk(item)
                    elif isinstance(item, str) and not ABSOLUTE_IRI.match(item):
                        found.add(key)
        return found

    lenient: set[str] = set()
    for path in files:
        raw = json.loads(path.read_text(encoding="utf-8"))
        lenient |= walk(raw)
    return lenient


def _term_definition(binding: PropBinding, lenient_terms: set[str] | None, term: str) -> dict | str:
    coercion = binding.coercion
    if lenient_terms and term in lenient_terms and coercion == "@id":
        coercion = None
    definition: dict = {"@id": binding.iri}
    if coercion:
        definition["@type"] = coercion
    if binding.is_list:
        definition["@container"] = "@set"
    if list(definition) == ["@id"]:
        return binding.iri
    return definition


def _term_bindings(classes: dict[str, ClassDef]) -> dict[str, dict[str, PropBinding]]:
    out: dict[str, dict[str, PropBinding]] = {}
    for cls in classes.values():
        for pname, binding in cls.properties.items():
            out.setdefault(pname, {})[cls.name] = binding
    return out


def _resolve_term(owners: dict[str, PropBinding]) -> tuple[PropBinding, dict[str, PropBinding]]:
    """One dominant binding (used by the most classes) plus per-class overrides.

    Four terms in DCAT-US bind to two different RDF properties depending on
    the owning class (e.g. `conformsTo` on Document binds to
    dcterms:identifier while five other classes bind it to
    dcterms:conformsTo). Collapsing to a single global mapping would silently
    pick one and mis-expand the other classes' data.
    """
    counts = Counter(b.key() for b in owners.values())
    dominant_key = sorted(counts.items(), key=lambda kv: (-kv[1], str(kv[0])))[0][0]
    dominant = next(b for b in owners.values() if b.key() == dominant_key)
    overrides = {c: b for c, b in sorted(owners.items()) if b.key() != dominant_key}
    return dominant, overrides


def build_context(
    classes: dict[str, ClassDef], namespaces: dict[str, str], lenient_terms: set[str] | None = None
) -> dict:
    ctx: dict = {"@version": 1.1}
    ctx.update({prefix: iri for prefix, iri in sorted(namespaces.items())})

    bindings = _term_bindings(classes)
    scoped: dict[str, dict] = {name: {} for name in classes}
    terms: dict[str, object] = {}
    for term, owners in sorted(bindings.items()):
        dominant, overrides = _resolve_term(owners)
        terms[term] = _term_definition(dominant, lenient_terms, term)
        for cname, binding in overrides.items():
            scoped[cname][term] = _term_definition(binding, lenient_terms, term)

    for cls in sorted(classes.values(), key=lambda c: c.name):
        if not cls.rdf_class:
            continue
        if scoped[cls.name]:
            ctx[cls.name] = {"@id": cls.rdf_class, "@context": dict(sorted(scoped[cls.name].items()))}
        else:
            ctx[cls.name] = cls.rdf_class

    ctx.update(dict(sorted(terms.items())))
    return {"@context": ctx}


# --------------------------------------------------------------------------
# 3. Expand the examples, with and without the binding
# --------------------------------------------------------------------------


def _retype(node, mapping: dict[str, str]):
    """Rewrite every '@type' value found anywhere in node via mapping, in place.

    Used only to exercise the real recovered context (recovered-context.jsonld)
    against today's examples: that context's class bindings are keyed by the
    class's full CURIE (e.g. 'skos:Concept', 'dcat:Dataset'), but every good
    example's own '@type' is the bare schema title ('Concept', 'Dataset').
    mapping comes straight from each class's own `_oldDocs.rdfClass` (already
    read by load_classes for an unrelated purpose) -- nothing here invents a
    binding; it only reformats a class name the schema already states, so the
    real context's own type-scoped terms have a chance to match.
    """
    if isinstance(node, dict):
        if "@type" in node:
            t = node["@type"]
            if isinstance(t, str):
                node["@type"] = mapping.get(t, t)
            elif isinstance(t, list):
                node["@type"] = [mapping.get(x, x) if isinstance(x, str) else x for x in t]
        for v in node.values():
            _retype(v, mapping)
    elif isinstance(node, list):
        for item in node:
            _retype(item, mapping)
    return node


def expand_file(
    path: Path, context: dict | None, base: str = BASE, retype: dict[str, str] | None = None
) -> tuple[Graph | None, str | None]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    if retype and isinstance(raw, dict):
        raw = _retype(raw, retype)
    if not isinstance(raw, dict):
        # A handful of "good" examples are a bare JSON string (a Concept or
        # an Identifier used in its string form, e.g. Identifier/good/
        # minimal_string_example.json is literally "dataset-12345"). With no
        # context, rdflib's json-ld parser has nothing to hang the value off
        # and raises; that failure is the "as published" measurement for
        # this file. With a context, the string is carried under `@graph`,
        # which is valid JSON-LD and correctly expands to zero triples: a
        # bare label is not a node, so a faithful binding finds nothing to
        # assert about it, not an error.
        if context is None:
            return None, "example root is not a JSON object"
        doc = {**context, "@graph": raw}
        g = Graph()
        try:
            g.parse(data=json.dumps(doc), format="json-ld", base=base)
        except Exception as exc:  # noqa: BLE001 - the failure itself is the measurement
            return None, f"{type(exc).__name__}: {exc}"
        return g, None
    doc = raw if context is None else {**context, **{k: v for k, v in raw.items() if k != "@context"}}
    g = Graph()
    try:
        g.parse(data=json.dumps(doc), format="json-ld", base=base)
    except Exception as exc:  # noqa: BLE001 - the failure itself is the measurement
        return None, f"{type(exc).__name__}: {exc}"
    return g, None


def measure(
    files: list[Path], context: dict | None, base: str = BASE, retype: dict[str, str] | None = None
) -> dict:
    total = 0
    predicates: Counter = Counter()
    zero_files: list[str] = []
    errors: list[dict] = []
    for path in files:
        g, err = expand_file(path, context, base, retype)
        if err:
            errors.append({"file": path.name, "error": err})
            continue
        n = len(g)
        total += n
        if n == 0:
            zero_files.append(path.name)
        for _, p, _ in g:
            predicates[str(p)] += 1
    dcat_triples = sum(v for k, v in predicates.items() if k.startswith(DCAT_NS))
    return {
        "files": len(files),
        "triples": total,
        "distinctPredicates": len(predicates),
        "dcatPredicateTriples": dcat_triples,
        "zeroTripleFiles": len(zero_files),
        "zeroTripleFileNames": sorted(zero_files),
        "errors": errors,
        "topPredicates": sorted(predicates.items(), key=lambda kv: (-kv[1], kv[0]))[:25],
    }


def merge_corpus(
    files: list[Path], context: dict | None, base: str = BASE, retype: dict[str, str] | None = None
) -> Graph:
    merged = Graph()
    for path in files:
        g, err = expand_file(path, context, base, retype)
        if err:
            continue
        for triple in g:
            merged.add(triple)
    return merged


# --------------------------------------------------------------------------
# 4. Validate against the deleted shapes
# --------------------------------------------------------------------------


def run_shacl(shapes_path: Path, data: Graph) -> dict:
    shapes = Graph().parse(str(shapes_path), format="turtle")
    conforms, results_graph, _results_text = pyshacl_validate(
        data, shacl_graph=shapes, inference="none", advanced=True, debug=False
    )
    severities: Counter = Counter()
    for result in results_graph.subjects(RDF.type, SH.ValidationResult):
        for severity in results_graph.objects(result, SH.resultSeverity):
            severities[str(severity).rsplit("#", 1)[-1]] += 1
    target_classes = {o for o in shapes.objects(None, SH.targetClass) if isinstance(o, URIRef)}
    present_types = {o for o in data.objects(None, RDF.type) if isinstance(o, URIRef)}
    matched = target_classes & present_types
    focus_nodes = {s for cls in matched for s in data.subjects(RDF.type, cls)}
    return {
        "shapeTriples": len(shapes),
        "conforms": bool(conforms),
        "violations": sum(severities.values()),
        "bySeverity": dict(severities),
        "targetClassCount": len(target_classes),
        "matchedClasses": sorted(str(c) for c in matched),
        "matchedClassCount": len(matched),
        "focusNodes": len(focus_nodes),
        "dataTriples": len(data),
    }


# --------------------------------------------------------------------------
# 5. A defect internal to the recovered files themselves, not this script
# --------------------------------------------------------------------------


def check_real_context_org_prefix(real_context: dict, shapes_namespaces: dict) -> dict:
    """Compare 'org:' as bound in the two recovered files from the same deletion.

    recovered-context.jsonld and recovered-shapes.ttl were both deleted by
    the same pull request, as part of the same profile release, and both are
    vendored here unmodified. They disagree about what 'org:' expands to:
    the context binds it to 'http://www.w3c.org/ns/org#' (an extra 'c'); the
    shapes file binds it to 'http://www.w3.org/ns/org#', the actual W3C
    Organization Ontology namespace. This is reported here, not corrected:
    substituting the shapes file's binding into the context would test a
    context DCAT-US never actually published. See main()'s
    'orgPrefixTypoImpact' for what changes if it were corrected anyway.
    """
    real_org = real_context["@context"].get("org")
    shapes_org = shapes_namespaces.get("org")
    return {
        "recoveredContextOrgPrefix": real_org,
        "recoveredShapesOrgPrefix": shapes_org,
        "agree": real_org == shapes_org,
    }


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=CORPUS / "jsonschema")
    args = parser.parse_args()

    classes = load_classes(DEFINITIONS)
    namespaces = load_shapes_namespaces(SHAPES_PATH)
    good_files = sorted(EXAMPLES.glob("*/good/*.json"))

    # Two contexts, both derived purely from the schema and the corpus, never
    # hand-written: "declared" applies the binding exactly as the schema
    # states it; "observed" additionally relaxes the specific terms the
    # corpus scan finds are not published as IRIs, so a resource-range term
    # is not forced into a manufactured relative IRI built out of a label.
    declared = build_context(classes, namespaces)
    lenient_terms = scan_bare_values(good_files, classes)
    observed = build_context(classes, namespaces, lenient_terms=lenient_terms)

    as_published = measure(good_files, None)
    with_declared = measure(good_files, declared)
    with_observed = measure(good_files, observed)

    corpus_published = merge_corpus(good_files, None)
    corpus_declared = merge_corpus(good_files, declared)
    corpus_observed = merge_corpus(good_files, observed)

    shacl_published = run_shacl(SHAPES_PATH, corpus_published)
    shacl_declared = run_shacl(SHAPES_PATH, corpus_declared)
    shacl_observed = run_shacl(SHAPES_PATH, corpus_observed)

    zero_bound_classes = sorted(c.name for c in classes.values() if not c.properties)

    # --- The real, historically published context (recovered-context.jsonld) ---
    # Pull request 120 deleted two files, not one: the shapes file above, and
    # this JSON-LD @context, the profile's own actual publication, not a
    # reconstruction. Two measurements, both against the unmodified file:
    #
    #  - "verbatim": the context injected exactly as retrieved, no rewriting.
    #    Its class bindings are keyed by full CURIE ('skos:Concept',
    #    'dcat:Dataset', ...); every good example's own '@type' is the bare
    #    schema title ('Concept', 'Dataset'). JSON-LD type-scoped contexts
    #    require an exact term match, so this measurement shows what actually
    #    happens if the file is dropped back in as-is: almost nothing.
    #  - "typed": the same unmodified context, with each example's '@type'
    #    mechanically rewritten from the schema's bare title to the CURIE the
    #    schema's own _oldDocs.rdfClass already records for that title (see
    #    title_to_rdf_class). This is the measurement that exercises what the
    #    real context actually binds, once its own naming convention is met.
    real_context = load_real_context(REAL_CONTEXT_PATH)
    retype_map = title_to_rdf_class(classes)

    real_verbatim = measure(good_files, real_context)
    real_typed = measure(good_files, real_context, retype=retype_map)

    corpus_real_verbatim = merge_corpus(good_files, real_context)
    corpus_real_typed = merge_corpus(good_files, real_context, retype=retype_map)

    shacl_real_verbatim = run_shacl(SHAPES_PATH, corpus_real_verbatim)
    shacl_real_typed = run_shacl(SHAPES_PATH, corpus_real_typed)

    # Sanity check pinned by demo/tests/test_dcat_conformance.py: the real
    # context does bind Concept's and Identifier's properties, contradicting
    # the schema-residue reading that they are unbound.
    real_concept_props = sorted(real_context["@context"]["skos:Concept"]["@context"].keys())
    real_identifier_props = sorted(real_context["@context"]["adms:Identifier"]["@context"].keys())

    # A defect internal to the two recovered files, independent of anything
    # this script assumes: they disagree about what 'org:' expands to (see
    # check_real_context_org_prefix). Quantified here, purely diagnostic, not
    # one of the four canonical measurements above: how many more focus
    # nodes and violations the shapes-file binding of 'org:' would reach, if
    # it were substituted into the real context. Not used to adjust any
    # reported figure; it is evidence for why a single violation count is
    # not defensible, not a correction to the published files.
    org_prefix_check = check_real_context_org_prefix(real_context, namespaces)
    org_typo_fixed_context = json.loads(json.dumps(real_context))
    org_typo_fixed_context["@context"]["org"] = namespaces.get("org", org_prefix_check["recoveredShapesOrgPrefix"])
    corpus_org_fixed = merge_corpus(good_files, org_typo_fixed_context, retype=retype_map)
    shacl_org_fixed = run_shacl(SHAPES_PATH, corpus_org_fixed)
    org_prefix_impact = {
        "description": "legacyShapesOverRealContextTypedCorpus, with recovered-context.jsonld's own "
                        "'org:' prefix replaced by recovered-shapes.ttl's, diagnostic only",
        "violations": shacl_org_fixed["violations"],
        "focusNodes": shacl_org_fixed["focusNodes"],
        "violationsDelta": shacl_org_fixed["violations"] - shacl_real_typed["violations"],
        "focusNodesDelta": shacl_org_fixed["focusNodes"] - shacl_real_typed["focusNodes"],
    }

    result = {
        "measured": "2026-08-25, this repository, no network, no model call",
        "commit": COMMIT_SHA,
        "realContextBaseCommit": REAL_CONTEXT_COMMIT_SHA,
        "classes": len(classes),
        "classesWithZeroBoundProperties": zero_bound_classes,
        "realContextBindsConceptProperties": real_concept_props,
        "realContextBindsIdentifierProperties": real_identifier_props,
        "recoveredFilesOrgPrefixCheck": org_prefix_check,
        "recoveredFilesOrgPrefixTypoImpactDiagnosticOnly": org_prefix_impact,
        "lenientTermCount": len(lenient_terms),
        "lenientTerms": sorted(lenient_terms),
        "examples": {
            "asPublished": as_published,
            "withDeclaredBinding": with_declared,
            "withObservedBinding": with_observed,
            "withRealContextVerbatim": real_verbatim,
            "withRealContextTyped": real_typed,
        },
        "corpusTriples": {
            "asPublished": len(corpus_published),
            "withDeclaredBinding": len(corpus_declared),
            "withObservedBinding": len(corpus_observed),
            "withRealContextVerbatim": len(corpus_real_verbatim),
            "withRealContextTyped": len(corpus_real_typed),
        },
        "shacl": {
            "legacyShapesOverPublishedCorpus": shacl_published,
            "legacyShapesOverDeclaredBoundCorpus": shacl_declared,
            "legacyShapesOverObservedBoundCorpus": shacl_observed,
            "legacyShapesOverRealContextVerbatimCorpus": shacl_real_verbatim,
            "legacyShapesOverRealContextTypedCorpus": shacl_real_typed,
        },
    }

    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / "generated-context.jsonld").write_text(json.dumps(declared, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    (args.out / "generated-context.observed.jsonld").write_text(json.dumps(observed, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    (ROOT / "demo" / "dcat_conformance_measurements.json").write_text(
        json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8"
    )

    print(f"classes {len(classes)}  good examples {len(good_files)}  lenient terms {len(lenient_terms)}")
    print(f"as published        : {as_published['triples']:5} triples  {as_published['distinctPredicates']:3} predicates  "
          f"{as_published['dcatPredicateTriples']:4} dcat  {as_published['zeroTripleFiles']:3} empty  "
          f"{len(as_published['errors'])} errors")
    print(f"declared binding    : {with_declared['triples']:5} triples  {with_declared['distinctPredicates']:3} predicates  "
          f"{with_declared['dcatPredicateTriples']:4} dcat  {with_declared['zeroTripleFiles']:3} empty  "
          f"{len(with_declared['errors'])} errors")
    print(f"observed binding    : {with_observed['triples']:5} triples  {with_observed['distinctPredicates']:3} predicates  "
          f"{with_observed['dcatPredicateTriples']:4} dcat  {with_observed['zeroTripleFiles']:3} empty  "
          f"{len(with_observed['errors'])} errors")
    print(f"real context verbatim: {real_verbatim['triples']:4} triples  {real_verbatim['distinctPredicates']:3} predicates  "
          f"{real_verbatim['dcatPredicateTriples']:4} dcat  {real_verbatim['zeroTripleFiles']:3} empty  "
          f"{len(real_verbatim['errors'])} errors")
    print(f"real context typed  : {real_typed['triples']:5} triples  {real_typed['distinctPredicates']:3} predicates  "
          f"{real_typed['dcatPredicateTriples']:4} dcat  {real_typed['zeroTripleFiles']:3} empty  "
          f"{len(real_typed['errors'])} errors")
    print(f"shacl over as-published corpus       : conforms={shacl_published['conforms']} "
          f"violations={shacl_published['violations']} focus_nodes={shacl_published['focusNodes']}")
    print(f"shacl over declared-bound corpus     : conforms={shacl_declared['conforms']} "
          f"violations={shacl_declared['violations']} focus_nodes={shacl_declared['focusNodes']}")
    print(f"shacl over observed-bound corpus     : conforms={shacl_observed['conforms']} "
          f"violations={shacl_observed['violations']} focus_nodes={shacl_observed['focusNodes']}")
    print(f"shacl over real-context-verbatim corpus: conforms={shacl_real_verbatim['conforms']} "
          f"violations={shacl_real_verbatim['violations']} focus_nodes={shacl_real_verbatim['focusNodes']}")
    print(f"shacl over real-context-typed corpus : conforms={shacl_real_typed['conforms']} "
          f"violations={shacl_real_typed['violations']} focus_nodes={shacl_real_typed['focusNodes']}")
    print(f"recovered files disagree on 'org:' -- context: {org_prefix_check['recoveredContextOrgPrefix']}  "
          f"shapes: {org_prefix_check['recoveredShapesOrgPrefix']}  agree={org_prefix_check['agree']}")
    print(f"  (diagnostic only) with shapes-file 'org:' substituted: "
          f"violations={org_prefix_impact['violations']} ({org_prefix_impact['violationsDelta']:+d})  "
          f"focus_nodes={org_prefix_impact['focusNodes']} ({org_prefix_impact['focusNodesDelta']:+d})")
    return 0


COMMIT_SHA = "7a6e803fb94ee9903e7e7405ec4afcc8da13383f"
REAL_CONTEXT_COMMIT_SHA = "04acd5c25579dd70a86a436b30a33e0784965cc4"

if __name__ == "__main__":
    sys.exit(main())
