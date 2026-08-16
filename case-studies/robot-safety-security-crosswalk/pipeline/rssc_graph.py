#!/usr/bin/env python3
"""Shared loader for the Robot Safety and Security Crosswalk (RSSC) artefact.

Imported by validate.py and coverage.py so that both scripts see exactly the
same graph. Nothing here interprets the crosswalk; it only finds the files,
parses them, and reports honestly on what was found and what was missing.

Layout assumed, all discovered by glob rather than by hard-coded filename so
that the ontology, crosswalk and shapes authors can name their files freely:

    ontology/*.ttl    vocabulary                            -> data graph
    crosswalk/*.ttl   the reified, cited mapping assertions -> data graph
    data/*.ttl        any further sourced instance data     -> data graph
    ./*.ttl           a built graph.ttl at the root         -> data graph
    shapes/*.ttl      SHACL shapes                          -> shapes graph

Four data locations rather than one because the artefact's own README describes
a build that emits graph.ttl at the root while the crosswalk sits in its own
directory. Loading all four means the validator sees the whole graph whichever
of those the crosswalk author actually used, and reports the ones that are
empty rather than assuming they do not exist.

Set RSSC_ROOT to point the loader at a different case-study root. That exists
for self-test fixtures and for nothing else; the two pipeline scripts take no
command-line arguments, as the house pipelines do.
"""

import hashlib
import os
from pathlib import Path

from rdflib import Graph, RDF, RDFS, URIRef
from rdflib.namespace import SKOS

ROOT = Path(os.environ.get("RSSC_ROOT", Path(__file__).resolve().parent.parent)).resolve()
ONTOLOGY_DIR = ROOT / "ontology"
CROSSWALK_DIR = ROOT / "crosswalk"
DATA_DIR = ROOT / "data"
SHAPES_DIR = ROOT / "shapes"

DATA_DIRS = [ONTOLOGY_DIR, CROSSWALK_DIR, DATA_DIR, ROOT]

RSSC = "https://tesseract.academy/ns/rssc#"

# Vocabulary terms the pipeline reads. Kept as module constants so that a
# rename in the ontology surfaces as a zero count here rather than silently.
EVIDENCED_ASSERTION = URIRef(RSSC + "EvidencedAssertion")
CROSSWALK_ASSERTION = URIRef(RSSC + "CrosswalkAssertion")
SAFETY_IMPACT_ASSERTION = URIRef(RSSC + "SafetyImpactAssertion")
SECURITY_LEVEL_CLAIM = URIRef(RSSC + "SecurityLevelClaim")
CONTROL_GAP = URIRef(RSSC + "ControlGap")

STANDARD = URIRef(RSSC + "Standard")
CLAUSE_REFERENCE = URIRef(RSSC + "ClauseReference")
SECURITY_REQUIREMENT = URIRef(RSSC + "SecurityRequirement")
FOUNDATIONAL_REQUIREMENT = URIRef(RSSC + "FoundationalRequirement")
AUTONOMY_BAND = URIRef(RSSC + "AutonomyBand")
SAFETY_FUNCTION = URIRef(RSSC + "SafetyFunction")
SUPPLY_CHAIN_ROLE = URIRef(RSSC + "SupplyChainRole")
SECURITY_LEVEL = URIRef(RSSC + "SecurityLevel")
SOURCE = URIRef(RSSC + "Source")

P_CITATION = URIRef(RSSC + "citation")
P_EVIDENCE_TYPE = URIRef(RSSC + "evidenceType")
P_CONFIDENCE = URIRef(RSSC + "confidence")
P_BASIS = URIRef(RSSC + "basis")
P_APPLIES_TO_BAND = URIRef(RSSC + "appliesToBand")
P_APPLIES_TO_ROLE = URIRef(RSSC + "appliesToRole")
P_SUBJECT_CLAUSE = URIRef(RSSC + "subjectClause")
P_TARGET_REQUIREMENT = URIRef(RSSC + "targetRequirement")
P_MAPPING_RELATION = URIRef(RSSC + "mappingRelation")
P_IMPACTING_REQUIREMENT = URIRef(RSSC + "impactingRequirement")
P_IMPACTED_SAFETY_FUNCTION = URIRef(RSSC + "impactedSafetyFunction")
P_SAFETY_CONSEQUENCE = URIRef(RSSC + "safetyConsequence")
P_CLAIM_SUBJECT = URIRef(RSSC + "claimSubject")
P_CLAIMED_LEVEL = URIRef(RSSC + "claimedLevel")
P_SECURITY_LEVEL_TYPE = URIRef(RSSC + "securityLevelType")
P_EXPOSURE = URIRef(RSSC + "exposure")
P_UNADDRESSED_BY = URIRef(RSSC + "unaddressedBy")
P_WOULD_BE_ADDRESSED_BY = URIRef(RSSC + "wouldBeAddressedBy")
P_GAP_KIND = URIRef(RSSC + "gapKind")
P_IN_STANDARD = URIRef(RSSC + "inStandard")
P_DESIGNATION = URIRef(RSSC + "designation")
P_CLAUSE_IDENTIFIER = URIRef(RSSC + "clauseIdentifier")
P_CLAUSE_TITLE = URIRef(RSSC + "clauseTitle")
P_NORMATIVE_STATUS = URIRef(RSSC + "normativeStatus")
P_REQUIREMENT_IDENTIFIER = URIRef(RSSC + "requirementIdentifier")
P_REQUIREMENT_TITLE = URIRef(RSSC + "requirementTitle")
P_DEFINED_BY_CLAUSE = URIRef(RSSC + "definedByClause")
P_DERIVED_FROM_FR = URIRef(RSSC + "derivedFromFoundationalRequirement")
P_SOURCE_URL = URIRef(RSSC + "sourceUrl")
P_PAYWALLED = URIRef(RSSC + "paywalled")

ASSERTION_CLASSES = [
    CROSSWALK_ASSERTION,
    SAFETY_IMPACT_ASSERTION,
    SECURITY_LEVEL_CLAIM,
    CONTROL_GAP,
]

# Every property whose value may point at a security-side anchor. Used by the
# coverage report to decide whether a foundational requirement was reached.
SECURITY_ANCHOR_PROPERTIES = [
    P_TARGET_REQUIREMENT,
    P_IMPACTING_REQUIREMENT,
    P_WOULD_BE_ADDRESSED_BY,
]


class LoadResult:
    """What was actually parsed, so that no count rests on an unread file."""

    def __init__(self):
        self.graph = Graph()
        self.files = []          # list of (Path, triple_count, sha256)
        self.missing_dirs = []   # directories that do not exist
        self.empty_dirs = []     # directories that exist but hold no .ttl
        self.errors = []         # list of (Path, exception text)

    @property
    def triples(self):
        return len(self.graph)

    @property
    def ok(self):
        return not self.errors


def _sha256(path):
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()[:16]


def load_dirs(dirs):
    """Parse every .ttl in the given directories into one graph.

    Files are parsed in sorted order so the result is deterministic. A parse
    failure is recorded rather than raised, because a partial load that is
    reported is more useful than a traceback that hides the other files.
    """
    result = LoadResult()
    for d in dirs:
        if not d.is_dir():
            result.missing_dirs.append(d)
            continue
        ttls = sorted(d.glob("*.ttl"))
        if not ttls:
            result.empty_dirs.append(d)
            continue
        for path in ttls:
            before = len(result.graph)
            try:
                result.graph.parse(path, format="turtle")
            except Exception as exc:  # noqa: BLE001 report, do not mask
                result.errors.append((path, f"{type(exc).__name__}: {exc}"))
                continue
            result.files.append((path, len(result.graph) - before, _sha256(path)))
    return result


def load_data():
    """The graph that is validated: vocabulary plus crosswalk plus any data."""
    return load_dirs(DATA_DIRS)


def load_shapes():
    """The SHACL shapes graph."""
    return load_dirs([SHAPES_DIR])


def subclass_closure(graph, root):
    """All classes rdfs:subClassOf* root, computed without an inference engine."""
    seen = {root}
    frontier = [root]
    while frontier:
        current = frontier.pop()
        for sub in graph.subjects(RDFS.subClassOf, current):
            if sub not in seen:
                seen.add(sub)
                frontier.append(sub)
    return seen


def instances_of(graph, classes):
    """Every node typed with any of the given classes."""
    out = set()
    for cls in classes:
        for node in graph.subjects(RDF.type, cls):
            out.add(node)
    return out


def short(node):
    """A compact, readable form of an IRI or literal for terminal output."""
    if node is None:
        return "(none)"
    text = str(node)
    for prefix, short_form in (
        ("https://tesseract.academy/ns/rssc#", "rssc:"),
        ("https://tesseract.academy/id/rssc/band/", "band:"),
        ("https://tesseract.academy/id/rssc/role/", "role:"),
        ("https://tesseract.academy/id/rssc/security-level/", "sl:"),
        ("https://tesseract.academy/id/rssc/security-level-type/", "slt:"),
        ("https://tesseract.academy/id/rssc/foundational-requirement/", "fr:"),
        ("https://tesseract.academy/id/rssc/evidence-type/", "ev:"),
        ("https://tesseract.academy/id/rssc/confidence/", "conf:"),
        ("https://tesseract.academy/id/rssc/gap-kind/", "gap:"),
        ("https://tesseract.academy/id/rssc/scheme/", "scheme:"),
        ("https://tesseract.academy/id/rssc/", "id:"),
        ("http://www.w3.org/2004/02/skos/core#", "skos:"),
        ("http://www.w3.org/1999/02/22-rdf-syntax-ns#", "rdf:"),
        ("http://www.w3.org/2000/01/rdf-schema#", "rdfs:"),
        ("http://www.w3.org/ns/shacl#", "sh:"),
        ("http://www.w3.org/ns/prov#", "prov:"),
        ("http://purl.org/dc/terms/", "dcterms:"),
    ):
        if text.startswith(prefix):
            return short_form + text[len(prefix):]
    return text


def notation(graph, node):
    """skos:notation if present, else skos:prefLabel, else the short IRI."""
    for value in graph.objects(node, SKOS.notation):
        return str(value)
    for value in graph.objects(node, SKOS.prefLabel):
        return str(value)
    return short(node)


def one(graph, subject, predicate, default=None):
    """The single value of a predicate, or default. Extra values are ignored
    here and are caught by the SHACL maxCount constraints, not silently."""
    for value in graph.objects(subject, predicate):
        return value
    return default
