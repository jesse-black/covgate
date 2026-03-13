use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::model::{
    CoverageOpportunity, CoverageReport, FileTotals, MetricKind, OpportunityKind, SourceSpan,
};

pub fn parse_path(path: &Path) -> Result<CoverageReport> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read coverage json: {}", path.display()))?;
    parse_str(&text)
}

pub fn parse_str(input: &str) -> Result<CoverageReport> {
    let repo_root = env::current_dir()
        .context("failed to determine current directory for llvm path normalization")?;
    parse_str_with_repo_root(input, &repo_root)
}

fn parse_str_with_repo_root(input: &str, repo_root: &Path) -> Result<CoverageReport> {
    let export: LlvmExport = serde_json::from_str(input).context("failed to parse llvm json")?;
    let mut opportunities = Vec::new();
    let mut region_totals_by_file = BTreeMap::new();
    let mut line_totals_by_file = BTreeMap::new();

    for data in export.data {
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

            region_totals_by_file.insert(
                path.clone(),
                FileTotals {
                    covered: region_covered,
                    total: region_total,
                },
            );

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

            if line_total > 0 {
                line_totals_by_file.insert(
                    path,
                    FileTotals {
                        covered: line_covered,
                        total: line_total,
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

#[derive(Debug, Deserialize)]
struct LlvmExport {
    data: Vec<LlvmData>,
}

#[derive(Debug, Deserialize)]
struct LlvmData {
    files: Vec<LlvmFile>,
}

#[derive(Debug, Deserialize)]
struct LlvmFile {
    filename: String,
    #[serde(default)]
    segments: Vec<Vec<serde_json::Value>>,
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
            if !has_count {
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
            if end_line < start_line {
                continue;
            }

            let count = number_at(start, 2)?;
            let has_count = bool_at(start, 3).unwrap_or(true);

            if !has_count {
                continue;
            }

            let covered = count > 0;

            for line in start_line..=end_line {
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

    use super::{normalize_path, parse_str};

    #[test]
    fn parses_basic_llvm_export() {
        let input = r#"
        {
          "data": [
            {
              "files": [
                {
                  "filename": "src/lib.rs",
                  "segments": [
                    [1, 1, 1, true, false, false],
                    [1, 2, 0, false, false, false],
                    [2, 1, 1, true, false, false],
                    [2, 2, 0, false, false, false],
                    [3, 1, 0, true, false, false],
                    [3, 2, 0, false, false, false],
                    [4, 1, 0, true, false, false],
                    [4, 2, 0, false, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        assert_eq!(report.opportunities.len(), 8); // 4 regions + 4 lines

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
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_str("{").is_err());
    }

    #[test]
    fn normalizes_absolute_paths_to_repo_relative() {
        let repo_root = Path::new("/workspace/covgate");
        let normalized = normalize_path("/workspace/covgate/src/lib.rs", repo_root);
        assert_eq!(normalized, PathBuf::from("src/lib.rs"));
    }
}
