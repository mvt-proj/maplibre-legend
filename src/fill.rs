use crate::{
    common::{Layer, extract_color, parse_expression, render_label, render_separator},
    error::LegendError,
};
use svg::Document;
use svg::node::element::{Rectangle, Text as SvgText};

pub fn render_fill(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let color_expr = paint.get("fill-color").ok_or_else(|| {
        LegendError::InvalidJson("Missing the 'fill-color' field in paint".to_string())
    })?;
    let cases = parse_expression(layer, color_expr)?;

    let opacity = paint
        .get("fill-opacity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0); // Mantenemos el valor por defecto
    let fill_outline_color =
        extract_color(paint.get("fill-outline-color")).unwrap_or("black".to_string()); // Mantenemos el valor por defecto

    let mut init_y = 10;
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
            let rect = Rectangle::new()
                .set("x", 10)
                .set("y", y)
                .set("width", 30)
                .set("height", 20)
                .set("fill", color.as_str())
                .set("fill-opacity", opacity);
            let text = SvgText::new("")
                .set("x", 45)
                .set("y", y + 15)
                .set("font-size", 14)
                .set("fill", "black")
                .add(svg::node::Text::new(label.clone()));
            doc = doc.add(rect).add(text);
        }
    } else {
        let color = extract_color(Some(color_expr))?;
        let rect = Rectangle::new()
            .set("x", 10)
            .set("y", 10)
            .set("width", 30)
            .set("height", 20)
            .set("fill", color)
            .set("fill-opacity", opacity)
            .set("stroke", fill_outline_color);
        doc = doc.add(rect);

        if has_label {
            render_label(layer, &mut doc, None, None, None)?;
        }
    }

    Ok((doc.to_string(), default_width, height))
}
