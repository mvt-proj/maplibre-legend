use crate::{
    common::{
        FONT_SIZE, ICON_HEIGHT, Layer, PADDING, ROW_HEIGHT, extract_color, get_fill_and_opacity,
        parse_expression, render_label, render_separator,
    },
    error::LegendError,
};
use svg::Document;
use svg::node::element::{Rectangle, Text as SvgText};

/// Renders a `fill` layer legend as an SVG.
///
/// - Single-color paint: one rectangle with the layer label alongside.
/// - Expression-based paint (`match`, `case`, `interpolate`, `step`, `coalesce`):
///   one rectangle per case, stacked vertically with labels.
///
/// Returns `(svg_string, width, height)`.
pub fn render_fill(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let color_expr = paint.get("fill-color").ok_or_else(|| {
        LegendError::InvalidJson(format!(
            "Layer '{}': missing 'fill-color' in paint",
            layer.id
        ))
    })?;
    let cases = parse_expression(layer, color_expr)?;
    let opacity = paint
        .get("fill-opacity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    let fill_outline_color =
        extract_color(paint.get("fill-outline-color")).unwrap_or("black".to_string());
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
            let rect = Rectangle::new()
                .set("x", PADDING)
                .set("y", y)
                .set("width", 30)
                .set("height", ICON_HEIGHT)
                .set("fill", fill_value.as_str())
                .set("fill-opacity", effective_opacity)
                .set("stroke", fill_outline_color.as_str())
                .set("stroke-width", "1");
            let text = SvgText::new("")
                .set("x", 45)
                .set("y", y + 15)
                .set("font-size", FONT_SIZE)
                .set("fill", "black")
                .add(svg::node::Text::new(label.clone()));
            doc = doc.add(rect).add(text);
        }
    } else {
        let color = extract_color(Some(color_expr))?;
        let (fill_value, effective_opacity) = get_fill_and_opacity(&color, opacity);
        let rect = Rectangle::new()
            .set("x", PADDING)
            .set("y", PADDING)
            .set("width", 30)
            .set("height", ICON_HEIGHT)
            .set("fill", fill_value)
            .set("fill-opacity", effective_opacity)
            .set("stroke", fill_outline_color)
            .set("stroke-width", "1");
        doc = doc.add(rect);
        if has_label {
            render_label(layer, &mut doc, None, None, None)?;
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
        serde_json::from_value(json!({"id": id, "type": "fill"})).unwrap()
    }

    fn make_layer_with_label(id: &str, label: &str) -> Layer {
        serde_json::from_value(json!({
            "id": id, "type": "fill",
            "metadata": {"legend": {"label": label}}
        }))
        .unwrap()
    }

    fn paint(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn test_render_fill_single_color() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-color": "#ff0000"}));
        // parse_expression returns 1 entry for a plain string → multi-case path:
        // height = ICON_HEIGHT(20) + 1 * ROW_HEIGHT(30) = 50
        let (svg, width, height) = render_fill(&layer, &p, 200, 40, false).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 50);
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn test_render_fill_match_expression() {
        let layer = make_layer("test");
        let p = paint(json!({
            "fill-color": ["match", ["get", "tipo"], "bosque", "#228B22", "#cccccc"]
        }));
        let (svg, _, height) = render_fill(&layer, &p, 200, 40, false).unwrap();
        assert!(svg.contains("#228B22"));
        assert!(svg.contains("bosque"));
        // 2 cases × ROW_HEIGHT(30) + ICON_HEIGHT(20) = 80
        assert_eq!(height, 80);
    }

    #[test]
    fn test_render_fill_multi_case_with_label_increases_height() {
        let layer = make_layer_with_label("lyr", "Mi Capa");
        let p = paint(json!({
            "fill-color": ["match", ["get", "tipo"], "a", "#ff0000", "#cccccc"]
        }));
        let (svg, _, height_with) = render_fill(&layer, &p, 200, 40, true).unwrap();
        assert!(svg.contains("Mi Capa"));
        // With label: +ROW_HEIGHT offset + ICON_HEIGHT extra
        let (_, _, height_without) = render_fill(&make_layer("lyr"), &p, 200, 40, false).unwrap();
        assert!(height_with > height_without);
    }

    #[test]
    fn test_render_fill_with_opacity() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-color": "#ff0000", "fill-opacity": 0.5}));
        let (svg, _, _) = render_fill(&layer, &p, 200, 40, false).unwrap();
        assert!(svg.contains("fill-opacity"));
        assert!(svg.contains("0.5"));
    }

    #[test]
    fn test_render_fill_missing_color_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({}));
        assert!(render_fill(&layer, &p, 200, 40, false).is_err());
    }
}
