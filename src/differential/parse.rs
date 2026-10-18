//! Parse differential (FCN-002): `font-parse` vs `read-fonts` (fontations)
//! on the same bytes, plus the estate-internal consistency rule between the
//! L0 parser and the L1 model assembly.

use crate::report::Report;

/// One font: a greppable name and the exact bytes.
#[derive(Debug, Clone)]
pub struct NamedFont {
    /// Stable name (hostile case name or corpus font name).
    pub name: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// Estate-internal rule: anything `font-parse` rejects at the SFNT level must
/// also fail `font-model`'s assembly — the model is never built on bytes the
/// parser rejected. (The converse is legal: a valid SFNT directory may still
/// lack the tables a full model needs.)
///
/// # Panics
/// Never — hostile input is the subject under test.
pub fn estate_internal(fonts: &[NamedFont]) -> Report {
    let mut report = Report::new();
    for font in fonts {
        let parse_ok = font_parse::FontRef::new(&font.bytes).is_ok();
        let model_ok = font_model::parse::from_sfnt(&font.bytes).is_ok();
        if !parse_ok && model_ok {
            report.finding(
                &font.name,
                "font-parse/font-model",
                format!(
                    "parser rejected the SFNT but the model assembly accepted it \
                     (parse_ok={parse_ok}, model_ok={model_ok})"
                ),
            );
        } else {
            report.agree();
        }
    }
    report
}

/// Raw `head` scalars as both parsers should read them.
#[cfg(feature = "rival-fontations")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeadScalars {
    units_per_em: u16,
    x_min: i16,
    y_min: i16,
    index_to_loc_format: i32,
}

/// Compare `font-parse` against fontations' `read-fonts` on every font:
/// accept/reject agreement first, then `head` and `hhea` scalar equality.
/// Requires the `rival-fontations` feature.
///
/// # Panics
/// Never — hostile input is the subject under test.
#[cfg(feature = "rival-fontations")]
pub fn vs_read_fonts(fonts: &[NamedFont]) -> Report {
    let mut report = Report::new();
    for font in fonts {
        let ours = font_parse::FontRef::new(&font.bytes);
        let theirs = read_fonts::FontRef::new(&font.bytes);
        match (ours, theirs) {
            (Err(_), Err(_)) => report.agree(),
            (Ok(_), Ok(_)) => compare_tables(font, &mut report),
            (ours_ok, theirs_ok) => {
                report.finding(
                    &font.name,
                    "font-parse/read-fonts",
                    format!("accept/reject disagreement: ours={ours_ok:?} theirs={}", theirs_ok.is_ok()),
                );
            }
        }
    }
    report
}

#[cfg(feature = "rival-fontations")]
fn compare_tables(font: &NamedFont, report: &mut Report) {
    use read_fonts::TableProvider;

    let ours = font_parse::FontRef::new(&font.bytes);
    let theirs = read_fonts::FontRef::new(&font.bytes);
    let (Some(ours), Ok(theirs)) = (ours.ok(), theirs) else {
        report.finding(
            &font.name,
            "font-parse/read-fonts",
            "both accepted at the directory level but one failed on re-parse".to_owned(),
        );
        return;
    };

    // head scalars, when both sides can see the table.
    match ours.table(font_parse::TableTag::Head) {
        Some(head_bytes) => {
            let ours_head = font_parse::HeadTable::parse(head_bytes);
            let theirs_head = theirs.head();
            match (ours_head, theirs_head) {
                (Ok(o), Ok(t)) => {
                    let scalars = |upem: u16, x_min: i16, y_min: i16, itlf: i32| HeadScalars {
                        units_per_em: upem,
                        x_min,
                        y_min,
                        index_to_loc_format: itlf,
                    };
                    let o = scalars(
                        o.units_per_em().unwrap_or(0),
                        o.x_min().unwrap_or(0),
                        o.y_min().unwrap_or(0),
                        o.index_to_loc_format().map(i32::from).unwrap_or(i32::MIN),
                    );
                    let t = scalars(
                        t.units_per_em(),
                        t.x_min(),
                        t.y_min(),
                        i32::from(t.index_to_loc_format()),
                    );
                    if o == t {
                        report.agree();
                    } else {
                        report.finding(
                            &font.name,
                            "font-parse/read-fonts:head",
                            format!("scalar disagreement: ours={o:?} theirs={t:?}"),
                        );
                    }
                }
                (o, t) => {
                    if o.is_err() && t.is_err() {
                        report.agree();
                    } else {
                        report.finding(
                            &font.name,
                            "font-parse/read-fonts:head",
                            format!(
                                "head accept/reject disagreement: ours={:?} theirs={:?}",
                                o.is_ok(),
                                t.is_ok()
                            ),
                        );
                    }
                }
            }
        }
        None => report.not_compared("head (absent in this font)"),
    }

    // hhea verticals.
    match ours.table(font_parse::TableTag::Hhea) {
        Some(hhea_bytes) => {
            let ours_hhea = font_parse::HheaTable::parse(hhea_bytes);
            let theirs_hhea = theirs.hhea();
            match (ours_hhea, theirs_hhea) {
                (Ok(o), Ok(t)) => {
                    let (t_asc, t_desc, t_gap) = (
                        t.ascender().to_i16(),
                        t.descender().to_i16(),
                        t.line_gap().to_i16(),
                    );
                    let (o_asc, o_desc, o_gap) = (o.ascender(), o.descender(), o.line_gap());
                    if o_asc == Ok(t_asc) && o_desc == Ok(t_desc) && o_gap == Ok(t_gap) {
                        report.agree();
                    } else {
                        report.finding(
                            &font.name,
                            "font-parse/read-fonts:hhea",
                            format!(
                                "vertical metrics disagreement: ours=({o_asc:?},{o_desc:?},{o_gap:?}) theirs=({t_asc},{t_desc},{t_gap})"
                            ),
                        );
                    }
                }
                (o, t) => {
                    if o.is_err() && t.is_err() {
                        report.agree();
                    } else {
                        report.finding(
                            &font.name,
                            "font-parse/read-fonts:hhea",
                            format!(
                                "hhea accept/reject disagreement: ours={:?} theirs={:?}",
                                o.is_ok(),
                                t.is_ok()
                            ),
                        );
                    }
                }
            }
        }
        None => report.not_compared("hhea (absent in this font)"),
    }

    // maxp num_glyphs: font-parse's accessor surface is still growing; the
    // comparison lands with it (tracked in the repo README roadmap).
    report.not_compared("maxp num_glyphs (font-parse accessor pending)");
}

/// Non-feature builds report the gap honestly instead of pretending.
#[cfg(not(feature = "rival-fontations"))]
pub fn vs_read_fonts(_fonts: &[NamedFont]) -> Report {
    let mut report = Report::new();
    report.not_compared("read-fonts differential (build with --features rival-fontations)");
    report
}
