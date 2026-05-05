use std::{collections::BTreeMap, path::PathBuf};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum MetricKind {
    Region,
    Line,
    Branch,
    Function,
    NamedFunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Normal,
    Verbose,
}

impl MetricKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Region => "region",
            Self::Line => "line",
            Self::Branch => "branch",
            Self::Function => "function",
            Self::NamedFunction => "named-function",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Region => "regions",
            Self::Line => "lines",
            Self::Branch => "branches",
            Self::Function => "functions",
            Self::NamedFunction => "named-functions",
        }
    }

    #[must_use]
    pub fn to_opportunity_kind(self) -> OpportunityKind {
        match self {
            Self::Region => OpportunityKind::Region,
            Self::Line => OpportunityKind::Line,
            Self::Branch => OpportunityKind::BranchOutcome,
            Self::Function => OpportunityKind::Function,
            Self::NamedFunction => OpportunityKind::Function,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GateRule {
    Percent {
        metric: MetricKind,
        minimum_percent: f64,
    },
    UncoveredCount {
        metric: MetricKind,
        maximum_count: usize,
    },
}

impl GateRule {
    #[must_use]
    pub fn metric(&self) -> MetricKind {
        match self {
            Self::Percent {
                metric,
                minimum_percent: _,
            } => *metric,
            Self::UncoveredCount {
                metric,
                maximum_count: _,
            } => *metric,
        }
    }

    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Percent {
                metric,
                minimum_percent: _,
            } => format!("fail-under-{}", metric.label()),
            Self::UncoveredCount {
                metric,
                maximum_count: _,
            } => format!("fail-uncovered-{}", metric.label()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct RuleOutcome {
    pub rule: GateRule,
    pub passed: bool,
    pub observed_percent: f64,
    pub observed_uncovered_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpanKey {
    pub start_line: u32,
    pub end_line: u32,
    pub start_col: Option<u32>,
    pub end_col: Option<u32>,
}

impl SpanKey {
    #[must_use]
    pub fn format_span(&self) -> String {
        match (self.start_col, self.end_col) {
            (Some(s_col), Some(e_col)) => {
                if self.start_line == self.end_line {
                    if s_col == e_col {
                        format!("{}:{}", self.start_line, s_col)
                    } else {
                        format!("{}:{}-{}", self.start_line, s_col, e_col)
                    }
                } else {
                    format!("{}:{}-{}:{}", self.start_line, s_col, self.end_line, e_col)
                }
            }
            _ => {
                if self.start_line == self.end_line {
                    format!("{}", self.start_line)
                } else {
                    format!("{}-{}", self.start_line, self.end_line)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub path: PathBuf,
    pub start_line: u32,
    pub end_line: u32,
    pub start_col: Option<u32>,
    pub end_col: Option<u32>,
}

impl SourceSpan {
    #[must_use]
    pub fn overlaps_line_range(&self, start: u32, end: u32) -> bool {
        self.start_line <= end && start <= self.end_line
    }

    #[must_use]
    pub fn key(&self) -> SpanKey {
        SpanKey {
            start_line: self.start_line,
            end_line: self.end_line,
            start_col: self.start_col,
            end_col: self.end_col,
        }
    }

    #[must_use]
    pub fn format_span(&self) -> String {
        self.key().format_span()
    }

    #[must_use]
    pub fn display(&self) -> String {
        format!("{}:{}", self.path.display(), self.format_span())
    }

    #[must_use]
    pub fn group_by_span(spans: &[Self]) -> BTreeMap<SpanKey, usize> {
        let mut counts = BTreeMap::new();
        for span in spans {
            *counts.entry(span.key()).or_default() += 1;
        }
        counts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpportunityKind {
    Region,
    Line,
    BranchOutcome,
    Function,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageOpportunity {
    pub kind: OpportunityKind,
    pub span: SourceSpan,
    pub covered: bool,
    pub is_named_function: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    pub opportunities: Vec<CoverageOpportunity>,
    pub totals_by_file: BTreeMap<MetricKind, BTreeMap<PathBuf, FileTotals>>,
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
#[must_use]
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
#[must_use]
pub struct GateResult {
    pub metrics: Vec<ComputedMetric>,
    pub rules: Vec<RuleOutcome>,
    pub passed: bool,
}
