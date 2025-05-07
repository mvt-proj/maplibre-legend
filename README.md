# maplibre-legend

A Rust crate to generate SVG legends from a [MapLibre GL style JSON][].

Given a MapLibre style (e.g. `style.json`), this library parses all layers and produces standalone SVG symbols or a combined legend, complete with optional labels and raster entries.

This project is a work in progress. It doesn't yet support all layer types, but it does cover the most common cases.
By using each layer's `metadata` attribute, you can flexibly customize how the legend is generated—especially the labels.


## Features

- Parse MapLibre GL style (v8) JSON into a structured `Style` model.
- Render individual layer legends (fill, line, circle) as SVG snippets.
- Optionally include raster layers.
- Render labels and customizable dimensions (`default_width`/`default_height`).
- Stack all layers into one combined SVG with separators.
- For finer details, the metadata attribute of the layer is used.

## Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
maplibre-legend = "0.1.0"  # replace with the latest version
````

Then in your code:

```rust
use maplibre_legend::MapLibreLegend;
```

## Usage

```rust
use maplibre_legend::MapLibreLegend;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let style_json = fs::read_to_string("style.json")?;

    let legend = MapLibreLegend::new(&style_json, 250,40, true, true)?;

    /// Render a single layer
    if let Some(svg) = legend.render_layer("vs2023", Some(true)) {
        fs::write("vs2023.svg", svg)?;
    }

    /// Render all layers
    let combined = legend.render_all(true);
    fs::write("combined.svg", combined)?;

    Ok(())
}

```

## Examples

Given the included [`style.json`][] (which defines various raster, fill, line and circle layers with `metadata.legend.label`), calling:

```rust
let svg = legend.render_all(true);
```

produces a full-page legend similar to:


| style.json | style2.json | style3.json |
|---|---|---|
| ![combined](https://github.com/user-attachments/assets/45f11696-c5d8-499a-8ab9-8a66a2cd82b0) | ![combined](https://github.com/user-attachments/assets/d865faf8-277f-48d7-8b19-541d0f984493) | ![combined](https://github.com/user-attachments/assets/f70e3ac7-eedf-4107-8ffd-d97de18e8888) |


## Crate Modules

* **`circle`**: renders circle layers
* **`line`**: renders line layers
* **`fill`**: renders polygon (fill) layers
* **`raster`**: renders raster (tile) layers
* **`default`**: renders a gray polygon
* **`common`**: shared types (`Style`, `Layer`, etc.)

## Contributing

1. Fork the repo and create a feature branch.
2. Run `cargo fmt` and `cargo clippy` to keep code clean.
3. Submit a pull request—feedback is welcome!

## License

This project is licensed under the MIT License. See the [LICENSE][] file for details.

[MapLibre GL style JSON]: https://maplibre.org/maplibre-gl-js-docs/style-spec/
[`style.json`]: ./example/style.json
[LICENSE]: ./LICENSE

```
```
