# Fill-Pattern Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add support for `fill-pattern` (sprite icon name, string only) in `fill` layer legends, reusing the existing sprite-loading infrastructure.

**Architecture:** `render_fill` in `src/fill.rs` gets a new `sprite_data` parameter and an early branch: if `paint["fill-pattern"]` is present, render a bordered swatch with the sprite icon overlaid as an `<image>`, skipping the existing `fill-color` logic entirely. `render_layer_svg` in `src/lib.rs` threads `sprite_data` into the `"fill"` match arm the same way it already does for `"symbol"`.

**Tech Stack:** Rust, `svg` crate 0.18 (`Image`, `Rectangle`, `Document`), `image` crate 0.25 (`DynamicImage`), existing `common::get_icon_data_url`.

## Global Constraints

- Spec source: `docs/superpowers/specs/2026-08-07-fill-pattern-support-design.md`.
- `fill-pattern` supported as a **string only**; array/expression `fill-pattern` returns `LegendError::InvalidJson("fill-pattern expressions are not yet supported")`.
- No tiling: the sprite icon is scaled to fill the existing 30×20px swatch, not repeated.
- Missing `sprite_data` or icon-not-found for `fill-pattern` must error, matching the existing `icon-image` behavior in `symbol.rs`.
- `cargo fmt` and `cargo clippy` must be clean before the final commit (project requirement, see `CLAUDE.md`).
- Existing `fill-color` behavior (single color and multi-case expressions) must be unchanged when `fill-pattern` is absent.

---

### Task 1: `fill-pattern` rendering in `src/fill.rs`

**Files:**
- Modify: `src/fill.rs:1-100` (imports, `render_fill` signature/body, new `render_fill_pattern` helper)
- Modify: `src/fill.rs:102-177` (existing test module — update call sites, add new tests)

**Interfaces:**
- Consumes: `common::get_icon_data_url(sprites: &[(image::DynamicImage, serde_json::Value)], icon_name: &str) -> Result<String, LegendError>` (existing, unchanged).
- Produces: `pub fn render_fill(layer: &Layer, paint: &serde_json::Map<String, serde_json::Value>, default_width: u32, default_height: u32, has_label: bool, sprite_data: &[(image::DynamicImage, serde_json::Value)]) -> Result<(String, u32, u32), LegendError>` — signature changes from the current 5-argument version by appending `sprite_data`. Task 2 depends on this exact signature.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `src/fill.rs`, after the existing `paint` helper function:

```rust
    fn fake_sprite_with_icon(icon_name: &str) -> Vec<(image::DynamicImage, serde_json::Value)> {
        let img = image::DynamicImage::new_rgba8(4, 4);
        let sprite_json = json!({
            icon_name: { "x": 0, "y": 0, "width": 4, "height": 4 }
        });
        vec![(img, sprite_json)]
    }

    #[test]
    fn test_render_fill_pattern_string() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-pattern": "pattern-icon"}));
        let sprites = fake_sprite_with_icon("pattern-icon");
        let (svg, width, height) = render_fill(&layer, &p, 200, 40, false, &sprites).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 40);
        assert!(svg.contains("<image"));
        assert!(svg.contains("data:image/png;base64,"));
    }

    #[test]
    fn test_render_fill_pattern_with_label() {
        let layer = make_layer_with_label("test", "Wetland");
        let p = paint(json!({"fill-pattern": "pattern-icon"}));
        let sprites = fake_sprite_with_icon("pattern-icon");
        let (svg, _, _) = render_fill(&layer, &p, 200, 40, true, &sprites).unwrap();
        assert!(svg.contains("Wetland"));
    }

    #[test]
    fn test_render_fill_pattern_missing_sprite_data_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-pattern": "pattern-icon"}));
        assert!(render_fill(&layer, &p, 200, 40, false, &[]).is_err());
    }

    #[test]
    fn test_render_fill_pattern_icon_not_found_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-pattern": "does-not-exist"}));
        let sprites = fake_sprite_with_icon("pattern-icon");
        assert!(render_fill(&layer, &p, 200, 40, false, &sprites).is_err());
    }

    #[test]
    fn test_render_fill_pattern_expression_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({
            "fill-pattern": ["match", ["get", "tipo"], "a", "icon-a", "icon-b"]
        }));
        let sprites = fake_sprite_with_icon("icon-a");
        let result = render_fill(&layer, &p, 200, 40, false, &sprites);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not yet supported"));
    }
```

Also update every existing call to `render_fill` in this file's test module to append `&[]` as the sixth argument:
- `test_render_fill_single_color`
- `test_render_fill_match_expression`
- `test_render_fill_multi_case_with_label_increases_height` (both calls: the `_with` and `_without` ones)
- `test_render_fill_with_opacity`
- `test_render_fill_missing_color_returns_err`

Example of the change for `test_render_fill_single_color`:

```rust
        let (svg, width, height) = render_fill(&layer, &p, 200, 40, false, &[]).unwrap();
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib fill:: 2>&1 | head -50`
Expected: compile error — `render_fill` called with 5 arguments but takes 6 (or vice versa, since the signature hasn't changed yet). This confirms the new tests are wired to the not-yet-existing signature.

- [ ] **Step 3: Update imports and `render_fill` signature**

Replace the import block at the top of `src/fill.rs`:

```rust
use crate::{
    common::{
        FONT_SIZE, ICON_HEIGHT, Layer, PADDING, ROW_HEIGHT, extract_color, get_fill_and_opacity,
        get_icon_data_url, parse_expression, render_label, render_separator,
    },
    error::LegendError,
};
use image::DynamicImage;
use serde_json::Value;
use svg::Document;
use svg::node::element::{Image, Rectangle, Text as SvgText};
```

Change the `render_fill` signature (keep the existing doc comment above it):

```rust
pub fn render_fill(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
    sprite_data: &[(DynamicImage, Value)],
) -> Result<(String, u32, u32), LegendError> {
    if let Some(pattern_value) = paint.get("fill-pattern") {
        return render_fill_pattern(
            layer,
            paint,
            pattern_value,
            sprite_data,
            default_width,
            default_height,
            has_label,
        );
    }

    let color_expr = paint.get("fill-color").ok_or_else(|| {
        // ... rest of the existing function body is unchanged from here down ...
```

Do not otherwise change the body of `render_fill` — the `fill-color` path stays exactly as it is today.

- [ ] **Step 4: Add the `render_fill_pattern` helper**

Add this new function immediately after `render_fill` (before the `#[cfg(test)]` module):

```rust
fn render_fill_pattern(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    pattern_value: &serde_json::Value,
    sprite_data: &[(DynamicImage, Value)],
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let icon_name = pattern_value.as_str().ok_or_else(|| {
        LegendError::InvalidJson("fill-pattern expressions are not yet supported".to_string())
    })?;

    if sprite_data.is_empty() {
        return Err(LegendError::InvalidJson(
            "Missing sprite data for 'fill-pattern'".to_string(),
        ));
    }

    let data_url = get_icon_data_url(sprite_data, icon_name)?;
    let fill_outline_color =
        extract_color(paint.get("fill-outline-color")).unwrap_or("black".to_string());

    let mut doc = Document::new()
        .set("width", default_width)
        .set("height", default_height);

    let rect = Rectangle::new()
        .set("x", PADDING)
        .set("y", PADDING)
        .set("width", 30)
        .set("height", ICON_HEIGHT)
        .set("fill", "none")
        .set("stroke", fill_outline_color.as_str())
        .set("stroke-width", "1");
    doc = doc.add(rect);

    let image = Image::new()
        .set("x", PADDING)
        .set("y", PADDING)
        .set("width", 30)
        .set("height", ICON_HEIGHT)
        .set("href", data_url);
    doc = doc.add(image);

    if has_label {
        render_label(layer, &mut doc, None, None, None)?;
    }

    Ok((doc.to_string(), default_width, default_height))
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib fill:: -- --nocapture`
Expected: all tests in `src/fill.rs`, including the 5 new ones, PASS.

- [ ] **Step 6: Commit**

```bash
git add src/fill.rs
git commit -m "feat: support fill-pattern in fill layer legends"
```

---

### Task 2: Thread `sprite_data` through `render_layer_svg` in `src/lib.rs`

**Files:**
- Modify: `src/lib.rs:254-256` (the `"fill"` arm of the inner `match` inside `render_layer_svg`)
- Modify: `src/lib.rs:286-343` (test module — add one integration test)

**Interfaces:**
- Consumes: `fill::render_fill(layer, paint, def_w, def_h, render_label, sprite_data)` (from Task 1).
- Produces: no new public interface — `render_layer_svg`'s own signature is unchanged (it already takes `sprite_data: &[(DynamicImage, Value)]`).

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `src/lib.rs`, after `fill_layer`:

```rust
    fn fake_sprite_with_icon(icon_name: &str) -> Vec<(image::DynamicImage, serde_json::Value)> {
        let img = image::DynamicImage::new_rgba8(4, 4);
        let sprite_json = json!({
            icon_name: { "x": 0, "y": 0, "width": 4, "height": 4 }
        });
        vec![(img, sprite_json)]
    }

    #[test]
    fn test_render_layer_svg_fill_pattern() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "wetland",
            "type": "fill",
            "paint": {"fill-pattern": "wetland-icon"}
        }))
        .unwrap();
        let sprites = fake_sprite_with_icon("wetland-icon");
        let (svg, width, height) =
            render_layer_svg(&layer, 200, 40, false, false, &sprites).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 40);
        assert!(svg.contains("<image"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib test_render_layer_svg_fill_pattern -- --nocapture`
Expected: FAIL — either a compile error (arg count mismatch on the `"fill"` arm's call to `render_fill`) or, if it compiles, an error result because `sprite_data` isn't actually being passed to `render_fill` yet (the `"fill"` arm still calls it with 5 args, which won't compile once Task 1 lands — so this will be a compile failure until Step 3).

- [ ] **Step 3: Pass `sprite_data` into the `"fill"` arm**

In `src/lib.rs`, inside `render_layer_svg`, change:

```rust
            match layer.layer_type.as_str() {
                "fill" => render_fill(layer, paint, def_w, def_h, render_label),
                "line" => render_line(layer, paint, def_w, def_h, render_label),
                "circle" => render_circle(layer, paint, def_w, def_h, render_label),
```

to:

```rust
            match layer.layer_type.as_str() {
                "fill" => render_fill(layer, paint, def_w, def_h, render_label, sprite_data),
                "line" => render_line(layer, paint, def_w, def_h, render_label),
                "circle" => render_circle(layer, paint, def_w, def_h, render_label),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib -- --nocapture`
Expected: full test suite PASSES, including `test_render_layer_svg_fill_pattern` and all pre-existing tests (confirms no regression in `fill`/`line`/`circle`/`symbol` dispatch).

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs
git commit -m "feat: pass sprite_data to fill layer renderer"
```

---

### Task 3: Manual example — `example/style7.json`

**Files:**
- Create: `example/style7.json`
- Modify: `example/src/main.rs:11` (loop bound `1..=6` → `1..=7`)

**Interfaces:**
- Consumes: `MapLibreLegend::new` / `LegendConfig` / `render_all` (existing public API, unchanged).
- Produces: `example/combined_7.svg` (generated artifact for manual inspection only — the repo's root `.gitignore` has a blanket `*.svg` rule, so none of the `combined_*.svg` files are git-tracked; do not force-add it).

- [ ] **Step 1: Create the style file**

Create `example/style7.json`:

```json
{
  "version": 8,
  "name": "Fill Pattern Demo",
  "sprite": "https://demotiles.maplibre.org/styles/osm-bright-gl-style/sprite",
  "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
  "sources": {
    "dummy-source": {
      "type": "geojson",
      "data": {
        "type": "FeatureCollection",
        "features": []
      }
    }
  },
  "layers": [
    {
      "id": "wetland-pattern",
      "type": "fill",
      "source": "dummy-source",
      "paint": {
        "fill-pattern": "circle_11",
        "fill-outline-color": "#2c7bb6"
      },
      "metadata": {
        "legend": {
          "label": "Humedal (patrón)"
        }
      }
    },
    {
      "id": "solid-fill-comparison",
      "type": "fill",
      "source": "dummy-source",
      "paint": {
        "fill-color": "#a2d0a4",
        "fill-outline-color": "#333333"
      },
      "metadata": {
        "legend": {
          "label": "Relleno sólido (comparación)"
        }
      }
    }
  ]
}
```

- [ ] **Step 2: Wire it into `main.rs`**

In `example/src/main.rs`, change:

```rust
    for i in 1..=6 {
```

to:

```rust
    for i in 1..=7 {
```

- [ ] **Step 3: Run the example**

Run: `cd example && cargo run`
Expected: exits successfully, produces `example/combined_7.svg` (requires network access to fetch the `demotiles.maplibre.org` sprite — if the sandbox has no network access, run this step manually outside the agent and report the result instead of failing the task on a network error).

- [ ] **Step 4: Verify the output visually**

Run: `grep -c "<image" example/combined_7.svg`
Expected: at least `1` (the `wetland-pattern` layer's sprite icon was embedded). Optionally open `example/combined_7.svg` in a browser or image viewer to confirm the pattern swatch renders next to the solid-color swatch.

- [ ] **Step 5: Commit**

`example/combined_7.svg` is gitignored (`*.svg` in the repo's root `.gitignore`), same as the other `combined_*.svg` files — only the source files are committed:

```bash
cd /home/jose/trabajos/mvt-project/maplibre-legend
git add example/style7.json example/src/main.rs
git commit -m "docs: add fill-pattern example"
```

---

### Task 4: Final verification and changelog

**Files:**
- Modify: `CHANGELOG.md:1-5` (new entry at the top)
- No code files modified (verification only)

**Interfaces:**
- Consumes: nothing new.
- Produces: nothing new — this task only verifies Tasks 1–3 and documents the change.

- [ ] **Step 1: Add a changelog entry**

In `CHANGELOG.md`, insert a new section right after the `# Changelog` header and its description line, before the existing `## [0.5.0] - 2026-05-24` entry:

```markdown
## [Unreleased]

### Added

- `fill` layers now support `fill-pattern` (sprite icon name as a string). The
  legend swatch shows the sprite icon in place of a solid color. Expression-based
  `fill-pattern` (`match`/`case`) is not yet supported and returns an error.
```

- [ ] **Step 2: Run the full workspace check**

Run: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test`
Expected: `cargo fmt --check` reports no diffs, `cargo clippy` reports no warnings, `cargo test` passes all tests (including the new ones from Tasks 1 and 2).

If `cargo fmt --check` reports diffs, run `cargo fmt` and re-check. If `cargo clippy` reports warnings, fix them in the relevant file from Task 1 or 2 before proceeding — do not suppress with `#[allow(...)]` unless the warning is a false positive.

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: changelog entry for fill-pattern support"
```
