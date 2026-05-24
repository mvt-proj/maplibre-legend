use crate::{
    common::{
        FONT_SIZE, Layer, PADDING, ROW_HEIGHT, extract_color, parse_expression, render_label,
        render_separator,
    },
    error::LegendError,
};
use svg::Document;
use svg::node::element::{Line, Text as SvgText};

/// Renders a `line` layer legend as an SVG.
///
/// - Single-color paint: one horizontal line segment with the layer label alongside.
/// - Expression-based paint: one line per case, stacked vertically with labels.
///
/// Respects `line-width`, `line-opacity`, `line-dasharray`, and the layout property `line-cap`.
///
/// Returns `(svg_string, width, height)`.
pub fn render_line(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let color_expr = paint.get("line-color").ok_or_else(|| {
        LegendError::InvalidJson(format!(
            "Layer '{}': missing 'line-color' in paint",
            layer.id
        ))
    })?;
    let cases = parse_expression(layer, color_expr)?;

    let line_width = paint
        .get("line-width")
        .and_then(|v| v.as_f64())
        .unwrap_or(3.0);

    let opacity = paint
        .get("line-opacity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let dasharray = paint
        .get("line-dasharray")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64())
                .map(|n| {
                    if n.fract() == 0.0 {
                        format!("{}", n as i64)
                    } else {
                        format!("{}", n)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        });

    // line-cap is a layout property
    let linecap = layer
        .layout
        .as_ref()
        .and_then(|l| l.get("line-cap"))
        .and_then(|v| v.as_str())
        .unwrap_or("butt")
        .to_string();

    let mut init_y = ROW_HEIGHT as i32;
    let dynamic_height = if cases.is_empty() {
        0
    } else {
        ROW_HEIGHT / 3 * 2 + cases.len() as u32 * ROW_HEIGHT
    };
    let height = if !cases.is_empty() {
        if has_label {
            init_y += ROW_HEIGHT as i32;
            dynamic_height + ROW_HEIGHT / 3 * 2
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
            let mut line = Line::new()
                .set("x1", PADDING)
                .set("y1", y)
                .set("x2", 40)
                .set("y2", y)
                .set("stroke", color.as_str())
                .set("stroke-width", line_width)
                .set("stroke-opacity", opacity)
                .set("stroke-linecap", linecap.as_str());
            if let Some(ref da) = dasharray {
                line = line.set("stroke-dasharray", da.as_str());
            }
            let text = SvgText::new("")
                .set("x", 45)
                .set("y", y + 5)
                .set("font-size", FONT_SIZE)
                .set("fill", "black")
                .add(svg::node::Text::new(label.clone()));
            doc = doc.add(line).add(text);
        }
    } else {
        let color = extract_color(Some(color_expr))?;
        let mut line = Line::new()
            .set("x1", PADDING)
            .set("y1", 20)
            .set("x2", 40)
            .set("y2", 20)
            .set("stroke", color)
            .set("stroke-width", line_width)
            .set("stroke-opacity", opacity)
            .set("stroke-linecap", linecap.as_str());
        if let Some(ref da) = dasharray {
            line = line.set("stroke-dasharray", da.as_str());
        }
        doc = doc.add(line);

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
        serde_json::from_value(json!({"id": id, "type": "line"})).unwrap()
    }

    fn make_layer_with_layout(id: &str, layout: serde_json::Value) -> Layer {
        serde_json::from_value(json!({"id": id, "type": "line", "layout": layout})).unwrap()
    }

    fn paint(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn test_render_line_single_color() {
        let layer = make_layer("test");
        let p = paint(json!({"line-color": "#ff0000"}));
        // parse_expression returns 1 entry for plain string → multi-case path:
        // height = ROW_HEIGHT/3*2(20) + 1 * ROW_HEIGHT(30) = 50
        let (svg, width, height) = render_line(&layer, &p, 200, 40, false).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 50);
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn test_render_line_match_expression() {
        let layer = make_layer("test");
        let p = paint(json!({
            "line-color": ["match", ["get", "tipo"], "prim", "#ff0000", "#cccccc"]
        }));
        let (svg, _, _) = render_line(&layer, &p, 200, 40, false).unwrap();
        assert!(svg.contains("#ff0000"));
        assert!(svg.contains("prim"));
    }

    #[test]
    fn test_render_line_dasharray_in_svg() {
        let layer = make_layer("test");
        let p = paint(json!({"line-color": "#000000", "line-dasharray": [4, 2]}));
        let (svg, _, _) = render_line(&layer, &p, 200, 40, false).unwrap();
        assert!(svg.contains("stroke-dasharray"));
        assert!(svg.contains('4'));
    }

    #[test]
    fn test_render_line_linecap_from_layout() {
        let layer = make_layer_with_layout("test", json!({"line-cap": "round"}));
        let p = paint(json!({"line-color": "#000000"}));
        let (svg, _, _) = render_line(&layer, &p, 200, 40, false).unwrap();
        assert!(svg.contains("round"));
    }

    #[test]
    fn test_render_line_missing_color_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({}));
        assert!(render_line(&layer, &p, 200, 40, false).is_err());
    }
}
