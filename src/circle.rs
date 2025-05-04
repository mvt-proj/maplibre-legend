use crate::common::{Layer, extract_color, parse_expression, render_label, render_separator};
use svg::Document;
use svg::node::element::{Circle, Text as SvgText};

pub fn render_circle(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Option<(String, u32, u32)> {
    let color_expr = paint.get("circle-color")?;
    let cases = parse_expression(layer, color_expr);

    let mut radius = paint
        .get("circle-radius")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);
    if radius > 25.0 {
        radius = 25.0;
    }

    let stroke_color =
        extract_color(paint.get("circle-stroke-color")).unwrap_or("black".to_string());
    let stroke_width = paint
        .get("circle-stroke-width")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let mut init_y = 10;
    let dynamic_height = cases
        .as_ref()
        .map_or(0, |cases| 20 + cases.len() as u32 * 30);
    let height = if cases.is_some() {
        if has_label {
            init_y += 30;
            dynamic_height + 20
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

    if let Some(cases) = cases {
        if has_label {
            render_label(layer, &mut doc, Some(10), Some(20), Some(true));
            render_separator(&mut doc, default_width, 0, 10);
        }
        for (i, (label, color)) in cases.iter().enumerate() {
            let y = init_y + i as i32 * 30;
            let circle = Circle::new()
                .set("cx", 20)
                .set("cy", y + 10)
                .set("r", 10)
                .set("fill", color.as_str());
            let text = SvgText::new("")
                .set("x", 40)
                .set("y", y + 15)
                .set("font-size", 14)
                .set("fill", "black")
                .add(svg::node::Text::new(label.clone()));
            doc = doc.add(circle).add(text);
        }
    } else {
        let color = extract_color(Some(color_expr))?;
        let cy = height / 2;
        let circle = Circle::new()
            .set("cx", 26)
            .set("cy", cy)
            .set("r", radius)
            .set("fill", color)
            .set("stroke", stroke_color)
            .set("stroke-width", stroke_width);
        doc = doc.add(circle);

        if has_label {
            render_label(layer, &mut doc, None, Some(cy as u32 + 5), None);
        }
    }

    Some((doc.to_string(), default_width, height))
}
