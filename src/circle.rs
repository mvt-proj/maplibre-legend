use crate::{
    common::{
        FONT_SIZE, ICON_HEIGHT, Layer, PADDING, ROW_HEIGHT, extract_color, get_fill_and_opacity,
        parse_expression, render_label, render_separator,
    },
    error::LegendError,
};
use svg::Document;
use svg::node::element::{Circle, Text as SvgText};

/// Renders a `circle` layer legend as an SVG.
///
/// - Single-color paint: one circle centered in the SVG, sized by `circle-radius` (capped at 25 px).
/// - Expression-based paint: one 10 px radius circle per case, stacked vertically with labels.
///
/// Returns `(svg_string, width, height)`.
pub fn render_circle(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let color_expr = paint.get("circle-color").ok_or_else(|| {
        LegendError::InvalidJson(format!(
            "Layer '{}': missing 'circle-color' in paint",
            layer.id
        ))
    })?;
    let cases = parse_expression(layer, color_expr)?;
    let mut radius = paint
        .get("circle-radius")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);
    if radius > 25.0 {
        radius = 25.0;
    }
    let opacity = paint
        .get("circle-opacity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    let stroke_color =
        extract_color(paint.get("circle-stroke-color")).unwrap_or("black".to_string());
    let stroke_width = paint
        .get("circle-stroke-width")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let mut init_y = PADDING as i32;
    let dynamic_height = if cases.is_empty() {
        0
    } else {
        ICON_HEIGHT + cases.len() as u32 * ROW_HEIGHT
    };
    let height = if !cases.is_empty() {
        if has_label {
            init_y += ROW_HEIGHT as i32;
            dynamic_height + ICON_HEIGHT
        } else {
            dynamic_height
        }
    } else if radius >= 20.0 {
        default_height + radius as u32
    } else {
        default_height
    };
    let mut doc = Document::new()
        .set("width", default_width)
        .set("height", height);
    if !cases.is_empty() {
        if has_label {
            render_label(layer, &mut doc, Some(10), Some(20), Some(true))?;
            render_separator(&mut doc, default_width, 0, 10);
        }
        for (i, (label, color)) in cases.iter().enumerate() {
            let y = init_y + i as i32 * ROW_HEIGHT as i32;
            let (fill_value, effective_opacity) = get_fill_and_opacity(color, opacity);
            let circle = Circle::new()
                .set("cx", 20)
                .set("cy", y + ICON_HEIGHT as i32 / 2)
                .set("r", PADDING)
                .set("fill", fill_value.as_str())
                .set("fill-opacity", effective_opacity)
                .set("stroke", stroke_color.as_str())
                .set("stroke-width", stroke_width);
            let text = SvgText::new("")
                .set("x", 40)
                .set("y", y + 15)
                .set("font-size", FONT_SIZE)
                .set("fill", "black")
                .add(svg::node::Text::new(label.clone()));
            doc = doc.add(circle).add(text);
        }
    } else {
        let color = extract_color(Some(color_expr))?;
        let (fill_value, effective_opacity) = get_fill_and_opacity(&color, opacity);
        let cy = height / 2;
        let circle = Circle::new()
            .set("cx", 26)
            .set("cy", cy)
            .set("r", radius)
            .set("fill", fill_value)
            .set("fill-opacity", effective_opacity)
            .set("stroke", stroke_color)
            .set("stroke-width", stroke_width);
        doc = doc.add(circle);
        if has_label {
            render_label(layer, &mut doc, None, Some(cy + 5), None)?;
        }
    }
    Ok((doc.to_string(), default_width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Layer;
    use serde_json::json;

    fn make_layer(id: &str) -> Layer {
        serde_json::from_value(json!({"id": id, "type": "circle"})).unwrap()
    }

    fn paint(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn test_render_circle_single_color() {
        let layer = make_layer("test");
        let p = paint(json!({"circle-color": "#ff0000"}));
        // parse_expression returns 1 entry for plain string → multi-case path:
        // height = ICON_HEIGHT(20) + 1 * ROW_HEIGHT(30) = 50
        let (svg, width, height) = render_circle(&layer, &p, 200, 40, false).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 50);
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn test_render_circle_match_expression() {
        let layer = make_layer("test");
        let p = paint(json!({
            "circle-color": ["match", ["get", "t"], "a", "#ff0000", "b", "#00ff00", "#cccccc"]
        }));
        let (svg, _, _) = render_circle(&layer, &p, 200, 40, false).unwrap();
        assert!(svg.contains("#ff0000"));
        assert!(svg.contains("#00ff00"));
    }

    #[test]
    fn test_render_circle_large_radius_does_not_error() {
        let layer = make_layer("test");
        // radius=50 is capped to 25 internally; function must succeed
        let p = paint(json!({"circle-color": "#aaaaaa", "circle-radius": 50}));
        let (svg, _, _) = render_circle(&layer, &p, 200, 80, false).unwrap();
        assert!(svg.contains("#aaaaaa"));
    }

    #[test]
    fn test_render_circle_with_stroke() {
        let layer = make_layer("test");
        let p = paint(json!({
            "circle-color": "#ff0000",
            "circle-stroke-color": "#000000",
            "circle-stroke-width": 2
        }));
        let (svg, _, _) = render_circle(&layer, &p, 200, 40, false).unwrap();
        assert!(svg.contains("#000000"));
    }

    #[test]
    fn test_render_circle_missing_color_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({}));
        assert!(render_circle(&layer, &p, 200, 40, false).is_err());
    }
}
