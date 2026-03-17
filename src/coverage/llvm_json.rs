use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::model::{
    CoverageOpportunity, CoverageReport, FileTotals, MetricKind, OpportunityKind, SourceSpan,
};

pub(crate) fn parse_str_with_repo_root(input: &str, repo_root: &Path) -> Result<CoverageReport> {
    let export: LlvmExport = serde_json::from_str(input).context("failed to parse llvm json")?;
    let mut opportunities = Vec::new();
    let mut region_totals_by_file = BTreeMap::new();
    let mut line_totals_by_file = BTreeMap::new();
    let mut branch_totals_by_file = BTreeMap::new();
    let mut function_totals_by_file = BTreeMap::new();

    for data in export.data {
        let known_file_paths: Vec<PathBuf> = data
            .files
            .iter()
            .map(|file| normalize_path(&file.filename, repo_root))
            .collect();

        let mut function_records_by_file: BTreeMap<PathBuf, BTreeMap<FunctionSpanKey, bool>> =
            BTreeMap::new();
        for function in data.functions {
            if function.filenames.is_empty() {
                continue;
            }
            let path =
                normalize_function_path(&function.filenames[0], repo_root, &known_file_paths);
            let mut start_line: Option<u32> = None;
            let mut end_line: Option<u32> = None;
            let mut region_covered = false;
            for region in function.regions {
                start_line =
                    Some(start_line.map_or(region.line_start, |cur| cur.min(region.line_start)));
                end_line = Some(end_line.map_or(region.line_end, |cur| cur.max(region.line_end)));
                region_covered |= region.execution_count > 0;
            }
            let (Some(start_line), Some(end_line)) = (start_line, end_line) else {
                continue;
            };
            let entry = function_records_by_file.entry(path).or_default();
            let key = FunctionSpanKey {
                start_line,
                end_line,
            };
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
                    },
                    covered: region.covered,
                });
            }

            let region_totals = file
                .summary
                .as_ref()
                .and_then(|summary| summary.regions.as_ref())
                .map(|summary| FileTotals {
                    covered: summary.covered,
                    total: summary.count,
                })
                .unwrap_or(FileTotals {
                    covered: region_covered,
                    total: region_total,
                });
            region_totals_by_file.insert(path.clone(), region_totals);

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
                    },
                    covered: line.covered,
                });
            }

            if let Some(summary) = file
                .summary
                .as_ref()
                .and_then(|summary| summary.lines.as_ref())
            {
                if summary.count > 0 {
                    line_totals_by_file.insert(
                        path.clone(),
                        FileTotals {
                            covered: summary.covered,
                            total: summary.count,
                        },
                    );
                }
            } else if line_total > 0 {
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
                    },
                    covered: branch.covered,
                });
            }

            if let Some(summary) = file
                .summary
                .as_ref()
                .and_then(|summary| summary.branches.as_ref())
            {
                if summary.count > 0 {
                    branch_totals_by_file.insert(
                        path.clone(),
                        FileTotals {
                            covered: summary.covered,
                            total: summary.count,
                        },
                    );
                }
            } else if branch_total > 0 {
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
                for (span, covered) in function_records {
                    if covered {
                        function_covered += 1;
                    }
                    opportunities.push(CoverageOpportunity {
                        kind: OpportunityKind::Function,
                        span: SourceSpan {
                            path: path.clone(),
                            start_line: span.start_line,
                            end_line: span.end_line,
                        },
                        covered,
                    });
                }
                if let Some(summary) = file
                    .summary
                    .as_ref()
                    .and_then(|summary| summary.functions.as_ref())
                {
                    function_totals_by_file.insert(
                        path,
                        FileTotals {
                            covered: summary.covered,
                            total: summary.count,
                        },
                    );
                } else {
                    function_totals_by_file.insert(
                        path,
                        FileTotals {
                            covered: function_covered,
                            total: function_total,
                        },
                    );
                }
            } else if let Some(summary) = file
                .summary
                .as_ref()
                .and_then(|summary| summary.functions.as_ref())
                .filter(|summary| summary.count > 0)
            {
                function_totals_by_file.insert(
                    path,
                    FileTotals {
                        covered: summary.covered,
                        total: summary.count,
                    },
                );
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

    Ok(CoverageReport {
        opportunities,
        totals_by_file,
    })
}

fn normalize_path(value: &str, repo_root: &Path) -> PathBuf {
    let path = lexical_normalize(Path::new(value));
    let repo_root = lexical_normalize(repo_root);
    if path.is_absolute() {
        path.strip_prefix(&repo_root)
            .map(lexical_normalize)
            .unwrap_or(path)
    } else {
        path
    }
}

fn lexical_normalize(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().components().collect()
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
    regions: Vec<LlvmFunctionRegion>,
}

#[derive(Debug, Deserialize)]
struct LlvmFunctionRegion {
    #[serde(deserialize_with = "de_u32_from_i64")]
    line_start: u32,
    #[serde(deserialize_with = "de_u32_from_i64")]
    _col_start: u32,
    #[serde(deserialize_with = "de_u32_from_i64")]
    line_end: u32,
    #[serde(deserialize_with = "de_u32_from_i64")]
    _col_end: u32,
    #[serde(default)]
    execution_count: u64,
    #[serde(default)]
    _file_id: u32,
    #[serde(default)]
    _expanded_file_id: u32,
    #[serde(default)]
    _kind: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FunctionSpanKey {
    start_line: u32,
    end_line: u32,
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
    #[serde(default)]
    summary: Option<LlvmFileSummary>,
}

#[derive(Debug, Deserialize)]
struct LlvmFileSummary {
    #[serde(default)]
    regions: Option<LlvmSummaryTotals>,
    #[serde(default)]
    lines: Option<LlvmSummaryTotals>,
    #[serde(default)]
    functions: Option<LlvmSummaryTotals>,
    #[serde(default)]
    branches: Option<LlvmSummaryTotals>,
}

#[derive(Debug, Deserialize)]
struct LlvmSummaryTotals {
    count: usize,
    covered: usize,
}

#[derive(Debug)]
struct LineRecord {
    line_number: u32,
    covered: bool,
}

#[derive(Debug)]
struct RegionRecord {
    start_line: u32,
    end_line: u32,
    covered: bool,
}

#[derive(Debug)]
struct BranchRecord {
    line_number: u32,
    covered: bool,
}

impl LlvmFile {
    fn segments_to_regions(&self) -> Result<Vec<RegionRecord>> {
        let mut regions = Vec::new();

        for window in self.segments.windows(2) {
            let start = &window[0];
            let end = &window[1];

            let start_line = number_at(start, 0)?;
            let end_line = number_at(end, 0)?;

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
                end_line,
                covered: count > 0,
            });
        }

        Ok(regions)
    }

    fn parse_lines(&self) -> Result<Vec<LineRecord>> {
        let mut line_states: std::collections::BTreeMap<u32, bool> =
            std::collections::BTreeMap::new();
        for window in self.segments.windows(2) {
            let start = &window[0];
            let end = &window[1];

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

            if branch.len() >= 6 {
                let true_count = number_at(branch, 4)?;
                let false_count = number_at(branch, 5)?;
                branches.push(BranchRecord {
                    line_number,
                    covered: true_count > 0,
                });
                branches.push(BranchRecord {
                    line_number,
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

    use super::{normalize_path, parse_str_with_repo_root};

    fn parse_str(input: &str) -> anyhow::Result<crate::model::CoverageReport> {
        parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
    }

    #[test]
    fn parses_basic_llvm_export() {
        let input = r#"
        {
          "data": [
            {
              "functions": [
                {
                  "count": 1,
                  "filenames": ["src/lib.rs"],
                  "regions": [[1,1,2,1,1,0,0,0]]
                },
                {
                  "count": 0,
                  "filenames": ["src/lib.rs"],
                  "regions": [[3,1,4,1,0,0,0,0]]
                }
              ],
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, true, false],
                    [1, 2, 0, false, false, false],
                    [2, 1, 1, true, true, false],
                    [2, 2, 0, false, false, false],
                    [3, 1, 0, true, true, false],
                    [3, 2, 0, false, false, false],
                    [4, 1, 0, true, true, false],
                    [4, 2, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        assert_eq!(report.opportunities.len(), 10); // 4 regions + 4 lines + 2 functions

        let region_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Region)
            .expect("region metric totals should exist")
            .get(&std::path::PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");
        assert_eq!(region_totals.covered, 2);
        assert_eq!(region_totals.total, 4);

        let line_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Line)
            .expect("line metric totals should exist")
            .get(&std::path::PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");
        assert_eq!(line_totals.covered, 2);
        assert_eq!(line_totals.total, 4);

        let function_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Function)
            .expect("function metric totals should exist")
            .get(&std::path::PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");
        assert_eq!(function_totals.covered, 1);
        assert_eq!(function_totals.total, 2);
    }

    #[test]
    fn parses_branch_metrics_when_branches_are_present() {
        let input = r#"
        {
          "data": [
            {
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [2, 1, 0, false, false, false]
                  ],
                  "branches": [
                    [1, 1, 1, true],
                    [1, 5, 0, true]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");

        let branch_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Branch)
            .expect("branch totals should be present");
        let file_totals = branch_totals
            .get(&PathBuf::from("src/lib.rs"))
            .expect("branch file totals should be present");
        assert_eq!(file_totals.covered, 1);
        assert_eq!(file_totals.total, 2);

        let branch_opportunities: Vec<_> = report
            .opportunities
            .iter()
            .filter(|op| op.kind == crate::model::OpportunityKind::BranchOutcome)
            .collect();
        assert_eq!(branch_opportunities.len(), 2);
    }

    #[test]
    fn parses_llvm_branch_tuples_using_true_false_counts() {
        let input = r#"
        {
          "data": [
            {
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [2, 1, 0, false, false, false]
                  ],
                  "branches": [
                    [2, 5, 2, 10, 1, 0, 0, 0, 4]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");

        let branch_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Branch)
            .expect("branch totals should be present");
        let file_totals = branch_totals
            .get(&PathBuf::from("src/lib.rs"))
            .expect("branch file totals should be present");
        assert_eq!(file_totals.covered, 1);
        assert_eq!(file_totals.total, 2);
    }

    #[test]
    fn parses_legacy_branch_entries_and_skips_has_count_false() {
        let input = r#"
        {
          "data": [
            {
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [2, 1, 0, false, false, false]
                  ],
                  "branches": [
                    [2, 1, 0, false],
                    [3, 1, 1, true]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");

        let branch_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Branch)
            .expect("branch totals should be present");
        let file_totals = branch_totals
            .get(&PathBuf::from("src/lib.rs"))
            .expect("branch file totals should be present");

        // The first legacy entry is skipped because has_count=false.
        assert_eq!(file_totals.covered, 1);
        assert_eq!(file_totals.total, 1);
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_str("{").is_err());
    }

    #[test]
    fn segment_boundary_does_not_overcount_lines() {
        // This tests the exact case reported: "a window from (1,1) to (2,1) gets counted as covering both lines 1 and 2".
        // With the fix, an end_col <= 1 should NOT include the end_line in the derivation.
        let input = r#"
        {
          "data": [
            {
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [2, 1, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        let line_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Line)
            .expect("line metric totals should exist")
            .get(&std::path::PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");

        // Only line 1 should be covered and counted.
        assert_eq!(line_totals.covered, 1);
        assert_eq!(line_totals.total, 1);
    }

    #[test]
    fn region_totals_skip_non_region_entries() {
        let input = r#"
        {
          "data": [
            {
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [2, 1, 0, true, true, false],
                    [3, 1, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        let region_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Region)
            .expect("region metric totals should exist")
            .get(&PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");

        assert_eq!(region_totals.covered, 0);
        assert_eq!(region_totals.total, 1);
    }

    #[test]
    fn region_totals_skip_gap_regions() {
        let input = r#"
        {
          "data": [
            {
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 3, true, true, true],
                    [2, 1, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        let region_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Region)
            .expect("region metric totals should exist")
            .get(&PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");

        assert_eq!(region_totals.covered, 0);
        assert_eq!(region_totals.total, 0);
    }

    #[test]
    fn prefers_file_summary_totals_over_segment_derived_totals() {
        let input = r#"
        {
          "data": [
            {
              "functions": [
                {
                  "count": 1,
                  "filenames": ["src/lib.rs"],
                  "regions": [[1,1,2,1,1,0,0,0]]
                }
              ],
              "files": [
                {
                  "filename": "src/lib.rs",
                  "summary": {
                    "regions": { "count": 10, "covered": 9 },
                    "lines": { "count": 7, "covered": 6 },
                    "functions": { "count": 3, "covered": 2 }
                  },
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [2, 1, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");

        let region_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Region)
            .expect("region totals should exist")
            .get(&PathBuf::from("src/lib.rs"))
            .expect("file region totals should exist");
        assert_eq!(region_totals.covered, 9);
        assert_eq!(region_totals.total, 10);

        let line_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Line)
            .expect("line totals should exist")
            .get(&PathBuf::from("src/lib.rs"))
            .expect("file line totals should exist");
        assert_eq!(line_totals.covered, 6);
        assert_eq!(line_totals.total, 7);

        let function_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Function)
            .expect("function totals should exist")
            .get(&PathBuf::from("src/lib.rs"))
            .expect("file function totals should exist");
        assert_eq!(function_totals.covered, 2);
        assert_eq!(function_totals.total, 3);
    }

    #[test]
    fn normalizes_absolute_paths_to_repo_relative() {
        let repo_root = Path::new("/workspace/covgate");
        let normalized = normalize_path("/workspace/covgate/src/lib.rs", repo_root);
        assert_eq!(normalized, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn skips_function_entries_without_filenames_or_regions() {
        let input = r#"
        {
          "data": [
            {
              "functions": [
                {
                  "count": 1,
                  "filenames": [],
                  "regions": [[1,1,2,1,1,0,0,0]]
                },
                {
                  "count": 1,
                  "filenames": ["src/lib.rs"],
                  "regions": []
                }
              ],
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [2, 1, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        assert!(
            !report
                .totals_by_file
                .contains_key(&crate::model::MetricKind::Function)
        );
    }

    #[test]
    fn rejects_negative_function_region_fields() {
        let input = r#"
        {
          "data": [
            {
              "functions": [
                {
                  "count": 1,
                  "filenames": ["src/lib.rs"],
                  "regions": [[-1,1,2,1,1,0,0,0]]
                }
              ],
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [2, 1, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let error = parse_str(input).expect_err("negative line should fail parsing");
        assert!(error.to_string().contains("failed to parse llvm json"));
    }

    #[test]
    fn marks_function_covered_when_regions_have_execution_count() {
        let input = r#"
        {
          "data": [
            {
              "functions": [
                {
                  "count": 0,
                  "filenames": ["src/lib.rs"],
                  "regions": [[10,1,12,1,3,0,0,0]]
                }
              ],
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [10, 1, 1, true, false, false],
                    [12, 1, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        let totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Function)
            .expect("function totals should exist")
            .get(&PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");

        assert_eq!(totals.covered, 1);
        assert_eq!(totals.total, 1);
    }

    #[test]
    fn merges_duplicate_function_spans_as_covered_if_any_variant_is_covered() {
        let input = r#"
        {
          "data": [
            {
              "functions": [
                {
                  "count": 0,
                  "filenames": ["src/lib.rs"],
                  "regions": [[20,1,25,1,0,0,0,0]]
                },
                {
                  "count": 1,
                  "filenames": ["src/lib.rs"],
                  "regions": [[20,1,25,1,1,0,0,0]]
                }
              ],
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [20, 1, 1, true, false, false],
                    [25, 1, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        let totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Function)
            .expect("function totals should exist")
            .get(&PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");

        assert_eq!(totals.covered, 1);
        assert_eq!(totals.total, 1);
    }

    #[test]
    fn prefers_longest_suffix_for_function_file_mapping() {
        let input = r#"
        {
          "data": [
            {
              "functions": [
                {
                  "count": 0,
                  "filenames": ["/tmp/build/pkg/src/lib.rs"],
                  "regions": [[10,1,10,5,0,0,0,0]]
                }
              ],
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [[1,1,1,true,false,false],[2,1,0,false,false,false]]
                },
                {
                  "filename": "pkg/src/lib.rs",
                  "segments": [[1,1,1,true,false,false],[2,1,0,false,false,false]]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        let function_totals = report
            .totals_by_file
            .get(&crate::model::MetricKind::Function)
            .expect("function totals should exist");

        assert!(
            !function_totals.contains_key(&PathBuf::from("src/lib.rs")),
            "function should not map to less specific suffix"
        );
        let mapped = function_totals
            .get(&PathBuf::from("pkg/src/lib.rs"))
            .expect("function should map to longest matching suffix");
        assert_eq!(mapped.covered, 0);
        assert_eq!(mapped.total, 1);
    }
}
