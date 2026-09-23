use maplibre_legend::{LegendConfig, MapLibreLegend};
use serde_json::json;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LegendConfig {
        default_width: 250,
        ..Default::default()
    };

    for i in 1..=7 {
        let style_json = fs::read_to_string(format!("style{}.json", i)).await?;
        let legend = MapLibreLegend::new(&style_json, config.clone()).await?;
        let combined = legend.render_all(true)?;
        fs::write(format!("combined_{}.svg", i), combined).await?;
    }

    let style_json = fs::read_to_string("style1.json").await?;
    let legend = MapLibreLegend::new(
        &style_json,
        LegendConfig {
            default_width: 250,
            include_raster: true,
            ..Default::default()
        },
    )
    .await?;
    let svg = legend.render_layer("vs2023", Some(true))?;
    fs::write("vs2023.svg", svg).await?;

    run_sdf_icon_color_demo(config).await?;

    Ok(())
}

/// Demonstrates the `icon-color` tinting added for SDF sprites (as produced by
/// `spreet --sdf`). Spins up a tiny local HTTP server so `MapLibreLegend` can load the sprite
/// exactly like it would a real one, then renders the same marker pixels four ways:
/// as a plain (non-SDF) sprite entry -- which reproduces the old blurry, always-black
/// rendering -- and as an SDF entry with no color, a red tint, and a blue tint.
async fn run_sdf_icon_color_demo(
    mut config: LegendConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let icon = generate_sdf_marker(64);
    let mut png_bytes = Vec::new();
    icon.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )?;

    let sprite_index = json!({
        "marker-raw": { "x": 0, "y": 0, "width": 64, "height": 64 },
        "marker-sdf": { "x": 0, "y": 0, "width": 64, "height": 64, "sdf": true }
    })
    .to_string();

    let sprite_url = serve_sprite(png_bytes, sprite_index).await;
    let style_json = sdf_demo_style(&sprite_url);

    config.default_width = 300;
    let legend = MapLibreLegend::new(&style_json, config).await?;
    let combined = legend.render_all(false)?;
    fs::write("combined_sdf_icon_color.svg", combined).await?;

    Ok(())
}

/// Renders a "marker" icon whose alpha channel encodes a signed distance field with the shape
/// boundary at 0.5, the same convention `spreet --sdf` output uses. RGB is fixed to black,
/// matching real SDF spritesheets (the color is meaningless without `icon-color` tinting).
fn generate_sdf_marker(size: u32) -> image::RgbaImage {
    let center = size as f32 / 2.0;
    let shape_radius = size as f32 * 0.35;
    let sdf_band = size as f32 * 0.12;
    image::RgbaImage::from_fn(size, size, |x, y| {
        let dx = x as f32 + 0.5 - center;
        let dy = y as f32 + 0.5 - center;
        let signed_dist = shape_radius - (dx * dx + dy * dy).sqrt();
        let t = (0.5 + signed_dist / (2.0 * sdf_band)).clamp(0.0, 1.0);
        image::Rgba([0, 0, 0, (t * 255.0).round() as u8])
    })
}

/// Serves a single PNG + JSON sprite pair over HTTP so it can be loaded through the library's
/// normal (network-based) sprite loading path. Returns the sprite base URL (without extension).
async fn serve_sprite(png: Vec<u8>, json: String) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let png = std::sync::Arc::new(png);
    let json = std::sync::Arc::new(json);

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let png = png.clone();
            let json = json.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let Ok(n) = socket.read(&mut buf).await else {
                    return;
                };
                let request = String::from_utf8_lossy(&buf[..n]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let (content_type, body): (&str, &[u8]) = if path.ends_with(".json") {
                    ("application/json", json.as_bytes())
                } else {
                    ("image/png", png.as_slice())
                };
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content_type,
                    body.len()
                );
                let _ = socket.write_all(header.as_bytes()).await;
                let _ = socket.write_all(body).await;
            });
        }
    });

    format!("http://{}/sprite", addr)
}

fn sdf_demo_style(sprite_url: &str) -> String {
    json!({
        "version": 8,
        "name": "SDF icon-color demo",
        "sprite": sprite_url,
        "sources": {
            "dummy-source": {
                "type": "geojson",
                "data": { "type": "FeatureCollection", "features": [] }
            }
        },
        "layers": [
            {
                "id": "marker-raw",
                "type": "symbol",
                "source": "dummy-source",
                "layout": { "icon-image": "marker-raw" },
                "metadata": { "legend": { "label": "Sin marcar como SDF (bug anterior)" } }
            },
            {
                "id": "marker-sdf-default",
                "type": "symbol",
                "source": "dummy-source",
                "layout": { "icon-image": "marker-sdf" },
                "metadata": { "legend": { "label": "SDF sin icon-color (negro por defecto)" } }
            },
            {
                "id": "marker-sdf-red",
                "type": "symbol",
                "source": "dummy-source",
                "layout": { "icon-image": "marker-sdf" },
                "paint": { "icon-color": "#e63946" },
                "metadata": { "legend": { "label": "SDF con icon-color rojo" } }
            },
            {
                "id": "marker-sdf-blue",
                "type": "symbol",
                "source": "dummy-source",
                "layout": { "icon-image": "marker-sdf" },
                "paint": { "icon-color": "#457b9d" },
                "metadata": { "legend": { "label": "SDF con icon-color azul" } }
            }
        ]
    })
    .to_string()
}

// use maplibre_legend::{LegendConfig, MapLibreLegend};
// use std::fs;
//
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let config = LegendConfig { default_width: 250, ..Default::default() };
//     for i in 1..=4 {
//         let style_json = fs::read_to_string(format!("style{}.json", i))?;
//         let legend = MapLibreLegend::new(&style_json, config.clone())?;
//         let combined = legend.render_all(true)?;
//         fs::write(format!("combined_{}.svg", i), combined)?;
//     }
//
//     let style_json = fs::read_to_string("style1.json")?;
//     let legend = MapLibreLegend::new(
//         &style_json,
//         LegendConfig { default_width: 250, include_raster: true, ..Default::default() },
//     )?;
//     let svg = legend.render_layer("vs2023", Some(true))?;
//     fs::write("vs2023.svg", svg)?;
//
//     Ok(())
// }
