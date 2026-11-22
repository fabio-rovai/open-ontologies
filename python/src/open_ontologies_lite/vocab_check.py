"""Closed-world vocabulary checking against a declared ontology.

RDF is open-world: an IRI nobody declared is unknown, not wrong. That is the
right default for the web and the wrong one for checking a generated graph,
because an extractor that invents `ex:hasProteinName` because it sounded
plausible produces RDF that parses, loads and satisfies SHACL without complaint.
Nothing in the standard toolchain objects.

This module closes that world deliberately. It reads the vocabulary an ontology
declares, works out which namespaces that ontology owns, and reports every term
used in the data that sits in one of those namespaces without being declared.

Two rules keep it honest. Instance IRIs are never policed, because individuals
belong to the data rather than the vocabulary. And a check with no vocabulary
loaded never returns a pass, because a green light from an empty ontology is the
exact failure this exists to prevent.

Runs on pyoxigraph, the same engine as everything else here, so it needs no
dependency beyond the base install and answers as the Rust engine's
`vocab_check` does.
"""

from __future__ import annotations

import pyoxigraph as ox

from .engine import resolve_format

OWL = "http://www.w3.org/2002/07/owl#"
RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
RDFS = "http://www.w3.org/2000/01/rdf-schema#"

#: Vocabularies every graph is entitled to use without declaring them.
STD_NS = (
    RDF,
    RDFS,
    OWL,
    "http://www.w3.org/2001/XMLSchema#",
    "http://www.w3.org/ns/shacl#",
    "http://www.w3.org/2004/02/skos/core#",
)

_CLASS_OR_PROPERTY = f"""FILTER(
    ?k = <{OWL}Class>
 || ?k = <{RDFS}Class>
 || ?k = <{OWL}ObjectProperty>
 || ?k = <{OWL}DatatypeProperty>
 || ?k = <{OWL}AnnotationProperty>
 || ?k = <{OWL}FunctionalProperty>
 || ?k = <{OWL}InverseFunctionalProperty>
 || ?k = <{RDF}Property>)"""

# An ontology that gives an IRI a domain or a range has declared it in substance,
# even without typing it, so those axioms count as declarations too.
_DEFINING_AXIOM = f"""FILTER(
    ?p = <{RDFS}domain>
 || ?p = <{RDFS}range>
 || ?p = <{RDFS}subClassOf>
 || ?p = <{RDFS}subPropertyOf>)"""


# Which role a declaration gives a term. A class used as a predicate, or a property
# used as the object of `rdf:type`, is declared but not in that role (#432). A term
# declared as both (OWL 2 punning) fits either role.
_CLASS_KINDS = (f"{OWL}Class", f"{RDFS}Class")
_CLASS_AXIOMS = (f"{RDFS}subClassOf",)
_PROPERTY_KINDS = (
    f"{OWL}ObjectProperty",
    f"{OWL}DatatypeProperty",
    f"{OWL}AnnotationProperty",
    f"{OWL}FunctionalProperty",
    f"{OWL}InverseFunctionalProperty",
    f"{RDF}Property",
)
_PROPERTY_AXIOMS = (f"{RDFS}domain", f"{RDFS}range", f"{RDFS}subPropertyOf")


def _in(var: str, iris: tuple[str, ...]) -> str:
    return "FILTER(" + " || ".join(f"?{var} = <{iri}>" for iri in iris) + ")"


def _namespace_of(iri: str) -> str:
    """Everything up to and including the last `#` or `/`."""
    cut = max(iri.rfind("#"), iri.rfind("/"))
    return iri[: cut + 1] if cut >= 0 else iri


def _store_from(text: str, fmt: str) -> ox.Store:
    store = ox.Store()
    if text.strip():
        store.load(text.encode("utf-8"), format=resolve_format(fmt))
    return store


def _iris(store: ox.Store, sparql: str, var: str) -> set[str]:
    """Collect the IRI bindings of one variable; literals and blanks are skipped."""
    out: set[str] = set()
    for solution in store.query(sparql):
        term = solution[var]
        if isinstance(term, ox.NamedNode):
            out.add(term.value)
    return out


def vocab_check(
    ontology: str,
    data: str,
    *,
    ontology_format: str = "turtle",
    data_format: str = "turtle",
    extra_namespaces: list[str] | None = None,
) -> dict:
    """Check `data` against the vocabulary `ontology` declares.

    `extra_namespaces` polices namespaces the ontology does not own, which is how
    you check data against a vocabulary you have not loaded.

    Returns a report with `conforms`, `undeclared_terms`, the namespaces policed,
    and the counts behind the verdict. `conforms` is False, never True, when
    there was no vocabulary to check against.
    """
    onto_store = _store_from(ontology, ontology_format)
    data_store = _store_from(data, data_format)

    declared = _iris(
        onto_store, f"SELECT DISTINCT ?t WHERE {{ ?t a ?k . {_CLASS_OR_PROPERTY} }}", "t"
    ) | _iris(
        onto_store, f"SELECT DISTINCT ?t WHERE {{ ?t ?p ?o . {_DEFINING_AXIOM} }}", "t"
    )

    classes = _iris(
        onto_store, f"SELECT DISTINCT ?t WHERE {{ ?t a ?k . {_in('k', _CLASS_KINDS)} }}", "t"
    ) | _iris(
        onto_store, f"SELECT DISTINCT ?t WHERE {{ ?t ?p ?o . {_in('p', _CLASS_AXIOMS)} }}", "t"
    )
    properties = _iris(
        onto_store, f"SELECT DISTINCT ?t WHERE {{ ?t a ?k . {_in('k', _PROPERTY_KINDS)} }}", "t"
    ) | _iris(
        onto_store, f"SELECT DISTINCT ?t WHERE {{ ?t ?p ?o . {_in('p', _PROPERTY_AXIOMS)} }}", "t"
    )

    extra = list(extra_namespaces or [])

    # A closed-world check with nothing to close over must never silently pass.
    if not declared and not extra:
        return {
            "conforms": False,
            "undeclared_terms": [],
            "wrong_role_terms": [],
            "checked_namespaces": [],
            "predicates_checked": 0,
            "types_checked": 0,
            "ontology_terms": 0,
            "warning": (
                "no ontology vocabulary found (0 declared terms): load an ontology, "
                "or pass extra_namespaces to police; nothing was checked"
            ),
        }

    policed = {_namespace_of(term) for term in declared}
    policed.update(extra)
    policed.difference_update(STD_NS)

    predicates = _iris(data_store, "SELECT DISTINCT ?p WHERE { ?s ?p ?o }", "p")
    types = _iris(data_store, "SELECT DISTINCT ?c WHERE { ?s a ?c }", "c")

    undeclared = sorted(
        iri
        for iri in predicates | types
        if _namespace_of(iri) in policed and iri not in declared
    )

    wrong_role = sorted(
        (
            {"term": iri, "used_as": used_as, "declared_as": declared_as}
            for used, used_as, fits, declared_as in (
                (predicates, "property", properties, "class"),
                (types, "class", classes, "property"),
            )
            for iri in used
            if iri in declared and iri not in fits
        ),
        key=lambda entry: (entry["term"], entry["used_as"]),
    )

    return {
        "conforms": not undeclared and not wrong_role,
        "undeclared_terms": undeclared,
        "wrong_role_terms": wrong_role,
        "checked_namespaces": sorted(policed),
        "predicates_checked": len(predicates),
        "types_checked": len(types),
        "ontology_terms": len(declared),
    }
