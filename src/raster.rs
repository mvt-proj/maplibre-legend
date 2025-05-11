use crate::{
    common::{Layer, render_label},
    error::LegendError,
};
use svg::Document;
use svg::node::element::Rectangle;

pub fn render_raster(
    layer: &Layer,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let total_w = 30.0;
    let total_h = 20.0;
    let cols = 4;
    let rows = 2;

    let cell_w = total_w / cols as f32;
    let cell_h = total_h / rows as f32;

    let mut doc = Document::new()
        .set("width", default_width)
        .set("height", default_height)
        .add(
            Rectangle::new()
                .set("x", 10)
                .set("y", 10)
                .set("width", total_w)
                .set("height", total_h)
                .set("fill", "#f8f9fa"),
        );

    let colors = ["#e9ecef", "#d8e2dc", "#c0d6d4", "#a8c8c8", "#90badc"];

    for i in 0..cols {
        for j in 0..rows {
            let seed = (i * 3 + j * 7) % colors.len();
            let color = colors[seed];

            let x = i as f32 * cell_w;
            let y = j as f32 * cell_h;

            doc = doc.add(
                Rectangle::new()
                    .set("x", 10.0 + x)
                    .set("y", 10.0 + y)
                    .set("width", cell_w)
                    .set("height", cell_h)
                    .set("fill", color)
                    .set("stroke", "#495057")
                    .set("stroke-width", 0.3),
            );
        }
    }

    doc = doc.add(
        Rectangle::new()
            .set("x", 10)
            .set("y", 10)
            .set("width", total_w - 0.0)
            .set("height", total_h - 0.0)
            .set("fill", "none")
            .set("stroke", "#495057")
            .set("stroke-width", 0.5),
    );

    if has_label {
        render_label(layer, &mut doc, None, None, None)?;
    }

    Ok((doc.to_string(), default_width, default_height))
}
