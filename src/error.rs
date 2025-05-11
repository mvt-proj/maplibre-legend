use thiserror::Error;

#[derive(Error, Debug)]
pub enum LegendError {
    #[error("Failed to fetch sprite PNG: {0}")]
    PngFetch(reqwest::Error),
    #[error("Failed to read sprite PNG: {0}")]
    PngRead(reqwest::Error),
    #[error("Failed to load sprite image: {0}")]
    ImageLoad(image::ImageError),
    #[error("Failed to fetch sprite JSON: {0}")]
    JsonFetch(reqwest::Error),
    #[error("Failed to parse sprite JSON: {0}")]
    JsonParse(reqwest::Error),
}
