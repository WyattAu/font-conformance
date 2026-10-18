//! Single-case reproduction runner: `cargo run --example case -- <name>`
//! runs one hostile case through fill/stroke/fill so a hang or finding can be
//! reproduced in isolation under `timeout`.

use font_conformance::differential::raster;
use font_conformance::hostile;

fn main() {
    let name = std::env::args().nth(1).unwrap_or_default();
    let deep = hostile::is_deep();

    if let Some(case) = hostile::path_cases(deep).iter().find(|c| c.name == name) {
        let op = std::env::args().nth(2).unwrap_or_else(|| "all".to_owned());
        println!("path case {name}: op={op}…");
        let single = if op == "all" {
            Vec::from(std::slice::from_ref(case))
        } else {
            Vec::new()
        };
        let report = raster::estate_internal(&single);
        println!("  all: {}", report.summary());
        match op.as_str() {
            "fill" | "all" => {
                let path = hostile::font_shape_path(&case.geom);
                match font_shape::fill_path(
                    &path,
                    font_shape::FillRule::NonZero,
                    case.width,
                    case.height,
                ) {
                    Ok(m) => println!("  fill: ok, {} bytes, ink {}", m.len(), font_shape::mask_coverage(&m)),
                    Err(e) => println!("  fill: err {}", font_shape::message(&e)),
                }
                if op == "fill" {
                    return;
                }
            }
            _ => {}
        }
        if matches!(op.as_str(), "stroke" | "all") {
            let path = hostile::font_shape_path(&case.geom);
            let stroked = font_shape::stroke_path(&path, 1.0, font_shape::StrokeStyle::new(1.0));
            println!("  stroke: ok, {} commands", stroked.commands().len());
            if op == "stroke" {
                return;
            }
        }
        for f in &report.findings {
            println!("  FINDING [{}] {}", f.oracle, f.detail);
        }
        return;
    }
    let bytes = hostile::byte_cases();
    if let Some(case) = bytes.iter().find(|c| c.name == name) {
        println!("byte case {name}: {} bytes…", case.bytes.len());
        println!(
            "  font-parse: {:?}",
            font_parse::FontRef::new(&case.bytes).is_ok()
        );
        println!(
            "  font-model: {:?}",
            font_model::parse::from_sfnt(&case.bytes).is_ok()
        );
        return;
    }
    eprintln!("no case named {name:?} (deep={deep}); available path cases:");
    for c in hostile::path_cases(deep) {
        eprintln!("  {}", c.name);
    }
}
