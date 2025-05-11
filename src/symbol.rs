use crate::{common::{Layer, parse_expression, render_label, render_separator}, error::LegendError};
use svg::Document;
use svg::node::element::{Image, Text as SvgText};
use reqwest::blocking::get;
use serde_json::Value;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::io::Cursor;

fn get_layout_object(layer: &Layer) -> Option<&serde_json::Map<String, Value>> {
    layer.layout.as_ref()?.as_object()
}

fn get_sprite(sprite_url: &str) -> Result<(DynamicImage, Value), LegendError> {
    let png_url_2x = format!("{}@2x.png", sprite_url);
    let json_url_2x = format!("{}@2x.json", sprite_url);
    let png_url = format!("{}.png", sprite_url);
    let json_url = format!("{}.json", sprite_url);

    let png_response = match get(&png_url_2x) {
        Ok(response) if response.status().is_success() => response,
        _ => get(&png_url).map_err(LegendError::PngFetch)?,
    };

    let png_data = png_response.bytes().map_err(LegendError::PngRead)?;
    let sprite_img = image::load_from_memory(&png_data).map_err(LegendError::ImageLoad)?;

    let json_response = match get(&json_url_2x) {
        Ok(response) if response.status().is_success() => response,
        _ => get(&json_url).map_err(LegendError::JsonFetch)?,
    };

    let sprite_json: Value = json_response.json().map_err(LegendError::JsonParse)?;

    Ok((sprite_img, sprite_json))
}

fn get_icon_data_url(
    sprite_img: &DynamicImage,
    sprite_json: &Value,
    icon_name: &str,
) -> Option<String> {
    let icon_info = sprite_json.get(icon_name)?;
    let x = icon_info["x"].as_u64()? as u32;
    let y = icon_info["y"].as_u64()? as u32;
    let width = icon_info["width"].as_u64()? as u32;
    let height = icon_info["height"].as_u64()? as u32;

    let icon_img = sprite_img.view(x, y, width, height).to_image();

    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    icon_img
        .write_to(&mut cursor, ImageFormat::Png)
        .expect("Could not write the icon to the buffer");

    let base64 = STANDARD.encode(&buf);
    Some(format!("data:image/png;base64,{}", base64))
}

pub fn render_symbol(
    layer: &Layer,
    default_width: u32,
    default_height: u32,
    has_label: bool,
    sprite_url: Option<&str>,
) -> Option<(String, u32, u32)> {
    let layout = get_layout_object(layer)?;
    let text_field = layout.get("text-field");
    let icon_image = layout.get("icon-image");

    let mut doc = Document::new().set("width", default_width);
    let mut height = default_height;

    if let Some(icon_image) = icon_image {
        let sprite_url = sprite_url?;
        let (sprite_img, sprite_json) = match get_sprite(sprite_url) {
            Ok(result) => result,
            Err(_) => return None,
        };
        // let (sprite_img, sprite_json) = get_sprite(&sprite_url);

        if let Some(icon_name) = icon_image.as_str() {
            if let Some(data_url) = get_icon_data_url(&sprite_img, &sprite_json, icon_name) {
                let image = Image::new()
                    .set("x", 10)
                    .set("y", 10)
                    .set("width", 20)
                    .set("height", 20)
                    .set("href", data_url);
                doc = doc.add(image);

                if has_label {
                    render_label(layer, &mut doc, Some(40), Some(25), Some(false));
                }
            }
        } else if let Some(_arr) = icon_image.as_array() {
            if let Some(cases) = parse_expression(layer, icon_image) {
                if has_label {
                    render_label(layer, &mut doc, Some(10), Some(20), Some(true));
                    render_separator(&mut doc, default_width, 0, 10);
                }
                let mut y = 10 +  20;
                for (label, icon_name) in cases {
                    if let Some(data_url) = get_icon_data_url(&sprite_img, &sprite_json, &icon_name)
                    {
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
                }
                height = y + 10;
                doc = doc.set("height", height);
            }
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
            render_label(layer, &mut doc, Some(30), Some(25), Some(false));
        }
        doc = doc.set("height", default_height);
    } else {
        return None;
    }

    Some((doc.to_string(), default_width, height))
}
