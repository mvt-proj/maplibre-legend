# maplibre-legend

A Rust crate to dynamically generate SVG layer legends from a [MapLibre GL style JSON][].

Given a MapLibre style (e.g. `style.json`), this library parses all layers and produces standalone SVG symbols or a combined legend, complete with optional labels and raster entries.

This project is a work in progress. It doesn't yet support all layer types, but it covers the most common cases. By using each layer's `metadata` attribute, you can flexibly customize how the legend is generated — especially the labels.

## Features

- Parse MapLibre GL style (v8) JSON into a structured `Style` model.
- Render individual layer legends as SVG: **fill**, **line**, **circle**, **symbol**, **fill-extrusion**, **background**, **heatmap**, **raster**.
- Sprite support: `sprite` field accepts both a single URL string and an array of URLs.
- Stack all layers into one combined SVG with separators.
- Optionally include raster layers.
- Customizable dimensions and label rendering via [`LegendConfig`].
- Per-layer label overrides through the `metadata.legend` object.

## Installation

```toml
[dependencies]
maplibre-legend = "0.5"
```

### Feature flags

| Feature | Default | Description |
|---------|:-------:|-------------|
| `async` | ✓ | Async sprite fetching via `reqwest` |
| `sync` | | Blocking sprite fetching — disable default features first |

**Async (default):**
```toml
maplibre-legend = "0.5"
```

**Sync:**
```toml
maplibre-legend = { version = "0.5", default-features = false, features = ["sync"] }
```

The two features are mutually exclusive.

## Usage

### Asynchronous

```rust
use maplibre_legend::{LegendConfig, MapLibreLegend};
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let style_json = fs::read_to_string("style.json").await?;

    let legend = MapLibreLegend::new(
        &style_json,
        LegendConfig {
            default_width: 250,
            ..Default::default()
        },
    )
    .await?;

    // Render all layers into one combined SVG (reversed order)
    let svg = legend.render_all(true)?;
    fs::write("legend.svg", svg).await?;

    // Or render a single layer by ID
    let svg = legend.render_layer("my-layer-id", None)?;
    fs::write("layer.svg", svg).await?;

    Ok(())
}
```

### Synchronous

```rust
use maplibre_legend::{LegendConfig, MapLibreLegend};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let style_json = fs::read_to_string("style.json")?;

    let legend = MapLibreLegend::new(
        &style_json,
        LegendConfig {
            default_width: 250,
            ..Default::default()
        },
    )?;

    let svg = legend.render_all(true)?;
    fs::write("legend.svg", svg)?;

    Ok(())
}
```

### `LegendConfig` options

```rust
LegendConfig {
    default_width: 200,    // SVG width in pixels
    default_height: 40,    // SVG height for single-entry layers
    has_label: true,       // render a title label above each layer
    include_raster: false, // include raster layers in render_all()
}
```

All fields are public. Use `..Default::default()` to keep the rest at their defaults.

## Examples

Given a MapLibre `style.json` with various fill, line, and circle layers:

```rust
let svg = legend.render_all(true)?;
```

| style.json | style2.json | style3.json |
|---|---|---|
| ![combined](https://github.com/user-attachments/assets/45f11696-c5d8-499a-8ab9-8a66a2cd82b0) | ![combined](https://github.com/user-attachments/assets/d865faf8-277f-48d7-8b19-541d0f984493) | ![combined_3](https://github.com/user-attachments/assets/929a0750-637a-4760-abfd-80952ad5baff) |

## Customizing legends with metadata

Each layer can override legend behavior through a `"legend"` object in its `metadata`:

```json
"metadata": {
    "legend": {
        "label": "Land value 2016 [USD/m²]",
        "default": "Other",
        "custom-labels": [
            "Up to $100",
            "$100 – $250",
            "$250 – $750",
            "Over $750"
        ]
    }
}
```

| Key | Type | Description |
|-----|------|-------------|
| `label` | string | Title for the legend entry. Falls back to the layer `id`. |
| `default` | string | Label for the expression's fallback/default color. |
| `custom-labels` | array of strings | Labels for each stop or case in the expression, in order. |

## Supported expressions

The following MapLibre paint expression types are parsed into legend entries:

| Expression | Behavior |
|-----------|----------|
| `match` | One entry per value + default |
| `case` | One entry per condition + default |
| `interpolate` | One entry per stop |
| `step` | One entry per threshold + base |
| `coalesce` | Delegates to the first inner match/case/interpolate/step |
| `literal` | Single entry |

## Modules

| Module | Layer type(s) |
|--------|--------------|
| `fill` | `fill` |
| `line` | `line` |
| `circle` | `circle` |
| `symbol` | `symbol` |
| `fill_extrusion` | `fill-extrusion` |
| `background` | `background` |
| `heatmap` | `heatmap` |
| `raster` | `raster` |
| `default` | unknown types (gray fallback) |
| `common` | shared types, expression parser, sprite utilities |
| `error` | `LegendError` |

## Contributing

1. Fork the repo and create a feature branch.
2. Run `cargo fmt` and `cargo clippy` before committing.
3. Add tests for new behavior.
4. Submit a pull request — feedback is welcome!

## License

BSD-3-Clause. See the [LICENSE][] file for details.

[MapLibre GL style JSON]: https://maplibre.org/maplibre-gl-js-docs/style-spec/
[`style.json`]: ./example/style.json
[LICENSE]: ./LICENSE
