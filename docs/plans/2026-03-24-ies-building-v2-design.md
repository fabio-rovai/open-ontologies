# IES Building Extension v2 — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a comprehensive IES Building domain ontology for the UK residential built environment, derived from the UK EPC data schema, SAP methodology, and building science fundamentals.

**Architecture:** BORO 4D extensionalist ontology extending IES Common. Every physical entity gets a temporal State class. Classification uses ClassOfEntity instances (type-instance pattern). The ontology should be able to ingest the full 105-column UK EPC dataset without dropping any domain-relevant column.

**Tech Stack:** Turtle/RDF, IES4 namespace, Open Ontologies MCP pipeline (validate → load → stats → lint → reason → query)

**Design constraint:** Built from domain knowledge only. No reference to any other IES building implementation. The EPC column schema and building science are the sole inputs.

---

## Source Material

### UK EPC Data Schema (105 columns from Open Data Communities)

The EPC register records every domestic energy assessment in England and Wales. Each row is one assessment of one dwelling. The columns fall into natural groups:

**Location & identity:** postcode, paon, saon, street, locality, towncity, district, county, LOCAL_AUTHORITY, CONSTITUENCY, BUILDING_REFERENCE_NUMBER

**Building form:** propertytype (D/S/T/F), BUILT_FORM (Detached/Semi-Detached/Mid-Terrace/etc.), CONSTRUCTION_AGE_BAND, TOTAL_FLOOR_AREA, numberrooms, NUMBER_HEATED_ROOMS, FLOOR_LEVEL, FLAT_TOP_STOREY, FLAT_STOREY_COUNT, EXTENSION_COUNT, FLOOR_HEIGHT

**Walls:** WALLS_DESCRIPTION, WALLS_ENERGY_EFF, WALLS_ENV_EFF

**Roof:** ROOF_DESCRIPTION, ROOF_ENERGY_EFF, ROOF_ENV_EFF

**Floor:** FLOOR_DESCRIPTION, FLOOR_ENERGY_EFF, FLOOR_ENV_EFF

**Windows:** WINDOWS_DESCRIPTION, WINDOWS_ENERGY_EFF, WINDOWS_ENV_EFF, GLAZED_TYPE, MULTI_GLAZE_PROPORTION, GLAZED_AREA

**Heating (main):** MAINHEAT_DESCRIPTION, MAINHEAT_ENERGY_EFF, MAINHEAT_ENV_EFF, MAIN_FUEL, MAINS_GAS_FLAG

**Heating (controls):** MAINHEATCONT_DESCRIPTION, MAINHEATC_ENERGY_EFF, MAINHEATC_ENV_EFF, MAIN_HEATING_CONTROLS

**Heating (secondary):** SECONDHEAT_DESCRIPTION, SHEATING_ENERGY_EFF, SHEATING_ENV_EFF

**Hot water:** HOTWATER_DESCRIPTION, HOT_WATER_ENERGY_EFF, HOT_WATER_ENV_EFF, SOLAR_WATER_HEATING_FLAG

**Lighting:** LIGHTING_DESCRIPTION, LIGHTING_ENERGY_EFF, LIGHTING_ENV_EFF, LOW_ENERGY_LIGHTING

**Ventilation:** MECHANICAL_VENTILATION, HEAT_LOSS_CORRIDOOR, UNHEATED_CORRIDOR_LENGTH

**Fireplaces:** NUMBER_OPEN_FIREPLACES

**Renewables:** PHOTO_SUPPLY, WIND_TURBINE_COUNT

**Energy ratings:** CURRENT_ENERGY_RATING (A-G), POTENTIAL_ENERGY_RATING, CURRENT_ENERGY_EFFICIENCY (1-100), POTENTIAL_ENERGY_EFFICIENCY

**Environmental impact:** ENVIRONMENT_IMPACT_CURRENT, ENVIRONMENT_IMPACT_POTENTIAL

**Emissions:** CO2_EMISSIONS_CURRENT, CO2_EMISS_CURR_PER_FLOOR_AREA, CO2_EMISSIONS_POTENTIAL

**Energy consumption:** ENERGY_CONSUMPTION_CURRENT, ENERGY_CONSUMPTION_POTENTIAL

**Costs:** HEATING_COST_CURRENT, HEATING_COST_POTENTIAL, HOT_WATER_COST_CURRENT, HOT_WATER_COST_POTENTIAL, LIGHTING_COST_CURRENT, LIGHTING_COST_POTENTIAL, ENERGY_TARIFF

**Assessment metadata:** inspectiondate, lodgementdate, TRANSACTION_TYPE, TENURE

### SAP Methodology (Standard Assessment Procedure)

SAP is the UK government's methodology for assessing the energy performance of dwellings. Key concepts:
- SAP rating: 1-100 scale based on energy cost per m² floor area
- EI rating: Environmental Impact rating based on CO2 emissions per m² floor area
- Both have current (as-assessed) and potential (with improvements) values
- Assessment involves inspecting every element of the building fabric and systems
- The assessor determines construction type, insulation levels, heating system details
- Fuel costs and carbon factors convert physical properties to scores

### Building Science Fundamentals

A dwelling's energy performance depends on:
1. **Building fabric** — the thermal envelope (walls, roof, floor, windows, doors) that determines heat loss
2. **Heating systems** — how heat is produced, distributed, and controlled
3. **Hot water** — how domestic hot water is produced and stored
4. **Lighting** — type and efficiency of lighting
5. **Ventilation** — air changes per hour, mechanical vs natural
6. **Renewables** — on-site generation (solar PV, solar thermal, wind)

Each fabric element has: construction type, insulation type/thickness, and a U-value (thermal transmittance). Each system has: fuel type, efficiency, and control method.

### BORO 4D Patterns (from IES)

- **Entity + State:** Every physical thing has temporal states. A Building exists over time; its BuildingState captures properties at a point in time.
- **ClassOfEntity:** Instead of subclasses for every variation, use ClassOfEntity individuals. "Detached House" is an instance of ClassOfBuilding, not a subclass of Building.
- **BoundingState:** Marks the start/end of a state (construction date, demolition date, installation date).
- **Event:** Something that happens (assessment, retrofit, installation). Events have participants and temporal bounds.
- **Identifiers:** GeoIdentity subclasses (UPRN, postcode) linked via isIdentifiedBy.

---

## Task Breakdown

### Task 1: Ontology Header + Spatial Hierarchy

Build the spatial foundation from the EPC "building form" columns.

**EPC columns modelled:** propertytype, BUILT_FORM, FLOOR_LEVEL, FLAT_TOP_STOREY, FLAT_STOREY_COUNT, EXTENSION_COUNT

Classes (Entity + State + ClassOf triad):
- Structure / StructureState / ClassOfStructure — a physical built structure
- StructureUnit / StructureUnitState / ClassOfStructureUnit — a self-contained unit within a structure (flat, maisonette)
- Storey / StoreyState / ClassOfStorey — a floor level
- Space / SpaceState / ClassOfSpace — a room or area
- AddressableLocation / AddressableLocationState / ClassOfAddressableLocation — anything with a postal address
- Building / BuildingState / ClassOfBuilding — a complete building (subClassOf Structure, ies:Facility)
- Dwelling / DwellingState / ClassOfDwelling — a residential unit (subClassOf StructureUnit)
- BuiltForm — ClassOfEntity for property types

BoundingStates: ConstructedState, DemolishedState

Properties: containsUnit, containsStorey, containsSpace, hasBuiltForm

BuiltForm individuals (from EPC BUILT_FORM values): Detached, SemiDetached, EndTerrace, MidTerrace, EnclosedEndTerrace, EnclosedMidTerrace, PurposeBuiltFlat, ConvertedFlat, Bungalow, Maisonette, ParkHome

**Expected:** ~24 classes + 4 properties + 11 individuals

### Task 2: Building Envelope — Walls, Roofs, Floors, Windows

Model every fabric element the EPC assessor inspects.

**EPC columns modelled:** WALLS_DESCRIPTION/EFF/ENV, ROOF_DESCRIPTION/EFF/ENV, FLOOR_DESCRIPTION/EFF/ENV, WINDOWS_DESCRIPTION/EFF/ENV, GLAZED_TYPE, MULTI_GLAZE_PROPORTION, GLAZED_AREA, NUMBER_OPEN_FIREPLACES, FLOOR_HEIGHT

For each fabric element, follow the pattern: Element / ElementState / ClassOfElement + construction type + insulation type

Wall classes:
- Wall / WallState / ClassOfWall
- WallSection / WallSectionState / ClassOfWallSection
- WallConstruction (ClassOfEntity — cavity, solid brick, timber frame, etc.)
- WallInsulation / WallInsulationState / ClassOfWallInsulation

Roof classes:
- Roof / RoofState / ClassOfRoof
- RoofSection / RoofSectionState / ClassOfRoofSection
- RoofConstruction (ClassOfEntity)
- RoofInsulation / RoofInsulationState
- RoofInsulationLocation (ClassOfEntity — between rafters, above rafters, flat roof)

Floor classes:
- Floor / FloorState / ClassOfFloor
- FloorSection / FloorSectionState / ClassOfFloorSection
- FloorConstruction (ClassOfEntity — suspended timber, solid concrete, etc.)
- FloorInsulation / FloorInsulationState

Window classes:
- Window / WindowState / ClassOfWindow
- GlazingType (ClassOfEntity — single, double, triple, secondary)
- DoorType (ClassOfEntity)

Other envelope:
- OpenFireplace / ClassOfOpenFireplace
- InsulationElement / InsulationElementState / ClassOfInsulationElement
- CeilingInsulation / CeilingInsulationState

Properties: hasWall, hasRoof, hasFloor, hasWindow, hasDoor, hasFireplace, insulationThickness, constructionMethod, glazingType, glazedProportion, glazedArea, floorHeight, energyEfficiency, environmentalEfficiency

**Expected:** ~45 classes + ~15 properties

### Task 3: Heating and Hot Water Systems

Model the complete heating chain from the EPC heating columns.

**EPC columns modelled:** MAINHEAT_DESCRIPTION/EFF/ENV, MAINHEATCONT_DESCRIPTION/EFF/ENV, SECONDHEAT_DESCRIPTION/EFF/ENV, HOTWATER_DESCRIPTION/EFF/ENV, MAIN_FUEL, MAINS_GAS_FLAG, MAIN_HEATING_CONTROLS, SOLAR_WATER_HEATING_FLAG

Heating system hierarchy (from building science — how heat moves from fuel to room):
- HeatingSystem / HeatingSystemState / ClassOfHeatingSystem
- HeatProductionSystem (what makes heat):
  - BoilerSystem (gas/oil/LPG/electric)
  - HeatPump (air source / ground source / water source)
  - ElectricHeater (storage heater / radiant / panel)
  - FurnaceSystem
  - CommunityHeatingSystem
- HeatDistributionSystem (how heat reaches rooms):
  - RadiatorSystem
  - UnderfloorHeating (hydronic / electric)
  - WarmAirSystem
  - ConvectorSystem
- HeatingControlSystem / ClassOfHeatingControlSystem

Hot water:
- HotWaterSystem / HotWaterSystemState / ClassOfHotWaterSystem
- SolarWaterHeatingSystem

Secondary heating:
- SecondaryHeatingSystem / SecondaryHeatingSystemState

Energy source:
- EnergySource / Fuel / FuelType (ClassOfEntity)
- MainsGasSupply, ElectricitySupply, RenewableEnergySource

FuelType individuals (from EPC MAIN_FUEL values): MainsGas, Electricity, Oil, LPG, SolidFuel, Biomass, CommunityHeatNetwork

Properties: hasHeatingSystem, hasHotWaterSystem, hasHeatingControl, hasSecondaryHeating, usesFuel, isHeatedBy, hasEnergySource, hasSolarWaterHeating

**Expected:** ~40 classes + ~10 properties + 7 individuals

### Task 4: Lighting, Ventilation, Renewables

Model the remaining building systems from EPC columns.

**EPC columns modelled:** LIGHTING_DESCRIPTION/EFF/ENV, LOW_ENERGY_LIGHTING, MECHANICAL_VENTILATION, HEAT_LOSS_CORRIDOOR, UNHEATED_CORRIDOR_LENGTH, PHOTO_SUPPLY, WIND_TURBINE_COUNT

Lighting:
- LightingSystem / LightingSystemState / ClassOfLightingSystem
- LowEnergyLightingProportion (Characteristic)

Ventilation:
- VentilationSystem / VentilationSystemState / ClassOfVentilationSystem
- MechanicalVentilation, NaturalVentilation (ClassOfVentilationSystem instances)
- HeatLossCorridor (asset — unheated corridor causing heat loss)

Renewables:
- PhotovoltaicSystem / PhotovoltaicSystemState
- WindTurbine / WindTurbineState

Properties: hasLighting, hasVentilation, hasPhotovoltaicSystem, hasWindTurbine, lowEnergyLightingProportion, unheatedCorridorLength

**Expected:** ~18 classes + ~6 properties

### Task 5: Energy Performance Assessment

Model the EPC assessment process and all its outputs.

**EPC columns modelled:** CURRENT/POTENTIAL_ENERGY_RATING, CURRENT/POTENTIAL_ENERGY_EFFICIENCY, ENVIRONMENT_IMPACT_CURRENT/POTENTIAL, ENERGY_CONSUMPTION_CURRENT/POTENTIAL, CO2_EMISSIONS_*, *_COST_*, inspectiondate, lodgementdate, TRANSACTION_TYPE, ENERGY_TARIFF

Assessment (Event):
- EnergyPerformanceAssessment (subClassOf ies:Event)
- AccreditedEnergyAssessor (subClassOf ies:PersonState — the assessor's role during the assessment)

Certificate (Document):
- EnergyPerformanceCertificate / EnergyPerformanceCertificateState / ClassOfEnergyPerformanceCertificate
- UKDomesticEPC / UKNonDomesticEPC (subclasses)
- EPCValidity (the period a certificate is valid)

Ratings:
- SAPRating (Characteristic — the SAP score 1-100)
- EnvironmentalImpactRating (Characteristic — EI score)
- EnergyRatingBand (ClassOfEntity — letter grade)
- EnergyRatingBand individuals: BandA, BandB, BandC, BandD, BandE, BandF, BandG

Measures:
- EnergyConsumption (Measure — kWh/m²/year)
- CO2Emission / CO2EmissionPerFloorArea (Measure)
- EnergyCostEstimate (Measure — £/year for heating, hot water, lighting)

Assessment activities (the things the assessor does):
- AssessWallConstruction, AssessRoofConstruction, AssessFloorConstruction
- AssessWindowInsulation, AssessHeatingSystem, AssessHotWaterSystem
- DetermineBuiltForm, DetermineConstructionAgeBand
- DetermineTotalFloorArea, DetermineVentilationType
- DetermineMainFuel, CountOpenFireplaces

Properties: hasSAPRating, hasEIR, hasEnergyRatingBand, hasCO2Emissions, hasEnergyConsumption, hasEnergyCost, assessedDwelling, resultedInCertificate, inspectionDate, lodgementDate, energyScore, currentRating, potentialRating

**Expected:** ~40 classes + ~15 properties + 7 individuals

### Task 6: Identifiers, Location, Construction Age, Tenure

Model the identity and administrative data from EPC columns.

**EPC columns modelled:** postcode, LOCAL_AUTHORITY, CONSTITUENCY, BUILDING_REFERENCE_NUMBER, CONSTRUCTION_AGE_BAND, TENURE, numberrooms, NUMBER_HEATED_ROOMS, TOTAL_FLOOR_AREA

Identifiers:
- UPRN (subClassOf ies:GeoIdentity)
- UDPRN (subClassOf ies:Identifier)
- BuildingReferenceNumber (subClassOf ies:Identifier)
- PostalCode (subClassOf ies:GeoIdentity)

Administrative:
- LocalAuthority (subClassOf ies:GovernmentOrganisation)
- Constituency

Construction age:
- ConstructionAgeBand (ClassOfEntity)
- Individuals: Pre1900, Band1900to1929, Band1930to1949, Band1950to1966, Band1967to1975, Band1976to1982, Band1983to1990, Band1991to1995, Band1996to2002, Band2003to2006, Band2007to2011, Band2012onwards

Tenure:
- Tenure (ClassOfEntity)
- Individuals: OwnerOccupied, RentedSocial, RentedPrivate, Unknown

Properties: hasUPRN, hasPostalCode, hasLocalAuthority, hasConstructionAgeBand, hasTenure, numberOfRooms, numberOfHeatedRooms, totalFloorArea

**Expected:** ~12 classes + ~8 properties + 16 individuals

### Task 7: Retrofit, Improvements, Aggregate Classes

Model improvement potential and collection patterns.

**EPC columns informing this:** POTENTIAL_ENERGY_RATING, POTENTIAL_ENERGY_EFFICIENCY, *_POTENTIAL columns

Retrofit:
- RetrofitIntervention (subClassOf ies:Event)
- PotentialImprovement (Characteristic — a recommended measure)
- ImprovementMeasure (ClassOfEntity)
- InstalledState, ReplacedState, UpgradedState (BoundingStates)

Aggregate/collection classes (for "all walls of a dwelling" patterns):
- AllWallsOfStructureUnit, AllWindowsOfStructureUnit
- AllFloorsOfStructureUnit, AllRoofsOfStructureUnit
- AllOpenFireplacesOfStructureUnit
- AllStoreysOfStructure
- FiniteClassOfElement (base for collection patterns)

Environmental:
- CO2Released (subClassOf ies:Measure)
- EnergyConsumptionPerFloorArea (Measure)
- ChemicalCompound, ChemicalCompoundRelease

Properties: hasImprovement, improvementCost, improvementSaving

**Expected:** ~25 classes + ~3 properties

### Task 8: Validate, Lint, Reason, Benchmark

Run the full pipeline:
```
onto_validate → onto_load → onto_stats → onto_lint → onto_reason --profile rdfs → onto_stats
```

Verify:
- 0 lint issues
- Every class has rdfs:label + rdfs:comment
- Every property has domain + range
- RDFS reasoning adds inferred triples
- Run SPARQL to verify hierarchy depth and pattern consistency

Then run the same benchmark against the NDTP production building ontology (blind comparison — first time seeing the results side by side).

### Task 9: Commit and Push

```bash
git add benchmark/generated/ies-building-extension.ttl
git commit -m "feat: IES Building Extension v2 — comprehensive domain ontology for UK energy performance"
git push
```

---

## Success Criteria (domain-driven, NOT comparative)

1. **Every EPC column has a home** — all 105 columns map to at least one class or property
2. **Full 4D pattern** — every physical entity has Entity + State + ClassOf
3. **0 lint issues** — all classes labelled and commented, all properties have domain/range
4. **Clean reasoning** — RDFS materialises inferred triples without errors
5. **Ingestible** — the ontology can accept real EPC CSV data via onto_ingest
6. **Complete building science coverage** — fabric, systems, assessment, emissions, renewables, retrofit
