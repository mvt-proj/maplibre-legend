use serde::Deserialize;
use serde_json::Value;
use svg::Document;
use svg::node::element::{Line, Text as SvgText};

enum ExpressionKind {
    Match,
    Case,
    Interpolate,
}

impl ExpressionKind {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "match" => Some(Self::Match),
            "case" => Some(Self::Case),
            "interpolate" => Some(Self::Interpolate),
            _ => None,
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
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Style {
    pub layers: Vec<Layer>,
}

fn get_legend_object(layer: &Layer) -> Option<&serde_json::Map<String, Value>> {
    layer.metadata.as_ref()?.get("legend")?.as_object()
}

fn get_layer_label(layer: &Layer) -> String {
    get_legend_object(layer)
        .and_then(|l| l.get("label"))
        .and_then(|l| l.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| layer.id.clone())
}

fn get_layer_default_label(layer: &Layer) -> String {
    get_legend_object(layer)
        .and_then(|l| l.get("default"))
        .and_then(|l| l.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| layer.id.clone())
}

fn get_custom_labels(layer: &Layer) -> Vec<String> {
    get_legend_object(layer)
        .and_then(|l| l.get("custom-labels"))
        .and_then(|l| l.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

pub fn render_label(
    layer: &Layer,
    doc: &mut Document,
    x: Option<u32>,
    y: Option<u32>,
    is_bold: Option<bool>,
) {
    let label = get_layer_label(layer);
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

pub fn extract_color(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(_) => Some("#cccccc".to_string()),
        _ => None,
    }
}

pub fn format_condition(cond: &serde_json::Value) -> String {
    let arr = cond.as_array();
    if let Some(expr) = arr {
        if expr.is_empty() {
            return "cond".to_string();
        }

        let op = expr[0].as_str().unwrap_or("");

        match op {
            "!" => {
                if let Some(inner) = expr.get(1) {
                    if let Some(inner_arr) = inner.as_array() {
                        if inner_arr.get(0) == Some(&serde_json::Value::String("has".into())) {
                            if let Some(field) = inner_arr.get(1).and_then(|v| v.as_str()) {
                                return format!("without {}", field);
                            }
                        }
                    }
                }
                return "undefined".to_string();
            }
            "has" => {
                if let Some(field) = expr.get(1).and_then(|v| v.as_str()) {
                    return format!("has {}", field);
                }
            }
            "==" | "!=" | ">" | ">=" | "<" | "<=" => {
                if let Some(get_expr) = expr.get(1).and_then(|v| v.as_array()) {
                    if get_expr.get(0) == Some(&serde_json::Value::String("get".into())) {
                        if let Some(field) = get_expr.get(1).and_then(|v| v.as_str()) {
                            let value = match &expr[2] {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Number(n) => n.to_string(),
                                serde_json::Value::Bool(b) => b.to_string(),
                                _ => "value".to_string(),
                            };
                            return format!("{field} {op} {value}");
                        }
                    }
                }
            }
            _ => {}
        }
    }
    "cond".to_string()
}

fn parse_match(layer: &Layer, arr: &Vec<Value>) -> Option<Vec<(String, String)>> {
    if arr.len() < 4 {
        return None;
    }

    let _field = arr.get(1).and_then(|v| {
        if let Some(get_arr) = v.as_array() {
            if get_arr.len() == 2 && get_arr[0] == "get" {
                return get_arr[1].as_str();
            }
        }
        None
    })?;

    let labels = get_custom_labels(layer);
    let mut result = Vec::new();
    let mut i = 2;
    let mut label_index = 0;

    while i + 1 < arr.len() - 1 {
        let value = arr.get(i)?.as_str()?;
        let color = arr.get(i + 1)?.as_str().unwrap_or("#cccccc").to_string();
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
            get_layer_default_label(layer)
        };
        result.push((default_label, default_color.to_string()));
    }

    Some(result)
}

fn parse_case(layer: &Layer, arr: &Vec<Value>) -> Option<Vec<(String, String)>> {
    let labels = get_custom_labels(layer);
    let mut result = Vec::new();
    let mut i = 1;
    let mut label_index = 0;

    while i + 1 < arr.len() {
        if let (Some(cond), Some(color)) = (arr.get(i), arr.get(i + 1)) {
            let color = color.as_str().unwrap_or("#cccccc").to_string();
            let label = if !labels.is_empty() && label_index < labels.len() {
                labels[label_index].clone()
            } else {
                format_condition(cond)
            };
            result.push((label, color));
            label_index += 1;
        }
        i += 2;
    }

    if arr.len() % 2 == 0 {
        if let Some(default_color) = arr.last().and_then(|v| v.as_str()) {
            let default_label = if !labels.is_empty() && label_index < labels.len() {
                labels[label_index].clone()
            } else {
                get_layer_default_label(layer)
            };
            result.push((default_label, default_color.to_string()));
        }
    }

    Some(result)
}

fn parse_interpolate(layer: &Layer, arr: &Vec<Value>) -> Option<Vec<(String, String)>> {
    if arr.len() < 4 {
        return None;
    }
    let labels = get_custom_labels(layer);

    let field = arr.get(2).and_then(|v| {
        if let Some(get_arr) = v.as_array() {
            if get_arr.len() == 2 && get_arr[0] == "get" {
                return get_arr[1].as_str();
            }
        }
        None
    })?;

    let mut result = Vec::new();
    let mut i = 3;
    let mut label_index = 0;
    while i + 1 < arr.len() {
        let val = arr.get(i)?.as_f64()?;
        let color = arr.get(i + 1)?.as_str().unwrap_or("#cccccc").to_string();
        let label = if !labels.is_empty() && label_index < labels.len() {
            labels[label_index].clone()
        } else {
            format!("{field} ≥ {val}")
        };
        result.push((label, color));
        i += 2;
        label_index += 1;
    }

    Some(result)
}

pub fn parse_expression(layer: &Layer, value: &serde_json::Value) -> Option<Vec<(String, String)>> {
    let arr = value.as_array()?;
    let first = arr.first()?.as_str()?;

    match ExpressionKind::from_str(first)? {
        ExpressionKind::Match => parse_match(layer, arr),
        ExpressionKind::Case => parse_case(layer, arr),
        ExpressionKind::Interpolate => parse_interpolate(layer, arr),
    }
}
