use crate::math::{clamp, overshoot, scale};
use crate::path::{get_random_box_point, intersects_element, path, should_overshoot};
use crate::types::*;

use playwright::api::{ElementHandle, Page};
use rand::Rng;
use std::sync::Arc;

/// Overshoot constants (matching ghost-cursor TS defaults).
const OVERSHOOT_SPREAD: f64 = 10.0;
const OVERSHOOT_RADIUS: f64 = 120.0;
const EXP_SCALE_START: f64 = 90.0;
const ACTION_TIMEOUT_MS: f64 = 30_000.0;

fn compute_scroll_steps(delta_x: f64, delta_y: f64, scroll_speed: f64) -> Vec<(f64, f64)> {
    let scroll_speed = clamp(scroll_speed, 1.0, 100.0);

    let mut delta_x = delta_x;
    let mut delta_y = delta_y;

    let x_direction = if delta_x < 0.0 { -1.0 } else { 1.0 };
    let y_direction = if delta_y < 0.0 { -1.0 } else { 1.0 };

    delta_x = delta_x.abs();
    delta_y = delta_y.abs();

    let larger_distance_is_x = delta_x > delta_y;
    let (larger_distance, shorter_distance) = if larger_distance_is_x {
        (delta_x, delta_y)
    } else {
        (delta_y, delta_x)
    };

    if larger_distance <= f64::EPSILON {
        return vec![];
    }

    let larger_distance_scroll_step = if scroll_speed < EXP_SCALE_START {
        scroll_speed
    } else {
        scale(
            scroll_speed,
            [EXP_SCALE_START, 100.0],
            [EXP_SCALE_START, larger_distance.max(EXP_SCALE_START)],
        )
    };

    let num_steps = (larger_distance / larger_distance_scroll_step).floor().max(1.0) as usize;
    let larger_distance_remainder = larger_distance % larger_distance_scroll_step;
    let shorter_distance_scroll_step = (shorter_distance / num_steps as f64).floor();
    let shorter_distance_remainder = shorter_distance % num_steps as f64;

    let mut steps = Vec::with_capacity(num_steps);
    for i in 0..num_steps {
        let mut longer_distance_delta = larger_distance_scroll_step;
        let mut shorter_distance_delta = shorter_distance_scroll_step;
        if i == num_steps - 1 {
            longer_distance_delta += larger_distance_remainder;
            shorter_distance_delta += shorter_distance_remainder;
        }

        let (mut step_x, mut step_y) = if larger_distance_is_x {
            (longer_distance_delta, shorter_distance_delta)
        } else {
            (shorter_distance_delta, longer_distance_delta)
        };
        step_x *= x_direction;
        step_y *= y_direction;
        steps.push((step_x, step_y));
    }

    steps
}

fn resolve_scroll_to_partial(
    destination: ScrollToDestination,
    doc_height: f64,
    doc_width: f64,
) -> PartialVector {
    match destination {
        ScrollToDestination::Top => PartialVector {
            x: None,
            y: Some(0.0),
        },
        ScrollToDestination::Bottom => PartialVector {
            x: None,
            y: Some(doc_height),
        },
        ScrollToDestination::Left => PartialVector {
            x: Some(0.0),
            y: None,
        },
        ScrollToDestination::Right => PartialVector {
            x: Some(doc_width),
            y: None,
        },
        ScrollToDestination::Position(v) => v,
    }
}

fn compute_scroll_delta_to_target(to: PartialVector, scroll_top: f64, scroll_left: f64) -> PartialVector {
    PartialVector {
        x: to.x.map(|x| x - scroll_left),
        y: to.y.map(|y| y - scroll_top),
    }
}

fn should_apply_misclick(rate: f64, roll: f64) -> bool {
    let clamped_rate = clamp(rate, 0.0, 1.0);
    roll < clamped_rate
}

fn compute_misclick_destination(bbox: BoundingBox, distance: f64, angle: f64) -> Vector {
    let center = Vector {
        x: bbox.x + bbox.width / 2.0,
        y: bbox.y + bbox.height / 2.0,
    };
    let distance = distance.max(0.0);
    let radius_x = bbox.width / 2.0 + distance;
    let radius_y = bbox.height / 2.0 + distance;

    Vector {
        x: center.x + radius_x * angle.cos(),
        y: center.y + radius_y * angle.sin(),
    }
}

/// Target accepted by TS-like cursor methods.
pub enum CursorTarget<'a> {
    Selector(&'a str),
    Element(&'a ElementHandle),
}

/// A human-like mouse cursor that generates realistic movements on a playwright Page.
pub struct GhostCursor {
    page: Page,
    location: Vector,
    moving: bool,
    mouse_helper_installed: bool,
    default_options: DefaultOptions,
}

impl GhostCursor {
    /// Create a new GhostCursor starting at the origin (0, 0).
    pub fn new(page: Page) -> Self {
        Self::new_with_options(page, GhostCursorOptions::default())
    }

    /// Create a new GhostCursor starting at a custom position.
    pub fn new_with_start(page: Page, start: Vector) -> Self {
        Self::new_with_options(page, GhostCursorOptions {
            start: Some(start),
            ..Default::default()
        })
    }

    /// Create a new GhostCursor with explicit options.
    pub fn new_with_options(page: Page, options: GhostCursorOptions) -> Self {
        Self {
            page,
            location: options.start.unwrap_or(ORIGIN),
            moving: false,
            mouse_helper_installed: false,
            default_options: options.default_options,
        }
    }

    /// Create and initialize a cursor with startup behaviors.
    ///
    /// - `visible=true`: installs mouse helper.
    /// - `perform_random_moves=true`: performs at least one random move and leaves random mode enabled.
    pub async fn new_with_options_async(
        page: Page,
        options: GhostCursorOptions,
    ) -> Result<Self, Arc<playwright::Error>> {
        let visible = options.visible;
        let perform_random_moves = options.perform_random_moves;
        let random_options = options.default_options.random_move_options.clone();

        let mut cursor = Self::new_with_options(page, options);

        if visible {
            cursor.install_mouse_helper().await?;
        }

        if perform_random_moves {
            cursor.toggle_random_move(true);
            cursor.random_move(random_options.as_ref()).await?;
        }

        Ok(cursor)
    }

    /// Read current cursor-level default options.
    pub fn default_options(&self) -> &DefaultOptions {
        &self.default_options
    }

    /// Replace cursor-level default options.
    pub fn set_default_options(&mut self, options: DefaultOptions) {
        self.default_options = options;
    }

    /// Get the current cursor location.
    pub fn location(&self) -> Vector {
        self.location
    }

    /// TS-like alias for location().
    pub fn get_location(&self) -> Vector {
        self.location()
    }

    /// Toggle random movement mode on/off.
    pub fn toggle_random_move(&mut self, random: bool) {
        self.moving = !random;
    }

        /// Install a visible mouse helper for debugging interactions.
        pub async fn install_mouse_helper(&mut self) -> Result<(), Arc<playwright::Error>> {
                let _: bool = self
                        .page
                        .evaluate(
                                r#"() => {
    const w = window;
    if (w.__ghostCursorRemoveMouseHelper) {
        return true;
    }

    const box = document.createElement('p-mouse-pointer');
    const styleElement = document.createElement('style');
    styleElement.setAttribute('data-ghost-cursor-helper', 'true');
    styleElement.textContent = `
        p-mouse-pointer {
            pointer-events: none;
            position: absolute;
            top: 0;
            left: 0;
            z-index: 10000;
            width: 20px;
            height: 20px;
            margin: -10px 0 0 -10px;
            border: 1px solid white;
            border-radius: 10px;
            background: rgba(0,0,0,.4);
            box-sizing: border-box;
            transition: background .2s, border-radius .2s, border-color .2s;
        }
        p-mouse-pointer.button-1 { background: rgba(0,0,0,0.9); transition: none; }
        p-mouse-pointer.button-2 { border-color: rgba(0,0,255,0.9); transition: none; }
        p-mouse-pointer.button-3 { border-radius: 4px; transition: none; }
        p-mouse-pointer.button-4 { border-color: rgba(255,0,0,0.9); transition: none; }
        p-mouse-pointer.button-5 { border-color: rgba(0,255,0,0.9); transition: none; }
        p-mouse-pointer-hide { display: none; }
    `;

    const updateButtons = (buttons) => {
        for (let i = 0; i < 5; i++) {
            box.classList.toggle(`button-${i + 1}`, Boolean(buttons & (1 << i)));
        }
    };

    const onMouseMove = (event) => {
        box.style.left = `${event.pageX}px`;
        box.style.top = `${event.pageY}px`;
        box.classList.remove('p-mouse-pointer-hide');
        updateButtons(event.buttons);
    };
    const onMouseDown = (event) => {
        updateButtons(event.buttons);
        box.classList.add(`button-${event.which}`);
        box.classList.remove('p-mouse-pointer-hide');
    };
    const onMouseUp = (event) => {
        updateButtons(event.buttons);
        box.classList.remove(`button-${event.which}`);
        box.classList.remove('p-mouse-pointer-hide');
    };
    const onMouseLeave = (event) => {
        updateButtons(event.buttons);
        box.classList.add('p-mouse-pointer-hide');
    };
    const onMouseEnter = (event) => {
        updateButtons(event.buttons);
        box.classList.remove('p-mouse-pointer-hide');
    };

    document.head.appendChild(styleElement);
    document.body.appendChild(box);
    document.addEventListener('mousemove', onMouseMove, true);
    document.addEventListener('mousedown', onMouseDown, true);
    document.addEventListener('mouseup', onMouseUp, true);
    document.addEventListener('mouseleave', onMouseLeave, true);
    document.addEventListener('mouseenter', onMouseEnter, true);

    w.__ghostCursorRemoveMouseHelper = () => {
        document.removeEventListener('mousemove', onMouseMove, true);
        document.removeEventListener('mousedown', onMouseDown, true);
        document.removeEventListener('mouseup', onMouseUp, true);
        document.removeEventListener('mouseleave', onMouseLeave, true);
        document.removeEventListener('mouseenter', onMouseEnter, true);
        box.remove();
        styleElement.remove();
        delete w.__ghostCursorRemoveMouseHelper;
    };

    return true;
}"#,
                                (),
                        )
                        .await?;

                self.mouse_helper_installed = true;
                Ok(())
        }

        /// Remove the visible mouse helper if present.
        pub async fn remove_mouse_helper(&mut self) -> Result<(), Arc<playwright::Error>> {
                let _: bool = self
                        .page
                        .evaluate(
                                r#"() => {
    const w = window;
    if (w.__ghostCursorRemoveMouseHelper) {
        w.__ghostCursorRemoveMouseHelper();
    }
    return true;
}"#,
                                (),
                        )
                        .await?;

                self.mouse_helper_installed = false;
                Ok(())
        }

        /// Whether helper is considered installed in this cursor instance.
        pub fn is_mouse_helper_installed(&self) -> bool {
                self.mouse_helper_installed
        }

    /// Perform a single random movement step.
    pub async fn random_move(&mut self, options: Option<&RandomMoveOptions>) -> Result<(), Arc<playwright::Error>> {
        let resolved = self.resolve_random_move_options(options);

        if self.moving {
            return Ok(());
        }

        let target = self.get_random_page_point().await?;
        let path_options = PathOptions {
            spread_override: None,
            move_speed: resolved.move_speed,
            use_timestamps: false,
        };
        self.dispatch_path(target, &path_options).await?;

        self.wait_after_move(resolved.move_delay, resolved.randomize_move_delay).await;
        Ok(())
    }

    /// Perform random movements repeatedly until disabled or until optional iteration limit.
    pub async fn random_move_loop(
        &mut self,
        options: Option<&RandomMoveOptions>,
        max_iterations: Option<usize>,
    ) -> Result<(), Arc<playwright::Error>> {
        let limit = max_iterations.unwrap_or(usize::MAX);
        for _ in 0..limit {
            if self.moving {
                break;
            }
            self.random_move(options).await?;
        }
        Ok(())
    }

    /// Keep performing random moves until random mode is disabled or iteration cap is reached.
    pub async fn random_move_until_stopped(
        &mut self,
        options: Option<&RandomMoveOptions>,
        max_iterations: Option<usize>,
    ) -> Result<(), Arc<playwright::Error>> {
        let limit = max_iterations.unwrap_or(usize::MAX);
        let mut iterations = 0usize;

        while !self.moving && iterations < limit {
            self.random_move(options).await?;
            iterations += 1;
        }

        Ok(())
    }

    /// Resolve a selector to an element, with optional wait timeout.
    pub async fn get_element(
        &self,
        selector: &str,
        options: Option<&GetElementOptions>,
    ) -> Result<ElementHandle, Arc<playwright::Error>> {
        let resolved = self.resolve_get_element_options(options);
        let selector_candidates = Self::selector_candidates(selector);
        let is_xpath = Self::is_xpath_selector(selector);

        if let Some(timeout) = resolved.wait_for_selector {
            for candidate in &selector_candidates {
                match self
                    .page
                    .wait_for_selector_builder(candidate)
                    .timeout(timeout)
                    .wait_for_selector()
                    .await
                {
                    Ok(Some(element)) => return Ok(element),
                    Ok(None) => {}
                    Err(err) => {
                        if !is_xpath {
                            return Err(err);
                        }
                    }
                }
            }
        }

        for candidate in &selector_candidates {
            match self.page.query_selector(candidate).await {
                Ok(Some(element)) => return Ok(element),
                Ok(None) => {}
                Err(err) => {
                    if !is_xpath {
                        return Err(err);
                    }
                }
            }
        }

        Err(playwright::Error::ObjectNotFound.into())
    }

    /// Move the mouse to a selector or element, TS `move(...)` style.
    pub async fn move_target(
        &mut self,
        target: CursorTarget<'_>,
        options: Option<&MoveOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        let resolved = self.resolve_move_options(options, self.default_options.move_options.as_ref());

        match target {
            CursorTarget::Selector(selector) => {
                let get_opts = GetElementOptions {
                    wait_for_selector: resolved.wait_for_selector,
                };
                let element = self.get_element(selector, Some(&get_opts)).await?;
                self.move_to_element(&element, Some(&resolved)).await
            }
            CursorTarget::Element(element) => self.move_to_element(element, Some(&resolved)).await,
        }
    }

    /// TS-like alias for move_target.
    pub async fn r#move(
        &mut self,
        target: CursorTarget<'_>,
        options: Option<&MoveOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        self.move_target(target, options).await
    }

    /// Move the mouse by a delta from the current location.
    pub async fn move_by(&mut self, delta: Vector, options: Option<&MoveOptions>) -> Result<(), Arc<playwright::Error>> {
        let destination = Vector {
            x: self.location.x + delta.x,
            y: self.location.y + delta.y,
        };
        self.move_to(destination, options).await
    }

    /// Move the mouse to a specific destination point using a human-like path.
    pub async fn move_to(&mut self, dest: Vector, options: Option<&MoveOptions>) -> Result<(), Arc<playwright::Error>> {
        let resolved = self.resolve_move_options(options, self.default_options.move_to_options.as_ref());
        let was_random = !self.moving;
        self.toggle_random_move(false);

        let result = async {
            let path_options = PathOptions {
                spread_override: resolved.spread_override,
                move_speed: resolved.move_speed,
                use_timestamps: false,
            };

            self.dispatch_path(dest, &path_options).await?;
            self.wait_after_move(resolved.move_delay, resolved.randomize_move_delay).await;
            Ok(())
        }
        .await;

        self.toggle_random_move(was_random);
        result
    }

    /// Move the mouse to an element by getting its bounding box and moving to a random point within.
    pub async fn move_to_element(
        &mut self,
        element: &ElementHandle,
        options: Option<&MoveOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        let resolved = self.resolve_move_options(options, self.default_options.move_options.as_ref());
        let was_random = !self.moving;
        self.toggle_random_move(false);
        let scroll_options = self.scroll_into_view_from_move_options(&resolved);
        let result = async {
            self.scroll_into_view_element_impl(element, &scroll_options).await?;
            self.move_to_element_impl(element, &resolved, true, true).await
        }
        .await;
        self.toggle_random_move(was_random);
        result
    }

    async fn move_to_element_impl(
        &mut self,
        element: &ElementHandle,
        options: &MoveOptions,
        apply_delay: bool,
        require_intersection: bool,
    ) -> Result<(), Arc<playwright::Error>> {
        if !Self::is_element_attached(element).await {
            return Err(playwright::Error::ObjectNotFound.into());
        }

        let max_tries = options.max_tries.unwrap_or(3);

        for attempt in 0..=max_tries {
            if !Self::is_element_attached(element).await {
                return Err(playwright::Error::ObjectNotFound.into());
            }

            let bbox = match Self::element_bounding_box(element).await {
                Ok(bbox) => bbox,
                Err(err) => {
                    if !Self::is_element_attached(element).await {
                        return Err(playwright::Error::ObjectNotFound.into());
                    }
                    if attempt == max_tries {
                        return Err(err);
                    }
                    continue;
                }
            };
            let destination = match options.destination {
                Some(relative) => Vector {
                    x: bbox.x + relative.x,
                    y: bbox.y + relative.y,
                },
                None => get_random_box_point(&bbox, options.padding_percentage),
            };

            let path_options = PathOptions {
                spread_override: options.spread_override,
                move_speed: options.move_speed,
                use_timestamps: false,
            };

            if should_overshoot(self.location, destination, options.overshoot_threshold) {
                let overshot = overshoot(destination, OVERSHOOT_RADIUS);
                self.dispatch_path_target(PathTarget::Point(overshot), &path_options).await?;

                let to_target = PathOptions {
                    spread_override: Some(OVERSHOOT_SPREAD),
                    move_speed: options.move_speed,
                    use_timestamps: false,
                };
                let destination_box = BoundingBox {
                    x: destination.x,
                    y: destination.y,
                    width: bbox.width,
                    height: bbox.height,
                };
                self.dispatch_path_target(PathTarget::Box(destination_box), &to_target).await?;
            } else {
                self.dispatch_path_target(PathTarget::Point(destination), &path_options).await?;
            }

            if !require_intersection {
                if apply_delay {
                    self.wait_after_move(options.move_delay, options.randomize_move_delay).await;
                }
                return Ok(());
            }

            if !Self::is_element_attached(element).await {
                return Err(playwright::Error::ObjectNotFound.into());
            }

            let new_bbox = Self::element_bounding_box(element).await?;
            if intersects_element(self.location, &new_bbox) {
                if apply_delay {
                    self.wait_after_move(options.move_delay, options.randomize_move_delay).await;
                }
                return Ok(());
            }

            if attempt == max_tries {
                return Err(playwright::Error::InvalidParams.into());
            }
        }

        Err(playwright::Error::InvalidParams.into())
    }

    /// TS-style click entrypoint: optional target + optional click options.
    pub async fn click_target(
        &mut self,
        target: Option<CursorTarget<'_>>,
        options: Option<&ClickOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        let resolved = self.resolve_click_options(options);
        let was_random = !self.moving;
        self.toggle_random_move(false);

        let result = async {
            if let Some(target) = target {
                let move_options = MoveOptions {
                    padding_percentage: resolved.padding_percentage,
                    destination: resolved.destination,
                    wait_for_selector: resolved.wait_for_selector,
                    scroll_speed: resolved.scroll_speed,
                    scroll_delay: resolved.scroll_delay,
                    in_viewport_margin: resolved.in_viewport_margin,
                    move_speed: resolved.move_speed,
                    move_delay: 0.0,
                    randomize_move_delay: false,
                    max_tries: resolved.max_tries,
                    spread_override: resolved.spread_override,
                    overshoot_threshold: resolved.overshoot_threshold,
                };

                match target {
                    CursorTarget::Selector(selector) => {
                        let get_opts = GetElementOptions {
                            wait_for_selector: resolved.wait_for_selector,
                        };
                        let element = self.get_element(selector, Some(&get_opts)).await?;

                        let mut move_options = move_options.clone();
                        let mut require_intersection = true;
                        if move_options.destination.is_none() {
                            if let Some(relative_destination) = self
                                .resolve_click_misclick_destination(&element, &resolved)
                                .await?
                            {
                                move_options.destination = Some(relative_destination);
                                require_intersection = false;
                            }
                        }

                        let scroll_opts = self.scroll_into_view_from_click_options(&resolved);
                        self.scroll_into_view_element_impl(&element, &scroll_opts).await?;
                        self.move_to_element_impl(&element, &move_options, false, require_intersection)
                            .await?;
                    }
                    CursorTarget::Element(element) => {
                        let mut move_options = move_options.clone();
                        let mut require_intersection = true;
                        if move_options.destination.is_none() {
                            if let Some(relative_destination) = self
                                .resolve_click_misclick_destination(element, &resolved)
                                .await?
                            {
                                move_options.destination = Some(relative_destination);
                                require_intersection = false;
                            }
                        }

                        let scroll_opts = self.scroll_into_view_from_click_options(&resolved);
                        self.scroll_into_view_element_impl(element, &scroll_opts).await?;
                        self.move_to_element_impl(element, &move_options, false, require_intersection)
                            .await?;
                    }
                }
            }

            self.click(Some(&resolved)).await
        }
        .await;

        self.toggle_random_move(was_random);
        result
    }

    /// Move the mouse to an element, then click.
    pub async fn click_element(
        &mut self,
        element: &ElementHandle,
        options: Option<&ClickOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        self.click_target(Some(CursorTarget::Element(element)), options).await
    }

    /// Move the mouse to a selector, then click.
    pub async fn click_selector(
        &mut self,
        selector: &str,
        options: Option<&ClickOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        self.click_target(Some(CursorTarget::Selector(selector)), options).await
    }

    /// Click at the current cursor position.
    pub async fn click(&self, options: Option<&ClickOptions>) -> Result<(), Arc<playwright::Error>> {
        let resolved = self.resolve_click_options(options);

        if resolved.hesitate > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(resolved.hesitate as u64)).await;
        }

        self.page.mouse.down(resolved.button, resolved.click_count).await?;

        if resolved.wait_for_click > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(resolved.wait_for_click as u64))
                .await;
        }

        self.page.mouse.up(resolved.button, resolved.click_count).await?;

        self.wait_after_move(resolved.move_delay, resolved.randomize_move_delay).await;

        Ok(())
    }

    /// Ensure target is in viewport and optionally adjust with margin.
    pub async fn scroll_into_view(
        &self,
        target: CursorTarget<'_>,
        options: Option<&ScrollIntoViewOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        let resolved = self.resolve_scroll_into_view_options(options);

        match target {
            CursorTarget::Selector(selector) => {
                let get_opts = GetElementOptions {
                    wait_for_selector: resolved.wait_for_selector,
                };
                let element = self.get_element(selector, Some(&get_opts)).await?;
                self.scroll_into_view_element_impl(&element, &resolved).await
            }
            CursorTarget::Element(element) => self.scroll_into_view_element_impl(element, &resolved).await,
        }
    }

    /// Convenience wrapper to scroll selector into view.
    pub async fn scroll_into_view_selector(
        &self,
        selector: &str,
        options: Option<&ScrollIntoViewOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        self.scroll_into_view(CursorTarget::Selector(selector), options).await
    }

    /// Convenience wrapper to scroll element into view.
    pub async fn scroll_into_view_element(
        &self,
        element: &ElementHandle,
        options: Option<&ScrollIntoViewOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        self.scroll_into_view(CursorTarget::Element(element), options).await
    }

    async fn scroll_into_view_element_impl(
        &self,
        element: &ElementHandle,
        resolved: &ScrollIntoViewOptions,
    ) -> Result<(), Arc<playwright::Error>> {
        let [before_x, before_y] = self.get_scroll_position().await?;

        // playwright-rust driver expects a concrete float timeout value.
        element
            .scroll_into_view_if_needed(Some(ACTION_TIMEOUT_MS))
            .await?;
        let [after_x, after_y] = self.get_scroll_position().await?;
        let mut did_scroll = (before_x - after_x).abs() > f64::EPSILON
            || (before_y - after_y).abs() > f64::EPSILON;

        if resolved.in_viewport_margin > 0.0 {
            let bbox = Self::element_bounding_box(element).await?;
            let [viewport_width, viewport_height]: [f64; 2] = self
                .page
                .evaluate(
                    "() => [document.body.clientWidth, document.body.clientHeight]",
                    (),
                )
                .await?;

            let margin = resolved.in_viewport_margin;
            let mut delta_x = 0.0;
            let mut delta_y = 0.0;

            if bbox.y < margin {
                delta_y = bbox.y - margin;
            } else if bbox.y + bbox.height > viewport_height - margin {
                delta_y = bbox.y + bbox.height - (viewport_height - margin);
            }

            if bbox.x < margin {
                delta_x = bbox.x - margin;
            } else if bbox.x + bbox.width > viewport_width - margin {
                delta_x = bbox.x + bbox.width - (viewport_width - margin);
            }

            if delta_x.abs() > f64::EPSILON || delta_y.abs() > f64::EPSILON {
                let scroll_opts = ScrollOptions {
                    scroll_speed: resolved.scroll_speed,
                    scroll_delay: resolved.scroll_delay,
                };
                self.scroll(
                    PartialVector {
                        x: Some(delta_x),
                        y: Some(delta_y),
                    },
                    Some(&scroll_opts),
                )
                .await?;
                did_scroll = true;
            }
        }

        if did_scroll && resolved.scroll_delay > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(resolved.scroll_delay as u64)).await;
        }

        Ok(())
    }

    /// Scroll page by a partial vector.
    pub async fn scroll(
        &self,
        delta: PartialVector,
        options: Option<&ScrollOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        let resolved = self.resolve_scroll_options(options);
        let steps = compute_scroll_steps(
            delta.x.unwrap_or(0.0),
            delta.y.unwrap_or(0.0),
            resolved.scroll_speed,
        );

        for (step_x, step_y) in steps {

            let _: [f64; 2] = self
                .page
                .evaluate(
                    "([dx, dy]) => { window.scrollBy(dx, dy); return [window.scrollX, window.scrollY]; }",
                    [step_x, step_y],
                )
                .await?;
        }

        if resolved.scroll_delay > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(resolved.scroll_delay as u64)).await;
        }

        Ok(())
    }

    /// Scroll to a destination point or edge.
    pub async fn scroll_to(
        &self,
        destination: ScrollToDestination,
        options: Option<&ScrollOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        let [doc_height, doc_width, scroll_top, scroll_left]: [f64; 4] = self
            .page
            .evaluate(
                "() => [document.body.scrollHeight, document.body.scrollWidth, window.scrollY, window.scrollX]",
                (),
            )
            .await?;

        let to = resolve_scroll_to_partial(destination, doc_height, doc_width);
        let delta = compute_scroll_delta_to_target(to, scroll_top, scroll_left);

        self.scroll(delta, options).await
    }

    /// Press mouse down at the current position.
    pub async fn mouse_down(&self) -> Result<(), Arc<playwright::Error>> {
        self.page.mouse.down(None, None).await
    }

    /// Press mouse down at the current position with explicit button options.
    pub async fn mouse_down_with_options(
        &self,
        options: Option<&MouseButtonOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        let resolved = options.cloned().unwrap_or_default();
        self.page
            .mouse
            .down(resolved.button, resolved.click_count)
            .await
    }

    /// Release mouse at the current position.
    pub async fn mouse_up(&self) -> Result<(), Arc<playwright::Error>> {
        self.page.mouse.up(None, None).await
    }

    /// Release mouse at the current position with explicit button options.
    pub async fn mouse_up_with_options(
        &self,
        options: Option<&MouseButtonOptions>,
    ) -> Result<(), Arc<playwright::Error>> {
        let resolved = options.cloned().unwrap_or_default();
        self.page.mouse.up(resolved.button, resolved.click_count).await
    }

    fn resolve_move_options(&self, options: Option<&MoveOptions>, defaults: Option<&MoveOptions>) -> MoveOptions {
        options
            .cloned()
            .or_else(|| defaults.cloned())
            .unwrap_or_default()
    }

    fn resolve_click_options(&self, options: Option<&ClickOptions>) -> ClickOptions {
        options
            .cloned()
            .or_else(|| self.default_options.click_options.clone())
            .unwrap_or_default()
    }

    fn resolve_get_element_options(&self, options: Option<&GetElementOptions>) -> GetElementOptions {
        options
            .cloned()
            .or_else(|| self.default_options.get_element_options.clone())
            .unwrap_or_default()
    }

    fn resolve_random_move_options(&self, options: Option<&RandomMoveOptions>) -> RandomMoveOptions {
        options
            .cloned()
            .or_else(|| self.default_options.random_move_options.clone())
            .unwrap_or_default()
    }

    fn resolve_scroll_options(&self, options: Option<&ScrollOptions>) -> ScrollOptions {
        options
            .cloned()
            .or_else(|| self.default_options.scroll_options.clone())
            .unwrap_or_default()
    }

    fn resolve_scroll_into_view_options(
        &self,
        options: Option<&ScrollIntoViewOptions>,
    ) -> ScrollIntoViewOptions {
        options
            .cloned()
            .or_else(|| self.default_options.scroll_into_view_options.clone())
            .unwrap_or_default()
    }

    fn scroll_into_view_from_move_options(&self, options: &MoveOptions) -> ScrollIntoViewOptions {
        let mut merged = self.resolve_scroll_into_view_options(None);
        if let Some(v) = options.scroll_speed {
            merged.scroll_speed = v;
        }
        if let Some(v) = options.scroll_delay {
            merged.scroll_delay = v;
        }
        if let Some(v) = options.in_viewport_margin {
            merged.in_viewport_margin = v;
        }
        if let Some(v) = options.wait_for_selector {
            merged.wait_for_selector = Some(v);
        }
        merged
    }

    fn scroll_into_view_from_click_options(&self, options: &ClickOptions) -> ScrollIntoViewOptions {
        let mut merged = self.resolve_scroll_into_view_options(None);
        if let Some(v) = options.scroll_speed {
            merged.scroll_speed = v;
        }
        if let Some(v) = options.scroll_delay {
            merged.scroll_delay = v;
        }
        if let Some(v) = options.in_viewport_margin {
            merged.in_viewport_margin = v;
        }
        if let Some(v) = options.wait_for_selector {
            merged.wait_for_selector = Some(v);
        }
        merged
    }

    fn normalize_selector(selector: &str) -> String {
        if selector.starts_with("//") || selector.starts_with("(//") {
            format!("xpath/.{}", selector)
        } else {
            selector.to_string()
        }
    }

    fn is_xpath_selector(selector: &str) -> bool {
        selector.starts_with("//") || selector.starts_with("(//")
    }

    fn selector_candidates(selector: &str) -> Vec<String> {
        if Self::is_xpath_selector(selector) {
            vec![
                Self::normalize_selector(selector),
                format!("xpath={}", selector),
                selector.to_string(),
            ]
        } else {
            vec![selector.to_string()]
        }
    }

    async fn resolve_click_misclick_destination(
        &self,
        element: &ElementHandle,
        options: &ClickOptions,
    ) -> Result<Option<Vector>, Arc<playwright::Error>> {
        if !should_apply_misclick(options.misclick_rate, rand::random::<f64>()) {
            return Ok(None);
        }

        let bbox = Self::element_bounding_box(element).await?;
        let angle = rand::thread_rng().gen_range(0.0..(std::f64::consts::PI * 2.0));
        let absolute_target = compute_misclick_destination(bbox, options.misclick_distance, angle);
        Ok(Some(Vector {
            x: absolute_target.x - bbox.x,
            y: absolute_target.y - bbox.y,
        }))
    }

    async fn is_element_attached(element: &ElementHandle) -> bool {
        matches!(element.owner_frame().await, Ok(Some(_)))
    }

    /// Internal: generate and dispatch a human-like path to the target.
    async fn dispatch_path(
        &mut self,
        target: Vector,
        options: &PathOptions,
    ) -> Result<(), Arc<playwright::Error>> {
        self.dispatch_path_target(PathTarget::Point(target), options).await
    }

    /// Internal: generate and dispatch a human-like path to the target type.
    async fn dispatch_path_target(
        &mut self,
        target: PathTarget,
        options: &PathOptions,
    ) -> Result<(), Arc<playwright::Error>> {
        let points = path(self.location, target, options);

        for p in &points {
            let v = p.vector();
            self.page.mouse.r#move(v.x, v.y, None).await?;
            self.location = v;
        }

        Ok(())
    }

    async fn wait_after_move(&self, move_delay: f64, randomize_move_delay: bool) {
        let delay = if randomize_move_delay {
            move_delay * rand::random::<f64>()
        } else {
            move_delay
        };

        if delay > 0.0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
        }
    }

    async fn element_bounding_box(element: &ElementHandle) -> Result<BoundingBox, Arc<playwright::Error>> {
        let float_rect = element.bounding_box().await?;
        match float_rect {
            Some(r) => Ok(BoundingBox::new(r.x, r.y, r.width, r.height)),
            None => Err(playwright::Error::InvalidParams.into()),
        }
    }

    async fn get_random_page_point(&self) -> Result<Vector, Arc<playwright::Error>> {
        let [width, height]: [f64; 2] = self
            .page
            .evaluate(
                "() => [document.body.clientWidth || window.innerWidth, document.body.clientHeight || window.innerHeight]",
                (),
            )
            .await?;

        let mut rng = rand::thread_rng();
        Ok(Vector {
            x: if width > 0.0 {
                rng.gen_range(0.0..width)
            } else {
                0.0
            },
            y: if height > 0.0 {
                rng.gen_range(0.0..height)
            } else {
                0.0
            },
        })
    }

    async fn get_scroll_position(&self) -> Result<[f64; 2], Arc<playwright::Error>> {
        self.page
            .evaluate("() => [window.scrollX, window.scrollY]", ())
            .await
    }
}

/// Deprecated helper kept for API familiarity with the original project.
#[deprecated(note = "Prefer `GhostCursor::new_with_options` instead")]
pub fn create_cursor(page: Page, options: GhostCursorOptions) -> GhostCursor {
    GhostCursor::new_with_options(page, options)
}

#[cfg(test)]
mod tests {
    use super::{
        compute_misclick_destination, compute_scroll_delta_to_target, compute_scroll_steps,
        resolve_scroll_to_partial, should_apply_misclick, GhostCursor,
    };
    use crate::types::{BoundingBox, PartialVector, ScrollToDestination};

    #[test]
    fn test_normalize_selector_css() {
        assert_eq!(GhostCursor::normalize_selector("#login"), "#login");
    }

    #[test]
    fn test_normalize_selector_xpath_slash_prefix() {
        assert_eq!(GhostCursor::normalize_selector("//div[@id='x']"), "xpath/.//div[@id='x']");
    }

    #[test]
    fn test_normalize_selector_xpath_group_prefix() {
        assert_eq!(GhostCursor::normalize_selector("(//button)[1]"), "xpath/.(//button)[1]");
    }

    #[test]
    fn test_selector_candidates_xpath() {
        let candidates = GhostCursor::selector_candidates("//button[@id='x']");
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0], "xpath/.//button[@id='x']");
        assert_eq!(candidates[1], "xpath=//button[@id='x']");
        assert_eq!(candidates[2], "//button[@id='x']");
    }

    #[test]
    fn test_should_apply_misclick_clamps_rate() {
        assert!(should_apply_misclick(2.0, 0.5));
        assert!(!should_apply_misclick(-1.0, 0.0));
        assert!(!should_apply_misclick(0.1, 0.5));
    }

    #[test]
    fn test_compute_misclick_destination_outside_target_bbox() {
        let bbox = BoundingBox {
            x: 100.0,
            y: 100.0,
            width: 50.0,
            height: 30.0,
        };
        let point = compute_misclick_destination(bbox, 20.0, 0.0);
        assert!(point.x > bbox.x + bbox.width);
    }

    #[test]
    fn test_compute_scroll_steps_zero_delta() {
        let steps = compute_scroll_steps(0.0, 0.0, 50.0);
        assert!(steps.is_empty());
    }

    #[test]
    fn test_compute_scroll_steps_preserves_total_delta() {
        let steps = compute_scroll_steps(225.0, 50.0, 40.0);
        let total_x: f64 = steps.iter().map(|(x, _)| *x).sum();
        let total_y: f64 = steps.iter().map(|(_, y)| *y).sum();

        assert!((total_x - 225.0).abs() < 1e-6);
        assert!((total_y - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_scroll_steps_preserves_sign() {
        let steps = compute_scroll_steps(-120.0, 60.0, 90.0);
        let total_x: f64 = steps.iter().map(|(x, _)| *x).sum();
        let total_y: f64 = steps.iter().map(|(_, y)| *y).sum();

        assert!(total_x < 0.0);
        assert!(total_y > 0.0);
    }

    #[test]
    fn test_compute_scroll_steps_high_speed_collapses_steps() {
        let steps = compute_scroll_steps(120.0, 40.0, 100.0);
        assert_eq!(steps.len(), 1);
        assert!((steps[0].0 - 120.0).abs() < 1e-6);
        assert!((steps[0].1 - 40.0).abs() < 1e-6);
    }

    #[test]
    fn test_resolve_scroll_to_partial_edges() {
        let top = resolve_scroll_to_partial(ScrollToDestination::Top, 4000.0, 2200.0);
        assert_eq!(top.x, None);
        assert_eq!(top.y, Some(0.0));

        let right = resolve_scroll_to_partial(ScrollToDestination::Right, 4000.0, 2200.0);
        assert_eq!(right.x, Some(2200.0));
        assert_eq!(right.y, None);
    }

    #[test]
    fn test_compute_scroll_delta_to_target() {
        let target = PartialVector {
            x: Some(800.0),
            y: Some(300.0),
        };
        let delta = compute_scroll_delta_to_target(target, 100.0, 200.0);

        assert_eq!(delta.x, Some(600.0));
        assert_eq!(delta.y, Some(200.0));
    }

    #[test]
    fn test_resolve_scroll_to_partial_custom_position() {
        let custom = PartialVector {
            x: Some(123.0),
            y: None,
        };
        let resolved = resolve_scroll_to_partial(
            ScrollToDestination::Position(custom),
            4000.0,
            2200.0,
        );

        assert_eq!(resolved.x, Some(123.0));
        assert_eq!(resolved.y, None);
    }

    #[test]
    fn test_compute_scroll_delta_to_target_partial_none() {
        let delta = compute_scroll_delta_to_target(
            PartialVector {
                x: None,
                y: Some(300.0),
            },
            100.0,
            200.0,
        );

        assert_eq!(delta.x, None);
        assert_eq!(delta.y, Some(200.0));
    }
}
