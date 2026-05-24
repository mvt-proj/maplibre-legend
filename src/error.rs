use image::ImageError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

/// Errors that can occur while parsing a MapLibre style or rendering a legend.
#[derive(Error, Debug)]
pub enum LegendError {
    /// HTTP request to fetch the sprite PNG failed.
    #[error("Failed to fetch sprite PNG: {0}")]
    PngFetch(ReqwestError),
    /// Reading the bytes from the sprite PNG response failed.
    #[error("Failed to read sprite PNG: {0}")]
    PngRead(ReqwestError),
    /// Decoding the sprite image data failed.
    #[error("Failed to load sprite image: {0}")]
    ImageLoad(ImageError),
    /// HTTP request to fetch the sprite JSON failed.
    #[error("Failed to fetch sprite JSON: {0}")]
    JsonFetch(ReqwestError),
    /// Parsing the sprite JSON response failed.
    #[error("Failed to parse sprite JSON: {0}")]
    JsonParse(ReqwestError),
    /// Generic HTTP fetch error.
    #[error("Failed to fetch url: {0}")]
    Fetch(ReqwestError),
    /// A required JSON field is missing, has the wrong type, or contains an unexpected value.
    #[error("Invalid JSON object: {0}")]
    InvalidJson(String),
    /// A MapLibre paint expression is malformed or uses an unsupported operator.
    #[error("Invalid expression: {0}")]
    InvalidExpression(String),
    /// Top-level style JSON could not be deserialized into a [`Style`](crate::common::Style).
    #[error("JSON deserialization failed: {0}")]
    Deserialization(SerdeJsonError),
}
