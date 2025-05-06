use crate::common::{Layer, render_label};
use svg::Document;
use svg::node::element::Rectangle;

pub fn render_default(
    layer: &Layer,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Option<(String, u32, u32)> {
    let color = "#cccccc";
    let outline_color = "#333333";
    let opacity = 0.8;


    let mut doc = Document::new()
        .set("width", default_width)
        .set("height", default_height);

    let rect = Rectangle::new()
        .set("x", 10)
        .set("y", 10)
        .set("width", 30)
        .set("height", 20)
        .set("fill", color)
        .set("fill-opacity", opacity)
        .set("stroke", outline_color);
    doc = doc.add(rect);

    if has_label {
        render_label(layer, &mut doc, None, None, None);
    }

    Some((doc.to_string(), default_width, default_height))
}
