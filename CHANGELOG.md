# Changelog

All notable changes to this project will be documented in this file.

## [0.5.0] - 2026-05-24

### Breaking changes

#### `MapLibreLegend::new` now takes `LegendConfig` instead of individual parameters

The five positional parameters (`default_width`, `default_height`, `has_label`,
`include_raster`) have been replaced by a single [`LegendConfig`] struct.

**Before (0.4.x):**

```rust
let legend = MapLibreLegend::new(&style_json, 250, 40, true, false).await?;
```

**After (0.5.0):**

```rust
use maplibre_legend::{LegendConfig, MapLibreLegend};

let legend = MapLibreLegend::new(
    &style_json,
    LegendConfig {
        default_width: 250,
        ..Default::default()   // default_height: 40, has_label: true, include_raster: false
    },
).await?;
```

The `sync` feature follows the same pattern (non-async `new`).

`LegendConfig` implements `Default` and `Clone`. All fields are public so you can
set only the ones you need and let the rest fall back to their defaults.

#### `MapLibreLegend` fields replaced by `config`

The individual public fields (`default_width`, `default_height`, `has_label`,
`include_raster`) no longer exist on `MapLibreLegend`. They are now accessible
through `legend.config.*`.

**Before (0.4.x):**

```rust
println!("{}", legend.default_width);
legend.has_label = false;
```

**After (0.5.0):**

```rust
println!("{}", legend.config.default_width);
legend.config.has_label = false;
```

### Added

- `LegendConfig` struct — groups all rendering options with sensible defaults.
- `FALLBACK_COLOR`, `ROW_HEIGHT`, `ICON_HEIGHT`, `PADDING`, `FONT_SIZE` public
  constants in `common` (available for downstream use).
- Doc comments on all public and private functions across every module.
- 47 new unit and integration tests (8 → 55 total).
- Error messages now include the layer ID where relevant, e.g.:
  `"Layer 'my-layer': missing 'fill-color' in paint"`.

### Changed

- **reqwest 0.12 → 0.13**: the `rustls-tls` feature was renamed. Cargo features
  updated to `rustls` + `webpki-roots`. No API changes in the code itself.
- Magic numbers replaced by named constants throughout the rendering modules.

---

## [0.4.4] - 2025-05-20

- Fix missing stroke borders in expression-based fill and circle legends.
- Add opacity parsing for transparent fills (8-digit hex colors).

## [0.4.3] - 2025-05-19

- Add sample diverging style.

## [0.4.2] - 2025-05-18

- `cargo clippy` and `cargo fmt` cleanup.

## [0.4.1] - 2025-05-17

- Initial public release with fill, circle, line, symbol, heatmap, raster,
  fill-extrusion, and background layer support.
