use maplibre_legend::MapLibreLegend;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    for i in 1..=5 {
        let style_json = fs::read_to_string(format!("style{}.json", i)).await?;
        let legend = MapLibreLegend::new(&style_json, 250, 40, true, false).await?;
        let combined = legend.render_all(true)?;
        fs::write(format!("combined_{}.svg", i), combined).await?;
    }

    let style_json = fs::read_to_string("style1.json").await?;
    let legend = MapLibreLegend::new(&style_json, 250, 40, true, true).await?;
    let svg = legend.render_layer("vs2023", Some(true))?;
    fs::write("vs2023.svg", svg).await?;

    Ok(())
}

// use maplibre_legend::MapLibreLegend;
// use std::fs;
//
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     for i in 1..=4 {
//         let style_json = fs::read_to_string(format!("style{}.json", i))?;
//         let legend = MapLibreLegend::new(&style_json, 250, 40, true, false)?;
//         let combined = legend.render_all(true)?;
//         fs::write(format!("combined_{}.svg", i), combined)?;
//     }
//
//     let style_json = fs::read_to_string("style1.json")?;
//     let legend = MapLibreLegend::new(&style_json, 250, 40, true, true)?;
//     let svg = legend.render_layer("vs2023", Some(true))?;
//     fs::write("vs2023.svg", svg)?;
//
//     Ok(())
// }
