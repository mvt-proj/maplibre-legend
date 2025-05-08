use maplibre_legend::MapLibreLegend;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let style_json = fs::read_to_string("style3.json")?;

    let legend = MapLibreLegend::new(&style_json, 250,40, true, true)?;

    // if let Some(svg) = legend.render_layer("vs2023", Some(true)) {
    //     fs::write("vs2023.svg", svg)?;
    // }

    let combined = legend.render_all(false);
    fs::write("combined.svg", combined)?;

    Ok(())
}
