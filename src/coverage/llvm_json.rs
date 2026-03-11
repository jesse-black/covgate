use std::{
    collections::BTreeMap,
    fs,
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
    let export: LlvmExport = serde_json::from_str(input).context("failed to parse llvm json")?;
    let mut opportunities = Vec::new();
    let mut totals_by_file = BTreeMap::new();

    for data in export.data {
        for file in data.files {
            let path = normalize_path(&file.filename);
            let mut covered = 0usize;
            let mut total = 0usize;

            for region in file.segments_to_regions()? {
                total += 1;
                if region.covered {
                    covered += 1;
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

            totals_by_file.insert(path, FileTotals { covered, total });
        }
    }

    Ok(CoverageReport {
        metric_kind: MetricKind::Region,
        opportunities,
        totals_by_file,
    })
}

fn normalize_path(value: &str) -> PathBuf {
    PathBuf::from(value)
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
    segments: Vec<Vec<serde_json::Value>>,
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
    use super::parse_str;

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
                    [3, 1, 0, true, false, false],
                    [5, 1, 0, true, false, false]
                  ]
                }
              ]
            }
          ]
        }
        "#;

        let report = parse_str(input).expect("llvm export should parse");
        assert_eq!(report.opportunities.len(), 2);
        assert!(report.opportunities[0].covered);
        assert!(!report.opportunities[1].covered);
        let totals = report
            .totals_by_file
            .get(&std::path::PathBuf::from("src/lib.rs"))
            .expect("file totals should exist");
        assert_eq!(totals.covered, 1);
        assert_eq!(totals.total, 2);
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_str("{").is_err());
    }
}
