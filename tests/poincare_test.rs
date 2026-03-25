#![cfg(feature = "embeddings")]

use open_ontologies::poincare::*;

#[test]
fn test_poincare_distance_same_point() {
    let p = vec![0.1, 0.2, 0.3];
    let d = poincare_distance(&p, &p);
    assert!(d.abs() < 1e-6, "Distance to self should be ~0, got {d}");
}

#[test]
fn test_poincare_distance_symmetric() {
    let a = vec![0.1, 0.2];
    let b = vec![0.3, 0.4];
    let d1 = poincare_distance(&a, &b);
    let d2 = poincare_distance(&b, &a);
    assert!((d1 - d2).abs() < 1e-6, "Distance should be symmetric");
}

#[test]
fn test_poincare_distance_origin_farther() {
    let origin = vec![0.0, 0.0];
    let near = vec![0.1, 0.0];
    let far = vec![0.9, 0.0];
    let d_near = poincare_distance(&origin, &near);
    let d_far = poincare_distance(&origin, &far);
    assert!(d_far > d_near, "Boundary point should be farther: {d_far} > {d_near}");
}

#[test]
fn test_cosine_similarity_identical() {
    let a = vec![1.0, 2.0, 3.0];
    let s = cosine_similarity(&a, &a);
    assert!((s - 1.0).abs() < 1e-6, "Cosine of identical vectors should be 1.0");
}

#[test]
fn test_cosine_similarity_orthogonal() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    let s = cosine_similarity(&a, &b);
    assert!(s.abs() < 1e-6, "Cosine of orthogonal vectors should be 0.0");
}

#[test]
fn test_exp_map_origin() {
    let v = vec![0.1, 0.0];
    let result = exp_map(&[0.0, 0.0], &v);
    assert_eq!(result.len(), 2);
    let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(norm < 1.0, "exp_map result should stay inside ball, norm={norm}");
}

#[test]
fn test_project_to_ball() {
    let p = vec![0.99, 0.99];
    let projected = project_to_ball(&p, 1e-5);
    let norm: f32 = projected.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(norm < 1.0, "Projected point should be inside ball, norm={norm}");
}

#[test]
fn test_rsgd_step_stays_in_ball() {
    let point = vec![0.5, 0.3];
    let grad = vec![0.1, -0.2];
    let updated = rsgd_step(&point, &grad, 0.01);
    let norm: f32 = updated.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(norm < 1.0, "RSGD step should keep point inside ball, norm={norm}");
}
