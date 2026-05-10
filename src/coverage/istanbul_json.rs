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
    let report: HashMap<String, IstanbulFileCoverage> =
        serde_json::from_str(input).context("failed to parse istanbul json")?;

    let mut opportunities = Vec::new();
    let mut line_totals_by_file = BTreeMap::new();
    let mut branch_totals_by_file = BTreeMap::new();
    let mut function_totals_by_file = BTreeMap::new();
    let mut named_function_totals_by_file = BTreeMap::new();

    for (file_name, coverage) in report {
        let path = normalize_path(&file_name, repo_root);

        let mut line_states = BTreeMap::<(u32, u32), bool>::new();
        for (statement_id, statement) in &coverage.statement_map {
            let hits = coverage.s.get(statement_id).copied().unwrap_or(0);
            let covered = hits > 0;
            line_states
                .entry((statement.start.line, statement.start.column.unwrap_or(0)))
                .and_modify(|seen| *seen = *seen || covered)
                .or_insert(covered);
        }

        if !line_states.is_empty() {
            let covered = line_states
                .values()
                .filter(|is_covered| **is_covered)
                .count();
            let total = line_states.len();
            for ((line, column), is_covered) in line_states {
                opportunities.push(CoverageOpportunity {
                    kind: OpportunityKind::Line,
                    span: SourceSpan {
                        path: path.clone(),
                        start_line: line,
                        end_line: line,
                        start_col: Some(column),
                        end_col: Some(column),
                    },
                    covered: is_covered,
                    is_named_function: None,
                    named_function_identity: None,
                });
            }
            line_totals_by_file.insert(path.clone(), FileTotals { covered, total });
        }

        let mut branch_records = Vec::new();
        for (branch_id, branch_map) in &coverage.branch_map {
            let outcomes = coverage.b.get(branch_id).cloned().unwrap_or_default();
            let fallback_start_line = branch_map
                .loc
                .as_ref()
                .and_then(|span| span.start.line)
                .or(branch_map.line)
                .or(branch_map.loc.as_ref().and_then(|span| span.end.line));
            let fallback_start_col = branch_map.loc.as_ref().and_then(|span| span.start.column);
            let fallback_end_line = branch_map
                .loc
                .as_ref()
                .and_then(|span| span.end.line)
                .or(fallback_start_line);
            let fallback_end_col = branch_map.loc.as_ref().and_then(|span| span.end.column);

            for (index, location) in branch_map.locations.iter().enumerate() {
                let start_line = location.start.line.or(fallback_start_line);
                let start_col = location.start.column.or(fallback_start_col);
                let end_line = location.end.line.or(fallback_end_line).or(start_line);
                let end_col = location.end.column.or(fallback_end_col).or(start_col);

                let Some(start_line) = start_line else {
                    continue;
                };
                let end_line = end_line.unwrap_or(start_line);
                branch_records.push(BranchRecord {
                    start_line,
                    start_col: start_col.unwrap_or(0),
                    end_line,
                    end_col: end_col.unwrap_or(0),
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
                        start_line: record.start_line,
                        end_line: record.end_line,
                        start_col: Some(record.start_col),
                        end_col: Some(record.end_col),
                    },
                    covered: record.covered,
                    is_named_function: None,
                    named_function_identity: None,
                });
            }
            branch_totals_by_file.insert(path.clone(), FileTotals { covered, total });
        }

        let mut function_records = Vec::new();
        for (function_id, function_map) in &coverage.fn_map {
            let covered = coverage.f.get(function_id).copied().unwrap_or(0) > 0;
            let is_named = is_istanbul_function_named(function_map.name.as_deref());
            function_records.push(FunctionRecord {
                start_line: function_map.loc.start.line,
                start_col: function_map.loc.start.column.unwrap_or(0),
                end_line: function_map.loc.end.line,
                end_col: function_map.loc.end.column.unwrap_or(0),
                covered,
                is_named,
                named_function_identity: is_named.then(|| function_id.clone()),
            });
        }

        if !function_records.is_empty() {
            let covered = function_records
                .iter()
                .filter(|function| function.covered)
                .count();
            let total = function_records.len();

            let named_covered = function_records
                .iter()
                .filter(|function| function.is_named && function.covered)
                .count();
            let named_total = function_records
                .iter()
                .filter(|function| function.is_named)
                .count();

            for function in function_records {
                opportunities.push(CoverageOpportunity {
                    kind: OpportunityKind::Function,
                    span: SourceSpan {
                        path: path.clone(),
                        start_line: function.start_line,
                        end_line: function.end_line,
                        start_col: Some(function.start_col),
                        end_col: Some(function.end_col),
                    },
                    covered: function.covered,
                    is_named_function: Some(function.is_named),
                    named_function_identity: function.named_function_identity,
                });
            }
            function_totals_by_file.insert(path.clone(), FileTotals { covered, total });
            if named_total > 0 {
                named_function_totals_by_file.insert(
                    path,
                    FileTotals {
                        covered: named_covered,
                        total: named_total,
                    },
                );
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
    if !named_function_totals_by_file.is_empty() {
        totals_by_file.insert(MetricKind::NamedFunction, named_function_totals_by_file);
    }

    Ok(CoverageReport {
        opportunities,
        totals_by_file,
    })
}

fn is_istanbul_function_named(name: Option<&str>) -> bool {
    let Some(name) = name else {
        return false;
    };
    if name.is_empty() || name == "<anonymous>" {
        return false;
    }
    if name.starts_with("(anonymous") && name.ends_with(')') {
        return false;
    }
    true
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

    relativize_absolute_path(Path::new(&normalized_value), repo_root)
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
    name: Option<String>,
    loc: IstanbulSpan,
}

#[derive(Debug, Deserialize)]
struct IstanbulBranchMap {
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    loc: Option<IstanbulOptionalSpan>,
    locations: Vec<IstanbulOptionalSpan>,
}

#[derive(Debug, Deserialize)]
struct IstanbulSpan {
    start: IstanbulPosition,
    end: IstanbulPosition,
}

#[derive(Debug, Deserialize)]
struct IstanbulPosition {
    line: u32,
    #[serde(default)]
    column: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct IstanbulOptionalSpan {
    start: IstanbulOptionalPosition,
    end: IstanbulOptionalPosition,
}

#[derive(Debug, Deserialize, Default)]
struct IstanbulOptionalPosition {
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    column: Option<u32>,
}

#[derive(Debug)]
struct FunctionRecord {
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    covered: bool,
    is_named: bool,
    named_function_identity: Option<String>,
}

#[derive(Debug)]
struct BranchRecord {
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    covered: bool,
}
