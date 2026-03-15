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
    let export: HashMap<String, HashMap<String, serde_json::Value>> =
        serde_json::from_str(input).context("failed to parse coverlet json")?;

    let mut opportunities = Vec::new();
    let mut line_totals_by_file = BTreeMap::new();
    let mut branch_totals_by_file = BTreeMap::new();
    let mut function_totals_by_file = BTreeMap::new();

    for classes_by_file in export.into_values() {
        for (file_name, class_value) in classes_by_file {
            let path = normalize_path(&file_name, repo_root);
            let mut line_hits_by_line = BTreeMap::<u32, bool>::new();
            let mut branch_records = Vec::<BranchRecord>::new();
            let mut function_records = Vec::<FunctionRecord>::new();

            let Some(classes) = class_value.as_object() else {
                continue;
            };

            for methods_value in classes.values() {
                let Some(methods) = methods_value.as_object() else {
                    continue;
                };
                for method_value in methods.values() {
                    let Ok(method) = serde_json::from_value::<CoverletMethod>(method_value.clone())
                    else {
                        continue;
                    };

                    for (&line_number, &hits) in &method.lines {
                        let covered = hits > 0;
                        line_hits_by_line
                            .entry(line_number)
                            .and_modify(|seen| *seen = *seen || covered)
                            .or_insert(covered);
                    }

                    branch_records.extend(method.branches);

                    let start_line = method.lines.keys().copied().min();
                    let end_line = method.lines.keys().copied().max();
                    if let (Some(start_line), Some(end_line)) = (start_line, end_line) {
                        let covered = method.lines.values().any(|hits| *hits > 0);
                        function_records.push(FunctionRecord {
                            start_line,
                            end_line,
                            covered,
                        });
                    }
                }
            }

            if !line_hits_by_line.is_empty() {
                let total = line_hits_by_line.len();
                let mut covered = 0usize;
                for (line_number, is_covered) in line_hits_by_line {
                    if is_covered {
                        covered += 1;
                    }
                    opportunities.push(CoverageOpportunity {
                        kind: OpportunityKind::Line,
                        span: SourceSpan {
                            path: path.clone(),
                            start_line: line_number,
                            end_line: line_number,
                        },
                        covered: is_covered,
                    });
                }
                line_totals_by_file.insert(path.clone(), FileTotals { covered, total });
            }

            if !branch_records.is_empty() {
                let mut covered = 0usize;
                let total = branch_records.len();
                for branch in branch_records {
                    let is_covered = branch.hits > 0;
                    if is_covered {
                        covered += 1;
                    }
                    opportunities.push(CoverageOpportunity {
                        kind: OpportunityKind::BranchOutcome,
                        span: SourceSpan {
                            path: path.clone(),
                            start_line: branch.line,
                            end_line: branch.line,
                        },
                        covered: is_covered,
                    });
                }
                branch_totals_by_file.insert(path.clone(), FileTotals { covered, total });
            }

            if !function_records.is_empty() {
                let mut covered = 0usize;
                let total = function_records.len();
                for function in function_records {
                    if function.covered {
                        covered += 1;
                    }
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

    if let Some(stripped) = normalized_value
        .strip_prefix(&format!("{repo_root_string}/"))
        .or_else(|| normalized_value.strip_prefix(&repo_root_string))
    {
        let trimmed = stripped.trim_start_matches('/');
        return lexical_normalize(Path::new(trimmed));
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
struct CoverletMethod {
    #[serde(rename = "Lines", deserialize_with = "deserialize_line_hits")]
    lines: HashMap<u32, u64>,
    #[serde(rename = "Branches", default)]
    branches: Vec<BranchRecord>,
}

#[derive(Debug, Deserialize)]
struct BranchRecord {
    #[serde(rename = "Line")]
    line: u32,
    #[serde(rename = "Hits")]
    hits: u64,
}

#[derive(Debug)]
struct FunctionRecord {
    start_line: u32,
    end_line: u32,
    covered: bool,
}

fn deserialize_line_hits<'de, D>(deserializer: D) -> Result<HashMap<u32, u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let parsed: HashMap<String, u64> = HashMap::deserialize(deserializer)?;
    parsed
        .into_iter()
        .map(|(line, hits)| {
            line.parse::<u32>()
                .map(|line_number| (line_number, hits))
                .map_err(serde::de::Error::custom)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::model::{MetricKind, OpportunityKind};

    use super::{normalize_path, parse_str_with_repo_root};

    #[test]
    fn parses_coverlet_lines_and_branches() {
        let input = r#"
        {
          "Demo.dll": {
            "/workspace/covgate/src/lib.cs": {
              "Demo.MathOps": {
                "System.Int32 Demo.MathOps::Add(System.Int32,System.Int32)": {
                  "Lines": {
                    "3": 1,
                    "4": 0
                  },
                  "Branches": [
                    {"Line": 4, "Hits": 1},
                    {"Line": 4, "Hits": 0}
                  ]
                }
              }
            }
          }
        }
        "#;

        let report = parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
            .expect("coverlet json should parse");

        let line_totals = report
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .get(&PathBuf::from("src/lib.cs"))
            .expect("file totals should exist");
        assert_eq!(line_totals.covered, 1);
        assert_eq!(line_totals.total, 2);

        let branch_totals = report
            .totals_by_file
            .get(&MetricKind::Branch)
            .expect("branch totals should exist")
            .get(&PathBuf::from("src/lib.cs"))
            .expect("file totals should exist");
        assert_eq!(branch_totals.covered, 1);
        assert_eq!(branch_totals.total, 2);

        let function_totals = report
            .totals_by_file
            .get(&MetricKind::Function)
            .expect("function totals should exist")
            .get(&PathBuf::from("src/lib.cs"))
            .expect("file totals should exist");
        assert_eq!(function_totals.covered, 1);
        assert_eq!(function_totals.total, 1);
    }

    #[test]
    fn computes_function_spans_from_method_lines() {
        let input = r#"
        {
          "Demo.dll": {
            "src/lib.cs": {
              "Demo.MathOps": {
                "Covered": {"Lines": {"10": 1, "11": 0, "15": 2}, "Branches": []},
                "Uncovered": {"Lines": {"20": 0, "21": 0}, "Branches": []}
              }
            }
          }
        }
        "#;

        let report = parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
            .expect("coverlet json should parse");

        let function_ops: Vec<_> = report
            .opportunities
            .iter()
            .filter(|op| op.kind == OpportunityKind::Function)
            .collect();
        assert_eq!(function_ops.len(), 2);
        assert!(
            function_ops
                .iter()
                .any(|op| { op.span.start_line == 10 && op.span.end_line == 15 && op.covered })
        );
        assert!(
            function_ops
                .iter()
                .any(|op| { op.span.start_line == 20 && op.span.end_line == 21 && !op.covered })
        );
    }

    #[test]
    fn normalizes_windows_path_separators() {
        let repo_root = Path::new("C:/workspace/covgate");
        let normalized = normalize_path("C:\\workspace\\covgate\\src\\lib.cs", repo_root);
        assert_eq!(normalized, PathBuf::from("src/lib.cs"));
    }

    #[test]
    fn merges_duplicate_lines_across_methods() {
        let input = r#"
        {
          "Demo.dll": {
            "src/lib.cs": {
              "Demo.MathOps": {
                "M1": {"Lines": {"10": 0, "11": 1}, "Branches": []},
                "M2": {"Lines": {"10": 2}, "Branches": []}
              }
            }
          }
        }
        "#;

        let report = parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
            .expect("coverlet json should parse");

        let line_totals = report
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .get(&PathBuf::from("src/lib.cs"))
            .expect("file totals should exist");
        assert_eq!(line_totals.total, 2);
        assert_eq!(line_totals.covered, 2);
    }

    #[test]
    fn skips_non_object_class_or_method_entries() {
        let input = r#"
        {
          "Demo.dll": {
            "src/lib.cs": {
              "IgnoredClass": 5,
              "Demo.MathOps": {
                "IgnoredMethod": 3,
                "RealMethod": {"Lines": {"5": 1}, "Branches": []}
              }
            }
          }
        }
        "#;

        let report = parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
            .expect("coverlet json should parse");
        let lines: Vec<_> = report
            .opportunities
            .iter()
            .filter(|op| op.kind == OpportunityKind::Line)
            .collect();
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn invalid_line_key_method_is_ignored() {
        let input = r#"
        {
          "Demo.dll": {
            "src/lib.cs": {
              "Demo.MathOps": {
                "BadMethod": {"Lines": {"not-a-line": 1}, "Branches": []},
                "GoodMethod": {"Lines": {"7": 1}, "Branches": []}
              }
            }
          }
        }
        "#;

        let report = parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
            .expect("coverlet json should parse");
        let line_totals = report
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .get(&PathBuf::from("src/lib.cs"))
            .expect("file totals should exist");
        assert_eq!(line_totals.total, 1);
        assert_eq!(line_totals.covered, 1);
    }

    #[test]
    fn keeps_absolute_paths_outside_repo_as_absolute() {
        let repo_root = Path::new("/workspace/covgate");
        let normalized = normalize_path("/tmp/other/src/lib.cs", repo_root);
        assert_eq!(normalized, PathBuf::from("/tmp/other/src/lib.cs"));
    }

    #[test]
    fn skips_function_metric_when_method_has_no_lines() {
        let input = r#"
        {
          "Demo.dll": {
            "src/lib.cs": {
              "Demo.MathOps": {
                "NoLines": {"Lines": {}, "Branches": []}
              }
            }
          }
        }
        "#;

        let report = parse_str_with_repo_root(input, Path::new("/workspace/covgate"))
            .expect("coverlet json should parse");

        assert!(!report.totals_by_file.contains_key(&MetricKind::Function));
    }
}
