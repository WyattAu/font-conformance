//! The report model every differential writes into (FCN-002/003/005 outputs).
//!
//! A run is green when `findings` is empty. `not_compared` is honest
//! bookkeeping — a differential that did not run is recorded, never silently
//! skipped; graduation to a full gate means emptying it.

/// One disagreement (or contract violation) found by a differential.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Which corpus case produced it (hostile case name or font name).
    pub case: String,
    /// Which oracle pair disagreed, e.g. `"font-parse/read-fonts"`.
    pub oracle: &'static str,
    /// What differed, with both sides' values.
    pub detail: String,
}

/// A per-pixel divergence that both rasterizers are entitled to, because
/// they integrate coverage differently (analytically vs sampled). Reported
/// so it is never silently swallowed, but **not** a finding: a rasterizer is
/// not wrong for choosing a different antialiasing model.
#[derive(Debug, Clone)]
pub struct AaDifference {
    /// The corpus case it was seen on.
    pub case: String,
    /// Mean absolute per-pixel coverage difference.
    pub mean_delta: f64,
}

/// The outcome of one differential run.
#[derive(Debug, Default, Clone)]
pub struct Report {
    /// Number of individual comparisons that agreed.
    pub agreed: u32,
    /// Every disagreement or violation found.
    pub findings: Vec<Finding>,
    /// Per-pixel divergences explained by the antialiasing model, with the
    /// ink invariant that *was* honoured.
    pub aa_differences: Vec<AaDifference>,
    /// Comparisons this build cannot run yet (feature off, oracle pending).
    pub not_compared: Vec<&'static str>,
}

impl Report {
    /// A fresh, empty report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one comparison that agreed.
    pub fn agree(&mut self) {
        self.agreed += 1;
    }

    /// Record a disagreement.
    pub fn finding(&mut self, case: &str, oracle: &'static str, detail: String) {
        self.findings.push(Finding {
            case: case.to_owned(),
            oracle,
            detail,
        });
    }

    /// Record a per-pixel divergence explained by the antialiasing model.
    pub fn aa_model(&mut self, case: &str, mean_delta: f64) {
        self.aa_differences.push(AaDifference {
            case: case.to_owned(),
            mean_delta,
        });
        self.agreed += 1;
    }

    /// Record that a named comparison did not run in this build.
    pub fn not_compared(&mut self, what: &'static str) {
        self.not_compared.push(what);
    }

    /// One-line human summary; findings listed in full by the caller.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{} agreed ({} AA-model), {} findings, {} not compared",
            self.agreed,
            self.aa_differences.len(),
            self.findings.len(),
            self.not_compared.len()
        )
    }
}
