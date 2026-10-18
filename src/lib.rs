//! Independent differential oracle for the estate text stack.
//!
//! `font-conformance` is the estate's **L3 dev-only measurer**: it never ships
//! inside anything, it exists to answer, per text crate, *does it agree with
//! an independent implementation, and what does it cost?* Pattern donor:
//! [`audiobook-conformance`] — corpus in, cross-check against oracles that
//! don't share our code, publish the numbers.
//!
//! | REQ | What proves it | Where |
//! |---|---|---|
//! | FCN-001 | [`manifest::Corpus`] — SHA-256-attested, fetched, never committed | `corpus/manifest.toml` + `scripts/fetch_corpus.py` |
//! | FCN-002 | [`differential::parse`] — `font-parse` vs `read-fonts` (feature `rival-fontations`) and vs `font-model`'s assembly | `differential::parse` |
//! | FCN-003 | [`differential::raster`] — `font-shape` vs `tiny-skia` today, `zeno`/`vello_cpu` next; coverage tolerance **and** throughput baselines recorded in-commit | `differential::raster` |
//! | FCN-004 | [`hostile`] — synthetic hostile geometry and bytes; Err-not-panic everywhere | `tests/panic_smoke.rs` |
//! | FCN-005 | [`differential::shape`] — pending `font-compose` (SPEC-ONLY) | `differential::shape` |
//! | FCN-006 | PR smoke + nightly deep | `.github/workflows/conformance.yml` |
//!
//! # Layer
//!
//! L3 — composition (dev-only). Estate deps: `font-parse` (L0),
//! `font-model` (L1), `font-shape` (L2) — all legal edges. Rival oracles are
//! external crates behind features so the estate supply chain stays minimal
//! by default.
//!
//! [`audiobook-conformance`]: https://github.com/WyattAu/audiobook-conformance

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod differential;
pub mod hostile;
pub mod manifest;
pub mod report;

/// The crate's version, for callers that report it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
