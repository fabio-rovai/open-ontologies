#![cfg(feature = "embeddings")]
//! Numerical stability tests for Poincaré ball geometry.
//! Tests edge cases: zero vectors, near-boundary points, identical points,
//! very high dimensions, and NaN/infinity propagation.

#[cfg(feature = "embeddings")]
mod stability {
    use open_ontologies::poincare::*;

    #[test]
    fn distance_identical_points_is_zero() {
        let p = vec![0.3, 0.4, 0.0];
        let d = poincare_distance(&p, &p);
        assert!(d.abs() < 1e-4, "Distance to self should be ~0, got {}", d);
    }

    #[test]
    fn distance_zero_vectors() {
        let zero = vec![0.0, 0.0, 0.0];
        let d = poincare_distance(&zero, &zero);
        assert!(d.is_finite(), "Distance between zero vectors should be finite, got {}", d);
        assert!(d.abs() < 1e-4, "Distance between zero vectors should be ~0, got {}", d);
    }

    #[test]
    fn distance_near_boundary() {
        // Points very close to the boundary of the unit ball (norm ≈ 0.999)
        let scale = 0.999 / (3.0f32).sqrt();
        let a = vec![scale, scale, scale];
        let b = vec![-scale, -scale, -scale];
        let d = poincare_distance(&a, &b);
        assert!(d.is_finite(), "Distance near boundary should be finite, got {}", d);
        assert!(d > 0.0, "Distance between distinct near-boundary points should be positive");
    }

    #[test]
    fn distance_one_at_origin() {
        let origin = vec![0.0, 0.0, 0.0];
        let p = vec![0.5, 0.0, 0.0];
        let d = poincare_distance(&origin, &p);
        assert!(d.is_finite(), "Distance from origin should be finite");
        assert!(d > 0.0, "Distance from origin to non-origin point should be positive");
    }

    #[test]
    fn distance_symmetry() {
        let a = vec![0.1, 0.2, 0.3];
        let b = vec![0.4, -0.1, 0.2];
        let d_ab = poincare_distance(&a, &b);
        let d_ba = poincare_distance(&b, &a);
        assert!((d_ab - d_ba).abs() < 1e-6, "Distance should be symmetric: {} vs {}", d_ab, d_ba);
    }

    #[test]
    fn exp_map_zero_tangent_returns_base() {
        let base = vec![0.3, 0.4, 0.0];
        let zero_v = vec![0.0, 0.0, 0.0];
        let result = exp_map(&base, &zero_v);
        for (a, b) in result.iter().zip(base.iter()) {
            assert!((a - b).abs() < 1e-5, "exp_map with zero tangent should return base point");
        }
    }

    #[test]
    fn exp_map_stays_in_ball() {
        // Large tangent vector — result must still be inside the unit ball
        let base = vec![0.5, 0.5, 0.0];
        let large_v = vec![100.0, 100.0, 100.0];
        let result = exp_map(&base, &large_v);
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm < 1.0, "exp_map result must stay inside unit ball, got norm {}", norm);
        assert!(result.iter().all(|x| x.is_finite()), "exp_map result must be finite");
    }

    #[test]
    fn project_to_ball_clamps_outside_points() {
        let outside = vec![2.0, 0.0, 0.0];
        let projected = project_to_ball(&outside, 1e-5);
        let norm: f32 = projected.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm < 1.0, "Projected point must be inside ball, got norm {}", norm);
    }

    #[test]
    fn project_to_ball_preserves_interior_points() {
        let inside = vec![0.3, 0.4, 0.0];
        let projected = project_to_ball(&inside, 1e-5);
        for (a, b) in projected.iter().zip(inside.iter()) {
            assert!((a - b).abs() < 1e-6, "Interior point should not change");
        }
    }

    #[test]
    fn rsgd_step_stays_in_ball() {
        let point = vec![0.8, 0.0, 0.0]; // near boundary
        let grad = vec![1.0, 1.0, 1.0];
        let result = rsgd_step(&point, &grad, 0.1);
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm < 1.0, "RSGD step must stay in ball, got norm {}", norm);
        assert!(result.iter().all(|x| x.is_finite()), "RSGD result must be finite");
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let zero = vec![0.0, 0.0, 0.0];
        let p = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&zero, &p);
        assert!(sim.is_finite(), "Cosine with zero vector should be finite");
        assert!(sim.abs() < 1e-5, "Cosine with zero vector should be 0, got {}", sim);
    }

    #[test]
    fn cosine_similarity_identical_is_one() {
        let p = vec![0.3, 0.4, 0.5];
        let sim = cosine_similarity(&p, &p);
        assert!((sim - 1.0).abs() < 1e-5, "Cosine of identical vectors should be 1, got {}", sim);
    }

    #[test]
    fn cosine_similarity_opposite_is_negative_one() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-5, "Cosine of opposite vectors should be -1, got {}", sim);
    }

    #[test]
    fn l2_normalize_zero_vector() {
        let zero = vec![0.0, 0.0, 0.0];
        let result = l2_normalize(&zero);
        assert!(result.iter().all(|x| x.is_finite()), "Normalizing zero should not produce NaN/Inf");
    }

    #[test]
    fn l2_normalize_produces_unit_vector() {
        let v = vec![3.0, 4.0, 0.0];
        let result = l2_normalize(&v);
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "Normalized vector should have unit length, got {}", norm);
    }

    #[test]
    fn high_dimensional_distance_is_finite() {
        let dim = 512;
        let a: Vec<f32> = (0..dim).map(|i| (i as f32 * 0.001) % 0.5).collect();
        let b: Vec<f32> = (0..dim).map(|i| ((i + 1) as f32 * 0.001) % 0.5).collect();
        let d = poincare_distance(&a, &b);
        assert!(d.is_finite(), "High-dimensional distance should be finite, got {}", d);
    }
}
