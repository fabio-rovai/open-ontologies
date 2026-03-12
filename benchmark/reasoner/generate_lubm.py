#!/usr/bin/env python3
"""Generate LUBM-style university ontology at configurable scale.

Produces valid OWL Turtle with class hierarchies, object/datatype properties,
someValuesFrom restrictions, cardinality restrictions, and disjoint classes.
"""
import sys
import os


def generate(num_axioms: int, output_path: str):
    """Generate a university ontology with roughly num_axioms axioms."""
    lines = []
    lines.append("@prefix owl: <http://www.w3.org/2002/07/owl#> .")
    lines.append("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .")
    lines.append("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .")
    lines.append("@prefix ub: <http://lubm.example.org/> .")
    lines.append("")

    # Base classes
    base_classes = [
        "Thing", "Person", "Student", "Faculty", "Staff",
        "Professor", "AssociateProfessor", "AssistantProfessor", "FullProfessor",
        "Lecturer", "GraduateStudent", "UndergraduateStudent",
        "ResearchAssistant", "TeachingAssistant",
        "Organization", "University", "Department", "College",
        "Course", "GraduateCourse", "UndergraduateCourse",
        "Publication", "Article", "Book", "TechnicalReport",
        "ResearchGroup", "Work", "Schedule",
    ]

    hierarchy = {
        "Person": "Thing", "Student": "Person", "Faculty": "Person", "Staff": "Person",
        "Professor": "Faculty", "Lecturer": "Faculty",
        "AssociateProfessor": "Professor", "AssistantProfessor": "Professor", "FullProfessor": "Professor",
        "GraduateStudent": "Student", "UndergraduateStudent": "Student",
        "ResearchAssistant": "GraduateStudent", "TeachingAssistant": "GraduateStudent",
        "Organization": "Thing", "University": "Organization",
        "Department": "Organization", "College": "Organization",
        "Course": "Thing", "GraduateCourse": "Course", "UndergraduateCourse": "Course",
        "Publication": "Thing", "Article": "Publication", "Book": "Publication",
        "TechnicalReport": "Publication",
        "ResearchGroup": "Organization", "Work": "Thing", "Schedule": "Thing",
    }

    axiom_count = 0

    # Emit base classes
    for cls in base_classes:
        parent = hierarchy.get(cls)
        if parent:
            lines.append(f"ub:{cls} a owl:Class ; rdfs:subClassOf ub:{parent} ; rdfs:label \"{cls}\" .")
        else:
            lines.append(f"ub:{cls} a owl:Class ; rdfs:label \"{cls}\" .")
        axiom_count += 2  # type + label (or subclass)

    lines.append("")

    # Object properties
    obj_props = [
        ("memberOf", "Person", "Organization"),
        ("worksFor", "Faculty", "Organization"),
        ("headOf", "Professor", "Department"),
        ("teacherOf", "Faculty", "Course"),
        ("takesCourse", "Student", "Course"),
        ("advisor", "GraduateStudent", "Professor"),
        ("publicationAuthor", "Publication", "Person"),
        ("subOrganizationOf", "Organization", "Organization"),
        ("researchInterest", "Faculty", "Thing"),
        ("affiliatedOrganization", "Person", "Organization"),
    ]

    for name, domain, range_ in obj_props:
        lines.append(f"ub:{name} a owl:ObjectProperty ;")
        lines.append(f"    rdfs:domain ub:{domain} ; rdfs:range ub:{range_} ;")
        lines.append(f'    rdfs:label "{name}" .')
        axiom_count += 4

    lines.append("")

    # Datatype properties
    dt_props = [
        ("name", "Thing", "xsd:string"),
        ("emailAddress", "Person", "xsd:string"),
        ("telephone", "Person", "xsd:string"),
        ("age", "Person", "xsd:integer"),
        ("title", "Publication", "xsd:string"),
        ("publicationDate", "Publication", "xsd:date"),
        ("credits", "Course", "xsd:integer"),
    ]

    for name, domain, range_ in dt_props:
        lines.append(f"ub:{name} a owl:DatatypeProperty ;")
        lines.append(f"    rdfs:domain ub:{domain} ; rdfs:range {range_} ;")
        lines.append(f'    rdfs:label "{name}" .')
        axiom_count += 4

    lines.append("")

    # Disjoint classes
    disjoint_pairs = [
        ("Student", "Faculty"), ("GraduateStudent", "UndergraduateStudent"),
        ("Professor", "Lecturer"), ("Article", "Book"),
        ("GraduateCourse", "UndergraduateCourse"),
    ]

    for a, b in disjoint_pairs:
        lines.append(f"[] a owl:AllDisjointClasses ; owl:members ( ub:{a} ub:{b} ) .")
        axiom_count += 1

    lines.append("")

    # Generate scaled classes + restrictions to reach target axiom count
    dept_count = 0
    while axiom_count < num_axioms:
        dept_count += 1
        dept = f"Department{dept_count}"
        lines.append(f"ub:{dept} a owl:Class ; rdfs:subClassOf ub:Department ;")
        lines.append(f'    rdfs:label "{dept}" .')
        axiom_count += 3

        # someValuesFrom restriction
        if axiom_count < num_axioms:
            lines.append(f"ub:{dept} rdfs:subClassOf [")
            lines.append(f"    a owl:Restriction ;")
            lines.append(f"    owl:onProperty ub:subOrganizationOf ;")
            lines.append(f"    owl:someValuesFrom ub:University")
            lines.append(f"] .")
            axiom_count += 1

        # Cardinality restriction
        if axiom_count < num_axioms:
            lines.append(f"ub:{dept} rdfs:subClassOf [")
            lines.append(f"    a owl:Restriction ;")
            lines.append(f"    owl:onProperty ub:headOf ;")
            lines.append(f"    owl:minCardinality 1")
            lines.append(f"] .")
            axiom_count += 1

        # Faculty for department
        if axiom_count < num_axioms:
            fac = f"FacultyOf{dept}"
            lines.append(f"ub:{fac} a owl:Class ;")
            lines.append(f"    rdfs:subClassOf ub:Faculty ;")
            lines.append(f'    rdfs:label "{fac}" ;')
            lines.append(f"    owl:equivalentClass [")
            lines.append(f"        a owl:Restriction ;")
            lines.append(f"        owl:onProperty ub:worksFor ;")
            lines.append(f"        owl:hasValue ub:{dept}")
            lines.append(f"    ] .")
            axiom_count += 4

        # Course for department
        if axiom_count < num_axioms:
            crs = f"CourseOf{dept}"
            lines.append(f"ub:{crs} a owl:Class ;")
            lines.append(f"    rdfs:subClassOf ub:Course ;")
            lines.append(f'    rdfs:label "{crs}" .')
            axiom_count += 3

        lines.append("")

    with open(output_path, "w") as f:
        f.write("\n".join(lines))

    print(f"Generated {output_path}: ~{axiom_count} axioms, {dept_count} departments")


if __name__ == "__main__":
    output_dir = os.path.join(os.path.dirname(__file__), "results")
    os.makedirs(output_dir, exist_ok=True)

    sizes = [1000, 5000, 10000, 50000]
    if len(sys.argv) > 1:
        sizes = [int(s) for s in sys.argv[1:]]

    for size in sizes:
        generate(size, os.path.join(output_dir, f"lubm_{size}.owl"))
