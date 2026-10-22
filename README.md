# font-conformance

Independent differential oracle for the estate text stack: the measurer,
not a layer. Nothing in the estate may depend on it at runtime.

| Field | Value |
|---|---|
| Layer | **L3: composition, dev-only** (`[package.metadata.layers] tier = "L3"`) |
| Spec | `engineering-standards/specs/font-conformance.md` (FCN-001…006) |
| Decision | [ADR-0003](../../engineering-standards/docs/adr/0003-adopt-harfrust-extend-font-stack-in-layers.md): the rasterizer keep/merge verdict is *deferred to this suite's numbers* |
| Status | **M1 scaffold**: hostile corpus + panic oracle + structural differentials run today; rival wiring and corpus attestation are the next commits |

## What runs today

```sh
cargo test                     # hostile corpus, no network, no rivals, minutes
cargo test --features rival-tiny-skia   # + font-shape vs tiny-skia over the hostile set
```

| REQ | State |
|---|---|
| FCN-001 | manifest + attestation machinery done (`corpus/manifest.toml`, `scripts/fetch_corpus.py`); entries commented until hashes are real: run `python3 scripts/fetch_corpus.py init` and paste |
| FCN-002 | estate-internal rule live; `vs_read_fonts` behind `rival-fontations` |
| FCN-003 | structural + tiny-skia coverage differential live; **zeno / vello_cpu / throughput benches are the next commits**: this suite's numbers gate the ADR-0004 rasterizer verdict, so they land before any rasterizer code moves |
| FCN-004 | live: hostile bytes × parser/model/table parses, hostile geometry × fill/stroke/stroke-fill, positive controls (`rect_half_pixel` = exactly 128) |
| FCN-005 | slot reserved (`differential::shape`), pending `font-compose` |
| FCN-006 | `conformance.yml`: PR smoke + nightly deep (fetch + `--all-features` + `FONT_CONFORMANCE_DEEP=1`) |

## Findings ledger (what the oracle has caught so far)

| # | Finding | Disposition |
|---|---|---|
| 1 | **`font-shape` silent hang**: `fill_path` never returned on ±1e30 geometry: `add_band`'s column walk saturated at `i32` bounds because `width` was never used to bound it (~2¹¹ no-op columns per band). | **Fixed at the source** (one clamp + comment), regression tests in `font-shape/tests/hostile_extent.rs`; ship as 0.1.1. The oracle's first catch, on its first run. |
| 2 | **`tiny-skia` 0.12 panics** (`alpha_runs.rs` unwrap) on NaN coordinates and on ±1e30 extents, where `font-shape` returns a mask. | Rival robustness finding; upstream report pending. Tracked in the test ledger (`TRACKED`), self-ratcheting. |
| 3 | `tiny-skia` 0.12 refuses inf/NaN-curve paths gracefully (`PathBuilder::finish → None`). | Parity note, not a bug. Tracked. |
| 4 | ~~Bowtie coverage disagreement~~: mean \|Δ\| 6.27 > tolerance 2 on the self-intersecting polygon. | **Settled by measurement (2026-10-18): not a bug: the antialiasing model.** On the minimal witness (a triangle whose pixel the diagonal halves exactly) `font-shape` returns 128 against an exact 127.5 (0.5 off, pure rounding); `tiny-skia` returns 160 (32.5 off, sampled coverage). Both honour total ink; neither must agree per pixel on a sloped edge. The differential now gates the **ink invariant** (`INK_TOLERANCE`) and files per-pixel divergence as an AA-model note. `font-shape`'s analytic coverage is the accurate one: the oracle's triage exonerated the estate crate and indicted the methodology. |

The estate thesis, measured: the never-panic discipline is not aesthetics: the rival rasterizer crashes on inputs our stack answers.

## Corpus rules (FCN-001)

Real fonts are **fetched, never committed**: `corpus/manifest.toml` holds
URL + SHA-256 + OFL license per entry; `scripts/fetch_corpus.py`
(stdlib-only) is the only downloader; the Rust side re-verifies every hash on
every run. A wrong hash is a red run. The hostile synthetic corpus is
different: it is code (`src/hostile.rs`), deterministic, committed, and
cheap, except the deep-only million-point subpath.

## Deviations from the spec, recorded

1. The spec says "fetched at build time"; the implementation fetches at
   **run time via an explicit step**: a build.rs network fetch would break
   hermetic builds (REPRODUCIBILITY.md). Spec amendment proposed with owner.
2. `maxp::num_glyphs` comparison is pending a font-parse accessor; recorded
   in every report's `not_compared` rather than skipped silently.

## Estate wiring

- Path deps (`../font-parse`, `../font-model`, `../font-shape`) so the suite
  always measures repo HEAD; switch to registry pins at first publish.
- `publish = false` until the `estate.yml` entry lands; then flip and add the
  `docs/layers.md` classification row.
- License files (MIT OR Apache-2.0) copy from the kit template at first real
  commit, per the template flow.
