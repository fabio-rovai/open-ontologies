# W3C Data Catalog Vocabulary (DCAT) - Version 3, Section 4: Conformance

Source: <https://www.w3.org/TR/vocab-dcat-3/#conformance>
Publication date: 2024-08-22 (W3C Recommendation)
Copyright: World Wide Web Consortium. Licensed under the
[W3C Software and Document Notice and License](https://www.w3.org/copyright/software-license-2023/).
Retrieved 2026-08-25 by fetching the published HTML and extracting the
`<section id="conformance">` element verbatim (tags stripped, text
unmodified, list structure preserved).

---

## 4. Conformance

As well as sections marked as non-normative, all authoring guidelines,
diagrams, examples, and notes in this specification are non-normative.
Everything else in this specification is normative.

The key words MAY, MUST, MUST NOT, and SHOULD in this document are to be
interpreted as described in BCP 14 [RFC2119] [RFC8174] when, and only when,
they appear in all capitals, as shown here.

A data catalog conforms to DCAT if:

- Access to data is organized into datasets, distributions, data services and
  dataset series.
- An RDF description of the catalog itself, the corresponding cataloged
  resources, and distributions is available (but the choice of RDF syntax,
  access protocol, and access policy are not mandated by this specification).
- The contents of all metadata fields that are held in the catalog and that
  contain data about the catalog itself, the corresponding cataloged
  resources, and distributions are included in this RDF description and are
  expressed using the appropriate classes and properties from DCAT, except
  where no such class or property exists.
- All classes and properties defined in DCAT are used in a way consistent
  with the semantics declared in this specification.

DCAT-compliant catalogs MAY include additional non-DCAT metadata fields and
additional RDF data in the catalog's RDF description.

A DCAT profile is a specification for a data catalog that adds additional
constraints to DCAT. A data catalog that conforms to the profile also
conforms to DCAT. Additional constraints in a profile MAY include:

- Cardinality constraints, including a minimum set of required metadata
  fields
- Sub-classes and sub-properties of the standard DCAT classes and properties
- Classes and properties for additional metadata fields not covered in DCAT
  vocabulary specification
- Controlled vocabularies or IRI sets as acceptable values for properties
- Requirements for specific access mechanisms (RDF syntaxes, protocols) to
  the catalog's RDF description

Note

The notion of profile used in this document denotes metadata specifications
that the Dublin Core community would call application profiles [DCAP].
