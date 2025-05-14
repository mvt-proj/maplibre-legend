use crate::{
    common::{Layer, get_icon_data_url, parse_expression, render_label, render_separator},
    error::LegendError,
};
use image::DynamicImage;
use serde_json::Value;
use svg::Document;
use svg::node::element::{Image, Text as SvgText};

pub fn get_layout_object(layer: &Layer) -> Result<&serde_json::Map<String, Value>, LegendError> {
    let layout = layer
        .layout
        .as_ref()
        .ok_or_else(|| LegendError::InvalidJson("Missing 'layout' field".to_string()))?;
    let layout_obj = layout.as_object().ok_or_else(|| {
        LegendError::InvalidJson("The 'layout' field is not an object".to_string())
    })?;
    Ok(layout_obj)
}

pub fn render_symbol(
    layer: &Layer,
    default_width: u32,
    default_height: u32,
    has_label: bool,
    sprite_data: Option<&(DynamicImage, Value)>,
) -> Result<(String, u32, u32), LegendError> {
    let layout = get_layout_object(layer)?;
    let text_field = layout.get("text-field");
    let icon_image = layout.get("icon-image");

    let mut doc = Document::new().set("width", default_width);
    let mut height = default_height;

    if let Some(icon_image) = icon_image {
        let (sprite_img, sprite_json) = sprite_data.ok_or_else(|| {
            LegendError::InvalidJson("Missing sprite data for 'icon-image'".to_string())
        })?;

        if let Some(icon_name) = icon_image.as_str() {
            let data_url = get_icon_data_url(sprite_img, sprite_json, icon_name)?;
            let image = Image::new()
                .set("x", 10)
                .set("y", 10)
                .set("width", 20)
                .set("height", 20)
                .set("href", data_url);
            doc = doc.add(image);

            if has_label {
                render_label(layer, &mut doc, Some(40), Some(25), Some(false))?;
            }
        } else if let Some(_arr) = icon_image.as_array() {
            let cases = parse_expression(layer, icon_image)?;
            if has_label {
                render_label(layer, &mut doc, Some(10), Some(20), Some(true))?;
                render_separator(&mut doc, default_width, 0, 10);
            }
            let mut y = if has_label { 40 } else { 10 };
            for (label, icon_name) in cases {
                let data_url = get_icon_data_url(sprite_img, sprite_json, &icon_name)?;
                let image = Image::new()
                    .set("x", 10)
                    .set("y", y)
                    .set("width", 20)
                    .set("height", 20)
                    .set("href", data_url);
                doc = doc.add(image);

                let text = SvgText::new("")
                    .set("x", 40)
                    .set("y", y + 15)
                    .set("font-size", 14)
                    .set("fill", "black")
                    .add(svg::node::Text::new(label));
                doc = doc.add(text);

                y += 30;
            }
            height = y + 10;
            doc = doc.set("height", height);
        } else {
            return Err(LegendError::InvalidJson(
                "The field 'icon-image' is neither a string nor an array".to_string(),
            ));
        }
    } else if text_field.is_some() {
        let t_text = SvgText::new("T")
            .set("x", 14)
            .set("y", 25)
            .set("font-size", 16)
            .set("font-weight", "bold")
            .set("fill", "black");
        doc = doc.add(t_text);

        if has_label {
            render_label(layer, &mut doc, Some(30), Some(25), Some(false))?;
        }
        doc = doc.set("height", default_height);
    } else {
        return Err(LegendError::InvalidJson(
            "Neither 'text-field' nor 'icon-image' are present in 'layout'".to_string(),
        ));
    }

    Ok((doc.to_string(), default_width, height))
}
