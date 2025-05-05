# maplibre-legend

A Rust crate to generate SVG legends from a [MapLibre GL style JSON][].

Given a MapLibre style (e.g. `style.json`), this library parses all layers and produces standalone SVG symbols or a combined legend, complete with optional labels and raster entries.

## Features

- Parse MapLibre GL style (v8) JSON into a structured `Style` model.  
- Render individual layer legends (fill, line, circle) as SVG snippets.  
- Optionally include raster layers.  
- Render labels and customizable dimensions (`default_width`/`default_height`).  
- Stack all layers into one combined SVG with separators.  

## Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
maplibre-legend = "0.1.0"  # replace with the latest version
serde_json         = "1"    # for JSON deserialization
````

Then in your code:

```rust
use maplibre_legend::MapLibreLegend;
```

## Usage

### 1. Load a style and create a legend

```rust
let json = std::fs::read_to_string("style.json")?;
let legend = MapLibreLegend::new(
    &json,       // the MapLibre style JSON
    32,          // default width for each symbol
    32,          // default height
    true,        // render labels by default
    false,       // do not include raster layers
)?;
```

*Creates a `MapLibreLegend` instance by deserializing your JSON.*&#x20;

### 2. Render a single layer

```rust
if let Some(svg) = legend.render_layer("parcelario-fill", None) {
    println!("{}", svg);
}
```

* The second argument (`None`) lets the legend fall back to the `has_label` value you provided.
* Passing `Some(false)` will suppress the label for that specific symbol.&#x20;

### 3. Render all layers at once

```rust
let combined_svg = legend.render_all();
std::fs::write("legend.svg", &combined_svg)?;
```

This stacks each layer symbol vertically, drawing a thin separator between entries, and wraps everything in a single `<svg>` element sized to fit all symbols.&#x20;

## Examples

Given the included [`style.json`][] (which defines various raster, fill, line and circle layers with `metadata.legend.label`), calling:

```rust
let svg = legend.render_all();
```

produces a full-page legend similar to:


**style.json**

![combined](https://github.com/user-attachments/assets/45f11696-c5d8-499a-8ab9-8a66a2cd82b0)



**style2.json**

![combined](https://github.com/user-attachments/assets/d865faf8-277f-48d7-8b19-541d0f984493)



**style3.json**

![combined](https://github.com/user-attachments/assets/f70e3ac7-eedf-4107-8ffd-d97de18e8888)


## Crate Modules

* **`circle`**: renders circle layers
* **`line`**: renders line layers
* **`fill`**: renders polygon (fill) layers
* **`raster`**: renders raster (tile) layers
* **`common`**: shared types (`Style`, `Layer`, etc.)

## Contributing

1. Fork the repo and create a feature branch.
2. Run `cargo fmt` and `cargo clippy` to keep code clean.
3. Add unit tests for new features.
4. Submit a pull request—feedback is welcome!

## License

This project is licensed under the MIT License. See the [LICENSE][] file for details.

[MapLibre GL style JSON]: https://maplibre.org/maplibre-gl-js-docs/style-spec/
[`style.json`]: ./example/style.json
[LICENSE]: ./LICENSE

```
```
