//! The hostile corpus: deterministic synthetic inputs that try to make the
//! text stack panic (FCN-004). Committed as code, not assets — every case is
//! generated, named, and cheap except the one deep-only geometry stress.
//!
//! Two families:
//! - **bytes** — corrupted SFNT headers and raw table blobs, aimed at
//!   `font-parse` (L0) and `font-model`'s assembly (L1);
//! - **paths** — geometry that is *valid Rust* but hostile math: NaN and ±inf
//!   coordinates, ±1e30 extents, degenerate and self-intersecting contours,
//!   aimed at `font-shape` (L2).

/// A corrupted font byte buffer.
#[derive(Debug, Clone)]
pub struct ByteCase {
    /// Stable, greppable case name.
    pub name: &'static str,
    /// The hostile bytes.
    pub bytes: Vec<u8>,
}

/// Geometry spec shared by every rasterizer under comparison. Kept as data
/// (not a built `Path2D`) so each rival rasterizer emits its own path from
/// the same spec — same bytes in, comparable ink out.
#[derive(Debug, Clone)]
pub enum Geom {
    /// A polygon: points, and whether it closes.
    Polygon(Vec<(f32, f32)>, bool),
    /// `move_to(start)` then `quad_to(ctrl, to)`.
    Quad {
        /// Starting point.
        start: (f32, f32),
        /// Control point.
        ctrl: (f32, f32),
        /// End point.
        to: (f32, f32),
    },
    /// `move_to(start)` then `cubic_to(c1, c2, to)`.
    Cubic {
        /// Starting point.
        start: (f32, f32),
        /// First control point.
        c1: (f32, f32),
        /// Second control point.
        c2: (f32, f32),
        /// End point.
        to: (f32, f32),
    },
    /// An axis-aligned rectangle.
    Rect(f32, f32, f32, f32),
}

/// A hostile geometry case with its target mask size.
#[derive(Debug, Clone)]
pub struct PathCase {
    /// Stable, greppable case name.
    pub name: &'static str,
    /// The geometry.
    pub geom: Geom,
    /// False when coordinates are NaN/inf/huge: rivals are then compared for
    /// Err-not-panic only, never pixel-by-pixel.
    pub finite: bool,
    /// Mask width in pixels.
    pub width: u32,
    /// Mask height in pixels.
    pub height: u32,
}

/// True when `FONT_CONFORMANCE_DEEP` is set: adds the expensive stress cases
/// (the million-point subpath) to the run.
#[must_use]
pub fn is_deep() -> bool {
    std::env::var("FONT_CONFORMANCE_DEEP").is_ok_and(|v| v == "1")
}

// -- byte-level construction helpers (panic-free by construction) -----------

fn zeros(n: usize) -> Vec<u8> {
    vec![0; n]
}

fn put_u16(v: &mut [u8], at: usize, val: u16) {
    if let Some(slot) = v.get_mut(at..at + 2) {
        slot.copy_from_slice(&val.to_be_bytes());
    }
}

fn put_u32(v: &mut [u8], at: usize, val: u32) {
    if let Some(slot) = v.get_mut(at..at + 4) {
        slot.copy_from_slice(&val.to_be_bytes());
    }
}

fn put_i16(v: &mut [u8], at: usize, val: i16) {
    if let Some(slot) = v.get_mut(at..at + 2) {
        slot.copy_from_slice(&val.to_be_bytes());
    }
}

/// A spec-conformant 54-byte `head` table: version 1.0, upem 1000, short
/// loca format. The positive control — every other head case corrupts it.
fn valid_head() -> Vec<u8> {
    let mut t = zeros(54);
    put_u32(&mut t, 0, 0x0001_0000); // version 1.0
    put_u16(&mut t, 18, 1_000); // unitsPerEm
    put_i16(&mut t, 50, 0); // indexToLocFormat: short
    t
}

/// A spec-conformant 36-byte `hhea` table.
fn valid_hhea() -> Vec<u8> {
    let mut t = zeros(36);
    put_u32(&mut t, 0, 0x0001_0000);
    put_i16(&mut t, 4, 800); // ascender
    put_i16(&mut t, 6, -200); // descender
    put_i16(&mut t, 8, 0); // lineGap
    t
}

fn sfnt_header(tag: &[u8; 4], num_tables: u16) -> Vec<u8> {
    let mut v = Vec::with_capacity(12);
    v.extend_from_slice(tag);
    v.extend_from_slice(&num_tables.to_be_bytes());
    v.extend_from_slice(&[0; 6]); // searchRange, entrySelector, rangeShift
    v
}

fn directory_entry(offset: u32, length: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(b"head");
    v.extend_from_slice(&0u32.to_be_bytes()); // checksum (unchecked here)
    v.extend_from_slice(&offset.to_be_bytes());
    v.extend_from_slice(&length.to_be_bytes());
    v
}

/// Every SFNT-level and table-level hostile byte case.
#[must_use]
pub fn byte_cases() -> Vec<ByteCase> {
    let cases = vec![
        // -- SFNT offset-table level (FontRef::new / from_sfnt) -------------
        ByteCase {
            name: "sfnt_empty",
            bytes: Vec::new(),
        },
        ByteCase {
            name: "sfnt_three_bytes",
            bytes: vec![0x00, 0x01, 0x00],
        },
        ByteCase {
            name: "sfnt_header_short",
            bytes: zeros(11),
        },
        ByteCase {
            name: "sfnt_zero_tables",
            bytes: sfnt_header(b"\x00\x01\x00\x00", 0),
        },
        ByteCase {
            name: "sfnt_otto_zero_tables",
            bytes: sfnt_header(b"OTTO", 0),
        },
        ByteCase {
            name: "sfnt_true_zero_tables",
            bytes: sfnt_header(b"true", 0),
        },
        ByteCase {
            name: "sfnt_count_no_directory",
            bytes: sfnt_header(b"\x00\x01\x00\x00", 0xFFFF),
        },
        ByteCase {
            name: "sfnt_entry_past_eof",
            bytes: [sfnt_header(b"\x00\x01\x00\x00", 1), directory_entry(0xFFFF_FFF0, 4)].concat(),
        },
        ByteCase {
            name: "sfnt_entry_offset_length_overflow",
            bytes: [
                sfnt_header(b"\x00\x01\x00\x00", 1),
                directory_entry(0xFFFF_FFF0, 0xFFFF_FFF0),
            ]
            .concat(),
        },
        ByteCase {
            name: "sfnt_table_over_directory",
            bytes: [sfnt_header(b"\x00\x01\x00\x00", 1), directory_entry(0, 12)].concat(),
        },
        // -- table level (HeadTable/HheaTable/MaxpTable::parse) -------------
        ByteCase {
            name: "tbl_head_truncated",
            bytes: zeros(53),
        },
        ByteCase {
            name: "tbl_head_upem_zero",
            bytes: {
                let mut t = valid_head();
                put_u16(&mut t, 18, 0);
                t
            },
        },
        ByteCase {
            name: "tbl_head_upem_above_limit",
            bytes: {
                let mut t = valid_head();
                put_u16(&mut t, 18, 20_000);
                t
            },
        },
        ByteCase {
            name: "tbl_head_bad_loc_format",
            bytes: {
                let mut t = valid_head();
                put_i16(&mut t, 50, 2);
                t
            },
        },
        ByteCase {
            name: "tbl_head_valid",
            bytes: valid_head(),
        },
        ByteCase {
            name: "tbl_hhea_truncated",
            bytes: zeros(35),
        },
        ByteCase {
            name: "tbl_hhea_valid",
            bytes: valid_hhea(),
        },
        ByteCase {
            name: "tbl_maxp_truncated",
            bytes: zeros(5),
        },
        ByteCase {
            name: "tbl_maxp_v05_minimum",
            bytes: {
                let mut t = zeros(6);
                put_u32(&mut t, 0, 0x0000_5000);
                put_u16(&mut t, 4, 1);
                t
            },
        },
        ByteCase {
            name: "tbl_maxp_v10_short",
            bytes: {
                let mut t = zeros(31);
                put_u32(&mut t, 0, 0x0001_0000);
                t
            },
        },
        ByteCase {
            name: "tbl_maxp_bogus_version",
            bytes: {
                let mut t = zeros(32);
                put_u32(&mut t, 0, 0xDEAD_BEEF);
                t
            },
        },
    ];
    cases
}

/// Every hostile geometry case. `deep` adds the million-point subpath.
#[must_use]
pub fn path_cases(deep: bool) -> Vec<PathCase> {
    let mut cases = vec![
        // Positive controls (finite, exact expectations exist).
        PathCase {
            name: "rect_half_pixel",
            geom: Geom::Rect(0.5, 0.0, 4.0, 1.0),
            finite: true,
            width: 8,
            height: 1,
        },
        PathCase {
            name: "rect_on_grid",
            geom: Geom::Rect(0.0, 0.0, 4.0, 4.0),
            finite: true,
            width: 8,
            height: 8,
        },
        PathCase {
            name: "bowtie_self_intersecting",
            geom: Geom::Polygon(vec![(0.0, 0.0), (8.0, 8.0), (8.0, 0.0), (0.0, 8.0)], true),
            finite: true,
            width: 9,
            height: 9,
        },
        PathCase {
            name: "colinear_sliver",
            geom: Geom::Polygon(vec![(0.0, 0.0), (4.0, 0.0), (8.0, 0.0)], true),
            finite: true,
            width: 9,
            height: 2,
        },
        PathCase {
            name: "two_point_closed",
            geom: Geom::Polygon(vec![(1.0, 1.0), (2.0, 2.0)], true),
            finite: true,
            width: 4,
            height: 4,
        },
        PathCase {
            name: "duplicate_point_storm",
            geom: Geom::Polygon(vec![(3.0, 3.0); 100], true),
            finite: true,
            width: 8,
            height: 8,
        },
        // Hostile math: Err-not-panic and never pixel-compared.
        PathCase {
            name: "nan_polygon",
            geom: Geom::Polygon(vec![(f32::NAN, 0.0), (1.0, 1.0), (2.0, 0.0)], true),
            finite: false,
            width: 4,
            height: 4,
        },
        PathCase {
            name: "inf_polygon",
            geom: Geom::Polygon(
                vec![(f32::INFINITY, 0.0), (1.0, f32::NEG_INFINITY), (2.0, 0.0)],
                true,
            ),
            finite: false,
            width: 4,
            height: 4,
        },
        PathCase {
            name: "huge_extent_polygon",
            geom: Geom::Polygon(vec![(1e30, 1e30), (-1e30, 0.0), (0.0, -1e30)], true),
            finite: false,
            width: 8,
            height: 8,
        },
        PathCase {
            name: "nan_quad_control",
            geom: Geom::Quad {
                start: (0.0, 0.0),
                ctrl: (f32::NAN, f32::NAN),
                to: (4.0, 4.0),
            },
            finite: false,
            width: 6,
            height: 6,
        },
        PathCase {
            name: "cubic_all_controls_coincident",
            geom: Geom::Cubic {
                start: (1.0, 1.0),
                c1: (4.0, 4.0),
                c2: (4.0, 4.0),
                to: (4.0, 4.0),
            },
            finite: true,
            width: 6,
            height: 6,
        },
    ];
    if deep {
        // One million-point closed subpath: the "million-point subpath" entry
        // in the threat list. ~4π-unit circle sampled a million times.
        const N: usize = 1_000_000;
        let mut pts = Vec::with_capacity(N);
        for i in 0..N {
            let t = (i as f32) * core::f32::consts::TAU / (N as f32);
            pts.push((t.cos() * 50.0 + 50.0, t.sin() * 50.0 + 50.0));
        }
        cases.push(PathCase {
            name: "million_point_subpath",
            geom: Geom::Polygon(pts, true),
            finite: true,
            width: 128,
            height: 128,
        });
    }
    cases
}

/// Emit a case into `font-shape`'s path model (verified builder APIs only).
#[must_use]
pub fn font_shape_path(geom: &Geom) -> font_shape::Path2D {
    let mut b = font_shape::PathBuilder::new();
    match geom {
        Geom::Polygon(points, closed) => {
            b.add_polygon(points, *closed);
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
            b.rect(*x, *y, *w, *h);
        }
    }
    b.build()
}
