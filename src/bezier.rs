use crate::math::{generate_bezier_anchors, clamp, direction, magnitude};
use crate::types::Vector;

/// A cubic Bezier curve defined by 4 control points.
#[derive(Debug, Clone, Copy)]
pub struct Bezier {
    p0: Vector,
    p1: Vector,
    p2: Vector,
    p3: Vector,
}

impl Bezier {
    /// Create a new cubic Bezier curve from 4 control points.
    pub fn new(p0: Vector, p1: Vector, p2: Vector, p3: Vector) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Evaluate the curve at parameter `t` in [0, 1].
    pub fn eval(&self, t: f64) -> Vector {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        Vector {
            x: uuu * self.p0.x + 3.0 * uu * t * self.p1.x + 3.0 * u * tt * self.p2.x + ttt * self.p3.x,
            y: uuu * self.p0.y + 3.0 * uu * t * self.p1.y + 3.0 * u * tt * self.p2.y + ttt * self.p3.y,
        }
    }

    /// Compute the approximate arc length using adaptive Simpson's rule.
    pub fn length(&self) -> f64 {
        adaptive_simpson(|t| self.speed_at(t), 0.0, 1.0, 1e-8)
    }

    /// Get a lookup table of `steps` evenly-spaced parameter values along the curve.
    /// Returns `steps` points evaluated at t = i/(steps-1) for i in 0..steps.
    pub fn get_lut(&self, steps: usize) -> Vec<Vector> {
        if steps == 0 {
            return vec![];
        }
        if steps == 1 {
            return vec![self.eval(0.0)];
        }
        (0..steps)
            .map(|i| self.eval(i as f64 / (steps - 1) as f64))
            .collect()
    }

    /// Speed (derivative magnitude) at parameter t.
    fn speed_at(&self, t: f64) -> f64 {
        let u = 1.0 - t;
        let dx = 3.0 * u * u * (self.p1.x - self.p0.x)
            + 6.0 * u * t * (self.p2.x - self.p1.x)
            + 3.0 * t * t * (self.p3.x - self.p2.x);
        let dy = 3.0 * u * u * (self.p1.y - self.p0.y)
            + 6.0 * u * t * (self.p2.y - self.p1.y)
            + 3.0 * t * t * (self.p3.y - self.p2.y);
        dx.hypot(dy)
    }
}

/// Adaptive Simpson's rule for numerical integration.
fn adaptive_simpson(f: impl Fn(f64) -> f64, a: f64, b: f64, tol: f64) -> f64 {
    let c = (a + b) / 2.0;
    let fa = f(a);
    let fb = f(b);
    let fc = f(c);
    let whole = simpson(fa, fb, fc, a, b);
    adaptive_simpson_inner(&f, a, b, fa, fb, fc, whole, tol, 20)
}

fn adaptive_simpson_inner(
    f: &dyn Fn(f64) -> f64,
    a: f64,
    b: f64,
    fa: f64,
    fb: f64,
    fc: f64,
    whole: f64,
    tol: f64,
    max_depth: u32,
) -> f64 {
    let c = (a + b) / 2.0;
    let d = (a + c) / 2.0;
    let e = (c + b) / 2.0;
    let fd = f(d);
    let fe = f(e);
    let left = simpson(fa, fd, fc, a, c);
    let right = simpson(fc, fe, fb, c, b);
    let combined = left + right;

    if max_depth == 0 || (combined - whole).abs() <= 15.0 * tol {
        left + right + (combined - whole) / 15.0
    } else {
        let left = adaptive_simpson_inner(f, a, c, fa, fd, fc, left, tol / 2.0, max_depth - 1);
        let right = adaptive_simpson_inner(f, c, b, fc, fe, fb, right, tol / 2.0, max_depth - 1);
        left + right
    }
}

fn simpson(fa: f64, fb: f64, fc: f64, a: f64, b: f64) -> f64 {
    ((b - a) / 6.0) * (fa + 4.0 * fc + fb)
}

/// Create a bezier curve from start to finish with random control points.
/// The spread is clamped to [2, 200] and defaults to the distance between points.
pub fn bezier_curve(start: Vector, finish: Vector, spread_override: Option<f64>) -> Bezier {
    const MIN_SPREAD: f64 = 2.0;
    const MAX_SPREAD: f64 = 200.0;

    let vec = direction(start, finish);
    let length = magnitude(vec);
    let spread = spread_override.unwrap_or_else(|| clamp(length, MIN_SPREAD, MAX_SPREAD));
    let anchors = generate_bezier_anchors(start, finish, spread);

    Bezier::new(start, anchors[0], anchors[1], finish)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_at_endpoints() {
        let p0 = Vector { x: 0.0, y: 0.0 };
        let p1 = Vector { x: 33.0, y: 66.0 };
        let p2 = Vector { x: 66.0, y: 33.0 };
        let p3 = Vector { x: 100.0, y: 100.0 };
        let b = Bezier::new(p0, p1, p2, p3);

        let start = b.eval(0.0);
        assert!((start.x - 0.0).abs() < 1e-10);
        assert!((start.y - 0.0).abs() < 1e-10);

        let end = b.eval(1.0);
        assert!((end.x - 100.0).abs() < 1e-10);
        assert!((end.y - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_eval_at_midpoint() {
        // True linear bezier: control points evenly spaced on y = x
        let p0 = Vector { x: 0.0, y: 0.0 };
        let p1 = Vector { x: 100.0 / 3.0, y: 100.0 / 3.0 };
        let p2 = Vector { x: 200.0 / 3.0, y: 200.0 / 3.0 };
        let p3 = Vector { x: 100.0, y: 100.0 };
        let b = Bezier::new(p0, p1, p2, p3);

        let mid = b.eval(0.5);
        assert!((mid.x - 50.0).abs() < 0.5);
        assert!((mid.y - 50.0).abs() < 0.5);
    }

    #[test]
    fn test_length_of_line() {
        // Bezier that is a straight line from (0,0) to (100,0) should have length ~100
        let p0 = Vector { x: 0.0, y: 0.0 };
        let p1 = Vector { x: 33.33, y: 0.0 };
        let p2 = Vector { x: 66.66, y: 0.0 };
        let p3 = Vector { x: 100.0, y: 0.0 };
        let b = Bezier::new(p0, p1, p2, p3);

        let len = b.length();
        assert!((len - 100.0).abs() < 0.01, "Length should be ~100, got {}", len);
    }

    #[test]
    fn test_get_lut_count() {
        let b = Bezier::new(
            Vector { x: 0.0, y: 0.0 },
            Vector { x: 25.0, y: 50.0 },
            Vector { x: 75.0, y: 50.0 },
            Vector { x: 100.0, y: 0.0 },
        );
        let lut = b.get_lut(10);
        assert_eq!(lut.len(), 10);
        assert!((lut[0].x - 0.0).abs() < 1e-10);
        assert!((lut[9].x - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_lut_empty() {
        let b = Bezier::new(
            Vector { x: 0.0, y: 0.0 },
            Vector { x: 25.0, y: 50.0 },
            Vector { x: 75.0, y: 50.0 },
            Vector { x: 100.0, y: 0.0 },
        );
        let lut = b.get_lut(0);
        assert!(lut.is_empty());
    }

    #[test]
    fn test_bezier_curve_creates_valid_curve() {
        let start = Vector { x: 0.0, y: 0.0 };
        let finish = Vector { x: 500.0, y: 500.0 };
        let curve = bezier_curve(start, finish, None);

        let start_pt = curve.eval(0.0);
        let end_pt = curve.eval(1.0);
        assert!((start_pt.x - 0.0).abs() < 1e-10);
        assert!((start_pt.y - 0.0).abs() < 1e-10);
        assert!((end_pt.x - 500.0).abs() < 1e-10);
        assert!((end_pt.y - 500.0).abs() < 1e-10);

        let len = curve.length();
        assert!(len > 0.0, "Curve length should be positive");
    }
}
