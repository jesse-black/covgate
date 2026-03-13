use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    Region,
    Line,
    Branch,
    Combined,
}

impl MetricKind {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "region" => Ok(Self::Region),
            "line" => Ok(Self::Line),
            "branch" => Ok(Self::Branch),
            "combined" => Ok(Self::Combined),
            _ => bail!("unsupported metric kind: {value}"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Region => "region",
            Self::Line => "line",
            Self::Branch => "branch",
            Self::Combined => "combined",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Region => "regions",
            Self::Line => "lines",
            Self::Branch => "branches",
            Self::Combined => "combined opportunities",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Threshold {
    pub metric: MetricKind,
    pub minimum_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub path: PathBuf,
    pub start_line: u32,
    pub end_line: u32,
}

impl SourceSpan {
    pub fn overlaps_line_range(&self, start: u32, end: u32) -> bool {
        self.start_line <= end && start <= self.end_line
    }

    pub fn display(&self) -> String {
        format!(
            "{}:{}-{}",
            self.path.display(),
            self.start_line,
            self.end_line
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpportunityKind {
    Region,
    Line,
    BranchOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageOpportunity {
    pub kind: OpportunityKind,
    pub span: SourceSpan,
    pub covered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    pub metric_kind: MetricKind,
    pub opportunities: Vec<CoverageOpportunity>,
    pub totals_by_file: BTreeMap<PathBuf, FileTotals>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTotals {
    pub covered: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub changed_lines: Vec<LineRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedMetric {
    pub metric: MetricKind,
    pub covered: usize,
    pub total: usize,
    pub percent: f64,
    pub uncovered_changed_opportunities: Vec<CoverageOpportunity>,
    pub changed_totals_by_file: BTreeMap<PathBuf, FileTotals>,
    pub totals_by_file: BTreeMap<PathBuf, FileTotals>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GateResult {
    pub metric: MetricKind,
    pub covered: usize,
    pub total: usize,
    pub percent: f64,
    pub threshold: Threshold,
    pub passed: bool,
    pub uncovered_changed_opportunities: Vec<CoverageOpportunity>,
    pub changed_totals_by_file: BTreeMap<PathBuf, FileTotals>,
    pub totals_by_file: BTreeMap<PathBuf, FileTotals>,
}
