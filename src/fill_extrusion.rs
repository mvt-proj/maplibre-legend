use crate::{
    common::{
        ICON_HEIGHT, Layer, PADDING, ROW_HEIGHT, extract_color, get_fill_and_opacity,
        parse_expression, render_label, render_separator,
    },
    error::LegendError,
};
use svg::Document;
use svg::node::element::{Polygon, Text as SvgText};

/// Renders a `fill-extrusion` layer.
///
/// Single-color layers are shown as an isometric 3D box to visually distinguish them
/// from plain `fill` layers. Expression-based layers render as a list of colored
/// rectangles (same approach as `fill`).
pub fn render_fill_extrusion(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let color_expr = paint.get("fill-extrusion-color").ok_or_else(|| {
        LegendError::InvalidJson(format!(
            "Layer '{}': missing 'fill-extrusion-color' in paint",
            layer.id
        ))
    })?;
    let is_expression = color_expr.is_array();
    let cases = if is_expression {
        parse_expression(layer, color_expr)?
    } else {
        vec![]
    };

    let opacity = paint
        .get("fill-extrusion-opacity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let mut init_y: i32 = PADDING as i32;
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
    } else {
        default_height
    };

    let mut doc = Document::new()
        .set("width", default_width)
        .set("height", height);

    if !cases.is_empty() {
        // Multi-case: rectangles with labels (same as fill)
        if has_label {
            render_label(layer, &mut doc, Some(10), Some(20), Some(true))?;
            render_separator(&mut doc, default_width, 0, 10);
        }
        for (i, (label, color)) in cases.iter().enumerate() {
            let y = init_y + i as i32 * ROW_HEIGHT as i32;
            let (fill_value, effective_opacity) = get_fill_and_opacity(color, opacity);
            let rect = svg::node::element::Rectangle::new()
                .set("x", PADDING)
                .set("y", y)
                .set("width", 30)
                .set("height", ICON_HEIGHT)
                .set("fill", fill_value.as_str())
                .set("fill-opacity", effective_opacity)
                .set("stroke", "#333333")
                .set("stroke-width", "1");
            let text = SvgText::new("")
                .set("x", 45)
                .set("y", y + 15)
                .set("font-size", 14)
                .set("fill", "black")
                .add(svg::node::Text::new(label.clone()));
            doc = doc.add(rect).add(text);
        }
    } else {
        // Single color: isometric 3D box
        // Geometry (depth offset = 8px right, 8px up):
        //   Top face:   (10,17) (30,17) (38, 9) (18, 9)
        //   Front face: (10,17) (30,17) (30,31) (10,31)
        //   Side face:  (30,17) (38, 9) (38,23) (30,31)
        let color = extract_color(Some(color_expr))?;
        let (fill_value, effective_opacity) = get_fill_and_opacity(&color, opacity);
        let stroke = "#333333";

        let top = Polygon::new()
            .set("points", "10,17 30,17 38,9 18,9")
            .set("fill", fill_value.as_str())
            .set("fill-opacity", effective_opacity * 0.65)
            .set("stroke", stroke)
            .set("stroke-width", 0.5);

        let front = Polygon::new()
            .set("points", "10,17 30,17 30,31 10,31")
            .set("fill", fill_value.as_str())
            .set("fill-opacity", effective_opacity)
            .set("stroke", stroke)
            .set("stroke-width", 0.5);

        let side = Polygon::new()
            .set("points", "30,17 38,9 38,23 30,31")
            .set("fill", fill_value.as_str())
            .set("fill-opacity", effective_opacity * 0.45)
            .set("stroke", stroke)
            .set("stroke-width", 0.5);

        doc = doc.add(top).add(front).add(side);

        if has_label {
            render_label(layer, &mut doc, Some(45), Some(25), None)?;
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
        serde_json::from_value(json!({"id": id, "type": "fill-extrusion"})).unwrap()
    }

    fn paint(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn test_render_fill_extrusion_single_color() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-extrusion-color": "#ff0000"}));
        let (svg, width, height) = render_fill_extrusion(&layer, &p, 200, 50, false).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 50);
        // Single-color renders as an isometric 3D box (Polygon elements)
        assert!(svg.contains("polygon") || svg.contains("Polygon") || svg.contains("points"));
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn test_render_fill_extrusion_match_expression() {
        let layer = make_layer("test");
        let p = paint(json!({
            "fill-extrusion-color": ["match", ["get", "zona"], "A", "#ff0000", "#cccccc"]
        }));
        let (svg, _, _) = render_fill_extrusion(&layer, &p, 200, 50, false).unwrap();
        assert!(svg.contains("#ff0000"));
        assert!(svg.contains('A'));
    }

    #[test]
    fn test_render_fill_extrusion_missing_color_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({}));
        assert!(render_fill_extrusion(&layer, &p, 200, 50, false).is_err());
    }

    #[test]
    fn test_render_fill_extrusion_multi_case_height() {
        let layer = make_layer("test");
        let p = paint(json!({
            "fill-extrusion-color": ["match", ["get", "z"], "a", "#ff0000", "b", "#00ff00", "#cccccc"]
        }));
        let (_, _, height) = render_fill_extrusion(&layer, &p, 200, 50, false).unwrap();
        // 3 cases × ROW_HEIGHT(30) + ICON_HEIGHT(20) = 110
        assert_eq!(height, 110);
    }
}
