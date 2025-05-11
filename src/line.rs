use crate::{
    common::{Layer, extract_color, parse_expression, render_label, render_separator},
    error::LegendError,
};
use svg::Document;
use svg::node::element::{Line, Text as SvgText};

pub fn render_line(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let color_expr = paint.get("line-color").ok_or_else(|| {
        LegendError::InvalidJson("Missing the 'line-color' field in paint".to_string())
    })?;
    let cases = parse_expression(layer, color_expr)?;

    let line_width = paint
        .get("line-width")
        .and_then(|v| v.as_f64())
        .unwrap_or(3.0); // Mantenemos el valor por defecto

    let mut init_y = 20;
    let dynamic_height = if cases.is_empty() {
        0
    } else {
        20 + cases.len() as u32 * 30
    };
    let height = if !cases.is_empty() {
        if has_label {
            init_y += 30;
            dynamic_height + 20
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
            let y = init_y + i as i32 * 30;
            let line = Line::new()
                .set("x1", 10)
                .set("y1", y)
                .set("x2", 40)
                .set("y2", y)
                .set("stroke", color.as_str())
                .set("stroke-width", 4);
            let text = SvgText::new("")
                .set("x", 45)
                .set("y", y + 5)
                .set("font-size", 14)
                .set("fill", "black")
                .add(svg::node::Text::new(label.clone()));
            doc = doc.add(line).add(text);
        }
    } else {
        let color = extract_color(Some(color_expr))?;
        let line = Line::new()
            .set("x1", 10)
            .set("y1", 20)
            .set("x2", 40)
            .set("y2", 20)
            .set("stroke", color)
            .set("stroke-width", line_width);
        doc = doc.add(line);

        if has_label {
            render_label(layer, &mut doc, None, None, None)?;
        }
    }

    Ok((doc.to_string(), default_width, height))
}
