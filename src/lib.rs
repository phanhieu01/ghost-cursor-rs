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
//! use ghost_cursor::{
//!     ClickOptions, CursorTarget, GhostCursor, GhostCursorOptions, PathOptions, PathTarget, Vector, path,
//! };
//!
//! // Generate movement data between two points (no browser needed)
//! let start = Vector { x: 100.0, y: 100.0 };
//! let end = Vector { x: 600.0, y: 700.0 };
//! let route = path(start, PathTarget::Point(end), &PathOptions::default());
//!
//! // Use with playwright:
//! // let cursor = GhostCursor::new(page);
//! // cursor.move_target(CursorTarget::Selector("#sign-up button"), None).await?;
//! // cursor.click_selector("#sign-up button", Some(&ClickOptions::default())).await?;
//!
//! // Optional async startup behavior for visible helper / initial random move:
//! // let cursor = GhostCursor::new_with_options_async(
//! //     page,
//! //     GhostCursorOptions {
//! //         visible: true,
//! //         perform_random_moves: true,
//! //         ..Default::default()
//! //     },
//! // ).await?;
//! ```

pub mod bezier;
pub mod cursor;
pub mod math;
pub mod path;
pub mod types;

#[allow(deprecated)]
pub use cursor::{create_cursor, CursorTarget, GhostCursor};
pub use path::path;
pub use types::*;
