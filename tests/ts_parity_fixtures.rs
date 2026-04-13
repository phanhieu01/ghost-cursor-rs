use ghost_cursor::{path, BoundingBox, PathOptions, PathPoint, PathTarget, Vector};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Deserialize)]
struct JsonVector {
    x: f64,
    y: f64,
}

impl From<JsonVector> for Vector {
    fn from(value: JsonVector) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct JsonBoundingBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl From<JsonBoundingBox> for BoundingBox {
    fn from(value: JsonBoundingBox) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Fixture {
    metadata: FixtureMetadata,
    params: FixtureParams,
    data: Vec<FixturePoint>,
}

#[derive(Debug, Deserialize)]
struct FixtureMetadata {
    case_id: String,
    seed: String,
    upstream_commit: String,
    upstream_version: String,
}

#[derive(Debug, Deserialize)]
struct FixtureParams {
    start: JsonVector,
    target: FixtureTarget,
    options: FixturePathOptions,
}

#[derive(Debug, Deserialize)]
struct FixtureTarget {
    kind: String,
    point: Option<JsonVector>,
    #[serde(rename = "box")]
    box_target: Option<JsonBoundingBox>,
}

#[derive(Debug, Deserialize)]
struct FixturePathOptions {
    spread_override: Option<f64>,
    move_speed: Option<f64>,
    use_timestamps: bool,
}

#[derive(Debug, Deserialize)]
struct FixturePoint {
    x: f64,
    y: f64,
    timestamp: Option<u64>,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from("fixtures/ts-parity")
}

fn fixture_files() -> Vec<PathBuf> {
    let mut files = vec![];
    for entry in fs::read_dir(fixtures_dir()).expect("fixtures/ts-parity must exist") {
        let entry = entry.expect("valid fixture entry");
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with(".json") && name != "index.json" {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn load_fixture(path: &PathBuf) -> Fixture {
    let raw = fs::read_to_string(path).expect("fixture json must be readable");
    serde_json::from_str(&raw).expect("fixture json must be valid")
}

fn path_target(target: &FixtureTarget) -> PathTarget {
    match target.kind.as_str() {
        "point" => PathTarget::Point(
            target
                .point
                .expect("point target must include point")
                .into(),
        ),
        "box" => PathTarget::Box(
            target
                .box_target
                .expect("box target must include box")
                .into(),
        ),
        other => panic!("unsupported target kind: {other}"),
    }
}

fn monotonic_timestamps(points: &[PathPoint]) -> bool {
    let mut last = None;
    for p in points {
        let current = match p {
            PathPoint::Timed(v) => Some(v.timestamp),
            PathPoint::Plain(_) => None,
        };

        if let (Some(prev), Some(now)) = (last, current) {
            if now < prev {
                return false;
            }
        }
        if current.is_some() {
            last = current;
        }
    }
    true
}

#[test]
fn fixtures_are_available_and_parse() {
    let files = fixture_files();
    assert!(
        !files.is_empty(),
        "No TS parity fixtures found. Run node scripts/ts-parity/generate-fixtures.js"
    );

    for file in files {
        let fixture = load_fixture(&file);
        assert!(!fixture.metadata.case_id.is_empty());
        assert!(!fixture.metadata.seed.is_empty());
        assert!(!fixture.metadata.upstream_commit.is_empty());
        assert!(!fixture.metadata.upstream_version.is_empty());
        assert!(!fixture.data.is_empty());
    }
}

#[test]
#[ignore = "parity gate: run manually while closing TS-vs-Rust gaps"]
fn compare_rust_output_with_ts_fixtures() {
    for file in fixture_files() {
        let fixture = load_fixture(&file);
        let options = PathOptions {
            spread_override: fixture.params.options.spread_override,
            move_speed: fixture.params.options.move_speed,
            use_timestamps: fixture.params.options.use_timestamps,
        };

        let rust_route = path(
            fixture.params.start.into(),
            path_target(&fixture.params.target),
            &options,
        );
        assert!(!rust_route.is_empty(), "empty route for {}", fixture.metadata.case_id);

        let ts_len = fixture.data.len() as f64;
        let rust_len = rust_route.len() as f64;
        let relative_len_delta = ((rust_len - ts_len) / ts_len).abs();

        // Start with loose thresholds to track drift without blocking implementation.
        let len_tolerance = if fixture.params.options.spread_override == Some(0.0) {
            0.15
        } else {
            0.35
        };

        assert!(
            relative_len_delta <= len_tolerance,
            "{}: length delta too high (ts={}, rust={}, delta={:.3})",
            fixture.metadata.case_id,
            ts_len,
            rust_len,
            relative_len_delta
        );

        let rust_start = rust_route.first().expect("non-empty rust route").vector();
        let rust_end = rust_route.last().expect("non-empty rust route").vector();
        let ts_start = &fixture.data[0];
        let ts_end = fixture.data.last().expect("non-empty ts data");

        let start_dx = (rust_start.x - ts_start.x).abs();
        let start_dy = (rust_start.y - ts_start.y).abs();
        assert!(
            start_dx <= 1e-6 && start_dy <= 1e-6,
            "{}: start point mismatch", fixture.metadata.case_id
        );

        let end_distance = ((rust_end.x - ts_end.x).powi(2) + (rust_end.y - ts_end.y).powi(2)).sqrt();
        let end_tolerance = if fixture.params.target.kind == "point" { 3.0 } else { 250.0 };
        assert!(
            end_distance <= end_tolerance,
            "{}: end point too far from TS fixture (distance={:.3})",
            fixture.metadata.case_id,
            end_distance
        );

        if fixture.params.options.use_timestamps {
            assert!(
                monotonic_timestamps(&rust_route),
                "{}: Rust timestamps are not monotonic",
                fixture.metadata.case_id
            );

            assert!(
                fixture.data.iter().all(|p| p.timestamp.is_some()),
                "{}: TS fixture expected timestamps in all points",
                fixture.metadata.case_id
            );
        }
    }
}
