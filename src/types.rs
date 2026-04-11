/// A 2D point/vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

/// A 2D point with an associated timestamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedVector {
    pub x: f64,
    pub y: f64,
    pub timestamp: u64,
}

/// An axis-aligned bounding box (equivalent to playwright's `FloatRect`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Options for generating a movement path.
#[derive(Debug, Clone)]
pub struct PathOptions {
    /// Override the spread of the generated bezier curve.
    pub spread_override: Option<f64>,
    /// Speed of mouse movement. Default is random.
    pub move_speed: Option<f64>,
    /// Whether to generate timestamps for each point.
    pub use_timestamps: bool,
}

impl Default for PathOptions {
    fn default() -> Self {
        Self {
            spread_override: None,
            move_speed: None,
            use_timestamps: false,
        }
    }
}

/// Options for selecting a target point within a bounding box.
#[derive(Debug, Clone)]
pub struct BoxOptions {
    /// Percentage of padding inside the element (0 = anywhere, 100 = center).
    pub padding_percentage: Option<f64>,
    /// Explicit destination relative to the element's top-left.
    pub destination: Option<Vector>,
}

impl Default for BoxOptions {
    fn default() -> Self {
        Self {
            padding_percentage: None,
            destination: None,
        }
    }
}

/// Target for path generation: either a point or a bounding box.
#[derive(Debug, Clone, Copy)]
pub enum PathTarget {
    Point(Vector),
    Box(BoundingBox),
}

/// A point along a generated path, with optional timestamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathPoint {
    Plain(Vector),
    Timed(TimedVector),
}

impl PathPoint {
    pub fn x(&self) -> f64 {
        match self {
            PathPoint::Plain(v) => v.x,
            PathPoint::Timed(v) => v.x,
        }
    }

    pub fn y(&self) -> f64 {
        match self {
            PathPoint::Plain(v) => v.y,
            PathPoint::Timed(v) => v.y,
        }
    }

    pub fn vector(&self) -> Vector {
        match self {
            PathPoint::Plain(v) => *v,
            PathPoint::Timed(tv) => Vector { x: tv.x, y: tv.y },
        }
    }
}

/// Options for move operations.
#[derive(Debug, Clone)]
pub struct MoveOptions {
    pub move_speed: Option<f64>,
    pub move_delay: f64,
    pub randomize_move_delay: bool,
    pub spread_override: Option<f64>,
    /// Distance threshold that triggers overshoot.
    pub overshoot_threshold: f64,
}

impl Default for MoveOptions {
    fn default() -> Self {
        Self {
            move_speed: None,
            move_delay: 0.0,
            randomize_move_delay: true,
            spread_override: None,
            overshoot_threshold: 500.0,
        }
    }
}

/// Options for click operations.
#[derive(Debug, Clone)]
pub struct ClickOptions {
    pub move_speed: Option<f64>,
    pub move_delay: f64,
    pub randomize_move_delay: bool,
    pub hesitate: f64,
    pub wait_for_click: f64,
    pub overshoot_threshold: f64,
    pub spread_override: Option<f64>,
}

impl Default for ClickOptions {
    fn default() -> Self {
        Self {
            move_speed: None,
            move_delay: 2000.0,
            randomize_move_delay: true,
            hesitate: 0.0,
            wait_for_click: 0.0,
            overshoot_threshold: 500.0,
            spread_override: None,
        }
    }
}

/// The origin point (0, 0).
pub const ORIGIN: Vector = Vector { x: 0.0, y: 0.0 };

impl BoundingBox {
    /// Create from individual components.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }
}
