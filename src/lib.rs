// Modules of the crate containing specific logic for rendering different types of layers.
mod circle;
mod common;
mod default;
mod error;
mod fill;
mod line;
mod raster;
mod symbol;

// Imports of required functions and types from the modules.
use circle::render_circle;
use common::get_sprite;
use common::{Layer, Style};
use default::render_default;
use error::LegendError;
use fill::render_fill;
use image::DynamicImage;
use line::render_line;
use raster::render_raster;
use serde_json::Value;
use symbol::render_symbol;

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
    /// Optional sprite data used for rendering symbol layers, containing the sprite image (PNG)
    /// and its associated JSON metadata. Populated during initialization if `style.sprite`
    /// specifies a valid URL; otherwise, `None`. The sprite data is loaded once to optimize
    /// rendering of multiple symbol layers.
    sprite_data: Option<(DynamicImage, Value)>,
}

impl MapLibreLegend {
    /// Creates a new `MapLibreLegend` instance from a JSON string and configuration parameters.
    ///
    /// # Parameters
    /// - `json`: A string containing the style in JSON format.
    /// - `default_width`: Default width for SVG renderings.
    /// - `default_height`: Default height for SVG renderings.
    /// - `has_label`: Whether to render labels on layers.
    /// - `include_raster`: Whether to include raster layers in rendering.
    ///
    /// # Returns
    /// - `Result<Self, LegendError>`: A `MapLibreLegend` instance if the JSON is valid,
    ///   or a `LegendError::Deserialization` if it is not.
    pub fn new(
        json: &str,
        default_width: u32,
        default_height: u32,
        has_label: bool,
        include_raster: bool,
    ) -> Result<Self, LegendError> {
        let style: Style =
            serde_json::from_str(json).map_err(|e| LegendError::Deserialization(e))?;
        let sprite_data = if let Some(sprite_url) = &style.sprite {
            Some(get_sprite(sprite_url)?)
        } else {
            None
        };
        Ok(Self {
            style,
            default_width,
            default_height,
            has_label,
            include_raster,
            sprite_data,
        })
    }

    /// Renders a specific layer as an SVG string, identified by its ID.
    ///
    /// # Parameters
    /// - `id`: The identifier of the layer to render.
    /// - `has_label`: An optional boolean indicating whether to render a label for the layer.
    ///   If `Some(true)` or `Some(false)`, uses the specified value; if `None`, falls back to
    ///   the default `self.has_label` value.
    ///
    /// # Returns
    /// - `Result<String, LegendError>`: A string containing the SVG representation of the layer if found and
    ///   renderable, or a `LegendError` if the layer does not exist or cannot be rendered.
    pub fn render_layer(&self, id: &str, has_label: Option<bool>) -> Result<String, LegendError> {
        let layer = self
            .style
            .layers
            .iter()
            .find(|l| l.id == id)
            .ok_or_else(|| LegendError::InvalidJson(format!("Layer with ID '{}' not found", id)))?;
        let (svg, _, _) = render_layer_svg(
            layer,
            self.default_width,
            self.default_height,
            has_label.unwrap_or(self.has_label),
            self.include_raster,
            &self.sprite_data,
        )?;
        Ok(svg)
    }

    /// Renders all layers in the style as a single combined SVG.
    ///
    /// Layers are stacked vertically with separator lines between them. The resulting SVG
    /// has a width equal to the maximum layer width and a height equal to the sum of layer heights.
    ///
    /// # Parameters
    /// - `rev`: If true, renders layers in reverse order.
    ///
    /// # Returns
    /// - `Result<String, LegendError>`: A string containing the combined SVG of all layers,
    ///   or a `LegendError` if any layer fails to render.
    pub fn render_all(&self, rev: bool) -> Result<String, LegendError> {
        let mut combined_body = String::new();
        let mut y_offset = 0;
        let mut max_width = 0;
        let total_layers = self.style.layers.len();

        // Create an iterator in normal or reversed order
        let layer_iter: Box<dyn Iterator<Item = (usize, &Layer)>> = if rev {
            Box::new(self.style.layers.iter().enumerate().rev())
        } else {
            Box::new(self.style.layers.iter().enumerate())
        };

        for (i, layer) in layer_iter {
            let (svg, w, h) = render_layer_svg(
                layer,
                self.default_width,
                self.default_height,
                self.has_label,
                self.include_raster,
                &self.sprite_data,
            )?;
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
            let is_last = if rev { i == 0 } else { i == total_layers - 1 };
            if !is_last {
                combined_body.push_str(&format!(
                    "<line x1='0' y1='{}' x2='{}' y2='{}' stroke='#333333' stroke-width='0.5'/>\n",
                    y_offset + h,
                    max_width,
                    y_offset + h
                ));
            }
            y_offset += h;
        }

        Ok(format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' viewBox='0 0 {w} {h}'>\n{body}</svg>",
            w = max_width,
            h = y_offset,
            body = combined_body
        ))
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
/// - `sprite_url`: Optional URL for sprite images used in symbol layers.
///
/// # Returns
/// - `Result<(String, u32, u32), LegendError>`: A tuple containing the SVG string, width, and height
///   if the layer is renderable, or a `LegendError` if it cannot be rendered.
fn render_layer_svg(
    layer: &Layer,
    def_w: u32,
    def_h: u32,
    render_label: bool,
    include_raster: bool,
    sprite_data: &Option<(DynamicImage, Value)>,
) -> Result<(String, u32, u32), LegendError> {
    match layer.layer_type.as_str() {
        "fill" | "line" | "circle" => {
            let paint = layer
                .paint
                .as_ref()
                .ok_or_else(|| {
                    LegendError::InvalidJson(format!(
                        "Missing the 'paint' field for layer '{}'",
                        layer.id
                    ))
                })?
                .as_object()
                .ok_or_else(|| {
                    LegendError::InvalidJson(format!(
                        "The 'paint' field is not an object for layer '{}'",
                        layer.id
                    ))
                })?;
            match layer.layer_type.as_str() {
                "fill" => render_fill(layer, paint, def_w, def_h, render_label),
                "line" => render_line(layer, paint, def_w, def_h, render_label),
                "circle" => render_circle(layer, paint, def_w, def_h, render_label),
                _ => Err(LegendError::InvalidJson(format!(
                    "Unknown layer type '{}'",
                    layer.layer_type
                ))),
            }
        }
        "symbol" => render_symbol(layer, def_w, def_h, render_label, sprite_data.as_ref()),
        "raster" if include_raster => render_raster(layer, def_w, def_h, render_label),
        "raster" => Ok(("<svg></svg>".to_string(), 0, 0)),
        _ => render_default(layer, def_w, def_h, render_label),
    }
}
