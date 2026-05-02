# Rights Decision Tree

A practical decision tree for assigning a canonical rights statement URI to each record. Walk through the questions in order; each leaf gives you a single URI to use.

This tree covers the **most common cases** for UK heritage collections. Edge cases (orphan works, multi-jurisdiction conflicts, specific donor agreements) require legal review beyond this tree.

---

## Q1: Was the work created by a UK Crown servant in the course of duty?

→ Yes: **go to Q1.1**
→ No: **go to Q2**

### Q1.1: Was it published?

→ Yes: **go to Q1.2**
→ No / unsure: **go to Q1.3**

### Q1.2: Has it been at least 50 years since publication?

→ Yes: `http://rightsstatements.org/vocab/NoC-OKLR/1.0/` — Crown Copyright expired
→ No: `https://www.nationalarchives.gov.uk/information-management/re-using-public-sector-information/uk-government-licensing-framework/crown-copyright/` — current Crown Copyright (consider OGL release)

### Q1.3: Has it been at least 125 years since creation OR 50 years since first commercial exploitation?

→ Yes: `http://rightsstatements.org/vocab/NoC-OKLR/1.0/` — Crown Copyright expired (unpublished works rule)
→ No: Crown Copyright still applies; consider [OGL v3](https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/) release if appropriate

---

## Q2: Was the work created by a US Federal employee in the course of duty?

→ Yes: `http://rightsstatements.org/vocab/NoC-US/1.0/` — Public Domain in US (note: status outside US not asserted)
→ No: **go to Q3**

---

## Q3: Was the work created before the relevant copyright term expired?

The applicable rule depends on jurisdiction and creation date. For UK works:

**For published photographs and most works:** copyright expires 70 years after the death of the author.
**For anonymous and pseudonymous works:** 70 years from publication date.
**For Crown Copyright:** see Q1 (different rules).

### Q3.1: Is the author known?

→ Yes: **go to Q3.2**
→ No / pseudonymous: **go to Q3.3**

### Q3.2: Did the author die more than 70 years ago?

→ Yes: `http://rightsstatements.org/vocab/NoC-OKLR/1.0/` (or equivalent national public-domain statement)
→ No: **go to Q4** (still in copyright)

### Q3.3: Was the work made public more than 70 years ago?

→ Yes: `http://rightsstatements.org/vocab/NoC-OKLR/1.0/`
→ No: `http://rightsstatements.org/vocab/CNE/1.0/` — Copyright Not Evaluated (orphan work, requires diligent search)

---

## Q4: Is the work still in copyright? Determine licensing.

### Q4.1: Has the rightsholder licensed the work to the institution under a documented agreement?

→ Yes: **go to Q4.2**
→ No: `http://rightsstatements.org/vocab/InC/1.0/` — In Copyright (no public reuse permitted; institution may use under fair dealing for archival/research)

### Q4.2: Does the licence permit external reuse?

→ Yes, with attribution: `https://creativecommons.org/licenses/by/4.0/` (CC BY) or equivalent
→ Yes, non-commercial: `https://creativecommons.org/licenses/by-nc/4.0/` (CC BY-NC)
→ Yes, public domain dedication: `https://creativecommons.org/publicdomain/zero/1.0/` (CC0)
→ Yes, share-alike: `https://creativecommons.org/licenses/by-sa/4.0/` (CC BY-SA)
→ No (educational use only): `http://rightsstatements.org/vocab/InC-EDU/1.0/`
→ No (rights cleared but specific terms): document the specific terms in `naph:rightsNote`

---

## Q5: Special cases

### Q5.1: Orphan work (rightsholder unknown despite diligent search)

For UK orphan works covered by the [Orphan Works Licensing Scheme](https://www.gov.uk/government/publications/orphan-works-overview):
→ With orphan works licence: `http://rightsstatements.org/vocab/InC-OW-EU/1.0/`

For orphan works without a formal licence:
→ `http://rightsstatements.org/vocab/CNE/1.0/` — Copyright Not Evaluated (with documented diligent search)

### Q5.2: Multiple rightsholders (correspondence — author + recipient)

Document each separately:

```turtle
ex:letter-001 naph:hasRightsStatement ex:rights-author-component ;
              naph:hasRightsStatement ex:rights-recipient-component ;
              naph:rightsNote "Letter — author and recipient may have separate rights" .
```

If one component blocks reuse, default to the more restrictive statement.

### Q5.3: Indigenous or community-controlled material

Even if legal copyright is clear, use [Local Contexts TK Labels](https://localcontexts.org/labels/) alongside:

```turtle
ex:item-X naph:hasRightsStatement ex:legal-rights ;
          naph:culturalRights ex:tk-label-attribution .
```

### Q5.4: Material with personality / data protection concerns

If the material identifies living individuals (photographs, recordings, personal records):

- Legal rights ≠ ethical right to publish
- Apply `naph:ethicsStatement` per [Module C.E.3](../../01-standard/modules/C-rights-licensing-ethics.md)
- Consider GDPR / Data Protection Act 2018 obligations separately from copyright

### Q5.5: Material from former classified collections

For declassified material:

- Document the declassification event in the provenance chain (`prov:Activity` with date)
- Rights status is determined by ordinary copyright rules from the declassification date forward
- Pre-declassification, the material was Crown Copyright and access was restricted

---

## Quick lookup table

| Common case | Canonical URI |
|---|---|
| UK Crown Copyright, current | `https://www.nationalarchives.gov.uk/.../crown-copyright/` |
| UK Crown Copyright, expired | `http://rightsstatements.org/vocab/NoC-OKLR/1.0/` |
| US Federal work | `http://rightsstatements.org/vocab/NoC-US/1.0/` |
| In copyright, no public reuse | `http://rightsstatements.org/vocab/InC/1.0/` |
| In copyright, educational use OK | `http://rightsstatements.org/vocab/InC-EDU/1.0/` |
| Orphan work (no diligent search) | `http://rightsstatements.org/vocab/CNE/1.0/` |
| Orphan work (with EU licence) | `http://rightsstatements.org/vocab/InC-OW-EU/1.0/` |
| CC BY 4.0 | `https://creativecommons.org/licenses/by/4.0/` |
| CC BY-NC 4.0 | `https://creativecommons.org/licenses/by-nc/4.0/` |
| CC0 | `https://creativecommons.org/publicdomain/zero/1.0/` |
| OGL v3 | `https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/` |

---

## After determining the rights URI

Always record:

```turtle
ex:rights-X a naph:RightsStatement ;
    naph:rightsURI <chosen-URI> ;
    naph:rightsLabel "<canonical label from authority>" ;
    naph:rightsNote "<optional context — e.g. specific declassification reference>" ;
    naph:rightsReviewedOn "2024-04-30"^^xsd:date ;
    naph:rightsReviewedBy ex:reviewer-A ;
    naph:rightsReviewBasis "<brief justification — e.g. CDPA 1988 s.163(3) Crown unpublished works>" .
```

---

## When this tree is not enough

Cases that require legal review beyond this tree:

- Joint authorship across jurisdictions
- Works with multiple sequential rightsholders
- Donated material with non-standard restrictions in the deed of gift
- Material involving Indigenous protocol that the institution has not previously navigated
- Pre-1923 material from non-UK / non-US jurisdictions

For these, document the case-by-case determination and consider engaging a copyright lawyer or your institution's legal counsel.

## Cross-references

- [Module C — Rights, Licensing & Ethics](../../01-standard/modules/C-rights-licensing-ethics.md)
- [rightsstatements.org documentation](https://rightsstatements.org/page/1.0/)
- [Local Contexts TK Labels](https://localcontexts.org/labels/)
- [Open Government Licence v3](https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/)
- [UK Orphan Works Licensing Scheme](https://www.gov.uk/government/publications/orphan-works-overview)
