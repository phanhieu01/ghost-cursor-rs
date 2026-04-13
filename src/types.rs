use playwright::api::MouseButton;

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
    pub padding_percentage: Option<f64>,
    pub destination: Option<Vector>,
    pub wait_for_selector: Option<f64>,
    pub scroll_speed: Option<f64>,
    pub scroll_delay: Option<f64>,
    pub in_viewport_margin: Option<f64>,
    pub move_speed: Option<f64>,
    pub move_delay: f64,
    pub randomize_move_delay: bool,
    pub max_tries: Option<u32>,
    pub spread_override: Option<f64>,
    /// Distance threshold that triggers overshoot.
    pub overshoot_threshold: f64,
}

impl Default for MoveOptions {
    fn default() -> Self {
        Self {
            padding_percentage: None,
            destination: None,
            wait_for_selector: None,
            scroll_speed: None,
            scroll_delay: None,
            in_viewport_margin: None,
            move_speed: None,
            move_delay: 0.0,
            randomize_move_delay: true,
            max_tries: None,
            spread_override: None,
            overshoot_threshold: 500.0,
        }
    }
}

/// Options for click operations.
#[derive(Debug, Clone)]
pub struct ClickOptions {
    pub padding_percentage: Option<f64>,
    pub destination: Option<Vector>,
    pub wait_for_selector: Option<f64>,
    pub scroll_speed: Option<f64>,
    pub scroll_delay: Option<f64>,
    pub in_viewport_margin: Option<f64>,
    pub move_speed: Option<f64>,
    pub move_delay: f64,
    pub randomize_move_delay: bool,
    pub max_tries: Option<u32>,
    pub hesitate: f64,
    pub wait_for_click: f64,
    pub button: Option<MouseButton>,
    pub click_count: Option<i32>,
    /// Probability [0.0, 1.0] of intentionally clicking outside the target element.
    pub misclick_rate: f64,
    /// Extra distance in pixels used when generating an outside-target click destination.
    pub misclick_distance: f64,
    pub overshoot_threshold: f64,
    pub spread_override: Option<f64>,
}

impl Default for ClickOptions {
    fn default() -> Self {
        Self {
            padding_percentage: None,
            destination: None,
            wait_for_selector: None,
            scroll_speed: None,
            scroll_delay: None,
            in_viewport_margin: None,
            move_speed: None,
            move_delay: 2000.0,
            randomize_move_delay: true,
            max_tries: None,
            hesitate: 0.0,
            wait_for_click: 0.0,
            button: Some(MouseButton::Left),
            click_count: Some(1),
            misclick_rate: 0.0,
            misclick_distance: 30.0,
            overshoot_threshold: 500.0,
            spread_override: None,
        }
    }
}

/// Options for low-level mouse button actions.
#[derive(Debug, Clone)]
pub struct MouseButtonOptions {
    pub button: Option<MouseButton>,
    pub click_count: Option<i32>,
}

impl Default for MouseButtonOptions {
    fn default() -> Self {
        Self {
            button: Some(MouseButton::Left),
            click_count: Some(1),
        }
    }
}

/// Options for resolving elements by selector.
#[derive(Debug, Clone)]
pub struct GetElementOptions {
    /// Time to wait for selector in milliseconds.
    pub wait_for_selector: Option<f64>,
}

impl Default for GetElementOptions {
    fn default() -> Self {
        Self {
            wait_for_selector: None,
        }
    }
}

/// Options for random movement behavior.
#[derive(Debug, Clone)]
pub struct RandomMoveOptions {
    pub move_delay: f64,
    pub randomize_move_delay: bool,
    pub move_speed: Option<f64>,
}

impl Default for RandomMoveOptions {
    fn default() -> Self {
        Self {
            move_delay: 2000.0,
            randomize_move_delay: true,
            move_speed: None,
        }
    }
}

/// Options for scrolling behavior.
#[derive(Debug, Clone)]
pub struct ScrollOptions {
    pub scroll_speed: f64,
    pub scroll_delay: f64,
}

impl Default for ScrollOptions {
    fn default() -> Self {
        Self {
            scroll_speed: 100.0,
            scroll_delay: 200.0,
        }
    }
}

/// Options for ensuring an element is in viewport.
#[derive(Debug, Clone)]
pub struct ScrollIntoViewOptions {
    pub scroll_speed: f64,
    pub scroll_delay: f64,
    pub in_viewport_margin: f64,
    pub wait_for_selector: Option<f64>,
}

impl Default for ScrollIntoViewOptions {
    fn default() -> Self {
        Self {
            scroll_speed: 100.0,
            scroll_delay: 200.0,
            in_viewport_margin: 0.0,
            wait_for_selector: None,
        }
    }
}

/// Partially specified vector for destination-like APIs.
#[derive(Debug, Clone, Copy, Default)]
pub struct PartialVector {
    pub x: Option<f64>,
    pub y: Option<f64>,
}

/// Scroll destination for scroll_to.
#[derive(Debug, Clone, Copy)]
pub enum ScrollToDestination {
    Top,
    Bottom,
    Left,
    Right,
    Position(PartialVector),
}

/// Cursor-level defaults merged when method-level options are omitted.
#[derive(Debug, Clone, Default)]
pub struct DefaultOptions {
    pub random_move_options: Option<RandomMoveOptions>,
    pub move_options: Option<MoveOptions>,
    pub move_to_options: Option<MoveOptions>,
    pub click_options: Option<ClickOptions>,
    pub scroll_options: Option<ScrollOptions>,
    pub scroll_into_view_options: Option<ScrollIntoViewOptions>,
    pub get_element_options: Option<GetElementOptions>,
}

/// Construction options for GhostCursor.
#[derive(Debug, Clone, Default)]
pub struct GhostCursorOptions {
    pub start: Option<Vector>,
    pub perform_random_moves: bool,
    pub visible: bool,
    pub default_options: DefaultOptions,
}

/// The origin point (0, 0).
pub const ORIGIN: Vector = Vector { x: 0.0, y: 0.0 };

impl BoundingBox {
    /// Create from individual components.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }
}
