//! End-to-end test: load ontology → embed → search → align with embeddings.
//! Only runs if the ONNX model is available.

#[cfg(feature = "embeddings")]
mod tests {
    use open_ontologies::graph::GraphStore;
    use open_ontologies::state::StateDb;
    use open_ontologies::vecstore::VecStore;
    use open_ontologies::embed::TextEmbedder;
    use open_ontologies::structembed::StructuralTrainer;
    use open_ontologies::poincare::cosine_similarity;
    use std::sync::Arc;

    fn model_available() -> bool {
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        model_dir.join("bge-small-en-v1.5.onnx").exists()
            && model_dir.join("tokenizer.json").exists()
    }

    #[test]
    fn test_e2e_embed_search_align() {
        if !model_available() {
            eprintln!("Skipping e2e: model not downloaded");
            return;
        }

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let db = StateDb::open(&path).unwrap();
        let graph = Arc::new(GraphStore::new());

        // Load a small ontology
        graph.load_turtle(r#"
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

        // Step 1: Load embedder
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        let embedder = TextEmbedder::load(
            &model_dir.join("bge-small-en-v1.5.onnx"),
            &model_dir.join("tokenizer.json"),
        ).unwrap();

        // Step 2: Train structural embeddings
        let trainer = StructuralTrainer::new(32, 100, 0.01);
        let struct_embeddings = trainer.train(&graph).unwrap();

        // Step 3: Build vector store
        let mut vecstore = VecStore::new(db.clone());
        for (iri, struct_vec) in &struct_embeddings {
            let label = iri.rsplit('/').next().unwrap_or(iri);
            let text_vec = embedder.embed(label).unwrap();
            vecstore.upsert(iri, &text_vec, struct_vec);
        }
        assert_eq!(vecstore.len(), 6, "Should have 6 embedded classes");

        // Step 4: Search — "pet" should return Dog or Cat in top 3
        let pet_vec = embedder.embed("pet").unwrap();
        let results = vecstore.search_cosine(&pet_vec, 6);
        let top3_iris: Vec<&str> = results.iter().take(3).map(|r| r.0.as_str()).collect();
        assert!(
            top3_iris.iter().any(|iri| iri.contains("Dog") || iri.contains("Cat") || iri.contains("Animal")),
            "Searching 'pet' should find Dog, Cat, or Animal in top 3: {:?}", top3_iris
        );

        // Step 5: Structural search — siblings should be close
        let dog_struct = vecstore.get_struct_vec("http://example.org/Dog").unwrap();
        let struct_results = vecstore.search_poincare(dog_struct, 6);
        // Cat should be closer to Dog than Vehicle in structural space
        let cat_rank = struct_results.iter().position(|r| r.0.contains("Cat"));
        let vehicle_rank = struct_results.iter().position(|r| r.0.contains("Vehicle"));
        if let (Some(cat_r), Some(veh_r)) = (cat_rank, vehicle_rank) {
            assert!(cat_r < veh_r, "Cat should rank closer to Dog than Vehicle structurally");
        }

        // Step 6: Similarity check
        let dog_text = vecstore.get_text_vec("http://example.org/Dog").unwrap();
        let cat_text = vecstore.get_text_vec("http://example.org/Cat").unwrap();
        let car_text = vecstore.get_text_vec("http://example.org/Car").unwrap();
        let dog_cat_sim = cosine_similarity(dog_text, cat_text);
        let dog_car_sim = cosine_similarity(dog_text, car_text);
        assert!(dog_cat_sim > dog_car_sim, "Dog-Cat ({dog_cat_sim}) should be more similar than Dog-Car ({dog_car_sim})");

        // Step 7: Persist and reload
        vecstore.persist().unwrap();

        let mut vecstore2 = VecStore::new(db);
        vecstore2.load_from_db().unwrap();
        assert_eq!(vecstore2.len(), 6, "Reloaded store should have 6 entries");

        // Verify reloaded embeddings match
        let reloaded_dog = vecstore2.get_text_vec("http://example.org/Dog").unwrap();
        let reload_sim = cosine_similarity(dog_text, reloaded_dog);
        assert!((reload_sim - 1.0).abs() < 1e-5, "Reloaded embedding should match original");

        println!("E2E test passed: load → embed → search → persist → reload");
    }
}
