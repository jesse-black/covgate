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
}

impl MetricKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Region => "region",
            Self::Line => "line",
            Self::Branch => "branch",
            Self::Function => "function",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Region => "regions",
            Self::Line => "lines",
            Self::Branch => "branches",
            Self::Function => "functions",
        }
    }

    pub fn to_opportunity_kind(self) -> OpportunityKind {
        match self {
            Self::Region => OpportunityKind::Region,
            Self::Line => OpportunityKind::Line,
            Self::Branch => OpportunityKind::BranchOutcome,
            Self::Function => OpportunityKind::Function,
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
    pub fn metric(&self) -> MetricKind {
        match self {
            Self::Percent { metric, .. } => *metric,
            Self::UncoveredCount { metric, .. } => *metric,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Percent { metric, .. } => format!("fail-under-{}", metric.label()),
            Self::UncoveredCount { metric, .. } => format!("fail-uncovered-{}", metric.label()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
    pub fn overlaps_line_range(&self, start: u32, end: u32) -> bool {
        self.start_line <= end && start <= self.end_line
    }

    pub fn key(&self) -> SpanKey {
        SpanKey {
            start_line: self.start_line,
            end_line: self.end_line,
            start_col: self.start_col,
            end_col: self.end_col,
        }
    }

    pub fn format_span(&self) -> String {
        self.key().format_span()
    }

    pub fn display(&self) -> String {
        format!("{}:{}", self.path.display(), self.format_span())
    }

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
    pub metrics: Vec<ComputedMetric>,
    pub rules: Vec<RuleOutcome>,
    pub passed: bool,
}

#[cfg(test)]
mod tests {
    use super::{MetricKind, OpportunityKind, SourceSpan, SpanKey};
    use std::path::PathBuf;

    #[test]
    fn parses_function_metric_kind() {
        let metric: MetricKind =
            serde_json::from_str("\"function\"").expect("function should parse");
        assert_eq!(metric, MetricKind::Function);
        assert_eq!(metric.as_str(), "function");
        assert_eq!(metric.label(), "functions");
        assert_eq!(metric.to_opportunity_kind(), OpportunityKind::Function);
    }

    #[test]
    fn formats_spans_with_and_without_columns() {
        let path = PathBuf::from("src/lib.rs");

        // Single line, no columns
        let s1 = SourceSpan {
            path: path.clone(),
            start_line: 10,
            end_line: 10,
            start_col: None,
            end_col: None,
        };
        assert_eq!(s1.format_span(), "10");
        assert_eq!(s1.display(), "src/lib.rs:10");

        // Multi line, no columns
        let s2 = SourceSpan {
            path: path.clone(),
            start_line: 10,
            end_line: 12,
            start_col: None,
            end_col: None,
        };
        assert_eq!(s2.format_span(), "10-12");

        // Single line, single column
        let s3 = SourceSpan {
            path: path.clone(),
            start_line: 10,
            end_line: 10,
            start_col: Some(5),
            end_col: Some(5),
        };
        assert_eq!(s3.format_span(), "10:5");

        // Single line, column range
        let s4 = SourceSpan {
            path: path.clone(),
            start_line: 10,
            end_line: 10,
            start_col: Some(5),
            end_col: Some(15),
        };
        assert_eq!(s4.format_span(), "10:5-15");

        // Multi line, column range
        let s5 = SourceSpan {
            path: path.clone(),
            start_line: 10,
            end_line: 11,
            start_col: Some(5),
            end_col: Some(10),
        };
        assert_eq!(s5.format_span(), "10:5-11:10");
    }

    #[test]
    fn groups_spans_and_counts_occurrences() {
        let path = PathBuf::from("src/lib.rs");
        let s1 = SourceSpan {
            path: path.clone(),
            start_line: 10,
            end_line: 10,
            start_col: None,
            end_col: None,
        };
        let s2 = s1.clone();
        let s3 = SourceSpan {
            path: path.clone(),
            start_line: 20,
            end_line: 20,
            start_col: None,
            end_col: None,
        };

        let groups = SourceSpan::group_by_span(&[s1, s2, s3]);
        assert_eq!(groups.len(), 2);
        assert_eq!(
            groups[&SpanKey {
                start_line: 10,
                end_line: 10,
                start_col: None,
                end_col: None
            }],
            2
        );
        assert_eq!(
            groups[&SpanKey {
                start_line: 20,
                end_line: 20,
                start_col: None,
                end_col: None
            }],
            1
        );
    }
}
