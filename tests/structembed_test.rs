#![cfg(feature = "embeddings")]

use open_ontologies::structembed::StructuralTrainer;
use open_ontologies::graph::GraphStore;
use std::sync::Arc;

#[test]
fn test_train_simple_hierarchy() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Animal a owl:Class ; rdfs:label "Animal" .
        ex:Mammal a owl:Class ; rdfs:label "Mammal" ; rdfs:subClassOf ex:Animal .
        ex:Dog a owl:Class ; rdfs:label "Dog" ; rdfs:subClassOf ex:Mammal .
        ex:Cat a owl:Class ; rdfs:label "Cat" ; rdfs:subClassOf ex:Mammal .
        ex:Vehicle a owl:Class ; rdfs:label "Vehicle" .
        ex:Car a owl:Class ; rdfs:label "Car" ; rdfs:subClassOf ex:Vehicle .
    "#, None).unwrap();

    let trainer = StructuralTrainer::new(10, 500, 0.1);
    let embeddings = trainer.train(&store).unwrap();

    assert_eq!(embeddings.len(), 6);

    let dog = &embeddings["http://example.org/Dog"];
    let cat = &embeddings["http://example.org/Cat"];
    let car = &embeddings["http://example.org/Car"];

    let dog_cat = open_ontologies::poincare::poincare_distance(dog, cat);
    let dog_car = open_ontologies::poincare::poincare_distance(dog, car);
    assert!(dog_cat < dog_car, "Dog-Cat ({dog_cat}) should be closer than Dog-Car ({dog_car})");
}

#[test]
fn test_train_parent_closer_than_unrelated() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Animal a owl:Class .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:Vehicle a owl:Class .
    "#, None).unwrap();

    let trainer = StructuralTrainer::new(10, 500, 0.1);
    let embeddings = trainer.train(&store).unwrap();

    let dog = &embeddings["http://example.org/Dog"];
    let animal = &embeddings["http://example.org/Animal"];
    let vehicle = &embeddings["http://example.org/Vehicle"];

    let dog_animal = open_ontologies::poincare::poincare_distance(dog, animal);
    let dog_vehicle = open_ontologies::poincare::poincare_distance(dog, vehicle);
    assert!(dog_animal < dog_vehicle,
        "Dog-Animal ({dog_animal}) should be closer than Dog-Vehicle ({dog_vehicle})");
}

#[test]
fn test_root_near_origin() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Thing a owl:Class .
        ex:Animal a owl:Class ; rdfs:subClassOf ex:Thing .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
    "#, None).unwrap();

    let trainer = StructuralTrainer::new(10, 500, 0.1);
    let embeddings = trainer.train(&store).unwrap();

    let thing_norm: f32 = embeddings["http://example.org/Thing"].iter().map(|x| x*x).sum::<f32>().sqrt();
    let dog_norm: f32 = embeddings["http://example.org/Dog"].iter().map(|x| x*x).sum::<f32>().sqrt();

    assert!(thing_norm < dog_norm,
        "Root Thing ({thing_norm}) should have smaller norm than leaf Dog ({dog_norm})");
}
