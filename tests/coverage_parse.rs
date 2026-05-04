use std::{fs, path::PathBuf, sync::Mutex};

use covgate::{coverage::load_from_path, git, model::MetricKind};
use tempfile::tempdir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(PathBuf);

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn run_git(repo: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git should run");
    assert!(output.status.success());
}

fn with_path_override(path: &str, f: impl FnOnce()) {
    let original = std::env::var("PATH").ok();
    // SAFETY: these tests serialize global cwd and env mutation through CWD_LOCK.
    unsafe { std::env::set_var("PATH", path) };
    f();
    match original {
        Some(value) => {
            // SAFETY: these tests serialize global cwd and env mutation through CWD_LOCK.
            unsafe { std::env::set_var("PATH", value) };
        }
        None => {
            // SAFETY: these tests serialize global cwd and env mutation through CWD_LOCK.
            unsafe { std::env::remove_var("PATH") };
        }
    }
}

#[test]
fn load_from_path_uses_git_repo_root_for_absolute_coverlet_paths_from_subdir() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("temp dir should exist");
    let repo = temp.path();
    fs::create_dir_all(repo.join("src")).expect("src dir should exist");
    fs::write(repo.join("README.md"), "initial\n").expect("readme should write");

    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.email", "covgate@example.com"]);
    run_git(repo, &["config", "user.name", "Covgate Tests"]);
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "initial"]);

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(repo.join("src")).expect("should chdir into repo subdir");

    let absolute_source = repo.join("src").join("lib.cs");
    let coverage_path = repo.join("coverage.json");
    fs::write(
        &coverage_path,
        format!(
            r#"{{
          "Demo.dll": {{
            "{}": {{
              "Demo.Math": {{
                "System.Int32 Demo.Math::Add()": {{
                  "Lines": {{"3": 1}},
                  "Branches": []
                }}
              }}
            }}
          }}
        }}"#,
            absolute_source.display()
        ),
    )
    .expect("coverage file should write");

    let report = load_from_path(&coverage_path).expect("coverage should parse");
    assert!(
        report
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .contains_key(&PathBuf::from("src/lib.cs"))
    );
}

#[test]
fn load_from_path_requires_git_repo_for_path_normalization() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("temp dir should exist");
    let workspace = temp.path();
    fs::create_dir_all(workspace.join("src")).expect("src dir should exist");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(workspace).expect("should chdir into temp workspace");

    let absolute_source = workspace.join("src").join("lib.cs");
    let coverage_path = workspace.join("coverage.json");
    fs::write(
        &coverage_path,
        format!(
            r#"{{
          "Demo.dll": {{
            "{}": {{
              "Demo.Math": {{
                "System.Int32 Demo.Math::Add()": {{
                  "Lines": {{"3": 1}},
                  "Branches": []
                }}
              }}
            }}
          }}
        }}"#,
            absolute_source.display()
        ),
    )
    .expect("coverage file should write");

    let err = load_from_path(&coverage_path).expect_err("parse should require a git repo");
    assert!(
        err.to_string()
            .contains(git::GIT_REPOSITORY_REQUIRED_MESSAGE)
    );
}

#[test]
fn load_from_path_reports_git_repo_lookup_failure_when_git_is_missing() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("temp dir should exist");
    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(temp.path()).expect("should chdir into temp workspace");

    with_path_override("", || {
        let coverage_path = temp.path().join("coverage.json");
        fs::write(
            &coverage_path,
            r#"{
              "Demo.dll": {
                "/workspace/src/lib.cs": {
                  "Demo.Math": {
                    "System.Int32 Demo.Math::Add()": {
                      "Lines": {"3": 1},
                      "Branches": []
                    }
                  }
                }
              }
            }"#,
        )
        .expect("coverage file should write");

        let err = load_from_path(&coverage_path)
            .expect_err("parse should fail when git lookup cannot run");

        assert!(err.to_string().contains(git::GIT_REQUIRED_MESSAGE));
    });
}

#[test]
fn load_from_path_reports_repo_root_context_for_non_path_git_spawn_failure() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("temp dir should exist");
    let workspace = temp.path();
    let git_stub = workspace.join("git");
    fs::write(&git_stub, "").expect("git stub should write");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&git_stub)
            .expect("metadata should exist")
            .permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&git_stub, perms).expect("permissions should update");
    }

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(workspace).expect("should chdir into temp workspace");

    let path = workspace.display().to_string();
    with_path_override(&path, || {
        let coverage_path = workspace.join("coverage.json");
        fs::write(
            &coverage_path,
            r#"{
              "Demo.dll": {
                "/workspace/src/lib.cs": {
                  "Demo.Math": {
                    "System.Int32 Demo.Math::Add()": {
                      "Lines": {"3": 1},
                      "Branches": []
                    }
                  }
                }
              }
            }"#,
        )
        .expect("coverage file should write");

        let err =
            load_from_path(&coverage_path).expect_err("parse should surface git repo root context");

        assert!(
            err.to_string()
                .contains("failed to determine repository root for coverage path normalization")
        );
    });
}

#[test]
fn load_from_path_requires_git_repo_when_repo_root_command_returns_empty_output() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("temp dir should exist");
    let workspace = temp.path();
    let git_stub = workspace.join("git");
    fs::write(
        &git_stub,
        "#!/bin/sh\nif [ \"$1\" = \"rev-parse\" ] && [ \"$2\" = \"--show-toplevel\" ]; then\n  exit 0\nfi\nexit 99\n",
    )
    .expect("git stub should write");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&git_stub)
            .expect("metadata should exist")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&git_stub, perms).expect("permissions should update");
    }

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(workspace).expect("should chdir into temp workspace");

    let path = workspace.display().to_string();
    with_path_override(&path, || {
        let coverage_path = workspace.join("coverage.json");
        fs::write(
            &coverage_path,
            r#"{
              "Demo.dll": {
                "/workspace/src/lib.cs": {
                  "Demo.Math": {
                    "System.Int32 Demo.Math::Add()": {
                      "Lines": {"3": 1},
                      "Branches": []
                    }
                  }
                }
              }
            }"#,
        )
        .expect("coverage file should write");

        let err = load_from_path(&coverage_path)
            .expect_err("parse should require a git repo when repo root is empty");

        assert!(
            err.to_string()
                .contains(git::GIT_REPOSITORY_REQUIRED_MESSAGE)
        );
    });
}

#[test]
fn parses_istanbul_line_branch_and_function_totals() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::MetricKind;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "src/math.js": {
        "path": "src/math.js",
        "statementMap": {
          "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}},
          "1": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 10}}
        },
        "s": {"0": 1, "1": 0},
        "branchMap": {
          "0": {
            "loc": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 10}},
            "type": "if",
            "locations": [
              {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 10}},
              {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 10}}
            ]
          }
        },
        "b": {"0": [1, 0]},
        "fnMap": {
          "0": {
            "name": "compute",
            "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}},
            "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 3, "column": 1}},
            "line": 1
          }
        },
        "f": {"0": 1}
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("istanbul json should parse");

    let line_totals = report
        .totals_by_file
        .get(&MetricKind::Line)
        .expect("line totals should exist")
        .get(&PathBuf::from("src/math.js"))
        .expect("line totals should include fixture file");
    assert_eq!(line_totals.covered, 1);
    assert_eq!(line_totals.total, 2);

    let branch_totals = report
        .totals_by_file
        .get(&MetricKind::Branch)
        .expect("branch totals should exist")
        .get(&PathBuf::from("src/math.js"))
        .expect("branch totals should include fixture file");
    assert_eq!(branch_totals.covered, 1);
    assert_eq!(branch_totals.total, 2);

    let function_totals = report
        .totals_by_file
        .get(&MetricKind::Function)
        .expect("function totals should exist")
        .get(&PathBuf::from("src/math.js"))
        .expect("function totals should include fixture file");
    assert_eq!(function_totals.covered, 1);
    assert_eq!(function_totals.total, 1);
}

#[test]
fn istanbul_parse_rejects_invalid_json() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::Path;
    let error = parse_with_repo_root("{", Path::new("/workspace/covgate"))
        .expect_err("invalid json should fail");
    assert!(error.to_string().contains("failed to parse coverage json"));
}

#[test]
fn parses_checked_in_vitest_fixture_with_empty_branch_locations() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::{MetricKind, OpportunityKind};
    use std::path::{Path, PathBuf};

    let input = include_str!("fixtures/vitest/empty-branch-locations/coverage.json");

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("checked-in vitest fixture should parse");

    let branch_totals = report
        .totals_by_file
        .get(&MetricKind::Branch)
        .expect("branch totals should exist");
    assert!(
        branch_totals.contains_key(&PathBuf::from("src/auth/authService.ts")),
        "fixture should include authService branch totals"
    );
    assert!(
        branch_totals.contains_key(&PathBuf::from("src/auth/msalConfig.ts")),
        "fixture should include msalConfig branch totals"
    );

    let auth_service_branches: Vec<_> = report
        .opportunities
        .iter()
        .filter(|opportunity| {
            opportunity.kind == OpportunityKind::BranchOutcome
                && opportunity.span.path == Path::new("src/auth/authService.ts")
                && opportunity.span.start_line == 10
                && opportunity.span.end_line == 11
        })
        .collect();
    assert_eq!(
        auth_service_branches.len(),
        2,
        "line 10-11 authService branch should preserve both outcome spans"
    );
}

#[test]
fn merges_overlapping_statement_lines_as_covered_when_any_statement_hits() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::MetricKind;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "src/math.js": {
        "statementMap": {
          "0": {"start": {"line": 2}, "end": {"line": 2}},
          "1": {"start": {"line": 2}, "end": {"line": 2}}
        },
        "s": {"0": 0, "1": 1},
        "branchMap": {},
        "b": {},
        "fnMap": {},
        "f": {}
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("istanbul json should parse");

    let line_totals = report
        .totals_by_file
        .get(&MetricKind::Line)
        .expect("line totals should exist")
        .get(&PathBuf::from("src/math.js"))
        .expect("file totals should exist");
    assert_eq!(line_totals.covered, 1);
    assert_eq!(line_totals.total, 1);

    assert!(!report.totals_by_file.contains_key(&MetricKind::Branch));
    assert!(!report.totals_by_file.contains_key(&MetricKind::Function));
}

#[test]
fn counts_unique_statement_start_lines_for_line_totals() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::{MetricKind, OpportunityKind};
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "src/math.js": {
        "statementMap": {
          "0": {"start": {"line": 19}, "end": {"line": 22}},
          "1": {"start": {"line": 20}, "end": {"line": 20}},
          "2": {"start": {"line": 22}, "end": {"line": 22}}
        },
        "s": {"0": 1, "1": 0, "2": 1},
        "branchMap": {},
        "b": {},
        "fnMap": {},
        "f": {}
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("istanbul json should parse");

    let line_totals = report
        .totals_by_file
        .get(&MetricKind::Line)
        .expect("line totals should exist")
        .get(&PathBuf::from("src/math.js"))
        .expect("file totals should exist");
    assert_eq!(line_totals.covered, 2);
    assert_eq!(line_totals.total, 3);

    let line_20 = report
        .opportunities
        .iter()
        .find(|opportunity| {
            opportunity.kind == OpportunityKind::Line
                && opportunity.span.path == Path::new("src/math.js")
                && opportunity.span.start_line == 20
                && opportunity.span.end_line == 20
        })
        .expect("line 20 opportunity should exist");
    assert!(!line_20.covered, "line 20 should remain uncovered");
}

#[test]
fn checked_in_vitest_fixture_preserves_uncovered_nested_fixture_seed_line() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::OpportunityKind;
    use std::path::Path;

    let input = include_str!("fixtures/vitest/empty-branch-locations/coverage.json");

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("checked-in vitest fixture should parse");

    let line_20 = report
        .opportunities
        .iter()
        .find(|opportunity| {
            opportunity.kind == OpportunityKind::Line
                && opportunity.span.path == Path::new("src/fixtures/fixtureSeed.ts")
                && opportunity.span.start_line == 20
                && opportunity.span.end_line == 20
        })
        .expect("fixtureSeed line 20 opportunity should exist");
    assert!(
        !line_20.covered,
        "fixtureSeed line 20 should stay uncovered"
    );
}

#[test]
fn normalizes_repo_prefixed_and_absolute_paths() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::MetricKind;
    use std::path::{Path, PathBuf};

    let prefixed = parse_with_repo_root(
        r#"{
          "/workspace/covgate/src/math.js": {
            "statementMap": {"0": {"start": {"line": 1}, "end": {"line": 1}}},
            "s": {"0": 1},
            "branchMap": {},
            "b": {},
            "fnMap": {},
            "f": {}
          }
        }"#,
        Path::new("/workspace/covgate"),
    )
    .expect("prefixed path should parse");
    assert!(
        prefixed
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .contains_key(&PathBuf::from("src/math.js"))
    );

    let absolute_outside = parse_with_repo_root(
        r#"{
          "/opt/other/math.js": {
            "statementMap": {"0": {"start": {"line": 1}, "end": {"line": 1}}},
            "s": {"0": 1},
            "branchMap": {},
            "b": {},
            "fnMap": {},
            "f": {}
          }
        }"#,
        Path::new("/workspace/covgate"),
    )
    .expect("absolute outside path should parse");
    assert!(
        absolute_outside
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .contains_key(&PathBuf::from("/opt/other/math.js"))
    );
}

#[test]
fn does_not_strip_repo_root_text_prefix_when_not_path_boundary() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::MetricKind;
    use std::path::{Path, PathBuf};

    let report = parse_with_repo_root(
        r#"{
          "/workspace/covgate-old/src/math.js": {
            "statementMap": {"0": {"start": {"line": 1}, "end": {"line": 1}}},
            "s": {"0": 1},
            "branchMap": {},
            "b": {},
            "fnMap": {},
            "f": {}
          }
        }"#,
        Path::new("/workspace/covgate"),
    )
    .expect("path should parse");

    assert!(
        report
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .contains_key(&PathBuf::from("/workspace/covgate-old/src/math.js"))
    );
    assert!(
        !report
            .totals_by_file
            .get(&MetricKind::Line)
            .expect("line totals should exist")
            .contains_key(&PathBuf::from("-old/src/math.js"))
    );
}
