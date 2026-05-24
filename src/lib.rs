// Modules of the crate containing specific logic for rendering different types of layers.
mod background;
mod circle;
mod common;
mod default;
mod error;
mod fill;
mod fill_extrusion;
mod heatmap;
mod line;
mod raster;
mod symbol;

#[cfg(all(feature = "async", feature = "sync"))]
compile_error!("Features 'async' and 'sync' cannot be enabled at the same time.");

// Imports of required functions and types from the modules.
use crate::common::get_sprite;
use background::render_background;
use circle::render_circle;
use common::{Layer, Style};
use default::render_default;
pub use error::LegendError;
use fill::render_fill;
use fill_extrusion::render_fill_extrusion;
use heatmap::render_heatmap;
use image::DynamicImage;
use line::render_line;
use raster::render_raster;
use serde_json::Value;
use symbol::render_symbol;

/// Configuration for a [`MapLibreLegend`] instance.
///
/// All fields are public so they can be set directly. Use [`Default`] to get
/// reasonable defaults and override only what you need:
///
/// ```rust,ignore
/// let config = LegendConfig {
///     default_width: 300,
///     has_label: false,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct LegendConfig {
    /// Width in pixels for SVG renderings. Default: `200`.
    pub default_width: u32,
    /// Height in pixels for single-entry SVG renderings. Default: `40`.
    pub default_height: u32,
    /// Whether to render a title label above each layer legend. Default: `true`.
    pub has_label: bool,
    /// Whether to include `raster` layers in [`MapLibreLegend::render_all`]. Default: `false`.
    pub include_raster: bool,
}

impl Default for LegendConfig {
    fn default() -> Self {
        Self {
            default_width: 200,
            default_height: 40,
            has_label: true,
            include_raster: false,
        }
    }
}

/// A parsed MapLibre style with sprites loaded, ready to render SVG legends.
pub struct MapLibreLegend {
    /// The style deserialized from JSON.
    style: Style,
    /// Rendering configuration.
    pub config: LegendConfig,
    /// All loaded spritesheets (one per URL in `style.sprite`). Each entry holds the
    /// spritesheet image and its JSON metadata. Loaded once at construction time.
    sprite_data: Vec<(DynamicImage, Value)>,
}

impl MapLibreLegend {
    /// Creates a new `MapLibreLegend` instance from a style JSON string and a [`LegendConfig`].
    ///
    /// Fetches sprite sheets asynchronously if the style contains a `sprite` URL.
    ///
    /// # Errors
    /// Returns [`LegendError::Deserialization`] if the JSON is invalid, or a sprite-fetch
    /// error if the sprite URL cannot be reached.
    ///
    /// # Example
    /// ```rust,ignore
    /// let legend = MapLibreLegend::new(
    ///     &style_json,
    ///     LegendConfig { default_width: 300, ..Default::default() },
    /// ).await?;
    /// ```
    #[cfg(feature = "async")]
    pub async fn new(json: &str, config: LegendConfig) -> Result<Self, LegendError> {
        let style: Style = serde_json::from_str(json).map_err(LegendError::Deserialization)?;
        let sprite_data = if style.sprite.is_empty() {
            vec![]
        } else {
            get_sprite(&style.sprite).await?
        };
        Ok(Self {
            style,
            config,
            sprite_data,
        })
    }

    /// Creates a new `MapLibreLegend` instance from a style JSON string and a [`LegendConfig`].
    ///
    /// Fetches sprite sheets synchronously if the style contains a `sprite` URL.
    ///
    /// # Errors
    /// Returns [`LegendError::Deserialization`] if the JSON is invalid, or a sprite-fetch
    /// error if the sprite URL cannot be reached.
    #[cfg(feature = "sync")]
    pub fn new(json: &str, config: LegendConfig) -> Result<Self, LegendError> {
        let style: Style = serde_json::from_str(json).map_err(LegendError::Deserialization)?;
        let sprite_data = if style.sprite.is_empty() {
            vec![]
        } else {
            get_sprite(&style.sprite)?
        };
        Ok(Self {
            style,
            config,
            sprite_data,
        })
    }

    /// Renders a specific layer as an SVG string, identified by its ID.
    ///
    /// # Parameters
    /// - `id`: The identifier of the layer to render.
    /// - `has_label`: Overrides [`LegendConfig::has_label`] for this call only.
    ///   Pass `None` to use the value from the config.
    ///
    /// # Errors
    /// Returns [`LegendError::InvalidJson`] if no layer with the given ID exists.
    pub fn render_layer(&self, id: &str, has_label: Option<bool>) -> Result<String, LegendError> {
        let layer = self
            .style
            .layers
            .iter()
            .find(|l| l.id == id)
            .ok_or_else(|| LegendError::InvalidJson(format!("Layer with ID '{}' not found", id)))?;
        let (svg, _, _) = render_layer_svg(
            layer,
            self.config.default_width,
            self.config.default_height,
            has_label.unwrap_or(self.config.has_label),
            self.config.include_raster,
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
                self.config.default_width,
                self.config.default_height,
                self.config.has_label,
                self.config.include_raster,
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
/// Dispatches to the appropriate renderer based on `layer.layer_type`. Returns
/// `(svg_string, width, height)`. Width and height are both `0` for skipped layers
/// (e.g. `raster` when `include_raster` is false).
fn render_layer_svg(
    layer: &Layer,
    def_w: u32,
    def_h: u32,
    render_label: bool,
    include_raster: bool,
    sprite_data: &[(DynamicImage, Value)],
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
        "fill-extrusion" | "background" => {
            // Paint is optional for these types; fall back to render_default if absent.
            if let Some(paint) = layer.paint.as_ref().and_then(|p| p.as_object()) {
                match layer.layer_type.as_str() {
                    "fill-extrusion" => {
                        render_fill_extrusion(layer, paint, def_w, def_h, render_label)
                    }
                    "background" => render_background(layer, paint, def_w, def_h, render_label),
                    _ => unreachable!(),
                }
            } else {
                render_default(layer, def_w, def_h, render_label)
            }
        }
        "heatmap" => render_heatmap(layer, def_w, def_h, render_label),
        "symbol" => render_symbol(layer, def_w, def_h, render_label, sprite_data),
        "raster" if include_raster => render_raster(layer, def_w, def_h, render_label),
        "raster" => Ok(("<svg></svg>".to_string(), 0, 0)),
        _ => render_default(layer, def_w, def_h, render_label),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Layer;
    use serde_json::json;

    fn fill_layer(id: &str, color: &str) -> Layer {
        serde_json::from_value(json!({
            "id": id,
            "type": "fill",
            "paint": {"fill-color": color}
        }))
        .unwrap()
    }

    #[test]
    fn test_render_layer_svg_fill_single_color() {
        let layer = fill_layer("test", "#ff0000");
        // parse_expression for a plain color → 1 case → multi-case height = 50
        let (svg, width, height) = render_layer_svg(&layer, 200, 40, false, false, &[]).unwrap();
        assert!(svg.contains("#ff0000"));
        assert_eq!(width, 200);
        assert_eq!(height, 50);
    }

    #[test]
    fn test_render_layer_svg_unknown_type_uses_default() {
        let layer: Layer =
            serde_json::from_value(json!({"id": "x", "type": "custom-type"})).unwrap();
        let (svg, _, _) = render_layer_svg(&layer, 200, 40, false, false, &[]).unwrap();
        // render_default uses the gray fallback color
        assert!(svg.contains("cccccc"));
    }

    #[test]
    fn test_render_layer_svg_raster_excluded_returns_empty() {
        let layer: Layer = serde_json::from_value(json!({"id": "r", "type": "raster"})).unwrap();
        let (svg, width, height) = render_layer_svg(&layer, 200, 40, false, false, &[]).unwrap();
        assert_eq!(width, 0);
        assert_eq!(height, 0);
        assert_eq!(svg, "<svg></svg>");
    }

    #[test]
    fn test_render_layer_svg_raster_included() {
        let layer: Layer = serde_json::from_value(json!({"id": "r", "type": "raster"})).unwrap();
        let (svg, width, height) = render_layer_svg(&layer, 200, 40, false, true, &[]).unwrap();
        assert_eq!(width, 200);
        assert!(height > 0);
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_render_layer_svg_fill_missing_paint_returns_err() {
        let layer: Layer = serde_json::from_value(json!({"id": "x", "type": "fill"})).unwrap();
        assert!(render_layer_svg(&layer, 200, 40, false, false, &[]).is_err());
    }
}
