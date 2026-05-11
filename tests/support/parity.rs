#![allow(dead_code)]

use std::{fs, path::Path, path::PathBuf};

use super::fixtures::Fixture;
use super::git::{setup_fixture_worktree, write_worktree_diff};
use super::runner::covgate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverallTotals {
    pub covered: usize,
    pub total: usize,
}

pub struct MetricFixtureCase {
    fixture: Fixture,
    metric: &'static str,
}

impl MetricFixtureCase {
    pub fn new(fixture: Fixture, metric: &'static str) -> Self {
        Self { fixture, metric }
    }

    pub fn fixture_id(&self) -> String {
        self.fixture.id()
    }

    pub fn native_overall_totals(&self) -> Option<OverallTotals> {
        if let Some(native_summary) = self.captured_native_summary_overall_totals() {
            return Some(native_summary);
        }

        let parsed: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(self.fixture.coverage_json())
                .expect("coverage fixture should be readable"),
        )
        .expect("coverage fixture should parse as json");

        match self.fixture.language {
            "rust" | "cpp" | "swift" => llvm_native_overall_totals(&parsed, self.metric),
            "dotnet" => coverlet_native_overall_totals(&parsed, self.metric),
            "vitest" => istanbul_native_overall_totals(&parsed, self.metric),
            other => panic!("unsupported fixture language: {other}"),
        }
    }

    pub fn captured_native_summary_overall_totals(&self) -> Option<OverallTotals> {
        native_summary_overall_totals(self.fixture, self.metric)
    }

    pub fn covgate_markdown_overall_totals(&self) -> Option<OverallTotals> {
        let temp = tempfile::tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), self.fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);
        let markdown_output = temp.path().join("summary.md");
        let metric_config = match self.metric {
            "branch" => "fail-under-branches",
            "function" => "fail-under-functions",
            "line" => "fail-under-lines",
            "region" => "fail-under-regions",
            other => panic!("unsupported metric: {other}"),
        };
        fs::write(
            worktree.join("covgate.toml"),
            format!("[[gates]]\n{metric_config} = 0\n"),
        )
        .expect("config should be written");

        let output = covgate(&worktree)
            .check(&self.fixture.coverage_json())
            .args([
                "--diff-file".into(),
                diff_file.to_string_lossy().into_owned(),
                "--markdown-output".into(),
                markdown_output.to_string_lossy().into_owned(),
            ])
            .run();
        assert!(
            markdown_output.exists(),
            "covgate should always emit markdown for fixture {} metric {}; stdout={} stderr={}",
            self.fixture.id(),
            self.metric,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        parse_markdown_overall_totals(
            &fs::read_to_string(markdown_output).expect("markdown should be readable"),
            self.metric,
        )
    }
}

fn native_summary_overall_totals(fixture: Fixture, metric: &str) -> Option<OverallTotals> {
    let path = fixture.root().join("native-summary.json");
    if !path.exists() {
        return None;
    }

    let parsed: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(path).expect("native summary fixture should be readable"),
    )
    .expect("native summary fixture should parse as json");
    let section = parsed.get(metric)?;
    Some(OverallTotals {
        covered: section.get("covered")?.as_u64()? as usize,
        total: section.get("total")?.as_u64()? as usize,
    })
}

fn llvm_native_overall_totals(parsed: &serde_json::Value, metric: &str) -> Option<OverallTotals> {
    let totals = parsed.get("data")?.get(0)?.get("totals")?;
    llvm_summary_metric(totals, metric)
}

fn llvm_summary_metric(summary: &serde_json::Value, metric: &str) -> Option<OverallTotals> {
    let key = match metric {
        "region" => "regions",
        "line" => "lines",
        "branch" => "branches",
        "function" => "functions",
        _ => return None,
    };
    let section = summary.get(key)?;
    Some(OverallTotals {
        covered: section.get("covered")?.as_u64()? as usize,
        total: section.get("count")?.as_u64()? as usize,
    })
}

fn coverlet_native_overall_totals(
    parsed: &serde_json::Value,
    metric: &str,
) -> Option<OverallTotals> {
    let mut covered = 0usize;
    let mut total = 0usize;

    for module in parsed.as_object()?.values() {
        for document in module.as_object()?.values() {
            for class in document.as_object()?.values() {
                for method in class.as_object()?.values() {
                    match metric {
                        "line" => {
                            for hits in method.get("Lines")?.as_object()?.values() {
                                total += 1;
                                if hits.as_u64()? > 0 {
                                    covered += 1;
                                }
                            }
                        }
                        "branch" => {
                            for branch in method.get("Branches")?.as_array()? {
                                total += 1;
                                if branch.get("Hits")?.as_u64()? > 0 {
                                    covered += 1;
                                }
                            }
                        }
                        "function" => {
                            total += 1;
                            let line_hits = method
                                .get("Lines")?
                                .as_object()?
                                .values()
                                .any(|hits| hits.as_u64().is_some_and(|value| value > 0));
                            let branch_hits = method
                                .get("Branches")
                                .and_then(serde_json::Value::as_array)
                                .is_some_and(|branches| {
                                    branches.iter().any(|branch| {
                                        branch
                                            .get("Hits")
                                            .and_then(serde_json::Value::as_u64)
                                            .is_some_and(|value| value > 0)
                                    })
                                });
                            if line_hits || branch_hits {
                                covered += 1;
                            }
                        }
                        "region" => return None,
                        _ => return None,
                    }
                }
            }
        }
    }

    Some(OverallTotals { covered, total })
}

fn istanbul_native_overall_totals(
    parsed: &serde_json::Value,
    metric: &str,
) -> Option<OverallTotals> {
    let mut covered = 0usize;
    let mut total = 0usize;

    for file in parsed.as_object()?.values() {
        match metric {
            "line" => {
                for hits in file.get("s")?.as_object()?.values() {
                    total += 1;
                    if hits.as_u64()? > 0 {
                        covered += 1;
                    }
                }
            }
            "branch" => {
                for branch_hits in file.get("b")?.as_object()?.values() {
                    for hits in branch_hits.as_array()? {
                        total += 1;
                        if hits.as_u64()? > 0 {
                            covered += 1;
                        }
                    }
                }
            }
            "function" => {
                for hits in file.get("f")?.as_object()?.values() {
                    total += 1;
                    if hits.as_u64()? > 0 {
                        covered += 1;
                    }
                }
            }
            "region" => return None,
            _ => return None,
        }
    }

    Some(OverallTotals { covered, total })
}

fn parse_markdown_overall_totals(markdown: &str, metric: &str) -> Option<OverallTotals> {
    let heading = format!("#### {}", {
        let mut chars = metric.chars();
        let first = chars.next()?;
        first.to_uppercase().collect::<String>() + chars.as_str()
    });
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
            in_metric = trimmed == heading;
            continue;
        }
        if in_metric && trimmed.starts_with("| **Total** ") {
            let cells = trimmed
                .split('|')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>();
            return Some(OverallTotals {
                covered: cells.get(1)?.trim_matches('*').parse().ok()?,
                total: cells.get(2)?.trim_matches('*').parse().ok()?,
            });
        }
    }

    None
}

pub fn write_absolute_path_coverage_fixture(fixture: Fixture, worktree: &Path, destination: &Path) {
    let template = fixture.coverage_json();
    let relative_source = match fixture.language {
        "cpp" => "src/lib.cpp",
        "swift" => "Sources/CovgateDemo/CovgateDemo.swift",
        "dotnet" => "src/CovgateDemo/MathOps.cs",
        "vitest" if fixture.name == "empty-branch-locations" => "src/fixtures/fixtureSeed.ts",
        "vitest" if fixture.name == "tsx-line-summary" => "src/profileCard.tsx",
        "vitest" => "src/math.js",
        _ => "src/lib.rs",
    };
    let absolute_source_path = worktree.join(relative_source);
    let updated = fs::read_to_string(template)
        .expect("fixture coverage should be readable")
        .replace(
            &format!("\"{}\"", relative_source),
            &format!("\"{}\"", absolute_source_path.display()),
        );
    fs::write(destination, updated).expect("absolute-path coverage fixture should be written");
}

pub fn write_rebased_real_llvm_fixture(destination: &Path) {
    let template = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("llvm-real")
        .join("covgate-self-full.json");
    let text = fs::read_to_string(&template).expect("real llvm fixture should be readable");
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("real llvm fixture should parse as json");

    let old_manifest_path = parsed
        .get("cargo_llvm_cov")
        .and_then(|value| value.get("manifest_path"))
        .and_then(serde_json::Value::as_str)
        .expect("real llvm fixture should include cargo_llvm_cov.manifest_path");
    let old_root = Path::new(old_manifest_path)
        .parent()
        .expect("manifest path should have parent");
    let new_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let updated = text.replace(
        &old_root.display().to_string(),
        &new_root.display().to_string(),
    );
    fs::write(destination, updated).expect("rebased real llvm fixture should be written");
}
