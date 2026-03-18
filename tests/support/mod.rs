#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[derive(Clone, Copy, Debug)]
pub struct Fixture {
    pub language: &'static str,
    pub name: &'static str,
}

impl Fixture {
    pub fn root(self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(self.language)
            .join(self.name)
    }

    pub fn coverage_json(self) -> PathBuf {
        self.root().join("coverage.json")
    }

    pub fn id(self) -> String {
        format!("{}/{}", self.language, self.name)
    }
}

pub fn rust_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "rust",
        name: "basic-fail",
    }
}

pub fn rust_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "rust",
        name: "basic-pass",
    }
}

pub fn cpp_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "cpp",
        name: "basic-fail",
    }
}

pub fn cpp_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "cpp",
        name: "basic-pass",
    }
}

pub fn swift_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "swift",
        name: "basic-fail",
    }
}

pub fn swift_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "swift",
        name: "basic-pass",
    }
}

pub fn dotnet_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "dotnet",
        name: "basic-fail",
    }
}

pub fn dotnet_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "dotnet",
        name: "basic-pass",
    }
}

pub fn vitest_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "vitest",
        name: "basic-fail",
    }
}

pub fn vitest_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "vitest",
        name: "basic-pass",
    }
}

pub fn fail_fixtures_with_regions() -> Vec<Fixture> {
    vec![
        rust_basic_fail_fixture(),
        cpp_basic_fail_fixture(),
        swift_basic_fail_fixture(),
    ]
}

pub fn pass_fixtures_with_regions() -> Vec<Fixture> {
    vec![
        rust_basic_pass_fixture(),
        cpp_basic_pass_fixture(),
        swift_basic_pass_fixture(),
    ]
}

pub fn branch_capable_fail_fixtures() -> Vec<Fixture> {
    vec![
        cpp_basic_fail_fixture(),
        dotnet_basic_fail_fixture(),
        vitest_basic_fail_fixture(),
    ]
}

pub fn branch_capable_pass_fixtures() -> Vec<Fixture> {
    vec![
        cpp_basic_pass_fixture(),
        dotnet_basic_pass_fixture(),
        vitest_basic_pass_fixture(),
    ]
}

pub fn fail_fixtures_with_lines() -> Vec<Fixture> {
    vec![
        rust_basic_fail_fixture(),
        cpp_basic_fail_fixture(),
        swift_basic_fail_fixture(),
        dotnet_basic_fail_fixture(),
        vitest_basic_fail_fixture(),
    ]
}

pub fn pass_fixtures_with_lines() -> Vec<Fixture> {
    vec![
        rust_basic_pass_fixture(),
        cpp_basic_pass_fixture(),
        swift_basic_pass_fixture(),
        dotnet_basic_pass_fixture(),
        vitest_basic_pass_fixture(),
    ]
}

pub fn function_capable_fail_fixtures() -> Vec<Fixture> {
    vec![
        rust_basic_fail_fixture(),
        cpp_basic_fail_fixture(),
        swift_basic_fail_fixture(),
        dotnet_basic_fail_fixture(),
        vitest_basic_fail_fixture(),
    ]
}

pub fn function_capable_pass_fixtures() -> Vec<Fixture> {
    vec![
        rust_basic_pass_fixture(),
        cpp_basic_pass_fixture(),
        swift_basic_pass_fixture(),
        dotnet_basic_pass_fixture(),
        vitest_basic_pass_fixture(),
    ]
}

pub fn assert_fixture_has_no_branch_coverage(fixture: Fixture) {
    fn contains_non_empty_branches(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(map) => map.iter().any(|(key, nested)| {
                ((key == "branches" && nested.as_array().is_some_and(|items| !items.is_empty()))
                    || (key == "branchMap"
                        && nested
                            .as_object()
                            .is_some_and(|entries| !entries.is_empty())))
                    || contains_non_empty_branches(nested)
            }),
            serde_json::Value::Array(values) => values.iter().any(contains_non_empty_branches),
            _ => false,
        }
    }

    let parsed: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture.coverage_json()).expect("coverage fixture should be readable"),
    )
    .expect("coverage fixture should parse as json");
    assert!(
        !contains_non_empty_branches(&parsed),
        "fixture should not include non-empty branch coverage data"
    );
}

pub fn setup_fixture_worktree(temp_root: &Path, fixture: Fixture) -> PathBuf {
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
    let worktree = temp_root.join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    copy_tree(&overlay_src, &worktree);
    worktree
}

pub fn write_worktree_diff(temp_root: &Path, worktree: &Path) -> PathBuf {
    let diff_output = Command::new("git")
        .args(["diff", "--unified=0", "--no-ext-diff"])
        .current_dir(worktree)
        .output()
        .expect("git diff should run");
    assert!(diff_output.status.success(), "git diff should succeed");
    let diff_file = temp_root.join("scenario.diff");
    fs::write(&diff_file, diff_output.stdout).expect("diff file should be written");
    diff_file
}

pub fn run_covgate(worktree: &Path, fixture: Fixture, extra_args: &[String]) -> Output {
    run_covgate_with_coverage(worktree, &fixture.coverage_json(), extra_args)
}

pub fn run_covgate_raw(worktree: &Path, args: &[String]) -> Output {
    let binary = env!("CARGO_BIN_EXE_covgate");
    let mut command = Command::new(binary);
    command.args(args);
    command.current_dir(worktree);
    command.output().expect("covgate should run")
}

pub fn run_covgate_with_coverage(
    worktree: &Path,
    coverage_json: &Path,
    extra_args: &[String],
) -> Output {
    let binary = env!("CARGO_BIN_EXE_covgate");
    let mut command = Command::new(binary);
    command.arg("check");
    command.arg(coverage_json);
    command.args(extra_args);
    command.current_dir(worktree);
    command.output().expect("covgate should run")
}

pub fn write_absolute_path_coverage_fixture(fixture: Fixture, worktree: &Path, destination: &Path) {
    let template = fixture.coverage_json();
    let relative_source = match fixture.language {
        "cpp" => "src/lib.cpp",
        "swift" => "Sources/CovgateDemo/CovgateDemo.swift",
        "dotnet" => "src/CovgateDemo/MathOps.cs",
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

pub fn init_git_repo(path: &Path) {
    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "covgate@example.com"]);
    run_git(path, &["config", "user.name", "Covgate Tests"]);
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "baseline"]);
}

pub fn run_git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination tree should exist");
    for entry in fs::read_dir(source).expect("fixture tree should be readable") {
        let entry = entry.expect("dir entry");
        let file_type = entry.file_type().expect("file type");
        let dest = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).expect("fixture file should copy");
        }
    }
}

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

    pub fn covgate_markdown_overall_totals(&self) -> Option<OverallTotals> {
        let temp = tempfile::tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), self.fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);
        let markdown_output = temp.path().join("summary.md");
        let metric_flag = match self.metric {
            "branch" => "--fail-under-branches".to_string(),
            "function" => "--fail-under-functions".to_string(),
            "line" => "--fail-under-lines".to_string(),
            "region" => "--fail-under-regions".to_string(),
            other => panic!("unsupported metric: {other}"),
        };

        let output = run_covgate(
            &worktree,
            self.fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                metric_flag,
                "0".to_string(),
                "--markdown-output".to_string(),
                markdown_output.to_string_lossy().into_owned(),
            ],
        );
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
