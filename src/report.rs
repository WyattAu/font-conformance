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

/// The outcome of one differential run.
#[derive(Debug, Default, Clone)]
pub struct Report {
    /// Number of individual comparisons that agreed.
    pub agreed: u32,
    /// Every disagreement or violation found.
    pub findings: Vec<Finding>,
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

    /// Record that a named comparison did not run in this build.
    pub fn not_compared(&mut self, what: &'static str) {
        self.not_compared.push(what);
    }

    /// One-line human summary; findings listed in full by the caller.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{} agreed, {} findings, {} not compared",
            self.agreed,
            self.findings.len(),
            self.not_compared.len()
        )
    }
}
