// Modules of the crate containing specific logic for rendering different types of layers.
mod circle;
mod common;
mod fill;
mod line;
mod raster;

// Imports of required functions and types from the modules.
use circle::render_circle;
use common::{Layer, Style};
use fill::render_fill;
use line::render_line;
use raster::render_raster;

/// Structure representing a MapLibre legend, used to render SVG representations
/// of style layers based on a JSON specification.
pub struct MapLibreLegend {
    /// The style of the legend, deserialized from JSON.
    style: Style,
    /// Default width for SVG renderings.
    pub default_width: u32,
    /// Default height for SVG renderings.
    pub default_height: u32,
    /// Indicates whether labels should be rendered on layers.
    pub has_label: bool,
    /// Indicates whether raster layers should be included in rendering.
    pub include_raster: bool,
}

impl MapLibreLegend {
    /// Creates a new `MapLibreLegend` instance from a JSON string and configuration parameters.
    ///
    /// # Parameters
    /// - `json`: A string containing the style in JSON format.
    /// - `default_width`: Default width for SVG renderings.
    /// - `default_height`: Default height for SVG renderings.
    /// - `render_label`: Whether to render labels on layers.
    /// - `include_raster`: Whether to include raster layers in rendering.
    ///
    /// # Returns
    /// - `Result<Self, serde_json::Error>`: A `MapLibreLegend` instance if the JSON is valid,
    ///   or a deserialization error if it is not.
    pub fn new(
        json: &str,
        default_width: u32,
        default_height: u32,
        has_label: bool,
        include_raster: bool,
    ) -> serde_json::Result<Self> {
        let style: Style = serde_json::from_str(json)?;
        Ok(Self {
            style,
            default_width,
            default_height,
            has_label,
            include_raster,
        })
    }

    /// Renders a specific layer as an SVG, identified by its ID.
    ///
    /// # Parameters
    /// - `id`: The identifier of the layer to render.
    ///
    /// # Returns
    /// - `Option<String>`: A string containing the SVG of the layer if found and renderable,
    ///   or `None` if the layer does not exist or is not renderable.
    pub fn render_layer(&self, id: &str) -> Option<String> {
        let layer = self.style.layers.iter().find(|l| l.id == id)?;
        render_layer_svg(
            layer,
            self.default_width,
            self.default_height,
            self.has_label,
            self.include_raster,
        )
        .map(|(svg, _, _)| svg)
    }

    /// Renders all layers in the style as a single combined SVG.
    ///
    /// Layers are stacked vertically with separator lines between them. The resulting SVG
    /// has a width equal to the maximum layer width and a height equal to the sum of layer heights.
    ///
    /// # Returns
    /// - `String`: A string containing the combined SVG of all layers.
    pub fn render_all(&self) -> String {
        let mut combined_body = String::new();
        let mut y_offset = 0;
        let mut max_width = 0;
        let total_layers = self.style.layers.len();

        for (i, layer) in self.style.layers.iter().enumerate() {
            if let Some((svg, w, h)) = render_layer_svg(
                layer,
                self.default_width,
                self.default_height,
                self.has_label,
                self.include_raster,
            ) {
                let inner = svg
                    .lines()
                    .filter(|l| !l.contains("<svg") && !l.contains("</svg>"))
                    .collect::<Vec<_>>()
                    .join("\n");
                max_width = max_width.max(w);
                combined_body.push_str(&format!(
                    "<g transform='translate(0,{})'>{}\n</g>\n",
                    y_offset, inner
                ));
                if i < total_layers - 1 {
                    combined_body.push_str(&format!(
                        "<line x1='0' y1='{}' x2='{}' y2='{}' stroke='#333333' stroke-width='0.5'/>\n",
                        y_offset + h, max_width, y_offset + h
                    ));
                }
                y_offset += h;
            }
        }
        format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' viewBox='0 0 {w} {h}'>\n{body}</svg>",
            w = max_width,
            h = y_offset,
            body = combined_body
        )
    }
}

/// Renders a single layer as an SVG based on its type and properties.
///
/// # Parameters
/// - `layer`: The layer to render.
/// - `def_w`: Default width for the SVG.
/// - `def_h`: Default height for the SVG.
/// - `render_label`: Whether to render labels for the layer.
/// - `include_raster`: Whether to include raster layers in rendering.
///
/// # Returns
/// - `Option<(String, u32, u32)>`: A tuple containing the SVG string, width, and height if the layer
///   is renderable, or `None` if it is not.
fn render_layer_svg(
    layer: &Layer,
    def_w: u32,
    def_h: u32,
    render_label: bool,
    include_raster: bool,
) -> Option<(String, u32, u32)> {
    match layer.layer_type.as_str() {
        "fill" | "line" | "circle" => {
            let paint = layer.paint.as_ref()?.as_object()?;
            match layer.layer_type.as_str() {
                "fill" => render_fill(layer, paint, def_w, def_h, render_label),
                "line" => render_line(layer, paint, def_w, def_h, render_label),
                "circle" => render_circle(layer, paint, def_w, def_h, render_label),
                _ => None,
            }
        }
        "raster" if include_raster => render_raster(layer, def_w, def_h, render_label),
        _ => None,
    }
}
