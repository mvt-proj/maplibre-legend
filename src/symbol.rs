use crate::{
    common::{Layer, get_icon_data_url, parse_expression, render_label, render_separator},
    error::LegendError,
};
use image::DynamicImage;
use serde_json::Value;
use svg::Document;
use svg::node::element::{Image, Text as SvgText};

/// Extracts the `layout` object from a layer, returning an error if absent or not an object.
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

/// Renders a `symbol` layer legend as an SVG.
///
/// Priority: `icon-image` is rendered first (as a sprite icon), then `text-field` (as a bold "T").
/// - String `icon-image`: renders the named sprite icon; requires `sprite_data` to be loaded.
/// - Array `icon-image`: expression-based, renders one icon per case.
/// - `text-field` only: renders a bold "T" placeholder.
///
/// Returns [`LegendError::InvalidJson`] if neither `icon-image` nor `text-field` is present,
/// or if sprites are required but not loaded.
///
/// Returns `(svg_string, width, height)`.
pub fn render_symbol(
    layer: &Layer,
    default_width: u32,
    default_height: u32,
    has_label: bool,
    sprite_data: &[(DynamicImage, Value)],
) -> Result<(String, u32, u32), LegendError> {
    let layout = get_layout_object(layer)?;
    let text_field = layout.get("text-field");
    let icon_image = layout.get("icon-image");

    let mut doc = Document::new().set("width", default_width);
    let mut height = default_height;

    if let Some(icon_image) = icon_image {
        if sprite_data.is_empty() {
            return Err(LegendError::InvalidJson(
                "Missing sprite data for 'icon-image'".to_string(),
            ));
        }

        if let Some(icon_name) = icon_image.as_str() {
            let data_url = get_icon_data_url(sprite_data, icon_name)?;
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
                let data_url = get_icon_data_url(sprite_data, &icon_name)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Layer;
    use serde_json::json;

    fn make_layer_with_layout(id: &str, layout: serde_json::Value) -> Layer {
        serde_json::from_value(json!({"id": id, "type": "symbol", "layout": layout})).unwrap()
    }

    #[test]
    fn test_render_symbol_text_field() {
        let layer = make_layer_with_layout("sym", json!({"text-field": "{name}"}));
        let (svg, width, height) = render_symbol(&layer, 200, 40, false, &[]).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 40);
        // Should render a bold "T" placeholder for text-only symbols
        assert!(svg.contains(">T<") || svg.contains("T"));
    }

    #[test]
    fn test_render_symbol_missing_layout_returns_err() {
        let layer: Layer = serde_json::from_value(json!({"id": "sym", "type": "symbol"})).unwrap();
        assert!(render_symbol(&layer, 200, 40, false, &[]).is_err());
    }

    #[test]
    fn test_render_symbol_icon_without_sprite_returns_err() {
        let layer = make_layer_with_layout("sym", json!({"icon-image": "marker"}));
        // icon-image requires sprite data; empty slice → error
        assert!(render_symbol(&layer, 200, 40, false, &[]).is_err());
    }

    #[test]
    fn test_render_symbol_neither_text_nor_icon_returns_err() {
        let layer = make_layer_with_layout("sym", json!({"visibility": "visible"}));
        assert!(render_symbol(&layer, 200, 40, false, &[]).is_err());
    }
}
