use crate::{
    common::{
        FONT_SIZE, ICON_HEIGHT, Layer, PADDING, ROW_HEIGHT, extract_color, get_fill_and_opacity,
        get_icon_data_url, parse_expression, render_label, render_separator,
    },
    error::LegendError,
};
use image::DynamicImage;
use serde_json::Value;
use svg::Document;
use svg::node::element::{Image, Rectangle, Text as SvgText};

/// Renders a `fill` layer legend as an SVG.
///
/// - Single-color paint: one rectangle with the layer label alongside.
/// - Expression-based paint (`match`, `case`, `interpolate`, `step`, `coalesce`):
///   one rectangle per case, stacked vertically with labels.
/// - `fill-pattern` (string sprite icon name): renders the named sprite icon inside an
///   outlined rectangle instead of a solid color; requires `sprite_data` to be loaded.
///   Returns [`LegendError::InvalidJson`] if `sprite_data` is empty, the icon name is not
///   found in any loaded spritesheet, or `fill-pattern` is not a string (an expression array
///   yields a "not yet supported" error; any other non-string value yields a "must be a
///   string" error).
///
/// Returns `(svg_string, width, height)`.
pub fn render_fill(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    default_width: u32,
    default_height: u32,
    has_label: bool,
    sprite_data: &[(DynamicImage, Value)],
) -> Result<(String, u32, u32), LegendError> {
    if let Some(pattern_value) = paint.get("fill-pattern") {
        return render_fill_pattern(
            layer,
            paint,
            pattern_value,
            sprite_data,
            default_width,
            default_height,
            has_label,
        );
    }

    let color_expr = paint.get("fill-color").ok_or_else(|| {
        LegendError::InvalidJson(format!(
            "Layer '{}': missing 'fill-color' in paint",
            layer.id
        ))
    })?;
    let cases = parse_expression(layer, color_expr)?;
    let opacity = paint
        .get("fill-opacity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    let fill_outline_color =
        extract_color(paint.get("fill-outline-color")).unwrap_or("black".to_string());
    let mut init_y = PADDING as i32;
    let dynamic_height = if cases.is_empty() {
        0
    } else {
        ICON_HEIGHT + cases.len() as u32 * ROW_HEIGHT
    };
    let height = if !cases.is_empty() {
        if has_label {
            init_y += ROW_HEIGHT as i32;
            dynamic_height + ICON_HEIGHT
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
            let (fill_value, effective_opacity) = get_fill_and_opacity(color, opacity);
            let rect = Rectangle::new()
                .set("x", PADDING)
                .set("y", y)
                .set("width", 30)
                .set("height", ICON_HEIGHT)
                .set("fill", fill_value.as_str())
                .set("fill-opacity", effective_opacity)
                .set("stroke", fill_outline_color.as_str())
                .set("stroke-width", "1");
            let text = SvgText::new("")
                .set("x", 45)
                .set("y", y + 15)
                .set("font-size", FONT_SIZE)
                .set("fill", "black")
                .add(svg::node::Text::new(label.clone()));
            doc = doc.add(rect).add(text);
        }
    } else {
        let color = extract_color(Some(color_expr))?;
        let (fill_value, effective_opacity) = get_fill_and_opacity(&color, opacity);
        let rect = Rectangle::new()
            .set("x", PADDING)
            .set("y", PADDING)
            .set("width", 30)
            .set("height", ICON_HEIGHT)
            .set("fill", fill_value)
            .set("fill-opacity", effective_opacity)
            .set("stroke", fill_outline_color)
            .set("stroke-width", "1");
        doc = doc.add(rect);
        if has_label {
            render_label(layer, &mut doc, None, None, None)?;
        }
    }
    Ok((doc.to_string(), default_width, height))
}

fn render_fill_pattern(
    layer: &Layer,
    paint: &serde_json::Map<String, serde_json::Value>,
    pattern_value: &serde_json::Value,
    sprite_data: &[(DynamicImage, Value)],
    default_width: u32,
    _default_height: u32,
    has_label: bool,
) -> Result<(String, u32, u32), LegendError> {
    let icon_name = pattern_value.as_str().ok_or_else(|| {
        if pattern_value.is_array() {
            LegendError::InvalidJson("fill-pattern expressions are not yet supported".to_string())
        } else {
            LegendError::InvalidJson("fill-pattern must be a string (icon name)".to_string())
        }
    })?;

    if sprite_data.is_empty() {
        return Err(LegendError::InvalidJson(
            "Missing sprite data for 'fill-pattern'".to_string(),
        ));
    }

    let data_url = get_icon_data_url(sprite_data, icon_name, None, (30, ICON_HEIGHT))?;
    let fill_outline_color =
        extract_color(paint.get("fill-outline-color")).unwrap_or("black".to_string());
    let opacity = paint
        .get("fill-opacity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let swatch_y = if has_label {
        PADDING + ROW_HEIGHT
    } else {
        PADDING
    };
    let height = if has_label {
        ICON_HEIGHT + ROW_HEIGHT + ICON_HEIGHT
    } else {
        ICON_HEIGHT + ROW_HEIGHT
    };

    let mut doc = Document::new()
        .set("width", default_width)
        .set("height", height);

    if has_label {
        render_label(layer, &mut doc, Some(10), Some(20), Some(true))?;
        render_separator(&mut doc, default_width, 0, 10);
    }

    let rect = Rectangle::new()
        .set("x", PADDING)
        .set("y", swatch_y)
        .set("width", 30)
        .set("height", ICON_HEIGHT)
        .set("fill", "none")
        .set("stroke", fill_outline_color.as_str())
        .set("stroke-width", "1");
    doc = doc.add(rect);

    let image = Image::new()
        .set("x", PADDING)
        .set("y", swatch_y)
        .set("width", 30)
        .set("height", ICON_HEIGHT)
        .set("href", data_url)
        .set("opacity", opacity);
    doc = doc.add(image);

    Ok((doc.to_string(), default_width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Layer;
    use serde_json::json;

    fn make_layer(id: &str) -> Layer {
        serde_json::from_value(json!({"id": id, "type": "fill"})).unwrap()
    }

    fn make_layer_with_label(id: &str, label: &str) -> Layer {
        serde_json::from_value(json!({
            "id": id, "type": "fill",
            "metadata": {"legend": {"label": label}}
        }))
        .unwrap()
    }

    fn paint(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_value(v).unwrap()
    }

    fn fake_sprite_with_icon(icon_name: &str) -> Vec<(image::DynamicImage, serde_json::Value)> {
        let img = image::DynamicImage::new_rgba8(4, 4);
        let sprite_json = json!({
            icon_name: { "x": 0, "y": 0, "width": 4, "height": 4 }
        });
        vec![(img, sprite_json)]
    }

    #[test]
    fn test_render_fill_pattern_string() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-pattern": "pattern-icon"}));
        let sprites = fake_sprite_with_icon("pattern-icon");
        let (svg, width, height) = render_fill(&layer, &p, 200, 40, false, &sprites).unwrap();
        assert_eq!(width, 200);
        // No label: height = ICON_HEIGHT(20) + ROW_HEIGHT(30) = 50
        assert_eq!(height, 50);
        assert!(svg.contains("<image"));
        assert!(svg.contains("data:image/png;base64,"));
    }

    #[test]
    fn test_render_fill_pattern_with_label() {
        let layer = make_layer_with_label("test", "Wetland");
        let p = paint(json!({"fill-pattern": "pattern-icon"}));
        let sprites = fake_sprite_with_icon("pattern-icon");
        let (svg, _, height) = render_fill(&layer, &p, 200, 40, true, &sprites).unwrap();
        assert!(svg.contains("Wetland"));
        // With label: height = ICON_HEIGHT(20) + ROW_HEIGHT(30) + ICON_HEIGHT(20) = 70
        assert_eq!(height, 70);
    }

    #[test]
    fn test_render_fill_pattern_missing_sprite_data_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-pattern": "pattern-icon"}));
        assert!(render_fill(&layer, &p, 200, 40, false, &[]).is_err());
    }

    #[test]
    fn test_render_fill_pattern_icon_not_found_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-pattern": "does-not-exist"}));
        let sprites = fake_sprite_with_icon("pattern-icon");
        assert!(render_fill(&layer, &p, 200, 40, false, &sprites).is_err());
    }

    #[test]
    fn test_render_fill_pattern_expression_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({
            "fill-pattern": ["match", ["get", "tipo"], "a", "icon-a", "icon-b"]
        }));
        let sprites = fake_sprite_with_icon("icon-a");
        let result = render_fill(&layer, &p, 200, 40, false, &sprites);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not yet supported"));
    }

    #[test]
    fn test_render_fill_pattern_non_string_non_array_returns_clear_err() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-pattern": 42}));
        let sprites = fake_sprite_with_icon("pattern-icon");
        let result = render_fill(&layer, &p, 200, 40, false, &sprites);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("must be a string"));
        assert!(!err.to_string().contains("not yet supported"));
    }

    #[test]
    fn test_render_fill_pattern_honors_fill_opacity() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-pattern": "pattern-icon", "fill-opacity": 0.5}));
        let sprites = fake_sprite_with_icon("pattern-icon");
        let (svg, _, _) = render_fill(&layer, &p, 200, 40, false, &sprites).unwrap();
        assert!(svg.contains("opacity=\"0.5\""));
    }

    #[test]
    fn test_render_fill_single_color() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-color": "#ff0000"}));
        // parse_expression returns 1 entry for a plain string → multi-case path:
        // height = ICON_HEIGHT(20) + 1 * ROW_HEIGHT(30) = 50
        let (svg, width, height) = render_fill(&layer, &p, 200, 40, false, &[]).unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 50);
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn test_render_fill_match_expression() {
        let layer = make_layer("test");
        let p = paint(json!({
            "fill-color": ["match", ["get", "tipo"], "bosque", "#228B22", "#cccccc"]
        }));
        let (svg, _, height) = render_fill(&layer, &p, 200, 40, false, &[]).unwrap();
        assert!(svg.contains("#228B22"));
        assert!(svg.contains("bosque"));
        // 2 cases × ROW_HEIGHT(30) + ICON_HEIGHT(20) = 80
        assert_eq!(height, 80);
    }

    #[test]
    fn test_render_fill_multi_case_with_label_increases_height() {
        let layer = make_layer_with_label("lyr", "Mi Capa");
        let p = paint(json!({
            "fill-color": ["match", ["get", "tipo"], "a", "#ff0000", "#cccccc"]
        }));
        let (svg, _, height_with) = render_fill(&layer, &p, 200, 40, true, &[]).unwrap();
        assert!(svg.contains("Mi Capa"));
        // With label: +ROW_HEIGHT offset + ICON_HEIGHT extra
        let (_, _, height_without) =
            render_fill(&make_layer("lyr"), &p, 200, 40, false, &[]).unwrap();
        assert!(height_with > height_without);
    }

    #[test]
    fn test_render_fill_with_opacity() {
        let layer = make_layer("test");
        let p = paint(json!({"fill-color": "#ff0000", "fill-opacity": 0.5}));
        let (svg, _, _) = render_fill(&layer, &p, 200, 40, false, &[]).unwrap();
        assert!(svg.contains("fill-opacity"));
        assert!(svg.contains("0.5"));
    }

    #[test]
    fn test_render_fill_missing_color_returns_err() {
        let layer = make_layer("test");
        let p = paint(json!({}));
        assert!(render_fill(&layer, &p, 200, 40, false, &[]).is_err());
    }
}
