# ghost-cursor-rs

Rust port of Xetera/ghost-cursor, adapted for Playwright Rust usage (including Playwright/Juggler environments).

This crate focuses on behavior parity with the original TypeScript implementation where practical:
- human-like paths generated from Bezier curves + Fitts law
- element-target movement with overshoot and retries
- selector and element targets
- click semantics with configurable delays and button options
- optional intentional misclick behavior (configurable rate and distance)
- viewport and scroll helpers for realistic interactions

## Install

This project currently uses a local Playwright Rust dependency:

- `playwright = { path = "../playwright-rust" }`

## Quick Start

```rust
use ghost_cursor::{CursorTarget, GhostCursor};
use playwright::Playwright;

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let playwright = Playwright::initialize().await?;
playwright.install_chromium()?;

let browser = playwright
    .chromium()
    .launcher()
    .headless(true)
    .launch()
    .await?;

let context = browser.context_builder().build().await?;
let page = context.new_page().await?;

page.set_content_builder("<button id='go'>Go</button>")
    .set_content()
    .await?;

let mut cursor = GhostCursor::new(page.clone());
cursor
    .move_target(CursorTarget::Selector("#go"), None)
    .await?;
cursor.click_selector("#go", None).await?;

browser.close().await?;
# Ok(())
# }
```

## API Mapping (TS -> Rust)

| TypeScript ghost-cursor | Rust equivalent |
| --- | --- |
| `new GhostCursor(page, opts?)` | `GhostCursor::new(page)` or `GhostCursor::new_with_options(page, opts)` |
| startup flags (`visible`, random move on startup) | `GhostCursor::new_with_options_async(page, GhostCursorOptions { visible, perform_random_moves, .. })` |
| `cursor.move(selectorOrElement, options?)` | `cursor.move_target(CursorTarget::Selector("..."), Some(&MoveOptions))` or `CursorTarget::Element(&element)` |
| `cursor.moveTo(point, options?)` | `cursor.move_to(Vector { x, y }, Some(&MoveOptions))` |
| `cursor.moveBy(point, options?)` | `cursor.move_by(Vector { x, y }, Some(&MoveOptions))` |
| `cursor.click(selectorOrElement?, options?)` | `cursor.click_target(Some(CursorTarget::Selector("...")), Some(&ClickOptions))` |
| optional intentional outside-target click | `ClickOptions { misclick_rate, misclick_distance, .. }` |
| `cursor.click()` at current location | `cursor.click(Some(&ClickOptions))` |
| `cursor.getElement(selector, options?)` | `cursor.get_element(selector, Some(&GetElementOptions))` |
| `cursor.scrollIntoView(selectorOrElement, options?)` | `cursor.scroll_into_view(CursorTarget::Selector("..."), Some(&ScrollIntoViewOptions))` |
| `cursor.scroll(delta, options?)` | `cursor.scroll(PartialVector { x, y }, Some(&ScrollOptions))` |
| `cursor.scrollTo(destination, options?)` | `cursor.scroll_to(ScrollToDestination::Bottom, Some(&ScrollOptions))` |
| random move loop helpers | `random_move`, `random_move_loop`, `random_move_until_stopped` |
| visible mouse helper | `install_mouse_helper`, `remove_mouse_helper`, `is_mouse_helper_installed` |

## Runtime Tests

Integration tests live in `tests/playwright_integration.rs` and are ignored by default because they require a local browser runtime.

Run them manually:

```bash
cargo test -- --ignored
```

Unit tests still run with a plain `cargo test`.

## TS Parity Workflow

This repo includes a fixture-based parity harness against upstream TypeScript ghost-cursor.

1. Clone upstream reference (already vendored here):
    - `vendor/ghost-cursor-ts`
2. Build upstream JS output:
    - `cd vendor/ghost-cursor-ts`
    - `npx tsc -p tsconfig.build.json`
3. Generate deterministic TS fixtures:
    - `cd ../../`
    - `node scripts/ts-parity/generate-fixtures.js`
4. Run fixture sanity test (default suite):
    - `cargo test`
5. Run parity gate manually:
    - `cargo test --test ts_parity_fixtures -- --ignored`

Fixtures are stored in `fixtures/ts-parity` with upstream commit/version metadata.

## Notes for Playwright/Juggler

This port avoids direct CDP-only dependencies in core cursor behavior.
Where JS/runtime inspection is needed (bounding boxes, viewport, scroll offsets), it uses standard Playwright page/frame evaluation paths that work in Playwright-compatible runtimes.

XPath selector resolution in `get_element` uses a fallback candidate chain for better compatibility across Playwright runtimes.
