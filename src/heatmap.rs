use crate::{
    common::{Layer, render_label},
    error::LegendError,
};
use svg::Document;
use svg::node::element::{Definitions, RadialGradient, Rectangle, Stop};

pub fn render_heatmap(
    layer: &Layer,
    default_width: u32,
    default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let total_w = 30.0;
    let total_h = 20.0;

    let mut gradient = RadialGradient::new()
        .set("id", "heatmap-gradient")
        .set("cx", "25%")
        .set("cy", "25%")
        .set("r", "75%");
    let colors = ["#CC0000", "#FFFF99"];

    for (i, color) in colors.iter().enumerate() {
        let offset = (i as f32 / (colors.len() - 1) as f32) * 100.0;
        gradient = gradient.add(
            Stop::new()
                .set("offset", format!("{}%", offset))
                .set("stop-color", *color),
        );
    }

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
        )
        .add(Definitions::new().add(gradient))
        .add(
            Rectangle::new()
                .set("x", 10)
                .set("y", 10)
                .set("width", total_w)
                .set("height", total_h)
                .set("fill", "url(#heatmap-gradient)")
                .set("stroke", "#495057")
                .set("stroke-width", 0.5),
        );

    if has_label {
        render_label(layer, &mut doc, None, None, None)?;
    }

    Ok((doc.to_string(), default_width, default_height))
}
