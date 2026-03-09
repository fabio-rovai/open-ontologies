# BORO Ontology Comparison: Hand-crafted vs AI-generated

## Structural Metrics

| Metric                       | Hand-crafted | AI-generated | Delta |
| ---------------------------- | -----------: | -----------: | ----: |
| Total classes                |           43 |           13 |   +30 |
| Entity+State pairs           |            9 |            3 |    +6 |
| Unpaired entities (no State) |            9 |            5 |    +4 |
| ClassOf classes              |           10 |            0 |   +10 |
| BoundingState subclasses     |            6 |            2 |    +4 |
| Object properties            |           14 |            5 |    +9 |
| Datatype properties          |            4 |            3 |    +1 |
| Named individuals            |           21 |            8 |   +13 |
| Triples (in namespace)       |          344 |          117 |  +227 |
| Avg comment length (chars)   |          122 |           42 |   +80 |

## Classes in Hand-crafted but NOT in AI-generated

These classes represent intermediate abstractions or speculative
hierarchies that the AI considered unnecessary:

- `BuildingCondition`
- `BuildingElement`
- `BuildingElementState`
- `ClassOfBuilding`
- `ClassOfBuildingElement`
- `ClassOfBuildingElementState`
- `ClassOfBuildingState`
- `ClassOfDwelling`
- `ClassOfDwellingState`
- `ClassOfFloor`
- `ClassOfFloorState`
- `ClassOfRoom`
- `ClassOfRoomState`
- `Door`
- `DoorState`
- `ElementMaterial`
- `FloorLevelDesignation`
- `FloorState`
- `InspectionEvent`
- `InstalledState`
- `OccupancyType`
- `OccupiedState`
- `RemovedState`
- `Roof`
- `RoofState`
- `VacatedState`
- `Wall`
- `WallState`
- `Window`
- `WindowState`

## Properties in Hand-crafted but NOT in AI-generated

These properties serve the intermediate classes that the AI omitted:

- `constructedBy`
- `elementInRoom`
- `hasCondition`
- `hasElement`
- `hasFloorLevel`
- `hasMaterial`
- `hasOccupancyType`
- `inspectedEntity`
- `roomInDwelling`
- `roomCapacity`

## Entity+State Pair Comparison

| Entity                | Hand-crafted has State? | AI has State? |
| --------------------- | :---------------------: | :-----------: |
| Building              |           yes           |      yes      |
| BuildingCondition     |       entity only       |      ---      |
| BuildingElement       |           yes           |      ---      |
| ConstructionEvent     |       entity only       |  entity only  |
| DemolitionEvent       |       entity only       |  entity only  |
| Door                  |           yes           |      ---      |
| Dwelling              |           yes           |      yes      |
| ElementMaterial       |       entity only       |      ---      |
| Floor                 |           yes           |  entity only  |
| FloorLevelDesignation |       entity only       |      ---      |
| InspectionEvent       |       entity only       |      ---      |
| OccupancyEvent        |       entity only       |  entity only  |
| OccupancyType         |       entity only       |      ---      |
| RenovationEvent       |       entity only       |  entity only  |
| Roof                  |           yes           |      ---      |
| Room                  |           yes           |      yes      |
| Wall                  |           yes           |      ---      |
| Window                |           yes           |      ---      |

## ClassOf Hierarchy Comparison

| ClassOf Class               | Hand-crafted | AI-generated |
| --------------------------- | :----------: | :----------: |
| ClassOfBuilding             |     yes      |     ---      |
| ClassOfBuildingElement      |     yes      |     ---      |
| ClassOfBuildingElementState |     yes      |     ---      |
| ClassOfBuildingState        |     yes      |     ---      |
| ClassOfDwelling             |     yes      |     ---      |
| ClassOfDwellingState        |     yes      |     ---      |
| ClassOfFloor                |     yes      |     ---      |
| ClassOfFloorState           |     yes      |     ---      |
| ClassOfRoom                 |     yes      |     ---      |
| ClassOfRoomState            |     yes      |     ---      |

## Summary

- **Class reduction**: 70% fewer classes (43 -> 13)
- **Triple reduction**: 66% fewer triples (344 -> 117)
- **Comment verbosity**: hand-crafted averages 122 chars/comment vs AI's 42 chars/comment
- **ClassOf overhead**: hand-crafted has 10 ClassOf classes; AI has 0
- **BoundingState overhead**: hand-crafted has 6 BoundingState subclasses; AI has 2

## Key Differences Explained

### 1. Intermediate Entity Classes
The hand-crafted ontology creates a `BuildingElement` superclass with
concrete subclasses (`Wall`, `Roof`, `Window`, `Door`), each with their
own `*State` class. The AI version omits these entirely -- individual
building components are modelled as part-of relationships from Building,
classified via `ies:similarEntity` when needed.

### 2. ClassOf Powertype Hierarchies
The hand-crafted ontology creates dedicated `ClassOfX` and `ClassOfXState`
for every entity type (Building, Room, Floor, Dwelling, BuildingElement).
The AI version uses the existing `ies:ClassOfEntity` directly for
classification instances, avoiding 10 extra classes.

### 3. BoundingState Proliferation
The hand-crafted version creates 6 BoundingState subclasses
(`ConstructedState`, `DemolishedState`, `OccupiedState`, `VacatedState`,
`InstalledState`, `RemovedState`). The AI version keeps only the 2
essential ones (`ConstructedState`, `DemolishedState`) and handles
occupancy via events.

### 4. Comment Verbosity
Hand-crafted comments explain BORO theory in each definition.
AI-generated comments state what the class *is* concisely.

### 5. Speculative Classification Classes
The hand-crafted version includes `OccupancyType`, `BuildingCondition`,
`ElementMaterial`, and `FloorLevelDesignation` as intermediate
classification hierarchies. The AI version omits these because they
can be added when actually needed, following YAGNI principles.

## Why AI-native Ontology Engineering Matters

Traditional ontology engineering treats completeness as a virtue. Every
conceivable classification axis is modelled upfront, every entity gets
the full BORO treatment (Entity + State + ClassOf + ClassOfState), and
comments explain the methodology rather than the domain.

The result is an ontology that is *architecturally correct* but
operationally heavy. In this comparison:

- The hand-crafted version has **43 classes** to cover a domain
  that the AI covers with **13 classes** -- a 70% reduction.
- The hand-crafted version produces **344 triples** vs the AI's **117**
  -- 66% less data to store, query, and maintain.
- The AI's comments average **42 characters** vs **122**
  -- they say what something *is* rather than why BORO requires it.

AI-native ontology engineering does not abandon rigour. Both ontologies
use the same IES4 upper ontology, the same 4D perdurantist patterns,
and the same Entity+State temporal modelling. The AI simply applies
these patterns where they deliver value, rather than everywhere they
*could* be applied.

The practical benefit: an AI-generated ontology is easier to understand,
faster to query, cheaper to maintain, and can be extended incrementally
when new requirements emerge -- rather than trying to anticipate all
possible future needs upfront.
