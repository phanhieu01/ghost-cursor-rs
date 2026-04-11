use crate::types::Vector;
use rand::Rng;

/// Vector subtraction: a - b.
pub fn sub(a: Vector, b: Vector) -> Vector {
    Vector { x: a.x - b.x, y: a.y - b.y }
}

/// Scalar division: a / n.
pub fn div(a: Vector, n: f64) -> Vector {
    Vector { x: a.x / n, y: a.y / n }
}

/// Scalar multiplication: a * n.
pub fn mult(a: Vector, n: f64) -> Vector {
    Vector { x: a.x * n, y: a.y * n }
}

/// Vector addition: a + b.
pub fn add(a: Vector, b: Vector) -> Vector {
    Vector { x: a.x + b.x, y: a.y + b.y }
}

/// Extrapolate: b + (b - a). Returns a point beyond b in the direction a->b.
pub fn extrapolate(a: Vector, b: Vector) -> Vector {
    add(b, sub(b, a))
}

/// Linearly map `value` from range1 to range2.
pub fn scale(value: f64, range1: [f64; 2], range2: [f64; 2]) -> f64 {
    (value - range1[0]) * (range2[1] - range2[0]) / (range1[1] - range1[0]) + range2[0]
}

/// Direction vector from a to b: b - a.
pub fn direction(a: Vector, b: Vector) -> Vector {
    sub(b, a)
}

/// Perpendicular (90 degree rotation): {x: a.y, y: -a.x}.
pub fn perpendicular(a: Vector) -> Vector {
    Vector { x: a.y, y: -a.x }
}

/// Magnitude (length) of a vector.
pub fn magnitude(a: Vector) -> f64 {
    a.x.hypot(a.y)
}

/// Normalize to unit vector.
pub fn unit(a: Vector) -> Vector {
    let mag = magnitude(a);
    div(a, mag)
}

/// Set the magnitude of a vector.
pub fn set_magnitude(a: Vector, amount: f64) -> Vector {
    mult(unit(a), amount)
}

/// Random number in [min, max).
pub fn random_number_range(min: f64, max: f64) -> f64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min..max)
}

/// Random point on the line segment from a to b.
pub fn random_vector_on_line(a: Vector, b: Vector) -> Vector {
    let vec = direction(a, b);
    let multiplier: f64 = rand::thread_rng().gen();
    add(a, mult(vec, multiplier))
}

/// Random normal line perpendicular to the direction a->b, with a given range.
/// Returns the random midpoint and the perpendicular vector.
fn random_normal_line(a: Vector, b: Vector, range: f64) -> (Vector, Vector) {
    let rand_mid = random_vector_on_line(a, b);
    let normal_v = set_magnitude(perpendicular(direction(a, rand_mid)), range);
    (rand_mid, normal_v)
}

/// Generate two bezier anchor points between a and b with the given spread.
/// Both points are on the same random side of the a-b line.
pub fn generate_bezier_anchors(a: Vector, b: Vector, spread: f64) -> [Vector; 2] {
    let side: f64 = if rand::thread_rng().gen::<bool>() { 1.0 } else { -1.0 };

    let calc = || -> Vector {
        let (rand_mid, normal_v) = random_normal_line(a, b, spread);
        let choice = mult(normal_v, side);
        random_vector_on_line(rand_mid, add(rand_mid, choice))
    };

    let mut anchors = [calc(), calc()];
    // Sort by x coordinate (matching TS behavior)
    if anchors[0].x > anchors[1].x {
        anchors.swap(0, 1);
    }
    anchors
}

/// Clamp a value to [min, max].
pub fn clamp(target: f64, min: f64, max: f64) -> f64 {
    target.max(min).min(max)
}

/// Generate a random point in a circle of the given radius around the coordinate.
pub fn overshoot(coordinate: Vector, radius: f64) -> Vector {
    let mut rng = rand::thread_rng();
    let angle: f64 = rng.gen_range(0.0..2.0 * std::f64::consts::PI);
    let r = radius * rng.gen::<f64>().sqrt();
    let v = Vector {
        x: r * angle.cos(),
        y: r * angle.sin(),
    };
    add(coordinate, v)
}

/// Calculate the speed (derivative magnitude) at parameter t on a cubic bezier.
pub fn bezier_curve_speed(t: f64, p0: Vector, p1: Vector, p2: Vector, p3: Vector) -> f64 {
    let b1 = 3.0 * (1.0 - t).powi(2) * (p1.x - p0.x)
        + 6.0 * (1.0 - t) * t * (p2.x - p1.x)
        + 3.0 * t.powi(2) * (p3.x - p2.x);
    let b2 = 3.0 * (1.0 - t).powi(2) * (p1.y - p0.y)
        + 6.0 * (1.0 - t) * t * (p2.y - p1.y)
        + 3.0 * t.powi(2) * (p3.y - p2.y);
    b1.hypot(b2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ORIGIN;

    #[test]
    fn test_add() {
        let a = Vector { x: 1.0, y: 2.0 };
        let b = Vector { x: 3.0, y: 4.0 };
        let result = add(a, b);
        assert_eq!(result, Vector { x: 4.0, y: 6.0 });
    }

    #[test]
    fn test_sub() {
        let a = Vector { x: 5.0, y: 7.0 };
        let b = Vector { x: 2.0, y: 3.0 };
        let result = sub(a, b);
        assert_eq!(result, Vector { x: 3.0, y: 4.0 });
    }

    #[test]
    fn test_mult() {
        let v = Vector { x: 3.0, y: 4.0 };
        let result = mult(v, 2.0);
        assert_eq!(result, Vector { x: 6.0, y: 8.0 });
    }

    #[test]
    fn test_div() {
        let v = Vector { x: 6.0, y: 8.0 };
        let result = div(v, 2.0);
        assert_eq!(result, Vector { x: 3.0, y: 4.0 });
    }

    #[test]
    fn test_magnitude() {
        let v = Vector { x: 3.0, y: 4.0 };
        assert_eq!(magnitude(v), 5.0);
    }

    #[test]
    fn test_unit() {
        let v = Vector { x: 3.0, y: 4.0 };
        let u = unit(v);
        assert!((u.x - 0.6).abs() < 1e-10);
        assert!((u.y - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_extrapolate() {
        let a = Vector { x: 0.0, y: 0.0 };
        let b = Vector { x: 10.0, y: 0.0 };
        let result = extrapolate(a, b);
        assert_eq!(result, Vector { x: 20.0, y: 0.0 });
    }

    #[test]
    fn test_perpendicular() {
        let v = Vector { x: 1.0, y: 0.0 };
        let p = perpendicular(v);
        assert_eq!(p, Vector { x: 0.0, y: -1.0 });
    }

    #[test]
    fn test_direction() {
        let a = Vector { x: 1.0, y: 1.0 };
        let b = Vector { x: 4.0, y: 5.0 };
        let d = direction(a, b);
        assert_eq!(d, Vector { x: 3.0, y: 4.0 });
    }

    #[test]
    fn test_scale() {
        let result = scale(5.0, [0.0, 10.0], [0.0, 100.0]);
        assert_eq!(result, 50.0);
    }

    #[test]
    fn test_overshoot_is_within_radius() {
        let center = Vector { x: 100.0, y: 100.0 };
        let radius = 50.0;
        for _ in 0..100 {
            let p = overshoot(center, radius);
            let dist = magnitude(sub(p, center));
            assert!(dist <= radius + 1e-6, "Overshoot point {:?} is too far from center", p);
        }
    }

    #[test]
    fn test_set_magnitude() {
        let v = Vector { x: 3.0, y: 4.0 };
        let result = set_magnitude(v, 10.0);
        assert!((magnitude(result) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_bezier_curve_speed_at_endpoints() {
        let p0 = Vector { x: 0.0, y: 0.0 };
        let p1 = Vector { x: 25.0, y: 50.0 };
        let p2 = Vector { x: 75.0, y: 50.0 };
        let p3 = Vector { x: 100.0, y: 0.0 };
        // Speed should be > 0 at t=0.5
        let speed = bezier_curve_speed(0.5, p0, p1, p2, p3);
        assert!(speed > 0.0);
    }

    #[test]
    fn test_origin() {
        assert_eq!(ORIGIN, Vector { x: 0.0, y: 0.0 });
    }

    #[test]
    fn test_generate_bezier_anchors_sorted_by_x() {
        let a = Vector { x: 0.0, y: 0.0 };
        let b = Vector { x: 500.0, y: 500.0 };
        for _ in 0..50 {
            let anchors = generate_bezier_anchors(a, b, 100.0);
            assert!(anchors[0].x <= anchors[1].x, "Anchors should be sorted by x");
        }
    }
}
