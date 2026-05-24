# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                        # Build the library
cargo test                         # Run all tests
cargo test test_name               # Run a single test by name
cargo fmt                          # Format code (required before PRs)
cargo clippy                       # Lint (required before PRs)

# Build with specific features
cargo build --no-default-features --features sync   # Sync/blocking API
cargo build --features async                        # Async API (default)

# Run the example project
cd example && cargo run
```

## Architecture

This is a Rust library that parses MapLibre GL Style JSON (v8) and generates SVG legends for each layer.

**Data flow:**
1. `MapLibreLegend::new(style_json)` parses a style string into a `Style` struct (layers + optional sprite URL)
2. Optionally call `load_sprite()` to fetch the sprite PNG + JSON from the style's sprite URL
3. `render_all()` iterates layers and dispatches to per-layer-type renderers, stacking SVGs into a combined legend
4. `render_layer(id)` renders a single layer by ID

**Module layout:**

| Module | Responsibility |
|--------|---------------|
| `lib.rs` | Public API: `MapLibreLegend`, feature-gated `get_sprite()` |
| `common.rs` | Shared types (`Style`, `Layer`), expression parser, sprite utilities |
| `fill.rs`, `circle.rs`, `line.rs` | Vector geometry renderers |
| `fill_extrusion.rs` | 3D extrusion renderer (isometric box for single-color, rects for multi-case) |
| `background.rs` | Background color renderer |
| `symbol.rs` | Sprite icon or text placeholder renderer |
| `raster.rs`, `heatmap.rs` | Raster/heatmap renderers |
| `default.rs` | Gray fallback for unknown layer types |
| `error.rs` | `LegendError` using `thiserror` |

**Expression parsing (`common.rs`):**
The parser extracts legend entries from MapLibre paint expressions: `match`, `case`, `interpolate`, `step`, `coalesce`, and `literal`. Each expression produces a `Vec<(label, color)>`. `coalesce` unwraps inner expressions (match/case/interpolate/step) first; falls back to the last string argument. String transforms (`downcase`, `upcase`, `to-string`) are handled inline.

**Feature flags:**
- `async` (default) — uses `reqwest` async for sprite fetching
- `sync` — uses `reqwest` blocking; mutually exclusive with `async` (enforced via `compile_error!` in `lib.rs`)

**Sprite handling:**
`Style.sprite` is deserialized as `Vec<String>` (handles both a single URL string and an array of URLs). All spritesheets are loaded into `MapLibreLegend.sprite_data: Vec<(DynamicImage, Value)>`. Icon lookup in `get_icon_data_url` checks each spritesheet in order.

**Metadata customization:**
Layers can override legend behavior via their `metadata.legend` object:
```json
"metadata": {
  "legend": {
    "label": "Custom Title",
    "default": "Other",
    "custom-labels": ["Label A", "Label B"]
  }
}
```
