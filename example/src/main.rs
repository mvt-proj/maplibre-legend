use maplibre_legend::{LegendConfig, MapLibreLegend};
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LegendConfig {
        default_width: 250,
        ..Default::default()
    };

    for i in 1..=6 {
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

    Ok(())
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
