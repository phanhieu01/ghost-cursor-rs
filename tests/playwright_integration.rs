use ghost_cursor::{
    ClickOptions, CursorTarget, GhostCursor, MoveOptions, ScrollOptions, ScrollToDestination,
};
use playwright::Playwright;

#[tokio::test]
#[ignore = "requires installed Playwright browser runtime"]
async fn move_selector_places_cursor_inside_element() {
    let playwright = Playwright::initialize().await.unwrap();
    playwright.install_chromium().unwrap();

    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.unwrap();
    let context = browser.context_builder().build().await.unwrap();
    let page = context.new_page().await.unwrap();

    let html = r#"
<!doctype html>
<html>
  <body style="margin: 0; padding: 0;">
    <button id="target" style="position: absolute; left: 140px; top: 120px; width: 180px; height: 90px;">A</button>
  </body>
</html>
"#;

    page.set_content_builder(html).set_content().await.unwrap();

    let mut cursor = GhostCursor::new(page.clone());
    let mut options = MoveOptions::default();
    options.move_delay = 0.0;
    options.randomize_move_delay = false;
    options.overshoot_threshold = f64::MAX;
    options.scroll_delay = Some(0.0);

    cursor
        .move_target(CursorTarget::Selector("#target"), Some(&options))
        .await
        .unwrap();

    let element = page.query_selector("#target").await.unwrap().unwrap();
    let bbox = element.bounding_box().await.unwrap().unwrap();
    let location = cursor.location();

    assert!(
        location.x >= bbox.x && location.x <= bbox.x + bbox.width,
        "x out of bounds: {} not in [{}, {}]",
        location.x,
        bbox.x,
        bbox.x + bbox.width
    );
    assert!(
        location.y >= bbox.y && location.y <= bbox.y + bbox.height,
        "y out of bounds: {} not in [{}, {}]",
        location.y,
        bbox.y,
        bbox.y + bbox.height
    );

    browser.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires installed Playwright browser runtime"]
async fn click_selector_dispatches_click_handler() {
    let playwright = Playwright::initialize().await.unwrap();
    playwright.install_chromium().unwrap();

    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.unwrap();
    let context = browser.context_builder().build().await.unwrap();
    let page = context.new_page().await.unwrap();

    let html = r#"
<!doctype html>
<html>
  <body>
    <button id="target" onclick="window.__clicks = (window.__clicks || 0) + 1;">Click me</button>
  </body>
</html>
"#;

    page.set_content_builder(html).set_content().await.unwrap();

    let mut cursor = GhostCursor::new(page.clone());
    let mut options = ClickOptions::default();
    options.move_delay = 0.0;
    options.randomize_move_delay = false;
    options.hesitate = 0.0;
    options.wait_for_click = 0.0;
    options.scroll_delay = Some(0.0);
    options.overshoot_threshold = f64::MAX;

    cursor.click_selector("#target", Some(&options)).await.unwrap();

    let clicks: i32 = page.eval("() => window.__clicks || 0").await.unwrap();
    assert_eq!(clicks, 1);

    browser.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires installed Playwright browser runtime"]
async fn move_target_supports_xpath_selector() {
    let playwright = Playwright::initialize().await.unwrap();
    playwright.install_chromium().unwrap();

    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.unwrap();
    let context = browser.context_builder().build().await.unwrap();
    let page = context.new_page().await.unwrap();

    let html = r#"
<!doctype html>
<html>
  <body style="margin: 0; padding: 0;">
    <button id="target" style="position: absolute; left: 140px; top: 120px; width: 180px; height: 90px;">A</button>
  </body>
</html>
"#;

    page.set_content_builder(html).set_content().await.unwrap();

    let mut cursor = GhostCursor::new(page.clone());
    let mut options = MoveOptions::default();
    options.move_delay = 0.0;
    options.randomize_move_delay = false;
    options.overshoot_threshold = f64::MAX;
    options.scroll_delay = Some(0.0);

    cursor
        .move_target(CursorTarget::Selector("//button[@id='target']"), Some(&options))
        .await
        .unwrap();

    let element = page.query_selector("#target").await.unwrap().unwrap();
    let bbox = element.bounding_box().await.unwrap().unwrap();
    let location = cursor.location();

    assert!(location.x >= bbox.x && location.x <= bbox.x + bbox.width);
    assert!(location.y >= bbox.y && location.y <= bbox.y + bbox.height);

    browser.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires installed Playwright browser runtime"]
async fn click_selector_can_intentionally_misclick_outside_target() {
    let playwright = Playwright::initialize().await.unwrap();
    playwright.install_chromium().unwrap();

    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.unwrap();
    let context = browser.context_builder().build().await.unwrap();
    let page = context.new_page().await.unwrap();

    let html = r#"
<!doctype html>
<html>
  <body style="margin: 0; padding: 0;">
    <button id="target" style="position: absolute; left: 140px; top: 120px; width: 180px; height: 90px;">A</button>
    <script>
      window.__targetClicks = 0;
      window.__outsideClicks = 0;
      document.addEventListener('click', (event) => {
        if (event.target && event.target.id === 'target') {
          window.__targetClicks += 1;
        } else {
          window.__outsideClicks += 1;
        }
      });
    </script>
  </body>
</html>
"#;

    page.set_content_builder(html).set_content().await.unwrap();

    let mut cursor = GhostCursor::new(page.clone());
    let mut options = ClickOptions::default();
    options.move_delay = 0.0;
    options.randomize_move_delay = false;
    options.hesitate = 0.0;
    options.wait_for_click = 0.0;
    options.scroll_delay = Some(0.0);
    options.overshoot_threshold = f64::MAX;
    options.misclick_rate = 1.0;
    options.misclick_distance = 50.0;

    cursor.click_selector("#target", Some(&options)).await.unwrap();

    let target_clicks: i32 = page.eval("() => window.__targetClicks || 0").await.unwrap();
    let outside_clicks: i32 = page.eval("() => window.__outsideClicks || 0").await.unwrap();
    assert_eq!(target_clicks, 0);
    assert_eq!(outside_clicks, 1);

    browser.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires installed Playwright browser runtime"]
async fn move_to_element_fails_for_detached_handle() {
    let playwright = Playwright::initialize().await.unwrap();
    playwright.install_chromium().unwrap();

    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.unwrap();
    let context = browser.context_builder().build().await.unwrap();
    let page = context.new_page().await.unwrap();

    let html = r#"
<!doctype html>
<html>
  <body>
    <button id="target">A</button>
  </body>
</html>
"#;

    page.set_content_builder(html).set_content().await.unwrap();
    let element = page.query_selector("#target").await.unwrap().unwrap();
    let _: bool = page
        .evaluate(
            "() => { const el = document.querySelector('#target'); if (el) { el.remove(); } return true; }",
            (),
        )
        .await
        .unwrap();

    let mut cursor = GhostCursor::new(page.clone());
    let mut options = MoveOptions::default();
    options.move_delay = 0.0;
    options.randomize_move_delay = false;
    options.max_tries = Some(3);

    let result = cursor.move_to_element(&element, Some(&options)).await;
    assert!(result.is_err());

    browser.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires installed Playwright browser runtime"]
async fn scroll_to_bottom_changes_scroll_position() {
    let playwright = Playwright::initialize().await.unwrap();
    playwright.install_chromium().unwrap();

    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.unwrap();
    let context = browser.context_builder().build().await.unwrap();
    let page = context.new_page().await.unwrap();

    let html = r#"
<!doctype html>
<html>
  <body style="margin: 0;">
    <div style="height: 4000px; background: linear-gradient(#fff, #ddd);"></div>
  </body>
</html>
"#;

    page.set_content_builder(html).set_content().await.unwrap();

    let cursor = GhostCursor::new(page.clone());
    let options = ScrollOptions {
        scroll_speed: 100.0,
        scroll_delay: 0.0,
    };

    cursor
        .scroll_to(ScrollToDestination::Bottom, Some(&options))
        .await
        .unwrap();

    let scroll_y: f64 = page.eval("() => window.scrollY").await.unwrap();
    assert!(scroll_y > 100.0, "expected scrollY > 100, got {}", scroll_y);

    browser.close().await.unwrap();
}
