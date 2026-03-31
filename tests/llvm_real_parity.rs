mod support;

use std::{collections::BTreeMap, fs, path::PathBuf};

use covgate::{coverage, model::MetricKind};
use support::run_covgate_raw;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OverallTotals {
    covered: usize,
    total: usize,
}

#[test]
fn real_multi_file_llvm_export_markdown_totals_match_covgate_calculations() {
    let coverage_report = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("llvm-real")
        .join("covgate-self-full.json");

    let temp = tempfile::tempdir().expect("tempdir should exist");
    let diff_file = temp.path().join("empty.diff");
    let markdown_output = temp.path().join("summary.md");
    fs::write(&diff_file, "").expect("empty diff should be written");

    let output = run_covgate_raw(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).as_path(),
        &[
            "check".to_string(),
            coverage_report.to_string_lossy().into_owned(),
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "0".to_string(),
            "--fail-under-lines".to_string(),
            "0".to_string(),
            "--fail-under-functions".to_string(),
            "0".to_string(),
            "--markdown-output".to_string(),
            markdown_output.to_string_lossy().into_owned(),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "covgate run should succeed; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report =
        coverage::load_from_path(&coverage_report).expect("real llvm fixture should parse");
    let markdown =
        fs::read_to_string(&markdown_output).expect("markdown summary should be readable");

    let covgate_region =
        parse_markdown_totals(&markdown, "Region").expect("markdown region totals should exist");
    let covgate_line =
        parse_markdown_totals(&markdown, "Line").expect("markdown line totals should exist");
    let covgate_function = parse_markdown_totals(&markdown, "Function")
        .expect("markdown function totals should exist");

    assert_eq!(
        covgate_region,
        report_totals(&report.totals_by_file, MetricKind::Region)
    );
    assert_eq!(
        covgate_line,
        report_totals(&report.totals_by_file, MetricKind::Line)
    );
    assert_eq!(
        covgate_function,
        report_totals(&report.totals_by_file, MetricKind::Function)
    );
}

#[test]
fn real_multi_file_llvm_export_documents_summary_semantics_disagreement() {
    let coverage_report = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("llvm-real")
        .join("covgate-self-full.json");
    let native_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&coverage_report).expect("real llvm fixture should be readable"),
    )
    .expect("real llvm fixture should parse");
    let report =
        coverage::load_from_path(&coverage_report).expect("real llvm fixture should parse");

    let native_region = llvm_totals(&native_json, "regions").expect("region totals should exist");
    let native_line = llvm_totals(&native_json, "lines").expect("line totals should exist");
    let native_function =
        llvm_totals(&native_json, "functions").expect("function totals should exist");

    let covgate_region = report_totals(&report.totals_by_file, MetricKind::Region);
    let covgate_line = report_totals(&report.totals_by_file, MetricKind::Line);
    let covgate_function = report_totals(&report.totals_by_file, MetricKind::Function);

    assert_eq!(
        covgate_function, native_function,
        "function totals should stay aligned after LLVM name normalization"
    );
    assert_ne!(
        covgate_region, native_region,
        "region totals currently reflect covgate's calculation-backed model, not LLVM summary pass-through"
    );
    assert_ne!(
        covgate_line, native_line,
        "line totals currently reflect covgate's calculation-backed model, not LLVM summary pass-through"
    );
}

fn report_totals(
    totals_by_file: &BTreeMap<MetricKind, BTreeMap<PathBuf, covgate::model::FileTotals>>,
    metric: MetricKind,
) -> OverallTotals {
    let totals = totals_by_file
        .get(&metric)
        .unwrap_or_else(|| panic!("totals for {:?} should exist", metric));
    OverallTotals {
        covered: totals.values().map(|file| file.covered).sum(),
        total: totals.values().map(|file| file.total).sum(),
    }
}

fn llvm_totals(parsed: &serde_json::Value, section: &str) -> Option<OverallTotals> {
    let totals = parsed.get("data")?.get(0)?.get("totals")?.get(section)?;
    Some(OverallTotals {
        covered: totals.get("covered")?.as_u64()? as usize,
        total: totals.get("count")?.as_u64()? as usize,
    })
}

fn parse_markdown_totals(markdown: &str, metric_heading: &str) -> Option<OverallTotals> {
    let mut in_overall = false;
    let mut in_metric = false;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed == "### Overall Coverage" {
            in_overall = true;
            in_metric = false;
            continue;
        }
        if !in_overall {
            continue;
        }
        if trimmed.starts_with("#### ") {
            in_metric = trimmed == format!("#### {metric_heading}");
            continue;
        }
        if !in_metric || !trimmed.starts_with("| **Total** |") {
            continue;
        }

        let columns = trimmed.split('|').map(str::trim).collect::<Vec<_>>();
        if columns.len() < 5 {
            return None;
        }
        let covered = columns[2].trim_matches('*').parse().ok()?;
        let total = columns[3].trim_matches('*').parse().ok()?;
        return Some(OverallTotals { covered, total });
    }

    None
}
