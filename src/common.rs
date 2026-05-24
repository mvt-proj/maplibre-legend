use crate::error::LegendError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, GenericImageView, ImageFormat};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::io::Cursor;
use svg::Document;
use svg::node::element::{Line, Text as SvgText};

/// Fallback color used when a paint expression value cannot be resolved to a valid color.
pub const FALLBACK_COLOR: &str = "#cccccc";

/// Height in pixels allocated per legend row (icon + label).
pub const ROW_HEIGHT: u32 = 30;

/// Height in pixels of the visual icon element (rect, circle) inside each row.
pub const ICON_HEIGHT: u32 = 20;

/// Left and top margin in pixels for icon and label elements.
pub const PADDING: u32 = 10;

/// Font size in pixels for legend label text.
pub const FONT_SIZE: u32 = 14;

enum ExpressionKind {
    Match,
    Case,
    Interpolate,
    Step,
    Coalesce,
    Literal,
}

impl ExpressionKind {
    fn from_str(s: &str) -> Result<Self, LegendError> {
        match s {
            "match" => Ok(Self::Match),
            "case" => Ok(Self::Case),
            "interpolate" => Ok(Self::Interpolate),
            "step" => Ok(Self::Step),
            "coalesce" => Ok(Self::Coalesce),
            "literal" => Ok(Self::Literal),
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

/// Deserializes the MapLibre `sprite` field, which can be either a single URL string
/// or an array of URL strings.
fn deserialize_sprite_urls<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None => vec![],
        Some(Value::String(s)) => vec![s],
        Some(Value::Array(arr)) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(other) => {
            return Err(D::Error::custom(format!(
                "Expected string or array for 'sprite', got: {:?}",
                other
            )));
        }
    })
}

#[derive(Debug, Deserialize)]
pub struct Style {
    pub layers: Vec<Layer>,
    #[serde(default, deserialize_with = "deserialize_sprite_urls")]
    pub sprite: Vec<String>,
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
pub async fn get_sprite(sprite_urls: &[String]) -> Result<Vec<(DynamicImage, Value)>, LegendError> {
    let mut result = Vec::new();
    for url in sprite_urls {
        result.push(get_single_sprite_async(url).await?);
    }
    Ok(result)
}

#[cfg(feature = "async")]
async fn get_single_sprite_async(sprite_url: &str) -> Result<(DynamicImage, Value), LegendError> {
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
pub fn get_sprite(sprite_urls: &[String]) -> Result<Vec<(DynamicImage, Value)>, LegendError> {
    let mut result = Vec::new();
    for url in sprite_urls {
        result.push(get_single_sprite_sync(url)?);
    }
    Ok(result)
}

#[cfg(feature = "sync")]
fn get_single_sprite_sync(sprite_url: &str) -> Result<(DynamicImage, Value), LegendError> {
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
        _ => client
            .get(&json_url)
            .send()
            .map_err(LegendError::JsonFetch)?,
    };
    let sprite_json: Value = json_response.json().map_err(LegendError::JsonParse)?;

    Ok((sprite_img, sprite_json))
}

/// Searches all loaded spritesheets for `icon_name` and returns a base64-encoded PNG data URL.
/// Spritesheets are checked in order; the first match wins.
pub fn get_icon_data_url(
    sprites: &[(DynamicImage, Value)],
    icon_name: &str,
) -> Result<String, LegendError> {
    for (sprite_img, sprite_json) in sprites {
        if let Some(icon_info) = sprite_json.get(icon_name) {
            return extract_icon_from_sprite(sprite_img, icon_info, icon_name);
        }
    }
    Err(LegendError::InvalidJson(format!(
        "Icon '{}' not found in any sprite JSON",
        icon_name
    )))
}

fn extract_icon_from_sprite(
    sprite_img: &DynamicImage,
    icon_info: &Value,
    icon_name: &str,
) -> Result<String, LegendError> {
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
            .set("font-size", FONT_SIZE)
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

pub fn get_fill_and_opacity(color: &str, base_opacity: f64) -> (String, f64) {
    let trimmed = color.trim_start_matches('#');
    let len = trimmed.len();
    if len == 6 {
        (format!("#{}", trimmed), base_opacity)
    } else if len == 8 {
        let rgb = &trimmed[0..6];
        let alpha_hex = &trimmed[6..8];
        let alpha = match u8::from_str_radix(alpha_hex, 16) {
            Ok(val) => val as f64 / 255.0,
            Err(_) => 1.0,
        };
        let effective = alpha * base_opacity;
        if effective == 0.0 {
            ("none".to_string(), 1.0)
        } else {
            (format!("#{}", rgb), effective)
        }
    } else {
        // Fallback for non-hex or invalid; assume opaque
        (color.to_string(), base_opacity)
    }
}

pub fn extract_color(value: Option<&serde_json::Value>) -> Result<String, LegendError> {
    let value = value.ok_or_else(|| LegendError::InvalidJson("Missing JSON value".to_string()))?;
    match value {
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Array(_) => Ok(FALLBACK_COLOR.to_string()),
        _ => Err(LegendError::InvalidJson(format!(
            "JSON value is neither a string nor an array: {:?}",
            value
        ))),
    }
}

/// Extracts the feature property name from a MapLibre input expression.
///
/// Handles `["get", "field"]` directly, and recursively unwraps string transforms
/// (`downcase`, `upcase`, `to-string`, `to-number`) that wrap a `get`.
fn extract_field(expr: &serde_json::Value) -> Result<&str, LegendError> {
    if let Some(arr) = expr.as_array() {
        if arr.is_empty() {
            return Err(LegendError::InvalidExpression(
                "Empty expression array".to_string(),
            ));
        }
        let op = arr[0].as_str().ok_or_else(|| {
            LegendError::InvalidExpression("Operator is not a string".to_string())
        })?;
        match op {
            "get" => {
                if arr.len() != 2 {
                    return Err(LegendError::InvalidExpression(
                        "Invalid 'get' expression: must have exactly one argument".to_string(),
                    ));
                }
                arr[1].as_str().ok_or_else(|| {
                    LegendError::InvalidExpression(
                        "Field name in 'get' is not a string".to_string(),
                    )
                })
            }
            "downcase" | "upcase" | "to-string" | "to-number" => {
                if arr.len() != 2 {
                    return Err(LegendError::InvalidExpression(format!(
                        "Invalid '{}' expression: must have exactly one argument",
                        op
                    )));
                }
                extract_field(&arr[1])
            }
            _ => Err(LegendError::InvalidExpression(format!(
                "Unsupported operator in field extraction: {}",
                op
            ))),
        }
    } else {
        Err(LegendError::InvalidExpression(
            "Expression is not an array".to_string(),
        ))
    }
}

/// Converts a MapLibre filter expression into a human-readable legend label.
///
/// Supports `has`, `!` (not-has), and comparison operators (`==`, `!=`, `>`, `>=`, `<`, `<=`).
/// Unknown operators fall back to the string `"cond"`.
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
            if let Some(inner) = arr.get(1)
                && let Some(inner_arr) = inner.as_array()
                && inner_arr.first() == Some(&serde_json::Value::String("has".into()))
                && let Some(field) = inner_arr.get(1).and_then(|v| v.as_str())
            {
                return Ok(format!("without {}", field));
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

/// Parses a `["match", input, value, color, ..., default_color]` expression into legend entries.
///
/// Each `(value, color)` pair becomes one entry. Custom labels from layer metadata are applied
/// positionally if provided; otherwise the matched value is used as the label.
/// The final element is the default color, paired with the layer's `default` metadata label.
fn parse_match(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    if arr.len() < 4 {
        return Err(LegendError::InvalidExpression(format!(
            "Layer '{}': 'match' expression too short (need at least 4 elements)",
            layer.id
        )));
    }

    let _field = extract_field(&arr[1]).map_err(|e| {
        LegendError::InvalidExpression(format!(
            "Layer '{}': invalid input expression in 'match': {}",
            layer.id, e
        ))
    })?;

    let labels = get_custom_labels(layer)?;
    let mut result = Vec::new();
    let mut i = 2;
    let mut label_index = 0;

    while i + 1 < arr.len() - 1 {
        let value = arr.get(i).ok_or_else(|| {
            LegendError::InvalidExpression("Missing value in 'match' expression".to_string())
        })?;
        let value_str = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => {
                return Err(LegendError::InvalidExpression(
                    "Value is not a string or number in 'match'".to_string(),
                ));
            }
        };
        let color = arr
            .get(i + 1)
            .ok_or_else(|| {
                LegendError::InvalidExpression("Missing color in 'match' expression".to_string())
            })?
            .as_str()
            .unwrap_or(FALLBACK_COLOR)
            .to_string();
        let label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            value_str
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

/// Parses a `["case", cond1, color1, cond2, color2, ..., default_color]` expression.
///
/// Each condition is converted to a human-readable string via [`format_condition`].
/// Custom labels replace conditions when provided in layer metadata.
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
            .unwrap_or(FALLBACK_COLOR)
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

    if arr.len().is_multiple_of(2)
        && let Some(default_color) = arr.last().and_then(|v| v.as_str())
    {
        let default_label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            get_layer_default_label(layer)?
        };
        result.push((default_label, default_color.to_string()));
    }

    Ok(result)
}

/// Parses a `["interpolate", interp, ["get", field], stop0, color0, stop1, color1, ...]` expression.
///
/// Each `(stop, color)` pair becomes a legend entry labelled `"field ≥ stop"`.
/// Custom labels from layer metadata are applied positionally when provided.
fn parse_interpolate(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    if arr.len() < 4 {
        return Err(LegendError::InvalidExpression(format!(
            "Layer '{}': 'interpolate' expression too short (need at least 4 elements)",
            layer.id
        )));
    }
    let labels = get_custom_labels(layer)?;

    let field = arr
        .get(2)
        .and_then(|v| {
            if let Some(get_arr) = v.as_array()
                && get_arr.len() == 2
                && get_arr[0] == "get"
            {
                return get_arr[1].as_str();
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
            .unwrap_or(FALLBACK_COLOR)
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

/// Parses a `["step", ["get", field], base_color, threshold1, color1, ...]` expression.
///
/// The base entry is labelled `"field < threshold1"` (or the layer label if there are no thresholds).
/// Each subsequent `(threshold, color)` pair is labelled `"t ≤ field < next_t"`, or `"field ≥ t"` for
/// the last one. Custom labels from layer metadata are applied positionally when provided.
fn parse_step(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    // Minimum length: ["step", input, base_output]
    if arr.len() < 3 {
        return Err(LegendError::InvalidExpression(format!(
            "Layer '{}': 'step' expression too short (need operator, input, and base color)",
            layer.id
        )));
    }

    // Allow even or odd lengths, as long as pairs are valid
    if arr.len() > 3 && arr.len().is_multiple_of(2) {
        return Err(LegendError::InvalidExpression(format!(
            "Layer '{}': 'step' expression has incomplete threshold-color pair",
            layer.id
        )));
    }

    let labels = get_custom_labels(layer)?;

    // Extract the field from input expression (e.g., ["get", "cantidad"])
    let field = arr
        .get(1)
        .and_then(|v| {
            if let Some(get_arr) = v.as_array()
                && get_arr.len() == 2
                && get_arr[0] == "get"
            {
                return get_arr[1].as_str();
            }

            None
        })
        .ok_or_else(|| {
            LegendError::InvalidExpression("Invalid 'get' field in 'step' expression".to_string())
        })?;

    let mut result = Vec::new();
    let mut label_index = 0;

    // Handle base output (for values below the first threshold, if any)
    let base_color = arr
        .get(2)
        .ok_or_else(|| {
            LegendError::InvalidExpression("Missing base color in 'step' expression".to_string())
        })?
        .as_str()
        .ok_or_else(|| {
            LegendError::InvalidExpression("Base color is not a string in 'step'".to_string())
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
                LegendError::InvalidExpression("Missing threshold in 'step' expression".to_string())
            })?
            .as_f64()
            .ok_or_else(|| {
                LegendError::InvalidExpression("Threshold is not a number in 'step'".to_string())
            })?;
        let color = arr
            .get(i + 1)
            .ok_or_else(|| {
                LegendError::InvalidExpression("Missing color in 'step' expression".to_string())
            })?
            .as_str()
            .ok_or_else(|| {
                LegendError::InvalidExpression("Color is not a string in 'step'".to_string())
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

/// Tries each argument of a `coalesce` expression in order.
/// Returns entries from the first argument that parses as a multi-value expression
/// (match / case / interpolate / step). If none is found, returns the last string
/// argument as a single-color fallback entry.
fn parse_coalesce(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    for arg in arr.iter().skip(1) {
        if let Some(arg_arr) = arg.as_array()
            && let Some(op) = arg_arr.first().and_then(|v| v.as_str())
            && matches!(op, "match" | "case" | "interpolate" | "step")
            && let Ok(entries) = parse_expression(layer, arg)
            && entries.len() > 1
        {
            return Ok(entries);
        }
    }
    // Fallback: use last string value as the color
    let fallback_color = arr
        .iter()
        .rev()
        .find_map(|v| v.as_str())
        .unwrap_or(FALLBACK_COLOR)
        .to_string();
    let label = get_layer_label(layer)?;
    Ok(vec![(label, fallback_color)])
}

/// Treats `["literal", value]` as a plain color value.
fn parse_literal(layer: &Layer, arr: &[Value]) -> Result<Vec<(String, String)>, LegendError> {
    if arr.len() < 2 {
        return Err(LegendError::InvalidExpression(
            "'literal' requires a value argument".to_string(),
        ));
    }
    let label = get_layer_label(layer)?;
    let color = arr[1].as_str().unwrap_or(FALLBACK_COLOR).to_string();
    Ok(vec![(label, color)])
}

/// Parses a MapLibre paint value into a list of `(label, color)` legend entries.
///
/// - Plain strings, numbers, and booleans produce a single entry using the layer label.
/// - Arrays are dispatched by operator: `match`, `case`, `interpolate`, `step`, `coalesce`, `literal`.
///
/// Returns [`LegendError::InvalidExpression`] for unsupported or malformed expressions.
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
        ExpressionKind::Coalesce => parse_coalesce(layer, arr),
        ExpressionKind::Literal => parse_literal(layer, arr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_field() {
        assert_eq!(extract_field(&json!(["get", "estado"])).unwrap(), "estado");
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
        let result = parse_match(
            &layer.clone(),
            layer.paint.unwrap()["fill-color"].as_array().unwrap(),
        );
        assert_eq!(
            result.expect("Failed to parse match expression"),
            vec![
                (String::from("Baldio"), String::from("#a2d0a4")),
                (String::from("Otras"), String::from("#91836f"))
            ]
        );
    }

    #[test]
    fn test_parse_coalesce_with_inner_match() {
        let layer = json!({"id": "test", "type": "fill"});
        let layer: Layer = serde_json::from_value(layer).unwrap();

        // coalesce wrapping a match expression
        let expr = json!([
            "coalesce",
            [
                "match",
                ["get", "tipo"],
                "bosque",
                "#228B22",
                "agua",
                "#4169E1",
                "#cccccc"
            ],
            "#aaaaaa"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("bosque".to_string(), "#228B22".to_string()));
        assert_eq!(result[1], ("agua".to_string(), "#4169E1".to_string()));
    }

    #[test]
    fn test_parse_coalesce_fallback() {
        let layer = json!({"id": "test_layer", "type": "fill"});
        let layer: Layer = serde_json::from_value(layer).unwrap();

        // coalesce with only a get + fallback color — should return fallback
        let expr = json!(["coalesce", ["get", "color"], "#ff0000"]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "#ff0000");
    }

    #[test]
    fn test_parse_literal() {
        let layer = json!({"id": "test", "type": "fill"});
        let layer: Layer = serde_json::from_value(layer).unwrap();

        let expr = json!(["literal", "#ff0000"]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "#ff0000");
    }

    #[test]
    fn test_parse_case_basic() {
        let layer: Layer = serde_json::from_value(json!({"id": "test", "type": "fill"})).unwrap();
        let expr = json!([
            "case",
            ["has", "nombre"],
            "#ff0000",
            ["==", ["get", "tipo"], "bosque"],
            "#00ff00",
            "#cccccc"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "has nombre");
        assert_eq!(result[0].1, "#ff0000");
        assert_eq!(result[1].1, "#00ff00");
        assert_eq!(result[2].1, "#cccccc");
    }

    #[test]
    fn test_parse_case_with_custom_labels() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"custom-labels": ["Label A", "Label B", "Otros"]}}
        }))
        .unwrap();
        let expr = json!([
            "case",
            ["has", "a"],
            "#ff0000",
            ["has", "b"],
            "#00ff00",
            "#0000ff"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result[0].0, "Label A");
        assert_eq!(result[1].0, "Label B");
        assert_eq!(result[2].0, "Otros");
    }

    #[test]
    fn test_parse_interpolate_basic() {
        let layer: Layer = serde_json::from_value(json!({"id": "test", "type": "fill"})).unwrap();
        let expr = json!([
            "interpolate",
            ["linear"],
            ["get", "value"],
            0,
            "#ff0000",
            50,
            "#ffff00",
            100,
            "#00ff00"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "value ≥ 0");
        assert_eq!(result[0].1, "#ff0000");
        assert_eq!(result[2].0, "value ≥ 100");
        assert_eq!(result[2].1, "#00ff00");
    }

    #[test]
    fn test_parse_interpolate_with_custom_labels() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"custom-labels": ["Bajo", "Medio", "Alto"]}}
        }))
        .unwrap();
        let expr = json!([
            "interpolate",
            ["linear"],
            ["get", "pop"],
            0,
            "#ffffcc",
            500,
            "#fd8d3c",
            1000,
            "#800026"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result[0].0, "Bajo");
        assert_eq!(result[1].0, "Medio");
        assert_eq!(result[2].0, "Alto");
    }

    #[test]
    fn test_parse_step_basic() {
        let layer: Layer = serde_json::from_value(json!({"id": "test", "type": "fill"})).unwrap();
        let expr = json!([
            "step",
            ["get", "count"],
            "#ffffb2",
            10,
            "#fecc5c",
            50,
            "#fd8d3c",
            100,
            "#e31a1c"
        ]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].0, "count < 10");
        assert_eq!(result[0].1, "#ffffb2");
        assert_eq!(result[1].0, "10 ≤ count < 50");
        assert_eq!(result[3].0, "count ≥ 100");
    }

    #[test]
    fn test_parse_step_no_thresholds() {
        let layer: Layer = serde_json::from_value(json!({"id": "test", "type": "fill"})).unwrap();
        let expr = json!(["step", ["get", "value"], "#ff0000"]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "#ff0000");
    }

    #[test]
    fn test_parse_step_with_custom_labels() {
        let layer: Layer = serde_json::from_value(json!({
            "id": "test", "type": "fill",
            "metadata": {"legend": {"custom-labels": ["Bajo", "Alto"]}}
        }))
        .unwrap();
        let expr = json!(["step", ["get", "n"], "#aaaaaa", 100, "#ff0000"]);
        let result = parse_expression(&layer, &expr).unwrap();
        assert_eq!(result[0].0, "Bajo");
        assert_eq!(result[1].0, "Alto");
    }

    #[test]
    fn test_get_fill_and_opacity_6char() {
        let (color, opacity) = get_fill_and_opacity("#ff0000", 0.8);
        assert_eq!(color, "#ff0000");
        assert!((opacity - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_get_fill_and_opacity_8char_with_alpha() {
        // aa = 0x80 = 128 → 128/255 ≈ 0.502
        let (color, opacity) = get_fill_and_opacity("#ff000080", 1.0);
        assert_eq!(color, "#ff0000");
        let expected = 128.0_f64 / 255.0;
        assert!((opacity - expected).abs() < 0.001);
    }

    #[test]
    fn test_get_fill_and_opacity_fully_transparent() {
        // aa = 0x00 → effective = 0.0 → returned as "none"
        let (color, opacity) = get_fill_and_opacity("#ff000000", 1.0);
        assert_eq!(color, "none");
        assert_eq!(opacity, 1.0);
    }

    #[test]
    fn test_get_fill_and_opacity_non_hex() {
        // Named colors fall through to the base opacity unchanged
        let (color, opacity) = get_fill_and_opacity("red", 0.5);
        assert_eq!(color, "red");
        assert!((opacity - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_format_condition_equals() {
        let cond = json!(["==", ["get", "tipo"], "bosque"]);
        assert_eq!(format_condition(&cond).unwrap(), "tipo == bosque");
    }

    #[test]
    fn test_format_condition_has() {
        let cond = json!(["has", "nombre"]);
        assert_eq!(format_condition(&cond).unwrap(), "has nombre");
    }

    #[test]
    fn test_format_condition_not_has() {
        let cond = json!(["!", ["has", "nombre"]]);
        assert_eq!(format_condition(&cond).unwrap(), "without nombre");
    }

    #[test]
    fn test_parse_expression_plain_string() {
        let layer: Layer = serde_json::from_value(json!({"id": "lyr", "type": "fill"})).unwrap();
        let result = parse_expression(&layer, &json!("#abcdef")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "#abcdef");
    }

    #[test]
    fn test_parse_expression_plain_number() {
        let layer: Layer = serde_json::from_value(json!({"id": "lyr", "type": "fill"})).unwrap();
        let result = parse_expression(&layer, &json!(42)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "42");
    }

    #[test]
    fn test_deserialize_sprite_string() {
        let json = r#"{"layers": [], "sprite": "https://example.com/sprites"}"#;
        let style: Style = serde_json::from_str(json).unwrap();
        assert_eq!(
            style.sprite,
            vec!["https://example.com/sprites".to_string()]
        );
    }

    #[test]
    fn test_deserialize_sprite_array() {
        let json =
            r#"{"layers": [], "sprite": ["https://example.com/a", "https://example.com/b"]}"#;
        let style: Style = serde_json::from_str(json).unwrap();
        assert_eq!(
            style.sprite,
            vec![
                "https://example.com/a".to_string(),
                "https://example.com/b".to_string()
            ]
        );
    }

    #[test]
    fn test_deserialize_sprite_missing() {
        let json = r#"{"layers": []}"#;
        let style: Style = serde_json::from_str(json).unwrap();
        assert!(style.sprite.is_empty());
    }
}
