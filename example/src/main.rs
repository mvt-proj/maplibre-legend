use maplibre_legend::MapLibreLegend;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let style_json = fs::read_to_string("style.json")?;

    let legend = MapLibreLegend::new(&style_json, 250,40, true, true)?;

    if let Some(svg) = legend.render_layer("ejes_calles", Some(true)) {
        fs::write("ejes_calles.svg", svg)?;
    }

    if let Some(svg) = legend.render_layer("parcelario-fill", Some(true)) {
        fs::write("parcelario_fill.svg", svg)?;
    }

    if let Some(svg) = legend.render_layer("osm-base", Some(true)) {
        fs::write("osm.svg", svg)?;
    }

    if let Some(svg) = legend.render_layer("puntoscc", Some(true)) {
        fs::write("puntoscc.svg", svg)?;
    }
    if let Some(svg) = legend.render_layer("vs2023", Some(true)) {
        fs::write("vs2023.svg", svg)?;
    }



    let combined = legend.render_all();
    fs::write("combined.svg", combined)?;

    Ok(())
}
