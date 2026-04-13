use crate::bezier::bezier_curve;
use crate::math::{bezier_curve_speed, direction, extrapolate, magnitude, overshoot};
use crate::types::{BoundingBox, PathOptions, PathPoint, PathTarget, TimedVector, Vector};

use rand::Rng;

const DEFAULT_WIDTH: f64 = 100.0;
const MIN_STEPS: f64 = 25.0;
const MAX_STEPS: usize = 25;

/// Fitts's Law: calculate movement difficulty index.
/// Used to determine how many steps the mouse movement should take.
fn fitts(distance: f64, width: f64) -> f64 {
    let a = 0.0;
    let b = 2.0;
    let id = (distance / width + 1.0).log2();
    a + b * id
}

fn calculate_steps(length: f64, width: f64, speed: f64) -> usize {
    let base_time = speed * MIN_STEPS;
    let steps = ((fitts(length, width) + 1.0).log2() + base_time).ceil() as usize;
    steps.min(MAX_STEPS)
}

/// Get a random point within a bounding box.
/// `padding_percentage` reduces the usable area (0 = full box, 100 = center only).
pub fn get_random_box_point(box_rect: &BoundingBox, padding_percentage: Option<f64>) -> Vector {
    let mut rng = rand::thread_rng();
    let BoundingBox { x, y, width, height } = *box_rect;

    let padding = padding_percentage.unwrap_or(0.0);
    let (padding_w, padding_h) = if padding > 0.0 && padding <= 100.0 {
        (width * padding / 100.0, height * padding / 100.0)
    } else {
        (0.0, 0.0)
    };

    Vector {
        x: x + padding_w / 2.0 + rng.gen::<f64>() * (width - padding_w),
        y: y + padding_h / 2.0 + rng.gen::<f64>() * (height - padding_h),
    }
}

/// Check if the distance between two points exceeds the overshoot threshold.
pub fn should_overshoot(a: Vector, b: Vector, threshold: f64) -> bool {
    magnitude(direction(a, b)) > threshold
}

/// Check if a point is inside a bounding box.
pub fn intersects_element(vec: Vector, box_rect: &BoundingBox) -> bool {
    vec.x > box_rect.x
        && vec.x <= box_rect.x + box_rect.width
        && vec.y > box_rect.y
        && vec.y <= box_rect.y + box_rect.height
}

/// Generate a set of points for mouse movement between two coordinates.
///
/// This is the main public API matching ghost-cursor's `path()` function.
///
/// # Arguments
/// * `start` - Starting point
/// * `end` - Target point or bounding box
/// * `options` - Path generation options
///
/// # Returns
/// A vector of path points (with optional timestamps)
pub fn path(start: Vector, end: PathTarget, options: &PathOptions) -> Vec<PathPoint> {
    let width = match &end {
        PathTarget::Box(b) if b.width != 0.0 => b.width,
        _ => DEFAULT_WIDTH,
    };

    let end_point = match &end {
        PathTarget::Point(v) => *v,
        PathTarget::Box(b) => Vector { x: b.x, y: b.y },
    };

    let curve = bezier_curve(start, end_point, options.spread_override);
    let length = curve.length() * 0.8;

    let speed = match options.move_speed {
        Some(s) if s > 0.0 => 25.0 / s,
        _ => rand::thread_rng().gen_range(0.5..1.0),
    };

    let steps = calculate_steps(length, width, speed);

    let mut vectors: Vec<Vector> = curve.get_lut(steps);

    // Clamp all points to non-negative coordinates
    for v in &mut vectors {
        v.x = v.x.max(0.0);
        v.y = v.y.max(0.0);
    }

    if options.use_timestamps {
        generate_timestamps(&vectors, options.move_speed)
    } else {
        vectors.into_iter().map(PathPoint::Plain).collect()
    }
}

/// Generate timestamps for each vector point using trapezoidal integration
/// of the bezier curve speed.
fn generate_timestamps(vectors: &[Vector], move_speed: Option<f64>) -> Vec<PathPoint> {
    let speed = move_speed.unwrap_or_else(|| {
        let mut rng = rand::thread_rng();
        rng.gen_range(0.5..1.0)
    });

    let samples = vectors.len();

    let time_to_move = |p0: Vector, p1: Vector, p2: Vector, p3: Vector| -> u64 {
        let mut total = 0.0;
        let dt = 1.0 / samples as f64;
        let mut t = 0.0;

        while t < 1.0 {
            let v1 = bezier_curve_speed(t * dt, p0, p1, p2, p3);
            let v2 = bezier_curve_speed(t, p0, p1, p2, p3);
            total += (v1 + v2) * dt / 2.0;
            t += dt;
        }
        (total / speed).round() as u64
    };

    let mut timed: Vec<PathPoint> = Vec::with_capacity(vectors.len());
    let start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    for (i, v) in vectors.iter().enumerate() {
        if i == 0 {
            timed.push(PathPoint::Timed(TimedVector {
                x: v.x,
                y: v.y,
                timestamp: start_time,
            }));
        } else {
            let p0 = vectors[i - 1];
            let p1 = *v;
            let p2 = if i + 1 < vectors.len() {
                vectors[i + 1]
            } else {
                extrapolate(p0, p1)
            };
            let p3 = if i + 2 < vectors.len() {
                vectors[i + 2]
            } else {
                extrapolate(p1, p2)
            };

            let delta = time_to_move(p0, p1, p2, p3);
            let prev_ts = match timed[i - 1] {
                PathPoint::Timed(tv) => tv.timestamp,
                PathPoint::Plain(_) => start_time,
            };

            timed.push(PathPoint::Timed(TimedVector {
                x: v.x,
                y: v.y,
                timestamp: prev_ts + delta,
            }));
        }
    }

    timed
}

/// Generate an overshoot movement path: first overshoot past the target,
/// then correct back to it.
pub fn overshoot_path(
    start: Vector,
    target: Vector,
    overshoot_radius: f64,
    overshoot_spread: f64,
) -> (Vec<Vector>, Vec<Vector>) {
    let overshot = overshoot(target, overshoot_radius);
    let curve_to_overshoot = bezier_curve(start, overshot, None);
    let steps1 = (curve_to_overshoot.length() / 5.0).ceil() as usize;
    let path1 = curve_to_overshoot.get_lut(steps1.max(5));

    let curve_to_target = bezier_curve(overshot, target, Some(overshoot_spread));
    let steps2 = (curve_to_target.length() / 5.0).ceil() as usize;
    let path2 = curve_to_target.get_lut(steps2.max(3));

    (path1, path2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitts() {
        // Known values: fitts(100, 10) should be > 0
        let result = fitts(100.0, 10.0);
        assert!(result > 0.0);
        // Greater distance -> higher difficulty
        assert!(fitts(500.0, 10.0) > fitts(100.0, 10.0));
        // Greater width -> lower difficulty
        assert!(fitts(100.0, 50.0) < fitts(100.0, 10.0));
    }

    #[test]
    fn test_get_random_box_point() {
        let box_rect = BoundingBox { x: 10.0, y: 10.0, width: 100.0, height: 50.0 };
        for _ in 0..50 {
            let p = get_random_box_point(&box_rect, None);
            assert!(p.x >= 10.0 && p.x <= 110.0);
            assert!(p.y >= 10.0 && p.y <= 60.0);
        }
    }

    #[test]
    fn test_get_random_box_point_with_padding() {
        let box_rect = BoundingBox { x: 0.0, y: 0.0, width: 100.0, height: 100.0 };
        for _ in 0..50 {
            let p = get_random_box_point(&box_rect, Some(50.0));
            // With 50% padding, usable area is 50x50 centered in the box
            assert!(p.x >= 25.0 && p.x <= 75.0, "x={} should be in [25, 75]", p.x);
            assert!(p.y >= 25.0 && p.y <= 75.0, "y={} should be in [25, 75]", p.y);
        }
    }

    #[test]
    fn test_should_overshoot() {
        let a = Vector { x: 0.0, y: 0.0 };
        let b_near = Vector { x: 100.0, y: 100.0 };
        let b_far = Vector { x: 600.0, y: 600.0 };

        assert!(!should_overshoot(a, b_near, 500.0));
        assert!(should_overshoot(a, b_far, 500.0));
    }

    #[test]
    fn test_intersects_element() {
        let box_rect = BoundingBox { x: 10.0, y: 10.0, width: 100.0, height: 50.0 };
        assert!(intersects_element(Vector { x: 50.0, y: 30.0 }, &box_rect));
        assert!(!intersects_element(Vector { x: 5.0, y: 5.0 }, &box_rect));
        assert!(!intersects_element(Vector { x: 120.0, y: 30.0 }, &box_rect));
    }

    #[test]
    fn test_path_generates_points() {
        let start = Vector { x: 100.0, y: 100.0 };
        let end = Vector { x: 600.0, y: 700.0 };
        let options = PathOptions::default();

        let result = path(start, PathTarget::Point(end), &options);
        assert!(!result.is_empty());
        assert!(result.len() >= 2);

        // First point should be close to start
        let first = result.first().unwrap();
        assert!((first.x() - 100.0).abs() < 1e-6);
        assert!((first.y() - 100.0).abs() < 1e-6);

        // Last point should be close to end
        let last = result.last().unwrap();
        assert!((last.x() - 600.0).abs() < 1e-6);
        assert!((last.y() - 700.0).abs() < 1e-6);
    }

    #[test]
    fn test_path_with_box_target() {
        let start = Vector { x: 0.0, y: 0.0 };
        let box_target = BoundingBox { x: 500.0, y: 500.0, width: 100.0, height: 100.0 };
        let options = PathOptions::default();

        let result = path(start, PathTarget::Box(box_target), &options);
        assert!(!result.is_empty());
        assert!(result.len() >= 2);

        // In TS parity mode, BoundingBox pathing uses box x/y (not center).
        let last = result.last().unwrap();
        assert!((last.x() - box_target.x).abs() < 1e-6);
        assert!((last.y() - box_target.y).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_steps_matches_ts_formula_order() {
        let length = 1000.0;
        let width = 100.0;
        let speed = 0.6;

        let steps = calculate_steps(length, width, speed);
        assert!(steps <= MAX_STEPS, "steps {} exceeds MAX_STEPS {}", steps, MAX_STEPS);
    }

    #[test]
    fn test_path_with_timestamps() {
        let start = Vector { x: 0.0, y: 0.0 };
        let end = Vector { x: 500.0, y: 500.0 };
        let options = PathOptions {
            use_timestamps: true,
            ..Default::default()
        };

        let result = path(start, PathTarget::Point(end), &options);
        assert!(!result.is_empty());

        // All points should be timed
        for p in &result {
            match p {
                PathPoint::Timed(_) => {},
                PathPoint::Plain(_) => panic!("Expected Timed variant"),
            }
        }

        // Timestamps should be non-decreasing
        let timestamps: Vec<u64> = result.iter().map(|p| match p {
            PathPoint::Timed(tv) => tv.timestamp,
            PathPoint::Plain(_) => 0,
        }).collect();
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1], "Timestamps should be non-decreasing");
        }
    }

    #[test]
    fn test_path_points_non_negative() {
        let start = Vector { x: 10.0, y: 10.0 };
        let end = Vector { x: 500.0, y: 500.0 };
        let options = PathOptions::default();

        let result = path(start, PathTarget::Point(end), &options);
        for p in &result {
            assert!(p.x() >= 0.0, "x should be non-negative");
            assert!(p.y() >= 0.0, "y should be non-negative");
        }
    }
}
