use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::model::{
    CoverageOpportunity, CoverageReport, FileTotals, MetricKind, OpportunityKind, SourceSpan,
};

pub(crate) fn parse_str_with_repo_root(input: &str, repo_root: &Path) -> Result<CoverageReport> {
    let report: HashMap<String, IstanbulFileCoverage> =
        serde_json::from_str(input).context("failed to parse istanbul json")?;

    let mut opportunities = Vec::new();
    let mut line_totals_by_file = BTreeMap::new();
    let mut branch_totals_by_file = BTreeMap::new();
    let mut function_totals_by_file = BTreeMap::new();

    for (file_name, coverage) in report {
        let path = normalize_path(&file_name, repo_root);

        let mut lines = BTreeMap::<u32, bool>::new();
        for (statement_id, statement) in &coverage.statement_map {
            let hits = coverage.s.get(statement_id).copied().unwrap_or(0);
            let covered = hits > 0;
            for line in statement.start.line..=statement.end.line {
                lines
                    .entry(line)
                    .and_modify(|seen| *seen = *seen || covered)
                    .or_insert(covered);
            }
        }

        if !lines.is_empty() {
            let covered = lines.values().filter(|is_covered| **is_covered).count();
            let total = lines.len();
            for (line, is_covered) in lines {
                opportunities.push(CoverageOpportunity {
                    kind: OpportunityKind::Line,
                    span: SourceSpan {
                        path: path.clone(),
                        start_line: line,
                        end_line: line,
                    },
                    covered: is_covered,
                });
            }
            line_totals_by_file.insert(path.clone(), FileTotals { covered, total });
        }

        let mut branch_records = Vec::new();
        for (branch_id, branch_map) in &coverage.branch_map {
            let outcomes = coverage.b.get(branch_id).cloned().unwrap_or_default();
            for (index, location) in branch_map.locations.iter().enumerate() {
                branch_records.push(BranchRecord {
                    line: location.start.line,
                    covered: outcomes.get(index).copied().unwrap_or(0) > 0,
                });
            }
        }

        if !branch_records.is_empty() {
            let covered = branch_records
                .iter()
                .filter(|record| record.covered)
                .count();
            let total = branch_records.len();
            for record in branch_records {
                opportunities.push(CoverageOpportunity {
                    kind: OpportunityKind::BranchOutcome,
                    span: SourceSpan {
                        path: path.clone(),
                        start_line: record.line,
                        end_line: record.line,
                    },
                    covered: record.covered,
                });
            }
            branch_totals_by_file.insert(path.clone(), FileTotals { covered, total });
        }

        let mut function_records = Vec::new();
        for (function_id, function_map) in &coverage.fn_map {
            let covered = coverage.f.get(function_id).copied().unwrap_or(0) > 0;
            function_records.push(FunctionRecord {
                start_line: function_map.loc.start.line,
                end_line: function_map.loc.end.line,
                covered,
            });
        }

        if !function_records.is_empty() {
            let covered = function_records
                .iter()
                .filter(|function| function.covered)
                .count();
            let total = function_records.len();
            for function in function_records {
                opportunities.push(CoverageOpportunity {
                    kind: OpportunityKind::Function,
                    span: SourceSpan {
                        path: path.clone(),
                        start_line: function.start_line,
                        end_line: function.end_line,
                    },
                    covered: function.covered,
                });
            }
            function_totals_by_file.insert(path, FileTotals { covered, total });
        }
    }

    let mut totals_by_file = BTreeMap::new();
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
    let normalized_value = value.replace('\\', "/");
    let repo_root_string = repo_root.to_string_lossy().replace('\\', "/");

    if normalized_value == repo_root_string {
        return PathBuf::new();
    }

    if let Some(stripped) = normalized_value.strip_prefix(&format!("{repo_root_string}/")) {
        return lexical_normalize(Path::new(stripped));
    }

    let path = lexical_normalize(Path::new(&normalized_value));
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

#[derive(Debug, Deserialize)]
struct IstanbulFileCoverage {
    #[serde(rename = "statementMap", default)]
    statement_map: HashMap<String, IstanbulSpan>,
    #[serde(default)]
    s: HashMap<String, u64>,
    #[serde(rename = "branchMap", default)]
    branch_map: HashMap<String, IstanbulBranchMap>,
    #[serde(default)]
    b: HashMap<String, Vec<u64>>,
    #[serde(rename = "fnMap", default)]
    fn_map: HashMap<String, IstanbulFunctionMap>,
    #[serde(default)]
    f: HashMap<String, u64>,
}

#[derive(Debug, Deserialize)]
struct IstanbulFunctionMap {
    loc: IstanbulSpan,
}

#[derive(Debug, Deserialize)]
struct IstanbulBranchMap {
    locations: Vec<IstanbulSpan>,
}

#[derive(Debug, Deserialize)]
struct IstanbulSpan {
    start: IstanbulPosition,
    end: IstanbulPosition,
}

#[derive(Debug, Deserialize)]
struct IstanbulPosition {
    line: u32,
}

#[derive(Debug)]
struct FunctionRecord {
    start_line: u32,
    end_line: u32,
    covered: bool,
}

#[derive(Debug)]
struct BranchRecord {
    line: u32,
    covered: bool,
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::model::MetricKind;

    use super::parse_str_with_repo_root;

    #[test]
    fn parses_istanbul_line_branch_and_function_totals() {
        let input = r#"
        {
          "src/math.js": {
            "path": "src/math.js",
            "statementMap": {
              "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}},
              "1": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 10}}
            },
            "s": {"0": 1, "1": 0},
            "branchMap": {
              "0": {
                "loc": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 10}},
                "type": "if",
                "locations": [
                  {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 10}},
                  {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 10}}
                ]
              }
            },
            "b": {"0": [1, 0]},
            "fnMap": {
              "0": {
                "name": "compute",
                "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}},
                "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 3, "column": 1}},
                "line": 1
              }
            },
            "f": {"0": 1}
          }
        }
        "#;

        let report = parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
            .expect("istanbul json should parse");

        let line_totals = report
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .get(&PathBuf::from("src/math.js"))
            .expect("line totals should include fixture file");
        assert_eq!(line_totals.covered, 1);
        assert_eq!(line_totals.total, 2);

        let branch_totals = report
            .totals_by_file
            .get(&MetricKind::Branch)
            .expect("branch totals should exist")
            .get(&PathBuf::from("src/math.js"))
            .expect("branch totals should include fixture file");
        assert_eq!(branch_totals.covered, 1);
        assert_eq!(branch_totals.total, 2);

        let function_totals = report
            .totals_by_file
            .get(&MetricKind::Function)
            .expect("function totals should exist")
            .get(&PathBuf::from("src/math.js"))
            .expect("function totals should include fixture file");
        assert_eq!(function_totals.covered, 1);
        assert_eq!(function_totals.total, 1);
    }

    #[test]
    fn parse_rejects_invalid_json() {
        let error = parse_str_with_repo_root("{", Path::new("/workspace/covgate"))
            .expect_err("invalid json should fail");
        assert!(error.to_string().contains("failed to parse istanbul json"));
    }

    #[test]
    fn merges_overlapping_statement_lines_as_covered_when_any_statement_hits() {
        let input = r#"
        {
          "src/math.js": {
            "statementMap": {
              "0": {"start": {"line": 2}, "end": {"line": 2}},
              "1": {"start": {"line": 2}, "end": {"line": 2}}
            },
            "s": {"0": 0, "1": 1},
            "branchMap": {},
            "b": {},
            "fnMap": {},
            "f": {}
          }
        }
        "#;

        let report = parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
            .expect("istanbul json should parse");

        let line_totals = report
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .get(&PathBuf::from("src/math.js"))
            .expect("file totals should exist");
        assert_eq!(line_totals.covered, 1);
        assert_eq!(line_totals.total, 1);

        assert!(!report.totals_by_file.contains_key(&MetricKind::Branch));
        assert!(!report.totals_by_file.contains_key(&MetricKind::Function));
    }

    #[test]
    fn normalizes_repo_prefixed_and_absolute_paths() {
        let prefixed = parse_str_with_repo_root(
            r#"{
              "/workspace/covgate/src/math.js": {
                "statementMap": {"0": {"start": {"line": 1}, "end": {"line": 1}}},
                "s": {"0": 1},
                "branchMap": {},
                "b": {},
                "fnMap": {},
                "f": {}
              }
            }"#,
            Path::new("/workspace/covgate"),
        )
        .expect("prefixed path should parse");
        assert!(
            prefixed
                .totals_by_file
                .get(&MetricKind::Line)
                .expect("line totals should exist")
                .contains_key(&PathBuf::from("src/math.js"))
        );

        let absolute_outside = parse_str_with_repo_root(
            r#"{
              "/opt/other/math.js": {
                "statementMap": {"0": {"start": {"line": 1}, "end": {"line": 1}}},
                "s": {"0": 1},
                "branchMap": {},
                "b": {},
                "fnMap": {},
                "f": {}
              }
            }"#,
            Path::new("/workspace/covgate"),
        )
        .expect("absolute outside path should parse");
        assert!(
            absolute_outside
                .totals_by_file
                .get(&MetricKind::Line)
                .expect("line totals should exist")
                .contains_key(&PathBuf::from("/opt/other/math.js"))
        );
    }

    #[test]
    fn does_not_strip_repo_root_text_prefix_when_not_path_boundary() {
        let report = parse_str_with_repo_root(
            r#"{
              "/workspace/covgate-old/src/math.js": {
                "statementMap": {"0": {"start": {"line": 1}, "end": {"line": 1}}},
                "s": {"0": 1},
                "branchMap": {},
                "b": {},
                "fnMap": {},
                "f": {}
              }
            }"#,
            Path::new("/workspace/covgate"),
        )
        .expect("path should parse");

        assert!(
            report
                .totals_by_file
                .get(&MetricKind::Line)
                .expect("line totals should exist")
                .contains_key(&PathBuf::from("/workspace/covgate-old/src/math.js"))
        );
        assert!(
            !report
                .totals_by_file
                .get(&MetricKind::Line)
                .expect("line totals should exist")
                .contains_key(&PathBuf::from("-old/src/math.js"))
        );
    }
}
