mod support;

use std::{fs, path::PathBuf};

use support::run_covgate_raw;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OverallTotals {
    covered: usize,
    total: usize,
}

#[test]
fn real_multi_file_llvm_export_totals_match_native_summary() {
    let coverage_report = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("llvm-real")
        .join("covgate-self-full.json");
    let native_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&coverage_report).expect("real llvm fixture should be readable"),
    )
    .expect("real llvm fixture should parse");

    let native_region = llvm_totals(&native_json, "regions").expect("region totals should exist");
    let native_line = llvm_totals(&native_json, "lines").expect("line totals should exist");
    let native_function =
        llvm_totals(&native_json, "functions").expect("function totals should exist");

    let temp = tempfile::tempdir().expect("tempdir should exist");
    let diff_file = temp.path().join("empty.diff");
    let markdown_output = temp.path().join("summary.md");
    fs::write(&diff_file, "").expect("empty diff should be written");

    let output = run_covgate_raw(
        temp.path(),
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

    let markdown =
        fs::read_to_string(&markdown_output).expect("markdown summary should be readable");
    let covgate_region =
        parse_markdown_totals(&markdown, "Region").expect("markdown region totals should exist");
    let covgate_line =
        parse_markdown_totals(&markdown, "Line").expect("markdown line totals should exist");
    let covgate_function = parse_markdown_totals(&markdown, "Function")
        .expect("markdown function totals should exist");

    let mut mismatches = Vec::new();
    if covgate_region != native_region {
        mismatches.push(format!(
            "region native={native_region:?} covgate={covgate_region:?}"
        ));
    }
    if covgate_line != native_line {
        mismatches.push(format!(
            "line native={native_line:?} covgate={covgate_line:?}"
        ));
    }
    if covgate_function != native_function {
        mismatches.push(format!(
            "function native={native_function:?} covgate={covgate_function:?}"
        ));
    }

    assert!(
        mismatches.is_empty(),
        "real llvm export totals diverged: {}",
        mismatches.join("; ")
    );
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
