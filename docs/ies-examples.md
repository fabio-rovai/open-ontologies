# IES SPARQL Examples

The [Information Exchange Standard (IES)](https://github.com/IES-Org/ont-ies) is the UK National Digital Twin Programme's core ontology. It uses a 4D extensionalist (BORO) approach where entities have temporal states, events mark changes, and classification uses the type-instance pattern via `ClassOfEntity`.

These examples assume IES has been loaded into the store:

```bash
# Via marketplace
open-ontologies marketplace install ies

# Or via direct pull
open-ontologies pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/specification/ies-common.ttl
```

---

## 1. Find All Person Subclasses

IES models people through a hierarchy rooted at `Person`. This query finds all classes that specialise it.

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX ies: <http://informationexchangestandard.org/ont/ies/common/>

SELECT ?class ?label WHERE {
  ?class rdfs:subClassOf* ies:Person .
  OPTIONAL { ?class rdfs:label ?label }
  FILTER (?class != ies:Person)
}
ORDER BY ?label
```

**MCP usage:**

```
onto_load the IES ontology, then onto_query with the SPARQL above
```

---

## 2. List All EventParticipant Relationships

IES uses `EventParticipant` to link entities to events — a core 4D pattern. This query finds all properties that connect events to their participants.

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX ies: <http://informationexchangestandard.org/ont/ies/common/>

SELECT ?property ?label ?domain ?range WHERE {
  ?property rdfs:subPropertyOf* ies:isParticipantIn .
  OPTIONAL { ?property rdfs:label ?label }
  OPTIONAL { ?property rdfs:domain ?domain }
  OPTIONAL { ?property rdfs:range ?range }
}
ORDER BY ?label
```

---

## 3. Show 4D Temporal Patterns (States and BoundingStates)

The 4D extensionalist approach models change over time using `State` (a temporal part of an entity) and `BoundingState` (start/end markers). This query maps the temporal modelling pattern.

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX ies: <http://informationexchangestandard.org/ont/ies/common/>

SELECT ?stateClass ?label ?parent WHERE {
  ?stateClass rdfs:subClassOf* ies:State .
  ?stateClass rdfs:subClassOf ?parent .
  OPTIONAL { ?stateClass rdfs:label ?label }
  FILTER (?stateClass != ies:State)
}
ORDER BY ?parent ?label
```

To find bounding states specifically (the start/end markers of temporal parts):

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX ies: <http://informationexchangestandard.org/ont/ies/common/>

SELECT ?boundingState ?label WHERE {
  ?boundingState rdfs:subClassOf* ies:BoundingState .
  OPTIONAL { ?boundingState rdfs:label ?label }
  FILTER (?boundingState != ies:BoundingState)
}
ORDER BY ?label
```

---

## 4. Find All Location Subclasses and Spatial Relationships

IES has a rich spatial model. This query explores the Location hierarchy and its relationships.

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX ies: <http://informationexchangestandard.org/ont/ies/common/>

SELECT ?class ?label ?parent WHERE {
  ?class rdfs:subClassOf* ies:Location .
  ?class rdfs:subClassOf ?parent .
  OPTIONAL { ?class rdfs:label ?label }
  FILTER (?class != ies:Location)
}
ORDER BY ?parent ?label
```

To find spatial properties (inLocation, isPartOf, etc.):

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX ies: <http://informationexchangestandard.org/ont/ies/common/>

SELECT ?property ?label ?domain ?range WHERE {
  ?property rdfs:subPropertyOf* ies:inLocation .
  OPTIONAL { ?property rdfs:label ?label }
  OPTIONAL { ?property rdfs:domain ?domain }
  OPTIONAL { ?property rdfs:range ?range }
}
ORDER BY ?label
```

---

## 5. List All ClassOfEntity Instances (Type-Instance Pattern)

IES uses a distinctive type-instance pattern: instead of creating subclasses for every variation, it uses `ClassOfEntity` individuals to classify entities. For example, `ClassOfBuilding` instances might include "Detached House" or "Flat" — they are data, not schema.

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX ies: <http://informationexchangestandard.org/ont/ies/common/>

SELECT ?classOfClass ?label ?parent WHERE {
  ?classOfClass rdfs:subClassOf* ies:ClassOfEntity .
  OPTIONAL { ?classOfClass rdfs:label ?label }
  OPTIONAL {
    ?classOfClass rdfs:subClassOf ?parent .
    FILTER (?parent != ies:ClassOfEntity)
  }
  FILTER (?classOfClass != ies:ClassOfEntity)
}
ORDER BY ?label
```

---

## Full Workflow Example

Load IES, reason over it, then explore:

```text
# Step 1: Install from marketplace
onto_marketplace install ies

# Step 2: Check what loaded
onto_stats

# Step 3: Run OWL-RL reasoning to materialise inferred triples
onto_reason --profile owl-rl

# Step 4: Check inferred triple count
onto_stats

# Step 5: Lint for completeness
onto_lint

# Step 6: Run any of the SPARQL queries above
onto_query "SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }"
```
