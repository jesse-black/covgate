use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::model::{
    CoverageOpportunity, CoverageReport, FileTotals, MetricKind, OpportunityKind, SourceSpan,
};

use super::path::{lexical_normalize, relativize_absolute_path};

pub(crate) fn parse_with_repo_root(input: &str, repo_root: &Path) -> Result<CoverageReport> {
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
                            start_col: None,
                            end_col: None,
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
                            start_col: None,
                            end_col: None,
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
                            start_col: None,
                            end_col: None,
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

    relativize_absolute_path(Path::new(&normalized_value), repo_root)
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

    use super::normalize_path;

    #[test]
    fn normalizes_windows_path_separators() {
        let repo_root = Path::new("C:/workspace/covgate");
        let normalized = normalize_path("C:\\workspace\\covgate\\src\\lib.cs", repo_root);
        assert_eq!(normalized, PathBuf::from("src/lib.cs"));
    }

    #[test]
    fn keeps_absolute_paths_outside_repo_as_absolute() {
        let repo_root = Path::new("/workspace/covgate");
        let normalized = normalize_path("/tmp/other/src/lib.cs", repo_root);
        assert_eq!(normalized, PathBuf::from("/tmp/other/src/lib.cs"));
    }
}
