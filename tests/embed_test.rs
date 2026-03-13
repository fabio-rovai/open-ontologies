#[cfg(feature = "embeddings")]
mod tests {
    use open_ontologies::embed::TextEmbedder;
    use std::path::Path;

    #[test]
    fn test_model_not_found_error() {
        let result = TextEmbedder::load(
            Path::new("/nonexistent/model.onnx"),
            Path::new("/nonexistent/tokenizer.json"),
        );
        assert!(result.is_err(), "Should error when model file doesn't exist");
    }

    #[test]
    fn test_embed_text_if_model_exists() {
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        let model_path = model_dir.join("bge-small-en-v1.5.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            eprintln!("Skipping: model not downloaded. Run `open-ontologies init` first.");
            return;
        }

        let embedder = TextEmbedder::load(&model_path, &tokenizer_path).unwrap();
        let vec = embedder.embed("Dog").unwrap();
        assert_eq!(
            vec.len(),
            384,
            "bge-small-en-v1.5 should produce 384-dim vectors"
        );

        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.01,
            "Should be L2-normalized, got norm={norm}"
        );
    }

    #[test]
    fn test_similar_terms_closer() {
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        let model_path = model_dir.join("bge-small-en-v1.5.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            eprintln!("Skipping: model not downloaded.");
            return;
        }

        let embedder = TextEmbedder::load(&model_path, &tokenizer_path).unwrap();
        let dog = embedder.embed("Dog").unwrap();
        let cat = embedder.embed("Cat").unwrap();
        let car = embedder.embed("Automobile").unwrap();

        let dog_cat: f32 = dog.iter().zip(cat.iter()).map(|(a, b)| a * b).sum();
        let dog_car: f32 = dog.iter().zip(car.iter()).map(|(a, b)| a * b).sum();

        assert!(
            dog_cat > dog_car,
            "Dog-Cat similarity ({dog_cat}) should be > Dog-Car ({dog_car})"
        );
    }
}
