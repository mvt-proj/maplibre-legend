use crate::error::LegendError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, GenericImageView, ImageFormat};
use reqwest::blocking::get;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::io::Cursor;
use svg::Document;
use svg::node::element::{Line, Text as SvgText};

enum ExpressionKind {
    Match,
    Case,
    Interpolate,
}

impl ExpressionKind {
    fn from_str(s: &str) -> Result<Self, LegendError> {
        match s {
            "match" => Ok(Self::Match),
            "case" => Ok(Self::Case),
            "interpolate" => Ok(Self::Interpolate),
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

pub fn get_sprite(sprite_url: &str) -> Result<(DynamicImage, Value), LegendError> {
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
        .map_err(|e| LegendError::ImageLoad(e))?;

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
                    if inner_arr.get(0) == Some(&serde_json::Value::String("has".into())) {
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
            if let Some(get_expr) = arr.get(1).and_then(|v| v.as_array()) {
                if get_expr.get(0) == Some(&serde_json::Value::String("get".into())) {
                    if let Some(field) = get_expr.get(1).and_then(|v| v.as_str()) {
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
                        return Ok(format!("{field} {op} {value}"));
                    }
                }
            }
            Err(LegendError::InvalidExpression(
                "Invalid comparison expression".to_string(),
            ))
        }
        _ => Ok("cond".to_string()),
    }
}

fn parse_match(layer: &Layer, arr: &Vec<Value>) -> Result<Vec<(String, String)>, LegendError> {
    if arr.len() < 4 {
        return Err(LegendError::InvalidExpression(
            "Array 'match' too short".to_string(),
        ));
    }

    let _field = arr
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
            LegendError::InvalidExpression("Invalid 'get' field in 'match' expression".to_string())
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

fn parse_case(layer: &Layer, arr: &Vec<Value>) -> Result<Vec<(String, String)>, LegendError> {
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

fn parse_interpolate(
    layer: &Layer,
    arr: &Vec<Value>,
) -> Result<Vec<(String, String)>, LegendError> {
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
    }
}
