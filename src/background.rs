use crate::{
    common::{Layer, get_fill_and_opacity, render_label},
    error::LegendError,
};
use svg::Document;
use svg::node::element::Rectangle;

/// Renders a `background` layer legend as an SVG.
///
/// Shows a single rectangle filled with the background color and opacity.
/// Defaults to `#f0f0f0` if `background-color` is absent.
///
/// Returns `(svg_string, width, height)`.
pub fn render_background(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let color = paint
        .get("background-color")
        .and_then(|v| v.as_str())
        .unwrap_or("#f0f0f0");

    let opacity = paint
        .get("background-opacity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let (fill_value, effective_opacity) = get_fill_and_opacity(color, opacity);

    let mut doc = Document::new()
        .set("width", default_width)
        .set("height", default_height);

    let rect = Rectangle::new()
        .set("x", 10)
        .set("y", 10)
        .set("width", 30)
        .set("height", 20)
        .set("fill", fill_value)
        .set("fill-opacity", effective_opacity)
        .set("stroke", "#aaaaaa")
        .set("stroke-width", "1");

    doc = doc.add(rect);

    if has_label {
        render_label(layer, &mut doc, None, None, None)?;
    }

    Ok((doc.to_string(), default_width, default_height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Layer;
    use serde_json::json;

    fn make_layer(id: &str) -> Layer {
        serde_json::from_value(json!({"id": id, "type": "background"})).unwrap()
    }

    fn paint(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn test_render_background_basic() {
        let layer = make_layer("bg");
        let p = paint(json!({"background-color": "#f0f0f0"}));
        let (svg, width, height) = render_background(&layer, &p, 200, 40, false).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 40);
        assert!(svg.contains("#f0f0f0"));
    }

    #[test]
    fn test_render_background_with_opacity() {
        let layer = make_layer("bg");
        let p = paint(json!({"background-color": "#ffffff", "background-opacity": 0.5}));
        let (svg, _, _) = render_background(&layer, &p, 200, 40, false).unwrap();
        assert!(svg.contains("0.5"));
    }

    #[test]
    fn test_render_background_with_label() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "bg", "type": "background",
            "metadata": {"legend": {"label": "Fondo"}}
        }))
        .unwrap();
        let p = paint(json!({"background-color": "#ffffff"}));
        let (svg, _, _) = render_background(&layer, &p, 200, 40, true).unwrap();
        assert!(svg.contains("Fondo"));
    }
}
