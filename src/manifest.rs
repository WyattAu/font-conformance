//! The fetched-font manifest: SHA-256 attestation and cache verification
//! (FCN-001). Real fonts are never committed; a run proves which bytes it
//! measured or it does not run.

use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// One attested font entry from `corpus/manifest.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct FontEntry {
    /// Stable name; also the cache file name.
    pub name: String,
    /// Where the font is fetched from (run-time fetch, never build-time).
    pub url: String,
    /// Lowercase hex SHA-256 of the exact bytes the run must see.
    #[serde(default)]
    pub sha256: String,
    /// SPDX license expression of the font itself (OFL only, per the manifest).
    pub license: String,
    /// Where the license text lives.
    pub license_url: String,
    /// Free-form slices this font exercises (`"rtl"`, `"variable"`, …).
    #[serde(default)]
    pub tags: Vec<String>,
}

/// The parsed `corpus/manifest.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Corpus {
    /// Manifest schema; must be 1.
    pub schema: u32,
    /// Every attested font.
    #[serde(default)]
    pub fonts: Vec<FontEntry>,
}

/// Why a manifest or cache operation failed.
#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    /// The manifest is missing or unreadable.
    #[error("cannot read corpus manifest at {path}: {source}")]
    Unreadable {
        /// The path we tried.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The manifest is not valid TOML or does not match the schema.
    #[error("invalid corpus manifest: {0}")]
    Invalid(String),
    /// The manifest schema is not the one this build understands.
    #[error("unsupported manifest schema {found} (expected 1)")]
    Schema {
        /// The schema value found in the file.
        found: u32,
    },
    /// The font cache directory cannot be located (set FONT_CONFORMANCE_CACHE).
    #[error("no cache directory: HOME is unset; set FONT_CONFORMANCE_CACHE")]
    NoCacheDir,
    /// I/O while reading a cached font.
    #[error("cache read failed for {name}: {source}")]
    CacheRead {
        /// The entry name.
        name: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// A cached font's hash does not match its attestation.
    #[error("hash mismatch for {name}: expected {expected}, got {actual}")]
    HashMismatch {
        /// The entry name.
        name: String,
        /// Attested hash.
        expected: String,
        /// Actual hash of the cached bytes.
        actual: String,
    },
    /// The attestation is empty — the entry was never init'ed.
    #[error("unattested entry {name}: run scripts/fetch_corpus.py init")]
    Unattested {
        /// The entry name.
        name: String,
    },
}

impl Corpus {
    /// Parse the manifest from TOML text.
    ///
    /// # Errors
    /// [`CorpusError::Invalid`] when the text is not valid TOML or the wrong
    /// shape; [`CorpusError::Schema`] when `schema` is not 1.
    pub fn from_toml(text: &str) -> Result<Self, CorpusError> {
        let corpus: Corpus =
            toml::from_str(text).map_err(|e| CorpusError::Invalid(e.to_string()))?;
        if corpus.schema != 1 {
            return Err(CorpusError::Schema {
                found: corpus.schema,
            });
        }
        Ok(corpus)
    }

    /// Load the manifest from the repo's default location
    /// (`corpus/manifest.toml` relative to this crate).
    ///
    /// # Errors
    /// [`CorpusError::Unreadable`] when the file is missing; parse errors as
    /// in [`Corpus::from_toml`].
    pub fn from_repo_default() -> Result<Self, CorpusError> {
        let path = manifest_path();
        let text = std::fs::read_to_string(&path).map_err(|source| CorpusError::Unreadable {
            path: path.clone(),
            source,
        })?;
        Self::from_toml(&text)
    }

    /// The font cache directory: `$FONT_CONFORMANCE_CACHE`, else
    /// `$HOME/.cache/font-conformance`.
    ///
    /// # Errors
    /// [`CorpusError::NoCacheDir`] when neither variable is set.
    pub fn cache_dir() -> Result<PathBuf, CorpusError> {
        if let Ok(dir) = std::env::var("FONT_CONFORMANCE_CACHE") {
            return Ok(PathBuf::from(dir));
        }
        match std::env::var("HOME") {
            Ok(home) => Ok(PathBuf::from(home).join(".cache").join("font-conformance")),
            Err(_) => Err(CorpusError::NoCacheDir),
        }
    }

    /// Read and hash-verify one entry's cached bytes.
    ///
    /// # Errors
    /// [`CorpusError::Unattested`] when the entry has no hash yet,
    /// [`CorpusError::CacheRead`] when the file cannot be read,
    /// [`CorpusError::HashMismatch`] when the bytes are not the attested ones.
    pub fn verified_bytes(&self, entry: &FontEntry) -> Result<Vec<u8>, CorpusError> {
        if entry.sha256.is_empty() {
            return Err(CorpusError::Unattested {
                name: entry.name.clone(),
            });
        }
        let path = Self::cache_dir()?.join(&entry.name);
        let bytes =
            std::fs::read(&path).map_err(|source| CorpusError::CacheRead {
                name: entry.name.clone(),
                source,
            })?;
        let actual = hex_sha256(&bytes);
        if actual != entry.sha256 {
            return Err(CorpusError::HashMismatch {
                name: entry.name.clone(),
                expected: entry.sha256.clone(),
                actual,
            });
        }
        Ok(bytes)
    }

    /// Verify every entry; returns per-entry failures. An empty vec means the
    /// whole corpus is attested and cached byte-exact.
    ///
    /// # Errors
    /// [`CorpusError::NoCacheDir`] when the cache root cannot be resolved.
    pub fn verify_all(&self) -> Result<Vec<(String, CorpusError)>, CorpusError> {
        // Force the cache-dir error early so a missing HOME fails loudly
        // before we report per-font misses.
        Self::cache_dir()?;
        let mut failures = Vec::new();
        for entry in &self.fonts {
            if let Err(e) = self.verified_bytes(entry) {
                failures.push((entry.name.clone(), e));
            }
        }
        Ok(failures)
    }
}

/// Manifest path relative to this crate (works from the repo and from tests).
#[must_use]
pub fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/manifest.toml")
}

/// Lowercase hex SHA-256 of `bytes`.
#[must_use]
pub fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        // fmt::Write on a String never fails; the lint cannot see that.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

impl fmt::Display for FontEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.license)
    }
}
