//! FCN-004: the hostile corpus is Err-not-panic, everywhere.
//!
//! There is deliberately no `catch_unwind` here: these crates forbid panic at
//! the lint level, and a panic under hostile input **is** the finding — the
//! test going red the loud way is the oracle working.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    // Justification: test targets lift the deny-level lints at file scope per
    // the estate decoder-discipline pattern; assertions need unwraps by design.
    clippy::panic
)]

use font_conformance::differential::{parse, raster};
use font_conformance::hostile;

/// Every hostile byte buffer passes through the parser and the model assembly
/// without panicking (typed errors are the expected outcome).
#[test]
fn hostile_bytes_never_panic() {
    let fonts: Vec<parse::NamedFont> = hostile::byte_cases()
        .into_iter()
        .map(|c| parse::NamedFont {
            name: c.name.to_owned(),
            bytes: c.bytes,
        })
        .collect();

    for font in &fonts {
        // Raw views.
        let _ = font_parse::FontRef::new(&font.bytes);
        // Model assembly.
        let _ = font_model::parse::from_sfnt(&font.bytes);
        // Table-level parses, when the directory claims the table exists.
        if let Ok(r) = font_parse::FontRef::new(&font.bytes) {
            for tag in [
                font_parse::TableTag::Head,
                font_parse::TableTag::Hhea,
                font_parse::TableTag::Maxp,
            ] {
                if let Some(bytes) = r.table(tag) {
                    match tag {
                        font_parse::TableTag::Head => {
                            let _ = font_parse::HeadTable::parse(bytes);
                        }
                        font_parse::TableTag::Hhea => {
                            let _ = font_parse::HheaTable::parse(bytes);
                        }
                        font_parse::TableTag::Maxp => {
                            let _ = font_parse::MaxpTable::parse(bytes);
                        }
                        _ => unreachable!("exhaustive over the three tags above"),
                    }
                }
            }
        }
    }
}

/// The estate-internal consistency rule holds over the hostile set: bytes the
/// parser rejects never yield a model.
#[test]
fn hostile_bytes_respect_the_layer_rule() {
    let fonts: Vec<parse::NamedFont> = hostile::byte_cases()
        .into_iter()
        .map(|c| parse::NamedFont {
            name: c.name.to_owned(),
            bytes: c.bytes,
        })
        .collect();
    let report = parse::estate_internal(&fonts);
    assert!(
        report.findings.is_empty(),
        "layer rule violated: {:#?}",
        report.findings
    );
}

/// Every hostile geometry case fills, strokes, and stroke-fills without
/// panicking, and the structural report is clean.
#[test]
fn hostile_paths_never_panic_and_fill_total() {
    let cases = hostile::path_cases(hostile::is_deep());
    let report = raster::estate_internal(&cases);
    assert!(
        report.findings.is_empty(),
        "structural violations: {:#?}",
        report.findings
    );
}

/// The positive control: a half-pixel-shifted rectangle is exactly 128 on its
/// leading edge — the property from font-shape's docs must hold here too.
#[test]
fn positive_control_half_pixel_rect() {
    let cases = hostile::path_cases(false);
    let case = cases
        .iter()
        .find(|c| c.name == "rect_half_pixel")
        .expect("rect_half_pixel is in the corpus");
    let path = font_conformance::hostile::font_shape_path(&case.geom);
    let mask = font_shape::fill_path(&path, font_shape::FillRule::NonZero, 8, 1)
        .expect("finite control geometry fills");
    assert_eq!(mask[0], 128, "half a pixel of coverage is exactly 128");
    assert_eq!(mask[1], 255);
    assert_eq!(mask[4], 128);
    assert_eq!(mask[5], 0);
}
