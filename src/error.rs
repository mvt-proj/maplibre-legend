use image::ImageError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LegendError {
    #[error("Failed to fetch sprite PNG: {0}")]
    PngFetch(ReqwestError),
    #[error("Failed to read sprite PNG: {0}")]
    PngRead(ReqwestError),
    #[error("Failed to load sprite image: {0}")]
    ImageLoad(ImageError),
    #[error("Failed to fetch sprite JSON: {0}")]
    JsonFetch(ReqwestError),
    #[error("Failed to parse sprite JSON: {0}")]
    JsonParse(ReqwestError),
    #[error("Failed to fetch url: {0}")]
    Fetch(ReqwestError),
    #[error("Invalid JSON object: {0}")]
    InvalidJson(String),
    #[error("Invalid expression: {0}")]
    InvalidExpression(String),
    #[error("JSON deserialization failed: {0}")]
    Deserialization(SerdeJsonError),
}
