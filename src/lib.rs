//! # ghost-cursor
//!
//! Generate human-like mouse movements using Bezier curves and Fitts's Law.
//!
//! Port of [ghost-cursor](https://github.com/Xetera/ghost-cursor) to Rust,
//! with integration for [playwright-rust](https://github.com/sctg-development/playwright-rust).
//!
//! ## Quick Start
//!
//! ```no_run
//! use ghost_cursor::{GhostCursor, Vector, PathOptions, PathTarget, path};
//!
//! // Generate movement data between two points (no browser needed)
//! let start = Vector { x: 100.0, y: 100.0 };
//! let end = Vector { x: 600.0, y: 700.0 };
//! let route = path(start, PathTarget::Point(end), &PathOptions::default());
//!
//! // Use with playwright:
//! // let cursor = GhostCursor::new(page);
//! // cursor.click_selector("#sign-up button", &ClickOptions::default()).await?;
//! ```

pub mod bezier;
pub mod cursor;
pub mod math;
pub mod path;
pub mod types;

pub use cursor::GhostCursor;
pub use path::path;
pub use types::*;
