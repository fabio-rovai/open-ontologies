# [DCAT-US Schema v3.0](https://github.com/GSA/dcat-us/)

The FAIRness Project is introducing an update to the Data Catalog (DCAT)
standard for the United States (DCAT-US)!  This update, “DCAT-US v3.0 Schema,”
builds upon the requirements we received from agencies as well as data
creators, providers, and users, Data Inventory statutory requirements, and the
lessons learned over ten years of successful implementation of the Project
Open Data Metadata Standard (DCAT-US v1.1) used by Data.gov.

The update will improve the FAIRness, or Findability, Accessibility,
Interoperability, and Reusability of all types of federal data.  DCAT-US v3
will provide a *single* metadata standard able to support most requirements
for documentation of business, technical, statistical, and geospatial data
consistently.

Key features of the [DCAT-US v3.0 Schema](https://github.com/GSA/dcat-us/) are:

1. DCAT-US v3 is a valid [JSON Schema](https://json-schema.org/) following
   their [2020-12 version](https://json-schema.org/draft/2020-12) of that
   specification. A JSON Schema can be used to programmatically validate
   whether a JSON file is compliant with the DCAT-US v3 schema. See
   <https://github.com/GSA/dcat-us/tree/main/jsonschema/README.md> for more
   details.

1. DCAT-US v3 is closely related to the existing DCAT v1.1 metadata.  New
   metadata elements are added, but there are no major changes to existing
   elements.

1. DCAT-US v3 overcomes some of the limitations of DCAT v1.1 when documenting
   geospatial data, eliminating the need for a separate federal standard for
   this subset of data.

1. DCAT-US v3 is not a new standard; it is an implementation, of
   the [World Wide Web Consortium’s (W3C) DCAT standard](
   https://www.w3.org/TR/vocab-dcat-3/).

1. DCAT-US v3 supports new controlled vocabularies, allowing for consistently
   naming items like federal agencies, file formats, and units of measure.

Please review the [DCAT-US v3.0 Schema](https://github.com/GSA/dcat-us/) and
the material found in the links below, and provide feedback to help make this
standard as useful as possible to you and the broader federal data user
community.

## License

This document is a work of the United States Government and is in the public
domain. See [17 USC §105](https://uscode.house.gov/view.xhtml?req=granuleid:USC-prelim-title17-section105&num=0&edition=prelim)
and [the license file](LICENSE.md) for additional information.

Members of the public and US government employees who wish to contribute are
encouraged to do so, but by contributing, dedicate their work to the public
domain and waive all rights to their contribution under the terms of the [CC0 Public Domain Dedication](http://creativecommons.org/publicdomain/zero/1.0/).
