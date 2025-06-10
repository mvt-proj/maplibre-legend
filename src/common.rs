use crate::error::LegendError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, GenericImageView, ImageFormat};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::io::Cursor;
use svg::Document;
use svg::node::element::{Line, Text as SvgText};

enum ExpressionKind {
    Match,
    Case,
    Interpolate,
    Step,
}

impl ExpressionKind {
    fn from_str(s: &str) -> Result<Self, LegendError> {
        match s {
            "match" => Ok(Self::Match),
            "case" => Ok(Self::Case),
            "interpolate" => Ok(Self::Interpolate),
            "step" => Ok(Self::Step),
            _ => Err(LegendError::InvalidExpression(format!(
                "Unknown expression type: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Layer {
    pub id: String,
    #[serde(rename = "type")]
    pub layer_type: String,
    #[serde(default)]
    pub paint: Option<serde_json::Value>,
    #[serde(default)]
    pub layout: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Style {
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub sprite: Option<String>,
}

pub fn get_legend_object(layer: &Layer) -> Result<Option<&Map<String, Value>>, LegendError> {
    let metadata = match layer.metadata.as_ref() {
        Some(m) => m,
        None => return Ok(None),
    };

    let legend = match metadata.get("legend") {
        Some(l) => l,
        None => return Ok(None),
    };

    let legend_obj = legend.as_object().ok_or_else(|| {
        LegendError::InvalidJson("The 'legend' field is not an object".to_string())
    })?;

    Ok(Some(legend_obj))
}

pub fn get_layer_label(layer: &Layer) -> Result<String, LegendError> {
    let legend = get_legend_object(layer)?;
    let label = legend
        .and_then(|l| l.get("label"))
        .and_then(|l| l.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| layer.id.clone());
    Ok(label)
}

pub fn get_layer_default_label(layer: &Layer) -> Result<String, LegendError> {
    let legend = get_legend_object(layer)?;
    let default_label = legend
        .and_then(|l| l.get("default"))
        .and_then(|l| l.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| layer.id.clone());
    Ok(default_label)
}

pub fn get_custom_labels(layer: &Layer) -> Result<Vec<String>, LegendError> {
    let legend = get_legend_object(layer)?;
    let custom_labels = legend
        .and_then(|l| l.get("custom-labels"))
        .and_then(|l| l.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    Ok(custom_labels)
}

#[cfg(feature = "async")]
pub async fn get_sprite(sprite_url: &str) -> Result<(DynamicImage, Value), LegendError> {
    let client = reqwest::Client::new();

    let png_url_2x = format!("{}@2x.png", sprite_url);
    let png_url = format!("{}.png", sprite_url);
    let json_url_2x = format!("{}@2x.json", sprite_url);
    let json_url = format!("{}.json", sprite_url);

    let png_response = match client.get(&png_url_2x).send().await {
        Ok(resp) if resp.status().is_success() => resp,
        _ => client
            .get(&png_url)
            .send()
            .await
            .map_err(LegendError::PngFetch)?,
    };
    let png_data = png_response.bytes().await.map_err(LegendError::PngRead)?;
    let sprite_img = image::load_from_memory(&png_data).map_err(LegendError::ImageLoad)?;

    let json_response = match client.get(&json_url_2x).send().await {
        Ok(resp) if resp.status().is_success() => resp,
        _ => client
            .get(&json_url)
            .send()
            .await
            .map_err(LegendError::JsonFetch)?,
    };
    let sprite_json: Value = json_response.json().await.map_err(LegendError::JsonParse)?;

    Ok((sprite_img, sprite_json))
}

#[cfg(feature = "sync")]
pub fn get_sprite(sprite_url: &str) -> Result<(DynamicImage, Value), LegendError> {
    let client = reqwest::blocking::Client::new();

    let png_url_2x = format!("{}@2x.png", sprite_url);
    let png_url = format!("{}.png", sprite_url);
    let json_url_2x = format!("{}@2x.json", sprite_url);
    let json_url = format!("{}.json", sprite_url);

    let png_response = match client.get(&png_url_2x).send() {
        Ok(resp) if resp.status().is_success() => resp,
        _ => client.get(&png_url).send().map_err(LegendError::PngFetch)?,
    };
    let png_data = png_response.bytes().map_err(LegendError::PngRead)?;
    let sprite_img = image::load_from_memory(&png_data).map_err(LegendError::ImageLoad)?;

    let json_response = match client.get(&json_url_2x).send() {
        Ok(resp) if resp.status().is_success() => resp,
        _ => client.get(&json_url).send().map_err(LegendError::JsonFetch)?,
    };
    let sprite_json: Value = json_response.json().map_err(LegendError::JsonParse)?;

    Ok((sprite_img, sprite_json))
}

pub fn get_icon_data_url(
    sprite_img: &DynamicImage,
    sprite_json: &Value,
    icon_name: &str,
) -> Result<String, LegendError> {
    let icon_info = sprite_json.get(icon_name).ok_or_else(|| {
        LegendError::InvalidJson(format!("Icon '{}' not found in sprite JSON", icon_name))
    })?;
    let x = icon_info.get("x").and_then(|v| v.as_u64()).ok_or_else(|| {
        LegendError::InvalidJson(format!("Invalid 'x' field for icon '{}'", icon_name))
    })? as u32;
    let y = icon_info.get("y").and_then(|v| v.as_u64()).ok_or_else(|| {
        LegendError::InvalidJson(format!("Invalid 'y' field for icon '{}'", icon_name))
    })? as u32;
    let width = icon_info
        .get("width")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            LegendError::InvalidJson(format!("Invalid 'width' field for icon '{}'", icon_name))
        })? as u32;
    let height = icon_info
        .get("height")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            LegendError::InvalidJson(format!("Invalid 'height' field for icon '{}'", icon_name))
        })? as u32;

    let icon_img = sprite_img.view(x, y, width, height).to_image();

    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    icon_img
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(LegendError::ImageLoad)?;

    let base64 = STANDARD.encode(&buf);
    Ok(format!("data:image/png;base64,{}", base64))
}

pub fn render_label(
    layer: &Layer,
    doc: &mut Document,
    x: Option<u32>,
    y: Option<u32>,
    is_bold: Option<bool>,
) -> Result<(), LegendError> {
    let label = get_layer_label(layer)?;
    let x = x.unwrap_or(55);
    let y = y.unwrap_or(25);
    let is_bold = is_bold.unwrap_or_default();
    let font_weight = if is_bold { "bold" } else { "normal" };

    *doc = doc.clone().add(
        SvgText::new("")
            .set("x", x)
            .set("y", y)
            .set("font-size", 14)
            .set("fill", "black")
            .set("font-weight", font_weight)
            .add(svg::node::Text::new(label)),
    );
    Ok(())
}

pub fn render_separator(doc: &mut Document, default_width: u32, x: u32, y: u32) {
    let line_width = (default_width as f32 * 0.90) as u32;
    let line_x1 = x + (default_width - line_width) / 2;
    let line_x2 = line_x1 + line_width;
    let line_y = y + 16;

    let line = Line::new()
        .set("x1", line_x1)
        .set("x2", line_x2)
        .set("y1", line_y)
        .set("y2", line_y)
        .set("stroke", "#999999")
        .set("stroke-width", 0.5);

    *doc = doc.clone().add(line);
}

pub fn extract_color(value: Option<&serde_json::Value>) -> Result<String, LegendError> {
    let value = value.ok_or_else(|| LegendError::InvalidJson("Missing JSON value".to_string()))?;
    match value {
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Array(_) => Ok("#cccccc".to_string()),
        _ => Err(LegendError::InvalidJson(format!(
            "JSON value is neither a string nor an array: {:?}",
            value
        ))),
    }
}

fn extract_field(expr: &serde_json::Value) -> Result<&str, LegendError> {
    if let Some(arr) = expr.as_array() {
        if arr.is_empty() {
            return Err(LegendError::InvalidExpression(
                "Empty expression array".to_string(),
            ));
        }
        let op = arr[0]
            .as_str()
            .ok_or_else(|| LegendError::InvalidExpression("Operator is not a string".to_string()))?;
        match op {
            "get" => {
                if arr.len() != 2 {
                    return Err(LegendError::InvalidExpression(
                        "Invalid 'get' expression: must have exactly one argument".to_string(),
                    ));
                }
                arr[1]
                    .as_str()
                    .ok_or_else(|| LegendError::InvalidExpression(
                        "Field name in 'get' is not a string".to_string(),
                    ))
            }
            "downcase" | "upcase" | "to-string" | "to-number" => {
                if arr.len() != 2 {
                    return Err(LegendError::InvalidExpression(
                        format!("Invalid '{}' expression: must have exactly one argument", op),
                    ));
                }
                extract_field(&arr[1])
            }
            _ => Err(LegendError::InvalidExpression(
                format!("Unsupported operator in field extraction: {}", op),
            )),
        }
    } else {
        Err(LegendError::InvalidExpression(
            "Expression is not an array".to_string(),
        ))
    }
}

pub fn format_condition(cond: &serde_json::Value) -> Result<String, LegendError> {
    let arr = cond.as_array().ok_or_else(|| {
        LegendError::InvalidExpression("The condition is not an array".to_string())
    })?;
    if arr.is_empty() {
        return Ok("cond".to_string());
    }

    let op = arr[0].as_str().ok_or_else(|| {
        LegendError::InvalidExpression("The operator is not a string".to_string())
    })?;

    match op {
        "!" => {
            if let Some(inner) = arr.get(1) {
                if let Some(inner_arr) = inner.as_array() {
                    if inner_arr.first() == Some(&serde_json::Value::String("has".into())) {
                        if let Some(field) = inner_arr.get(1).and_then(|v| v.as_str()) {
                            return Ok(format!("without {}", field));
                        }
                    }
                }
            }
            Ok("undefined".to_string())
        }
        "has" => {
            if let Some(field) = arr.get(1).and_then(|v| v.as_str()) {
                Ok(format!("has {}", field))
            } else {
                Err(LegendError::InvalidExpression(
                    "Missing field in 'has' expression".to_string(),
                ))
            }
        }
        "==" | "!=" | ">" | ">=" | "<" | "<=" => {
            if arr.len() < 3 {
                return Err(LegendError::InvalidExpression(
                    "Comparison expression requires at least three elements".to_string(),
                ));
            }
            let field_expr = &arr[1];
            let field = extract_field(field_expr).map_err(|e| {
                LegendError::InvalidExpression(format!(
                    "Invalid field expression in comparison: {}",
                    e
                ))
            })?;
            let value = match &arr[2] {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => {
                    return Err(LegendError::InvalidExpression(
                        "Invalid value in comparison".to_string(),
                    ));
                }
            };
            Ok(format!("{} {} {}", field, op, value))
        }
        _ => Ok("cond".to_string()),
    }
}

fn parse_match(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    if arr.len() < 4 {
        return Err(LegendError::InvalidExpression(
            "Array 'match' too short".to_string(),
        ));
    }

    let _field = extract_field(&arr[1]).map_err(|e| {
        LegendError::InvalidExpression(format!(
            "Invalid input expression in 'match': {}",
            e
        ))
    })?;

    let labels = get_custom_labels(layer)?;
    let mut result = Vec::new();
    let mut i = 2;
    let mut label_index = 0;

    while i + 1 < arr.len() - 1 {
        let value = arr
            .get(i)
            .ok_or_else(|| {
                LegendError::InvalidExpression("Missing value in 'match' expression".to_string())
            })?
            .as_str()
            .ok_or_else(|| {
                LegendError::InvalidExpression("Value is not a string in 'match'".to_string())
            })?;
        let color = arr
            .get(i + 1)
            .ok_or_else(|| {
                LegendError::InvalidExpression("Missing color in 'match' expression".to_string())
            })?
            .as_str()
            .unwrap_or("#cccccc")
            .to_string();
        let label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            value.to_string()
        };
        result.push((label, color));
        i += 2;
        label_index += 1;
    }

    if let Some(default_color) = arr.last().and_then(|v| v.as_str()) {
        let default_label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            get_layer_default_label(layer)?
        };
        result.push((default_label, default_color.to_string()));
    }

    Ok(result)
}

fn parse_case(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    let labels = get_custom_labels(layer)?;
    let mut result = Vec::new();
    let mut i = 1;
    let mut label_index = 0;

    while i + 1 < arr.len() {
        let cond = arr.get(i).ok_or_else(|| {
            LegendError::InvalidExpression("Missing condition in 'case' expression".to_string())
        })?;
        let color = arr
            .get(i + 1)
            .ok_or_else(|| {
                LegendError::InvalidExpression("Missing color in 'case' expression".to_string())
            })?
            .as_str()
            .unwrap_or("#cccccc")
            .to_string();
        let label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            format_condition(cond)?
        };
        result.push((label, color));
        label_index += 1;
        i += 2;
    }

    if arr.len() % 2 == 0 {
        if let Some(default_color) = arr.last().and_then(|v| v.as_str()) {
            let default_label = if !labels.is_empty() && label_index < labels.len() {
                labels[label_index].clone()
            } else {
                get_layer_default_label(layer)?
            };
            result.push((default_label, default_color.to_string()));
        }
    }

    Ok(result)
}

fn parse_interpolate(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    if arr.len() < 4 {
        return Err(LegendError::InvalidExpression(
            "Array 'interpolate' too short".to_string(),
        ));
    }
    let labels = get_custom_labels(layer)?;

    let field = arr
        .get(2)
        .and_then(|v| {
            if let Some(get_arr) = v.as_array() {
                if get_arr.len() == 2 && get_arr[0] == "get" {
                    return get_arr[1].as_str();
                }
            }
            None
        })
        .ok_or_else(|| {
            LegendError::InvalidExpression(
                "Invalid 'get' field in 'interpolate' expression".to_string(),
            )
        })?;

    let mut result = Vec::new();
    let mut i = 3;
    let mut label_index = 0;
    while i + 1 < arr.len() {
        let val = arr
            .get(i)
            .ok_or_else(|| {
                LegendError::InvalidExpression(
                    "Missing value in 'interpolate' expression".to_string(),
                )
            })?
            .as_f64()
            .ok_or_else(|| {
                LegendError::InvalidExpression("Value is not a number in 'interpolate'".to_string())
            })?;
        let color = arr
            .get(i + 1)
            .ok_or_else(|| {
                LegendError::InvalidExpression(
                    "Missing color in 'interpolate' expression".to_string(),
                )
            })?
            .as_str()
            .unwrap_or("#cccccc")
            .to_string();
        let label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            format!("{field} ≥ {val}")
        };
        result.push((label, color));
        i += 2;
        label_index += 1;
    }

    Ok(result)
}

fn parse_step(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    // Minimum length: ["step", input, base_output]
    if arr.len() < 3 {
        return Err(LegendError::InvalidExpression(
            "Array 'step' too short: must have at least operator, input, and base output".to_string(),
        ));
    }

    // Allow even or odd lengths, as long as pairs are valid
    if arr.len() > 3 && arr.len() % 2 == 0 {
        return Err(LegendError::InvalidExpression(
            "Array 'step' has incomplete threshold-color pair".to_string(),
        ));
    }

    let labels = get_custom_labels(layer)?;

    // Extract the field from input expression (e.g., ["get", "cantidad"])
    let field = arr
        .get(1)
        .and_then(|v| {
            if let Some(get_arr) = v.as_array() {
                if get_arr.len() == 2 && get_arr[0] == "get" {
                    return get_arr[1].as_str();
                }
            }
            None
        })
        .ok_or_else(|| {
            LegendError::InvalidExpression(
                "Invalid 'get' field in 'step' expression".to_string(),
            )
        })?;

    let mut result = Vec::new();
    let mut label_index = 0;

    // Handle base output (for values below the first threshold, if any)
    let base_color = arr
        .get(2)
        .ok_or_else(|| {
            LegendError::InvalidExpression(
                "Missing base color in 'step' expression".to_string(),
            )
        })?
        .as_str()
        .ok_or_else(|| {
            LegendError::InvalidExpression(
                "Base color is not a string in 'step'".to_string(),
            )
        })?
        .to_string();

    let base_label = if arr.len() == 3 {
        // If no thresholds, use layer label or default
        if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            get_layer_label(layer)?
        }
    } else {
        // Use first threshold for base label
        if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            format!("{} < {}", field, arr[3].as_f64().unwrap_or(0.0))
        }
    };
    result.push((base_label, base_color));
    label_index += 1;

    // Process threshold-color pairs
    let mut i = 3;
    while i + 1 < arr.len() {
        let threshold = arr
            .get(i)
            .ok_or_else(|| {
                LegendError::InvalidExpression(
                    "Missing threshold in 'step' expression".to_string(),
                )
            })?
            .as_f64()
            .ok_or_else(|| {
                LegendError::InvalidExpression(
                    "Threshold is not a number in 'step'".to_string(),
                )
            })?;
        let color = arr
            .get(i + 1)
            .ok_or_else(|| {
                LegendError::InvalidExpression(
                    "Missing color in 'step' expression".to_string(),
                )
            })?
            .as_str()
            .ok_or_else(|| {
                LegendError::InvalidExpression(
                    "Color is not a string in 'step'".to_string(),
                )
            })?
            .to_string();

        let next_threshold = if i + 2 < arr.len() {
            arr[i + 2].as_f64()
        } else {
            None
        };

        let label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else if let Some(next) = next_threshold {
            format!("{} ≤ {} < {}", threshold, field, next)
        } else {
            format!("{} ≥ {}", field, threshold)
        };

        result.push((label, color));
        label_index += 1;
        i += 2;
    }

    Ok(result)
}

pub fn parse_expression(
    layer: &Layer,
    value: &serde_json::Value,
) -> Result<Vec<(String, String)>, LegendError> {
    if !value.is_array() {
        let value_str = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => {
                return Err(LegendError::InvalidExpression(format!(
                    "The value is neither a string, a number, nor a boolean. Layer: {}",
                    &layer.id
                )));
            }
        };
        let label = get_layer_label(layer)?;
        return Ok(vec![(label, value_str)]);
    }

    let arr = value.as_array().ok_or_else(|| {
        LegendError::InvalidExpression(format!("The value is not an array. Layer: {}", layer.id))
    })?;
    let first = arr
        .first()
        .ok_or_else(|| {
            LegendError::InvalidExpression(format!(
                "Empty array in the expression. Layer: {}",
                layer.id
            ))
        })?
        .as_str()
        .ok_or_else(|| {
            LegendError::InvalidExpression(format!(
                "The first element is not a string. Layer: {}",
                layer.id
            ))
        })?;

    match ExpressionKind::from_str(first)? {
        ExpressionKind::Match => parse_match(layer, arr),
        ExpressionKind::Case => parse_case(layer, arr),
        ExpressionKind::Interpolate => parse_interpolate(layer, arr),
        ExpressionKind::Step => parse_step(layer, arr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_field() {
        assert_eq!(
            extract_field(&json!(["get", "estado"])).unwrap(),
            "estado"
        );
        assert_eq!(
            extract_field(&json!(["downcase", ["get", "estado"]])).unwrap(),
            "estado"
        );
        assert_eq!(
            extract_field(&json!(["upcase", ["downcase", ["get", "estado"]]])).unwrap(),
            "estado"
        );
        assert!(extract_field(&json!(["downcase", ["invalid"]])).is_err());
    }

    #[test]
    fn test_parse_match_with_downcase() {
        let layer = json!({
            "id": "test",
            "type": "fill",
            "paint": {
                "fill-color": [
                    "match",
                    ["downcase", ["get", "estado"]],
                    "baldío",
                    "#a2d0a4",
                    "#91836f"
                ],
            },
            "metadata": {
                "legend": {
                    "label": "Test",
                    "custom-labels": ["Baldio"],
                    "default": "Otras"
                }
            }
        });
        let layer: Layer = serde_json::from_value(layer).unwrap();
        let result = parse_match(&layer.clone(), layer.paint.unwrap()["fill-color"].as_array().unwrap());
        assert_eq!(
            result.expect("Failed to parse match expression"),
            vec![(String::from("Baldio"), String::from("#a2d0a4")), (String::from("Otras"), String::from("#91836f"))]
        );
    }
}
