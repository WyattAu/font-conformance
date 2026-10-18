//! Shape differential (FCN-005): `font-compose` vs the HarfBuzz conformance
//! expectations. Pending — `font-compose` is SPEC-ONLY
//! ([specs/font-compose.md]); this module is the slot it lands in, kept
//! compiling so the suite stays green around it.

use crate::report::Report;

/// The shaping differential. Until `font-compose` exists this reports the gap.
#[must_use]
pub fn vs_harfbuzz() -> Report {
    let mut report = Report::new();
    report.not_compared("FCN-005 awaits font-compose (SPEC-ONLY, ADR-0003)");
    report
}
