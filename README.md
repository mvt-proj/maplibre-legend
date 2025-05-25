# maplibre-legend

A Rust crate to dynamically generate SVG layer legends from a [MapLibre GL style JSON][].

Given a MapLibre style (e.g. `style.json`), this library parses all layers and produces standalone SVG symbols or a combined legend, complete with optional labels and raster entries.

This project is a work in progress. It doesn't yet support all layer types, but it does cover the most common cases.
By using each layer's `metadata` attribute, you can flexibly customize how the legend is generated—especially the labels.


**Note:** Up to version `0.2.1`, the behavior was synchronous. Starting with version `0.3.0`, it is asynchronous.

This change was necessary because using the library in an asynchronous environment (such as with Tokio) previously caused issues by blocking the thread.


## Features

- Parse MapLibre GL style (v8) JSON into a structured `Style` model.
- Render individual layer legends (fill, line, circle) as SVG snippets.
- Symbol rendering is currently limited: it is supported only when the sprites field in style.json is a string URL, and does not support sprites defined as an array of string URLs.
- Optionally include raster layers.
- Render labels and customizable dimensions (`default_width`/`default_height`).
- Stack all layers into one combined SVG with separators.
- For finer details, the metadata attribute of the layer is used.

## Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
maplibre-legend = "0.4"  # replace with the latest version
````

### Features

This crate provides the following features to control HTTP request behavior:

* **`default`**: This enables the `async` feature by default.
* **`async`**: (Default) Enables asynchronous operations for fetching remote resources (like images or font icons). This uses `reqwest` with its asynchronous configuration.
    * To use this feature (which is the default), just add:
        ```toml
        maplibre-legend = "0.4"
        ```
* **`sync`**: Enables a synchronous/blocking API for fetching remote resources. This is useful for scripts or environments where an asynchronous runtime isn't required. This feature is mutually exclusive with `async` in terms of API.
    * To use this feature, you must disable the default features and specify `sync`:
        ```toml
        maplibre-legend = { version = "0.4", default-features = false, features = ["sync"] }
        ```

Make sure to select the appropriate feature based on your concurrency needs.

Then in your code:

```rust
use maplibre_legend::MapLibreLegend;
```

## Usage

### Asynchronous Example

```rust
use maplibre_legend::MapLibreLegend;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    for i in 1..=3 {
        let style_json = fs::read_to_string(format!("style{}.json", i)).await?;
        let legend = MapLibreLegend::new(&style_json, 250, 40, true, false).await?;
        let combined = legend.render_all(true)?;
        fs::write(format!("combined_{}.svg", i), combined).await?;
    }

    let style_json = fs::read_to_string("style1.json").await?;
    let legend = MapLibreLegend::new(&style_json, 250, 40, true, true).await?;
    let svg = legend.render_layer("vs2023", Some(true))?;
    fs::write("vs2023.svg", svg).await?;

    Ok(())
}

### Synchronous Example

```rust
use maplibre_legend::MapLibreLegend;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for i in 1..=4 {
        let style_json = fs::read_to_string(format!("style{}.json", i))?;
        let legend = MapLibreLegend::new(&style_json, 250, 40, true, false)?;
        let combined = legend.render_all(true)?;
        fs::write(format!("combined_{}.svg", i), combined)?;
    }

    let style_json = fs::read_to_string("style1.json")?;
    let legend = MapLibreLegend::new(&style_json, 250, 40, true, true)?;
    let svg = legend.render_layer("vs2023", Some(true))?;
    fs::write("vs2023.svg", svg)?;

    Ok(())
}
```


## Examples

### Legend Generation from MapLibre Styles

This Rust library allows you to generate legends based on a MapLibre `styles.json` file. It processes the style definition to extract information about layers and their visual representations, translating this into a structured legend.


Given the included [`style.json`][] (which defines various raster, fill, line and circle layers with `metadata.legend.label`), calling:

```rust
let svg = legend.render_all(true);
```

produces a full-page legend similar to:


| style.json | style2.json | style3.json |
|---|---|---|
| ![combined](https://github.com/user-attachments/assets/45f11696-c5d8-499a-8ab9-8a66a2cd82b0) | ![combined](https://github.com/user-attachments/assets/d865faf8-277f-48d7-8b19-541d0f984493) | ![combined_3](https://github.com/user-attachments/assets/929a0750-637a-4760-abfd-80952ad5baff)
 |


## Customizing Legends with Metadata

MapLibre styles allow for a `metadata` object within each layer definition. This library leverages a specific structure within this `metadata` to provide fine-grained control over how the legend for that layer is generated.

The customization options are defined within a `"legend"` object inside the layer's `metadata`.

```json
"metadata": {
    "legend": {
        "label": "Valor del Suelo 2016 [m2]",
        "default": "Otros",
        "custom-labels": [
            "Hasta U$D 100",
            "De U$D 100 as U$D 250",
            "De U$D 250 as U$D 750",
            "Mayor a U$D 750"
        ]
    }
}
```

Within the "legend" object in a layer's metadata, the following keys are used for customization:

- **label**
  - Type: string
  - Description: Sets the title for the legend entry. If omitted, the layer's id is used.

- **default**
  - Type: string
  - Description: Provides a label for data points not covered by the style's expressions (e.g., interpolate, case, match).

- **custom-labels**
  - Type: array of strings
  - Description: Supplies specific labels for the different categories or stops defined in expressions like interpolate, case, or match. The order should match the expression's stops/cases.

## Crate Modules

* **`circle`**: renders circle layers
* **`line`**: renders line layers
* **`fill`**: renders polygon (fill) layers
* **`symbol`**: renders icons o T letter for symbol layers
* **`raster`**: renders raster layers
* **`heatmap`**: renders heatmap layers
* **`default`**: renders a gray polygon
* **`common`**: shared types (`Style`, `Layer`, etc.)
* **`error`**: thiserror structure for error handling

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
