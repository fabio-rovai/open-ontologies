//! Poincaré ball model — distance, exponential map, and Riemannian SGD.
//! Curvature c = 1.0 (unit ball).

const EPS: f32 = 1e-5;

/// Poincaré ball distance: d(u,v) = arcosh(1 + 2||u-v||² / ((1-||u||²)(1-||v||²)))
pub fn poincare_distance(u: &[f32], v: &[f32]) -> f32 {
    let diff_sq: f32 = u.iter().zip(v.iter()).map(|(a, b)| (a - b).powi(2)).sum();
    let norm_u_sq: f32 = u.iter().map(|x| x * x).sum();
    let norm_v_sq: f32 = v.iter().map(|x| x * x).sum();
    let denom = (1.0 - norm_u_sq).max(EPS) * (1.0 - norm_v_sq).max(EPS);
    let x = 1.0 + 2.0 * diff_sq / denom;
    let x = x.max(1.0);
    (x + (x * x - 1.0).max(0.0).sqrt()).ln()
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < EPS || norm_b < EPS {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Conformal factor λ_x = 2 / (1 - ||x||²)
fn conformal_factor(x: &[f32]) -> f32 {
    let norm_sq: f32 = x.iter().map(|v| v * v).sum();
    2.0 / (1.0 - norm_sq).max(EPS)
}

/// Exponential map: maps a tangent vector at x to a point on the Poincaré ball.
pub fn exp_map(x: &[f32], v: &[f32]) -> Vec<f32> {
    let norm_v: f32 = v.iter().map(|a| a * a).sum::<f32>().sqrt();
    if norm_v < EPS {
        return x.to_vec();
    }
    let lambda = conformal_factor(x);
    let t = (lambda * norm_v / 2.0).tanh();
    let direction: Vec<f32> = v.iter().map(|a| t * a / norm_v).collect();
    let result = mobius_add(x, &direction);
    project_to_ball(&result, EPS)
}

/// Möbius addition: x ⊕ y
fn mobius_add(x: &[f32], y: &[f32]) -> Vec<f32> {
    let x_dot_y: f32 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let norm_x_sq: f32 = x.iter().map(|a| a * a).sum();
    let norm_y_sq: f32 = y.iter().map(|a| a * a).sum();
    let denom = 1.0 + 2.0 * x_dot_y + norm_x_sq * norm_y_sq;
    let denom = denom.max(EPS);
    let num_x = 1.0 + 2.0 * x_dot_y + norm_y_sq;
    let num_y = 1.0 - norm_x_sq;
    x.iter()
        .zip(y.iter())
        .map(|(xi, yi)| (num_x * xi + num_y * yi) / denom)
        .collect()
}

/// Project point back into the Poincaré ball (clamp norm < 1 - eps).
pub fn project_to_ball(p: &[f32], eps: f32) -> Vec<f32> {
    let norm: f32 = p.iter().map(|x| x * x).sum::<f32>().sqrt();
    let max_norm = 1.0 - eps;
    if norm >= max_norm {
        let scale = max_norm / norm;
        p.iter().map(|x| x * scale).collect()
    } else {
        p.to_vec()
    }
}

/// Riemannian SGD step on the Poincaré ball.
/// Rescales Euclidean gradient by (1 - ||x||²)² / 4, then applies exp_map.
pub fn rsgd_step(point: &[f32], euclidean_grad: &[f32], lr: f32) -> Vec<f32> {
    let norm_sq: f32 = point.iter().map(|x| x * x).sum();
    let scale = ((1.0 - norm_sq).max(EPS)).powi(2) / 4.0;
    let tangent: Vec<f32> = euclidean_grad.iter().map(|g| -lr * scale * g).collect();
    exp_map(point, &tangent)
}

/// L2-normalize a vector (project onto unit sphere for cosine space).
pub fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < EPS {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_triangle_inequality() {
        let a = vec![0.1, 0.2];
        let b = vec![0.3, -0.1];
        let c = vec![-0.2, 0.4];
        let d_ab = poincare_distance(&a, &b);
        let d_bc = poincare_distance(&b, &c);
        let d_ac = poincare_distance(&a, &c);
        assert!(d_ac <= d_ab + d_bc + 1e-5, "Triangle inequality violated");
    }

    #[test]
    fn mobius_add_identity() {
        let origin = vec![0.0, 0.0, 0.0];
        let p = vec![0.3, 0.4, 0.0];
        let result = mobius_add(&origin, &p);
        for (a, b) in result.iter().zip(p.iter()) {
            assert!((a - b).abs() < 1e-5, "Origin should be identity for Mobius addition");
        }
    }

    #[test]
    fn conformal_factor_at_origin() {
        let origin = vec![0.0, 0.0];
        let lambda = conformal_factor(&origin);
        assert!((lambda - 2.0).abs() < 1e-5, "Conformal factor at origin should be 2.0, got {}", lambda);
    }

    #[test]
    fn conformal_factor_increases_near_boundary() {
        let near_center = vec![0.1, 0.0];
        let near_edge = vec![0.9, 0.0];
        let lc = conformal_factor(&near_center);
        let le = conformal_factor(&near_edge);
        assert!(le > lc, "Conformal factor should increase near boundary");
    }
}
