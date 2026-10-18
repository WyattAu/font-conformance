//! Raster differential (FCN-003): `font-shape` vs the rival rasterizers on
//! the same geometry spec — coverage tolerance on finite geometry,
//! Err-not-panic everywhere, and throughput baselines recorded in-commit by
//! the criterion benches (added with the first rival).

use crate::hostile::{font_shape_path, PathCase};
#[cfg(feature = "rival-tiny-skia")]
use crate::hostile::Geom;
use crate::report::Report;
use std::sync::mpsc;
use std::time::Duration;

/// Wall-clock budget for one hostile case before it is declared a hang.
/// A hang is a finding (the worst kind — the crate never answers), not a
/// stuck CI run: the case runs in a watched thread and a budget overrun is
/// recorded and the suite moves on.
pub const CASE_BUDGET: Duration = Duration::from_secs(10);

fn run_case<T: Send + 'static>(case: &PathCase, f: impl FnOnce(&PathCase) -> T + Send + 'static) -> Option<T> {
    let case = case.clone();
    let (tx, rx) = mpsc::channel();
    // The thread is intentionally not joined on timeout: a hung case leaves
    // it behind (the test process exits after the run), and the finding is
    // recorded instead of the suite blocking forever.
    std::thread::spawn(move || {
        let _ = tx.send(f(&case));
    });
    rx.recv_timeout(CASE_BUDGET).ok()
}

/// Mean absolute per-pixel coverage difference tolerated between two
/// different-but-correct AA rasterizers, in 0..255 units. Generous by
/// design: this is a *disagreement* alarm, not a bit-equality gate.
pub const MEAN_ABS_TOLERANCE: f64 = 2.0;

/// Estate-internal structural properties over every hostile case: fills are
/// total (Ok mask of exactly `width*height`, or a typed Err), stroking is
/// total, and the stroked outline itself fills.
///
/// # Panics
/// Never — hostile geometry is the subject under test.
pub fn estate_internal(cases: &[PathCase]) -> Report {
    let mut report = Report::new();
    for case in cases {
        let watched = run_case(case, |case| {
            let path = font_shape_path(&case.geom);
            let fill_ok = font_shape::fill_path(
                &path,
                font_shape::FillRule::NonZero,
                case.width,
                case.height,
            );
            let stroked = font_shape::stroke_path(&path, 1.0, font_shape::StrokeStyle::new(1.0));
            let stroke_fill = font_shape::fill_path(
                &stroked,
                font_shape::FillRule::NonZero,
                case.width,
                case.height,
            );
            (fill_ok, stroke_fill)
        });
        let Some((fill, stroke_fill)) = watched else {
            report.finding(
                case.name,
                "font-shape:fill",
                format!("HANG: no result within {}s — the rasterizer never answered", CASE_BUDGET.as_secs()),
            );
            continue;
        };

        // fill: total, and a mask is exactly width*height when it returns one.
        match fill {
            Ok(mask) => {
                let expected = usize::try_from(u64::from(case.width) * u64::from(case.height))
                    .unwrap_or(usize::MAX);
                if mask.len() == expected {
                    report.agree();
                } else {
                    report.finding(
                        case.name,
                        "font-shape:fill",
                        format!("mask length {} != {width}*{height} = {expected}", mask.len(), width = case.width, height = case.height),
                    );
                }
            }
            Err(_e) => {
                // A typed Err is a legal outcome for hostile geometry.
                report.agree();
            }
        }

        // stroke: total by signature; its outline must also fill.
        match stroke_fill {
            Ok(_) => report.agree(),
            Err(e) => report.finding(
                case.name,
                "font-shape:stroke-then-fill",
                format!("stroked outline failed to fill: {}", font_shape::message(&e)),
            ),
        }
    }
    report
}

/// `font-shape` vs `tiny-skia` on every finite case (mean absolute coverage
/// difference) and Err-not-panic on every hostile case. Requires the
/// `rival-tiny-skia` feature.
///
/// # Panics
/// Never — hostile geometry is the subject under test.
#[cfg(feature = "rival-tiny-skia")]
pub fn vs_tiny_skia(cases: &[PathCase]) -> Report {
    let mut report = Report::new();
    for case in cases {
        let ours = font_shape_path(&case.geom);
        let Some(mut pixmap) = tiny_skia::Pixmap::new(case.width, case.height) else {
            report.finding(case.name, "tiny-skia", "Pixmap::new returned None".to_owned());
            continue;
        };
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(0, 0, 0, 255);
        let Some(theirs_path) = tiny_skia_path(&case.geom) else {
            report.finding(case.name, "tiny-skia", "PathBuilder::finish returned None".to_owned());
            continue;
        };
        // A rival panic is a finding (FCN-003), not an aborted run: hostile
        // input is the subject, and "panicked where font-shape returned a
        // mask" is exactly the comparison this suite exists to make.
        let rival_fill = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pixmap.fill_path(
                &theirs_path,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }));
        if rival_fill.is_err() {
            report.finding(
                case.name,
                "tiny-skia",
                "rival PANICKED on this case (font-shape returned a mask)".to_owned(),
            );
            continue;
        }

        // Their alpha channel is their coverage.
        let theirs_coverage: Vec<u8> = pixmap.pixels().iter().map(|p| p.alpha()).collect();

        if !case.finite {
            // Hostile math: the comparison is "both survived" — pixel data is
            // meaningless by construction. font-shape's own run happens below
            // so a panic surfaces as a test failure.
            let _ = font_shape::fill_path(
                &ours,
                font_shape::FillRule::NonZero,
                case.width,
                case.height,
            );
            report.agree();
            continue;
        }

        let Ok(our_mask) = font_shape::fill_path(
            &ours,
            font_shape::FillRule::NonZero,
            case.width,
            case.height,
        ) else {
            report.finding(
                case.name,
                "font-shape/tiny-skia",
                "font-shape returned Err on finite geometry tiny-skia accepted".to_owned(),
            );
            continue;
        };

        let (mut sum, mut count) = (0.0f64, 0.0f64);
        for (o, t) in our_mask.iter().zip(theirs_coverage.iter()) {
            let d = i16::from(*o) - i16::from(*t);
            sum += f64::from(d.unsigned_abs());
            count += 1.0;
        }
        if count == 0.0 {
            report.finding(case.name, "font-shape/tiny-skia", "zero-pixel comparison".to_owned());
            continue;
        }
        let mean = sum / count;
        if mean <= MEAN_ABS_TOLERANCE {
            report.agree();
        } else {
            report.finding(
                case.name,
                "font-shape/tiny-skia",
                format!(
                    "mean absolute coverage difference {mean:.2} exceeds tolerance {MEAN_ABS_TOLERANCE}"
                ),
            );
        }
    }
    report
}

#[cfg(feature = "rival-tiny-skia")]
fn tiny_skia_path(geom: &Geom) -> Option<tiny_skia::Path> {
    let mut b = tiny_skia::PathBuilder::new();
    match geom {
        Geom::Polygon(points, closed) => {
            let mut pts = points.iter();
            if let Some((x, y)) = pts.next() {
                b.move_to(*x, *y);
            }
            for (x, y) in pts {
                b.line_to(*x, *y);
            }
            if *closed {
                b.close();
            }
        }
        Geom::Quad { start, ctrl, to } => {
            b.move_to(start.0, start.1);
            b.quad_to(ctrl.0, ctrl.1, to.0, to.1);
        }
        Geom::Cubic { start, c1, c2, to } => {
            b.move_to(start.0, start.1);
            b.cubic_to(c1.0, c1.1, c2.0, c2.1, to.0, to.1);
        }
        Geom::Rect(x, y, w, h) => {
            b.push_rect(tiny_skia::Rect::from_xywh(*x, *y, *w, *h)?);
        }
    }
    b.finish()
}

/// Non-feature builds report the gap honestly instead of pretending.
#[cfg(not(feature = "rival-tiny-skia"))]
pub fn vs_tiny_skia(_cases: &[PathCase]) -> Report {
    let mut report = Report::new();
    report.not_compared("tiny-skia differential (build with --features rival-tiny-skia)");
    report
}

/// Rivals queued behind this one, recorded so a green run cannot hide them.
#[must_use]
pub fn pending_rivals() -> Vec<&'static str> {
    vec![
        "zeno differential (feature lands with the first wired comparison)",
        "vello_cpu differential (sparse-strip CPU path; lands with vello 0.3 pin)",
        "throughput benches (criterion; numbers recorded in the same commit as any rasterizer change)",
    ]
}
