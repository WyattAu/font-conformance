//! FCN-002 smoke: the parse differentials run over whatever corpus is
//! attested and cached. No cached corpus → the test reports and passes (the
//! nightly deep run owns the full-corpus verdict; FCN-006).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    // Justification: test targets lift the deny-level lints at file scope.
    clippy::panic
)]

use font_conformance::differential::parse;
use font_conformance::manifest::Corpus;

fn cached_fonts() -> Option<Vec<parse::NamedFont>> {
    let corpus = Corpus::from_repo_default().expect("manifest parses");
    let failures = corpus.verify_all().expect("cache dir resolves");
    if !failures.is_empty() {
        eprintln!(
            "corpus not fully attested/cached ({} entries missing or stale) — skipping: {failures:?}",
            failures.len()
        );
        return None;
    }
    let mut fonts = Vec::new();
    for entry in &corpus.fonts {
        let bytes = corpus.verified_bytes(entry).expect("verified above");
        fonts.push(parse::NamedFont {
            name: entry.name.clone(),
            bytes,
        });
    }
    Some(fonts)
}

#[test]
fn estate_internal_rule_over_corpus() {
    let Some(fonts) = cached_fonts() else {
        return;
    };
    let report = parse::estate_internal(&fonts);
    assert!(
        report.findings.is_empty(),
        "font-parse/font-model disagreement over the corpus: {:#?}",
        report.findings
    );
    println!("estate_internal: {}", report.summary());
}

#[cfg(feature = "rival-fontations")]
#[test]
fn parse_matches_read_fonts_over_corpus() {
    let Some(fonts) = cached_fonts() else {
        return;
    };
    let report = parse::vs_read_fonts(&fonts);
    assert!(
        report.findings.is_empty(),
        "font-parse/read-fonts disagreement over the corpus: {:#?}",
        report.findings
    );
    println!("vs_read_fonts: {} (not compared: {:?})", report.summary(), report.not_compared);
}

#[cfg(feature = "rival-tiny-skia")]
#[test]
fn raster_matches_tiny_skia_over_hostile_set() {
    use font_conformance::report::Finding;

    // The triage ledger (FCN-003 ratchet). Entries: (case, oracle prefix,
    // status). STRICT rules:
    // - a finding whose oracle starts with "font-shape:" is estate-owned and
    //   can NEVER be listed here — that assertion is below, unconditional;
    // - every tracked finding must match exactly one entry, and every entry
    //   must be matched by a live finding — when upstream fixes a bug, its
    //   entry starts failing the count assert and must be removed. The ledger
    //   cannot silently grow and cannot silently rot.
    const TRACKED: &[(&str, &str, &str)] = &[
        (
            "nan_polygon",
            "tiny-skia",
            "tiny-skia 0.12 PANICS (alpha_runs.rs unwrap) on NaN coords; font-shape returns a mask — upstream report pending",
        ),
        (
            "inf_polygon",
            "tiny-skia",
            "tiny-skia 0.12 refuses inf coords (PathBuilder::finish -> None) — graceful refusal, recorded for parity",
        ),
        (
            "huge_extent_polygon",
            "tiny-skia",
            "tiny-skia 0.12 PANICS on ±1e30 extents; font-shape returns a mask — upstream report pending",
        ),
        (
            "nan_quad_control",
            "tiny-skia",
            "tiny-skia 0.12 refuses NaN curve controls (finish -> None) — graceful refusal, recorded for parity",
        ),
    ];

    // Settled by measurement (2026-10-18), so no longer in the ledger: the
    // bowtie's mean |coverage delta| of 6.27 was the ANTIALIASING MODEL, not
    // a rasterizer bug. On the minimal witness — a triangle whose pixel the
    // diagonal halves exactly — font-shape returns 128 against an exact 127.5
    // (0.5 off, pure rounding) while tiny-skia returns 160 (32.5 off,
    // sampled coverage). Both honour total ink; neither is obliged to agree
    // per pixel on a sloped edge. The differential now checks the ink
    // invariant (`INK_TOLERANCE`) and files per-pixel divergence as an
    // AA-model note. font-shape's analytic coverage is the accurate one.

    let cases =
        font_conformance::hostile::path_cases(font_conformance::hostile::is_deep());
    let report = font_conformance::differential::raster::vs_tiny_skia(&cases);

    eprintln!("differential report: {}", report.summary());
    for d in &report.aa_differences {
        eprintln!(
            "  [aa-model] {}: mean |coverage delta| {:.2} (ink invariant honoured)",
            d.case, d.mean_delta
        );
    }
    for f in &report.findings {
        eprintln!("  [{}] {}: {}", f.oracle, f.case, f.detail);
    }

    // Estate-owned findings are never ratcheted — red, always.
    let estate: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|f| f.oracle.starts_with("font-shape:"))
        .collect();
    assert!(
        estate.is_empty(),
        "estate-owned raster findings (never waivable): {estate:#?}"
    );

    // Everything else is tracked: bijection between live findings and ledger.
    let tracked: Vec<&Finding> = report.findings.iter().collect();
    for f in &tracked {
        assert!(
            TRACKED.iter().any(|(c, o, _)| f.case == *c && f.oracle.starts_with(o)),
            "untracked finding needs triage + a ledger entry: {f:#?}"
        );
    }
    for (c, o, status) in TRACKED {
        assert!(
            tracked.iter().any(|f| f.case == *c && f.oracle.starts_with(o)),
            "ledger entry ({c}, {o}) matched no live finding — upstream fixed it: remove the entry ({status})"
        );
    }
    assert_eq!(
        tracked.len(),
        TRACKED.len(),
        "finding/ledger count mismatch — reconcile the ledger"
    );
    println!("vs_tiny_skia: {} ({} tracked)", report.summary(), tracked.len());
}
