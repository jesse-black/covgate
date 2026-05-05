use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use rustc_demangle::try_demangle;
use serde::Deserialize;

use crate::model::{
    CoverageOpportunity, CoverageReport, FileTotals, MetricKind, OpportunityKind, SourceSpan,
};

use super::path::relativize_absolute_path;

pub(crate) fn parse_with_repo_root(input: &str, repo_root: &Path) -> Result<CoverageReport> {
    let export: LlvmExport = serde_json::from_str(input).context("failed to parse llvm json")?;
    let mut opportunities = Vec::new();
    let mut region_totals_by_file = BTreeMap::new();
    let mut line_totals_by_file = BTreeMap::new();
    let mut branch_totals_by_file = BTreeMap::new();
    let mut function_totals_by_file = BTreeMap::new();
    let mut named_function_totals_by_file = BTreeMap::new();

    for data in export.data {
        let known_file_paths: Vec<PathBuf> = data
            .files
            .iter()
            .map(|file| normalize_path(&file.filename, repo_root))
            .collect();

        let mut function_records_by_file: BTreeMap<PathBuf, BTreeMap<FunctionKey, bool>> =
            BTreeMap::new();
        for function in data.functions {
            let Some(path) = function
                .filenames
                .first()
                .map(|f| normalize_function_path(f, repo_root, &known_file_paths))
            else {
                continue;
            };
            let mut start_line: Option<u32> = None;
            let mut start_col: Option<u32> = None;
            let mut end_line: Option<u32> = None;
            let mut end_col: Option<u32> = None;
            let mut region_covered = false;
            for region in function.regions {
                if start_line.is_none_or(|sl| region.line_start < sl) {
                    start_line = Some(region.line_start);
                    start_col = Some(region.col_start);
                } else if start_line == Some(region.line_start) {
                    start_col = Some(start_col.unwrap_or(region.col_start).min(region.col_start));
                }

                if end_line.is_none_or(|el| region.line_end > el) {
                    end_line = Some(region.line_end);
                    end_col = Some(region.col_end);
                } else if end_line == Some(region.line_end) {
                    end_col = Some(end_col.unwrap_or(region.col_end).max(region.col_end));
                }

                region_covered |= region.execution_count > 0;
            }
            let (Some(start_line), Some(start_col), Some(end_line), Some(end_col)) =
                (start_line, start_col, end_line, end_col)
            else {
                continue;
            };
            let entry = function_records_by_file.entry(path).or_default();
            let key = function
                .name
                .as_deref()
                .map(normalize_llvm_function_name)
                .map(|normalized_name| FunctionKey::NormalizedName {
                    normalized_name,
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                })
                .unwrap_or(FunctionKey::Span {
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                });
            let covered = function.count > 0 || region_covered;
            entry
                .entry(key)
                .and_modify(|existing| *existing = *existing || covered)
                .or_insert(covered);
        }

        for file in data.files {
            let path = normalize_path(&file.filename, repo_root);
            let mut region_covered = 0usize;
            let mut region_total = 0usize;

            for region in file.segments_to_regions()? {
                region_total += 1;
                if region.covered {
                    region_covered += 1;
                }
                opportunities.push(CoverageOpportunity {
                    kind: OpportunityKind::Region,
                    span: SourceSpan {
                        path: path.clone(),
                        start_line: region.start_line,
                        end_line: region.end_line,
                        start_col: Some(region.start_col),
                        end_col: Some(region.end_col),
                    },
                    covered: region.covered,
                    is_named_function: None,
                });
            }

            if region_total > 0 {
                region_totals_by_file.insert(
                    path.clone(),
                    FileTotals {
                        covered: region_covered,
                        total: region_total,
                    },
                );
            }

            let mut line_covered = 0usize;
            let mut line_total = 0usize;

            for line in file.parse_lines()? {
                line_total += 1;
                if line.covered {
                    line_covered += 1;
                }
                opportunities.push(CoverageOpportunity {
                    kind: OpportunityKind::Line,
                    span: SourceSpan {
                        path: path.clone(),
                        start_line: line.line_number,
                        end_line: line.line_number,
                        start_col: None,
                        end_col: None,
                    },
                    covered: line.covered,
                    is_named_function: None,
                });
            }

            if line_total > 0 {
                line_totals_by_file.insert(
                    path.clone(),
                    FileTotals {
                        covered: line_covered,
                        total: line_total,
                    },
                );
            }

            let mut branch_covered = 0usize;
            let mut branch_total = 0usize;

            for branch in file.parse_branches()? {
                branch_total += 1;
                if branch.covered {
                    branch_covered += 1;
                }
                opportunities.push(CoverageOpportunity {
                    kind: OpportunityKind::BranchOutcome,
                    span: SourceSpan {
                        path: path.clone(),
                        start_line: branch.line_number,
                        end_line: branch.line_number,
                        start_col: Some(branch.start_col),
                        end_col: Some(branch.start_col),
                    },
                    covered: branch.covered,
                    is_named_function: None,
                });
            }

            if branch_total > 0 {
                branch_totals_by_file.insert(
                    path.clone(),
                    FileTotals {
                        covered: branch_covered,
                        total: branch_total,
                    },
                );
            }

            if let Some(function_records) = function_records_by_file.remove(&path) {
                let mut function_covered = 0usize;
                let function_total = function_records.len();
                let mut named_function_covered = 0usize;
                let mut named_function_total = 0usize;

                for (key, covered) in function_records {
                    let (start_line, start_col, end_line, end_col) = match &key {
                        FunctionKey::Span {
                            start_line,
                            start_col,
                            end_line,
                            end_col,
                        } => (*start_line, *start_col, *end_line, *end_col),
                        FunctionKey::NormalizedName {
                            start_line,
                            start_col,
                            end_line,
                            end_col,
                            ..
                        } => (*start_line, *start_col, *end_line, *end_col),
                    };
                    let is_named = is_llvm_function_named(&key);
                    if covered {
                        function_covered += 1;
                        if is_named {
                            named_function_covered += 1;
                        }
                    }
                    if is_named {
                        named_function_total += 1;
                    }

                    opportunities.push(CoverageOpportunity {
                        kind: OpportunityKind::Function,
                        span: SourceSpan {
                            path: path.clone(),
                            start_line,
                            end_line,
                            start_col: Some(start_col),
                            end_col: Some(end_col),
                        },
                        covered,
                        is_named_function: Some(is_named),
                    });
                }
                function_totals_by_file.insert(
                    path.clone(),
                    FileTotals {
                        covered: function_covered,
                        total: function_total,
                    },
                );
                if named_function_total > 0 {
                    named_function_totals_by_file.insert(
                        path,
                        FileTotals {
                            covered: named_function_covered,
                            total: named_function_total,
                        },
                    );
                }
            }
        }
    }

    let mut totals_by_file = BTreeMap::new();
    if !region_totals_by_file.is_empty() {
        totals_by_file.insert(MetricKind::Region, region_totals_by_file);
    }
    if !line_totals_by_file.is_empty() {
        totals_by_file.insert(MetricKind::Line, line_totals_by_file);
    }
    if !branch_totals_by_file.is_empty() {
        totals_by_file.insert(MetricKind::Branch, branch_totals_by_file);
    }
    if !function_totals_by_file.is_empty() {
        totals_by_file.insert(MetricKind::Function, function_totals_by_file);
    }
    if !named_function_totals_by_file.is_empty() {
        totals_by_file.insert(MetricKind::NamedFunction, named_function_totals_by_file);
    }

    Ok(CoverageReport {
        opportunities,
        totals_by_file,
    })
}

fn is_llvm_function_named(key: &FunctionKey) -> bool {
    match key {
        FunctionKey::Span { .. } => false,
        FunctionKey::NormalizedName {
            normalized_name, ..
        } => !normalized_name
            .split("::")
            .any(|segment| segment.starts_with('{') && segment.ends_with('}')),
    }
}

fn normalize_path(value: &str, repo_root: &Path) -> PathBuf {
    relativize_absolute_path(Path::new(value), repo_root)
}

fn normalize_function_path(value: &str, repo_root: &Path, known_file_paths: &[PathBuf]) -> PathBuf {
    let normalized = normalize_path(value, repo_root);
    if known_file_paths.contains(&normalized) {
        return normalized;
    }

    let normalized_string = normalized.to_string_lossy();
    if let Some(candidate) = known_file_paths
        .iter()
        .filter(|candidate| {
            let candidate_string = candidate.to_string_lossy();
            normalized_string == candidate_string
                || normalized_string
                    .strip_suffix(candidate_string.as_ref())
                    .is_some_and(|prefix| prefix.ends_with('/'))
        })
        .max_by_key(|candidate| candidate.to_string_lossy().len())
    {
        return candidate.clone();
    }

    normalized
}

#[derive(Debug, Deserialize)]
struct LlvmExport {
    data: Vec<LlvmData>,
}

#[derive(Debug, Deserialize)]
struct LlvmData {
    files: Vec<LlvmFile>,
    #[serde(default)]
    functions: Vec<LlvmFunction>,
}

#[derive(Debug, Deserialize)]
struct LlvmFunction {
    #[serde(default)]
    count: u64,
    #[serde(default)]
    filenames: Vec<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    regions: Vec<LlvmFunctionRegion>,
}

#[derive(Debug, Deserialize)]
struct LlvmFunctionRegion {
    #[serde(deserialize_with = "de_u32_from_i64")]
    line_start: u32,
    #[serde(deserialize_with = "de_u32_from_i64")]
    col_start: u32,
    #[serde(deserialize_with = "de_u32_from_i64")]
    line_end: u32,
    #[serde(deserialize_with = "de_u32_from_i64")]
    col_end: u32,
    #[serde(default)]
    execution_count: u64,
    #[serde(default)]
    _file_id: u32,
    #[serde(default)]
    _expanded_file_id: u32,
    #[serde(default)]
    _kind: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FunctionKey {
    Span {
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    },
    NormalizedName {
        normalized_name: String,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    },
}

fn normalize_llvm_function_name(name: &str) -> String {
    try_demangle(name)
        .map(|demangled| format!("{demangled:#}"))
        .unwrap_or_else(|_| name.to_string())
}

fn de_u32_from_i64<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    if value < 0 {
        return Err(serde::de::Error::custom("negative value not allowed"));
    }
    u32::try_from(value).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct LlvmFile {
    filename: String,
    #[serde(default)]
    segments: Vec<Vec<serde_json::Value>>,
    #[serde(default)]
    branches: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug)]
struct LineRecord {
    line_number: u32,
    covered: bool,
}

#[derive(Debug)]
struct RegionRecord {
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    covered: bool,
}

#[derive(Debug)]
struct BranchRecord {
    line_number: u32,
    start_col: u32,
    covered: bool,
}

impl LlvmFile {
    fn segments_to_regions(&self) -> Result<Vec<RegionRecord>> {
        let mut regions = Vec::new();

        for window in self.segments.windows(2) {
            let [start, end] = window else {
                continue;
            };

            let start_line = number_at(start, 0)?;
            let start_col = number_at(start, 1)?;
            let end_line = number_at(end, 0)?;
            let end_col = number_at(end, 1)?;

            if end_line < start_line {
                continue;
            }

            let count = number_at(start, 2)?;
            let has_count = bool_at(start, 3).unwrap_or(true);
            let is_region_entry = bool_at(start, 4).unwrap_or(true);
            let is_gap_region = bool_at(start, 5).unwrap_or(false);
            if !has_count || !is_region_entry || is_gap_region {
                continue;
            }

            regions.push(RegionRecord {
                start_line,
                start_col,
                end_line,
                end_col,
                covered: count > 0,
            });
        }

        Ok(regions)
    }

    fn parse_lines(&self) -> Result<Vec<LineRecord>> {
        let mut line_states: std::collections::BTreeMap<u32, bool> =
            std::collections::BTreeMap::new();
        for window in self.segments.windows(2) {
            let [start, end] = window else {
                continue;
            };

            let start_line = number_at(start, 0)?;
            let end_line = number_at(end, 0)?;
            let end_col = number_at(end, 1)?;

            if end_line < start_line {
                continue;
            }

            let count = number_at(start, 2)?;
            let has_count = bool_at(start, 3).unwrap_or(true);

            if !has_count {
                continue;
            }

            let covered = count > 0;

            let actual_end_line = if end_col <= 1 && end_line > start_line {
                end_line - 1
            } else {
                end_line
            };

            for line in start_line..=actual_end_line {
                line_states
                    .entry(line)
                    .and_modify(|e| *e |= covered)
                    .or_insert(covered);
            }
        }

        let mut lines = Vec::new();
        for (line_number, covered) in line_states {
            lines.push(LineRecord {
                line_number,
                covered,
            });
        }
        Ok(lines)
    }

    fn parse_branches(&self) -> Result<Vec<BranchRecord>> {
        let mut branches = Vec::new();
        for branch in &self.branches {
            let line_number = number_at(branch, 0)?;
            let start_col = number_at(branch, 1)?;

            if branch.len() >= 6 {
                let true_count = number_at(branch, 4)?;
                let false_count = number_at(branch, 5)?;
                branches.push(BranchRecord {
                    line_number,
                    start_col,
                    covered: true_count > 0,
                });
                branches.push(BranchRecord {
                    line_number,
                    start_col,
                    covered: false_count > 0,
                });
                continue;
            }

            let count = number_at(branch, 2)?;
            let has_count = bool_at(branch, 3).unwrap_or(true);
            if !has_count {
                continue;
            }
            branches.push(BranchRecord {
                line_number,
                start_col,
                covered: count > 0,
            });
        }

        Ok(branches)
    }
}

fn number_at(values: &[serde_json::Value], index: usize) -> Result<u32> {
    let number = values
        .get(index)
        .and_then(serde_json::Value::as_u64)
        .context("llvm segment missing numeric field")?;
    u32::try_from(number).context("llvm segment numeric field out of range")
}

fn bool_at(values: &[serde_json::Value], index: usize) -> Option<bool> {
    values.get(index).and_then(serde_json::Value::as_bool)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        FunctionKey, is_llvm_function_named, normalize_llvm_function_name, normalize_path,
    };

    #[test]
    fn normalizes_absolute_paths_to_repo_relative() {
        let repo_root = Path::new("/workspace/covgate");
        let normalized = normalize_path("/workspace/covgate/src/lib.rs", repo_root);
        assert_eq!(normalized, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn demangles_rust_llvm_function_names_for_identity() {
        let normalized = normalize_llvm_function_name(
            "_RNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric",
        );
        assert_eq!(normalized, "covgate::metrics::compute_changed_metric");
    }

    #[test]
    fn leaves_non_rust_function_names_unchanged() {
        let normalized = normalize_llvm_function_name("plain_c_symbol_name");
        assert_eq!(normalized, "plain_c_symbol_name");
    }

    #[test]
    fn demangles_real_repro_rust_symbol_set_into_eight_identities() {
        let raw_names = [
            "_RNCNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric00B7_",
            "_RNCNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics0_00B7_",
            "_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric0B5_",
            "_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics0_0B5_",
            "_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics_0B5_",
            "_RNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric",
            "_RNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric",
            "_RNvNtNtCsiqc4wHYDJq1_7covgate7metrics5testss_30computes_changed_region_metric",
            "_RNvNtNtCsiqc4wHYDJq1_7covgate7metrics5testss_54metric_with_only_zero_totals_is_treated_as_unavailable",
            "_RNCNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric00B7_",
            "_RNCNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics0_00B7_",
            "_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric0B5_",
            "_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics0_0B5_",
            "_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics_0B5_",
        ];

        let normalized: std::collections::BTreeSet<_> = raw_names
            .into_iter()
            .map(normalize_llvm_function_name)
            .collect();

        let expected = std::collections::BTreeSet::from([
            "covgate::metrics::compute_changed_metric::{closure#0}::{closure#0}".to_string(),
            "covgate::metrics::compute_changed_metric::{closure#0}".to_string(),
            "covgate::metrics::compute_changed_metric".to_string(),
            "covgate::metrics::compute_changed_metric::{closure#1}".to_string(),
            "covgate::metrics::compute_changed_metric::{closure#2}".to_string(),
            "covgate::metrics::compute_changed_metric::{closure#2}::{closure#0}".to_string(),
            "covgate::metrics::tests::computes_changed_region_metric".to_string(),
            "covgate::metrics::tests::metric_with_only_zero_totals_is_treated_as_unavailable"
                .to_string(),
        ]);

        assert_eq!(normalized, expected);
    }

    #[test]
    fn identifies_named_functions_correctly() {
        let raw_names = [
            "_RNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric", // covgate::metrics::compute_changed_metric
            "_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric0B5_", // covgate::metrics::compute_changed_metric::{closure#0}
        ];

        let keys: Vec<_> = raw_names
            .into_iter()
            .map(|name| FunctionKey::NormalizedName {
                normalized_name: normalize_llvm_function_name(name),
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            })
            .collect();

        assert!(is_llvm_function_named(&keys[0]));
        assert!(!is_llvm_function_named(&keys[1]));

        let span_key = FunctionKey::Span {
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 1,
        };
        assert!(!is_llvm_function_named(&span_key));
    }

    #[test]
    fn verifies_named_function_totals() {
        let json = r#"{
            "data": [
                {
                    "files": [
                        {
                            "filename": "src/lib.rs",
                            "segments": [[1,1,1,true,true,false], [2,1,0,true,true,false]],
                            "branches": []
                        }
                    ],
                    "functions": [
                        {
                            "name": "_RNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric",
                            "filenames": ["src/lib.rs"],
                            "count": 1,
                            "regions": [[1,1,2,1,1,0,0,0]]
                        },
                        {
                            "name": "_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric0B5_",
                            "filenames": ["src/lib.rs"],
                            "count": 1,
                            "regions": [[1,1,2,1,1,0,0,0]]
                        }
                    ]
                }
            ],
            "type": "llvm.core.json.export",
            "version": "2.0.1"
        }"#;

        let repo_root = Path::new("/");
        let report = super::parse_with_repo_root(json, repo_root).unwrap();

        let path = PathBuf::from("src/lib.rs");
        let function_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Function)
            .and_then(|t| t.get(&path))
            .unwrap();
        assert_eq!(function_totals.total, 2);
        assert_eq!(function_totals.covered, 2);

        let named_function_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::NamedFunction)
            .and_then(|t| t.get(&path))
            .unwrap();
        assert_eq!(named_function_totals.total, 1);
        assert_eq!(named_function_totals.covered, 1);
    }
}
