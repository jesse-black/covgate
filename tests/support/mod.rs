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
