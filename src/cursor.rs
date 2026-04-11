use crate::path::{get_random_box_point, overshoot_path, path, should_overshoot};
use crate::types::*;

use playwright::api::{ElementHandle, Page};
use std::sync::Arc;

/// Overshoot constants (matching ghost-cursor TS defaults).
const OVERSHOOT_SPREAD: f64 = 10.0;
const OVERSHOOT_RADIUS: f64 = 120.0;

/// A human-like mouse cursor that generates realistic movements on a playwright Page.
pub struct GhostCursor {
    page: Page,
    location: Vector,
}

impl GhostCursor {
    /// Create a new GhostCursor starting at the origin (0, 0).
    pub fn new(page: Page) -> Self {
        Self {
            page,
            location: ORIGIN,
        }
    }

    /// Create a new GhostCursor starting at a custom position.
    pub fn new_with_start(page: Page, start: Vector) -> Self {
        Self {
            page,
            location: start,
        }
    }

    /// Get the current cursor location.
    pub fn location(&self) -> Vector {
        self.location
    }

    /// Move the mouse to a specific destination point using a human-like path.
    pub async fn move_to(&mut self, dest: Vector, options: &MoveOptions) -> Result<(), Arc<playwright::Error>> {
        let path_options = PathOptions {
            spread_override: options.spread_override,
            move_speed: options.move_speed,
            use_timestamps: false,
        };

        self.dispatch_path(dest, &path_options).await?;

        let delay = if options.randomize_move_delay {
            options.move_delay * rand::random::<f64>()
        } else {
            options.move_delay
        };
        if delay > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
        }

        Ok(())
    }

    /// Move the mouse to an element by getting its bounding box and moving to a random point within.
    pub async fn move_to_element(
        &mut self,
        element: &ElementHandle,
        options: &MoveOptions,
    ) -> Result<(), Arc<playwright::Error>> {
        let float_rect = element.bounding_box().await?;
        let bbox = match float_rect {
            Some(r) => BoundingBox::new(r.x, r.y, r.width, r.height),
            None => return Err(playwright::Error::InvalidParams.into()),
        };

        let destination = get_random_box_point(&bbox, None);

        if should_overshoot(self.location, destination, options.overshoot_threshold) {
            let (path1, path2) = overshoot_path(
                self.location,
                destination,
                OVERSHOOT_RADIUS,
                OVERSHOOT_SPREAD,
            );

            for v in path1 {
                self.page.mouse.r#move(v.x, v.y, None).await?;
                self.location = v;
            }
            for v in path2 {
                self.page.mouse.r#move(v.x, v.y, None).await?;
                self.location = v;
            }
        } else {
            let path_options = PathOptions {
                spread_override: options.spread_override,
                move_speed: options.move_speed,
                use_timestamps: false,
            };
            self.dispatch_path(destination, &path_options).await?;
        }

        let delay = if options.randomize_move_delay {
            options.move_delay * rand::random::<f64>()
        } else {
            options.move_delay
        };
        if delay > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
        }

        Ok(())
    }

    /// Move the mouse to a selector, then click.
    pub async fn click_selector(
        &mut self,
        selector: &str,
        options: &ClickOptions,
    ) -> Result<(), Arc<playwright::Error>> {
        let element = self.page.query_selector(selector).await?;
        let element = match element {
            Some(e) => e,
            None => return Err(playwright::Error::ObjectNotFound.into()),
        };

        let move_options = MoveOptions {
            move_speed: options.move_speed,
            move_delay: 0.0,
            randomize_move_delay: false,
            spread_override: options.spread_override,
            overshoot_threshold: options.overshoot_threshold,
        };
        self.move_to_element(&element, &move_options).await?;

        if options.hesitate > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(options.hesitate as u64)).await;
        }

        self.page.mouse.down(None, None).await?;
        if options.wait_for_click > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(options.wait_for_click as u64))
                .await;
        }
        self.page.mouse.up(None, None).await?;

        let delay = if options.randomize_move_delay {
            options.move_delay * rand::random::<f64>()
        } else {
            options.move_delay
        };
        if delay > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
        }

        Ok(())
    }

    /// Click at the current cursor position.
    pub async fn click(&self, options: &ClickOptions) -> Result<(), Arc<playwright::Error>> {
        if options.hesitate > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(options.hesitate as u64)).await;
        }

        self.page.mouse.down(None, None).await?;

        if options.wait_for_click > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(options.wait_for_click as u64))
                .await;
        }

        self.page.mouse.up(None, None).await?;

        Ok(())
    }

    /// Press mouse down at the current position.
    pub async fn mouse_down(&self) -> Result<(), Arc<playwright::Error>> {
        self.page.mouse.down(None, None).await
    }

    /// Release mouse at the current position.
    pub async fn mouse_up(&self) -> Result<(), Arc<playwright::Error>> {
        self.page.mouse.up(None, None).await
    }

    /// Internal: generate and dispatch a human-like path to the target.
    async fn dispatch_path(
        &mut self,
        target: Vector,
        options: &PathOptions,
    ) -> Result<(), Arc<playwright::Error>> {
        let points = path(self.location, PathTarget::Point(target), options);

        for p in &points {
            let v = p.vector();
            self.page.mouse.r#move(v.x, v.y, None).await?;
            self.location = v;
        }

        Ok(())
    }
}
