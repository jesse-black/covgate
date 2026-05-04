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

#[test]
fn parses_coverlet_lines_and_branches() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::MetricKind;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "Demo.dll": {
        "/workspace/covgate/src/lib.cs": {
          "Demo.MathOps": {
            "System.Int32 Demo.MathOps::Add(System.Int32,System.Int32)": {
              "Lines": {
                "3": 1,
                "4": 0
              },
              "Branches": [
                {"Line": 4, "Hits": 1},
                {"Line": 4, "Hits": 0}
              ]
            }
          }
        }
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("coverlet json should parse");

    let line_totals = report
        .totals_by_file
        .get(&MetricKind::Line)
        .expect("line totals should exist")
        .get(&PathBuf::from("src/lib.cs"))
        .expect("file totals should exist");
    assert_eq!(line_totals.covered, 1);
    assert_eq!(line_totals.total, 2);

    let branch_totals = report
        .totals_by_file
        .get(&MetricKind::Branch)
        .expect("branch totals should exist")
        .get(&PathBuf::from("src/lib.cs"))
        .expect("file totals should exist");
    assert_eq!(branch_totals.covered, 1);
    assert_eq!(branch_totals.total, 2);

    let function_totals = report
        .totals_by_file
        .get(&MetricKind::Function)
        .expect("function totals should exist")
        .get(&PathBuf::from("src/lib.cs"))
        .expect("file totals should exist");
    assert_eq!(function_totals.covered, 1);
    assert_eq!(function_totals.total, 1);
}

#[test]
fn computes_function_spans_from_method_lines() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::OpportunityKind;
    use std::path::Path;

    let input = r#"
    {
      "Demo.dll": {
        "src/lib.cs": {
          "Demo.MathOps": {
            "Covered": {"Lines": {"10": 1, "11": 0, "15": 2}, "Branches": []},
            "Uncovered": {"Lines": {"20": 0, "21": 0}, "Branches": []}
          }
        }
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("coverlet json should parse");

    let function_ops: Vec<_> = report
        .opportunities
        .iter()
        .filter(|op| op.kind == OpportunityKind::Function)
        .collect();
    assert_eq!(function_ops.len(), 2);
    assert!(
        function_ops
            .iter()
            .any(|op| { op.span.start_line == 10 && op.span.end_line == 15 && op.covered })
    );
    assert!(
        function_ops
            .iter()
            .any(|op| { op.span.start_line == 20 && op.span.end_line == 21 && !op.covered })
    );
}

#[test]
fn merges_duplicate_lines_across_methods() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::MetricKind;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "Demo.dll": {
        "src/lib.cs": {
          "Demo.MathOps": {
            "M1": {"Lines": {"10": 0, "11": 1}, "Branches": []},
            "M2": {"Lines": {"10": 2}, "Branches": []}
          }
        }
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("coverlet json should parse");

    let line_totals = report
        .totals_by_file
        .get(&MetricKind::Line)
        .expect("line totals should exist")
        .get(&PathBuf::from("src/lib.cs"))
        .expect("file totals should exist");
    assert_eq!(line_totals.total, 2);
    assert_eq!(line_totals.covered, 2);
}

#[test]
fn skips_non_object_class_or_method_entries() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::OpportunityKind;
    use std::path::Path;

    let input = r#"
    {
      "Demo.dll": {
        "src/lib.cs": {
          "IgnoredClass": 5,
          "Demo.MathOps": {
            "IgnoredMethod": 3,
            "RealMethod": {"Lines": {"5": 1}, "Branches": []}
          }
        }
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("coverlet json should parse");
    let lines: Vec<_> = report
        .opportunities
        .iter()
        .filter(|op| op.kind == OpportunityKind::Line)
        .collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn invalid_line_key_method_is_ignored() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::MetricKind;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "Demo.dll": {
        "src/lib.cs": {
          "Demo.MathOps": {
            "BadMethod": {"Lines": {"not-a-line": 1}, "Branches": []},
            "GoodMethod": {"Lines": {"7": 1}, "Branches": []}
          }
        }
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("coverlet json should parse");
    let line_totals = report
        .totals_by_file
        .get(&MetricKind::Line)
        .expect("line totals should exist")
        .get(&PathBuf::from("src/lib.cs"))
        .expect("file totals should exist");
    assert_eq!(line_totals.total, 1);
    assert_eq!(line_totals.covered, 1);
}

#[test]
fn skips_function_metric_when_method_has_no_lines() {
    use covgate::coverage::parse_with_repo_root;
    use covgate::model::MetricKind;
    use std::path::Path;

    let input = r#"
    {
      "Demo.dll": {
        "src/lib.cs": {
          "Demo.MathOps": {
            "NoLines": {"Lines": {}, "Branches": []}
          }
        }
      }
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("coverlet json should parse");

    assert!(!report.totals_by_file.contains_key(&MetricKind::Function));
}

#[test]
fn parses_basic_llvm_export() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "functions": [
            {
              "count": 1,
              "filenames": ["src/lib.rs"],
              "regions": [[1,1,2,1,1,0,0,0]]
            },
            {
              "count": 0,
              "filenames": ["src/lib.rs"],
              "regions": [[3,1,4,1,0,0,0,0]]
            }
          ],
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [1, 1, 1, true, true, false],
                [1, 2, 0, false, false, false],
                [2, 1, 1, true, true, false],
                [2, 2, 0, false, false, false],
                [3, 1, 0, true, true, false],
                [3, 2, 0, false, false, false],
                [4, 1, 0, true, true, false],
                [4, 2, 0, false, false, false]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");
    assert_eq!(report.opportunities.len(), 10); // 4 regions + 4 lines + 2 functions

    let region_totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Region)
        .expect("region metric totals should exist")
        .get(&PathBuf::from("src/lib.rs"))
        .expect("file totals should exist");
    assert_eq!(region_totals.covered, 2);
    assert_eq!(region_totals.total, 4);

    let line_totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Line)
        .expect("line metric totals should exist")
        .get(&PathBuf::from("src/lib.rs"))
        .expect("file totals should exist");
    assert_eq!(line_totals.covered, 2);
    assert_eq!(line_totals.total, 4);

    let function_totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Function)
        .expect("function metric totals should exist")
        .get(&PathBuf::from("src/lib.rs"))
        .expect("file totals should exist");
    assert_eq!(function_totals.covered, 1);
    assert_eq!(function_totals.total, 2);
}

#[test]
fn parses_branch_metrics_when_branches_are_present() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [1, 1, 1, true, false, false],
                [2, 1, 0, false, false, false]
              ],
              "branches": [
                [1, 1, 1, true],
                [1, 5, 0, true]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");

    let branch_totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Branch)
        .expect("branch totals should be present");
    let file_totals = branch_totals
        .get(&PathBuf::from("src/lib.rs"))
        .expect("branch file totals should be present");
    assert_eq!(file_totals.covered, 1);
    assert_eq!(file_totals.total, 2);

    let branch_opportunities: Vec<_> = report
        .opportunities
        .iter()
        .filter(|op| op.kind == covgate::model::OpportunityKind::BranchOutcome)
        .collect();
    assert_eq!(branch_opportunities.len(), 2);
}

#[test]
fn parses_llvm_branch_tuples_using_true_false_counts() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [1, 1, 1, true, false, false],
                [2, 1, 0, false, false, false]
              ],
              "branches": [
                [2, 5, 2, 10, 1, 0, 0, 0, 4]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");

    let branch_totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Branch)
        .expect("branch totals should be present");
    let file_totals = branch_totals
        .get(&PathBuf::from("src/lib.rs"))
        .expect("branch file totals should be present");
    assert_eq!(file_totals.covered, 1);
    assert_eq!(file_totals.total, 2);
}

#[test]
fn parses_legacy_branch_entries_and_skips_has_count_false() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [1, 1, 1, true, false, false],
                [2, 1, 0, false, false, false]
              ],
              "branches": [
                [2, 1, 0, false],
                [3, 1, 1, true]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");

    let branch_totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Branch)
        .expect("branch totals should be present");
    let file_totals = branch_totals
        .get(&PathBuf::from("src/lib.rs"))
        .expect("branch file totals should be present");

    // The first legacy entry is skipped because has_count=false.
    assert_eq!(file_totals.covered, 1);
    assert_eq!(file_totals.total, 1);
}

#[test]
fn llvm_parse_rejects_invalid_json() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::Path;
    assert!(parse_with_repo_root("{", Path::new("/workspace/covgate")).is_err());
}

#[test]
fn region_totals_ignore_non_entry_and_gap_segments() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [1, 1, 1, true, true, false],
                [2, 1, 1, true, false, false],
                [3, 1, 1, true, true, true],
                [4, 1, 1, true, true, false],
                [5, 1, 0, false, false, false]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");
    let totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Region)
        .expect("region totals should exist")
        .get(&PathBuf::from("src/lib.rs"))
        .expect("file totals should exist");

    assert_eq!(totals.covered, 2);
    assert_eq!(totals.total, 2);
}

#[test]
fn segment_boundary_does_not_overcount_lines() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [1, 1, 1, true, false, false],
                [2, 1, 0, false, false, false]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");
    let line_totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Line)
        .expect("line metric totals should exist")
        .get(&PathBuf::from("src/lib.rs"))
        .expect("file totals should exist");

    // Only line 1 should be covered and counted.
    assert_eq!(line_totals.covered, 1);
    assert_eq!(line_totals.total, 1);
}

#[test]
fn skips_segments_with_has_count_false_for_line_coverage() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"{
      "data": [{
        "files": [{
          "filename": "src/lib.rs",
          "segments": [
            [1, 1, 1, true, true, false],
            [2, 1, 0, false, true, false],
            [3, 1, 0, true, true, false],
            [4, 1, 0, true, true, false]
          ],
          "branches": []
        }],
        "functions": []
      }],
      "type": "llvm.coverage.json.export",
      "version": "2.0.1"
    }"#;

    let report = parse_with_repo_root(input, Path::new(".")).expect("parse");
    let lines = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Line)
        .unwrap()
        .get(&PathBuf::from("src/lib.rs"))
        .unwrap();
    // Line 1 is covered, Line 2 is skipped (hasCount false), Line 3 is uncovered.
    // So total should be 2.
    assert_eq!(lines.total, 2);
}

#[test]
fn skips_regions_with_backwards_range() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::Path;

    let input = r#"{
      "data": [{
        "files": [{
          "filename": "src/lib.rs",
          "segments": [
            [2, 1, 1, true, true, false],
            [1, 1, 0, true, false, false]
          ],
          "branches": []
        }],
        "functions": []
      }],
      "type": "llvm.coverage.json.export",
      "version": "2.0.1"
    }"#;

    let report = parse_with_repo_root(input, Path::new(".")).expect("parse");
    // No regions should be emitted because end < start
    assert!(
        !report
            .totals_by_file
            .contains_key(&covgate::model::MetricKind::Region)
    );
}

#[test]
fn skips_function_entries_without_filenames_or_regions() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::Path;

    let input = r#"
    {
      "data": [
        {
          "functions": [
            {
              "count": 1,
              "filenames": [],
              "regions": [[1,1,2,1,1,0,0,0]]
            },
            {
              "count": 1,
              "filenames": ["src/lib.rs"],
              "regions": []
            }
          ],
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [1, 1, 1, true, false, false],
                [2, 1, 0, false, false, false]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");
    assert!(
        !report
            .totals_by_file
            .contains_key(&covgate::model::MetricKind::Function)
    );
}

#[test]
fn rejects_negative_function_region_fields() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::Path;

    let input = r#"
    {
      "data": [
        {
          "functions": [
            {
              "count": 1,
              "filenames": ["src/lib.rs"],
              "regions": [[-1,1,2,1,1,0,0,0]]
            }
          ],
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [1, 1, 1, true, false, false],
                [2, 1, 0, false, false, false]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let error = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect_err("negative line should fail parsing");
    assert!(error.to_string().contains("failed to parse llvm json"));
}

#[test]
fn marks_function_covered_when_regions_have_execution_count() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "functions": [
            {
              "count": 0,
              "filenames": ["src/lib.rs"],
              "regions": [[10,1,12,1,3,0,0,0]]
            }
          ],
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [10, 1, 1, true, false, false],
                [12, 1, 0, false, false, false]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");
    let totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Function)
        .expect("function totals should exist")
        .get(&PathBuf::from("src/lib.rs"))
        .expect("file totals should exist");

    assert_eq!(totals.covered, 1);
    assert_eq!(totals.total, 1);
}

#[test]
fn merges_duplicate_function_spans_as_covered_if_any_variant_is_covered() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "functions": [
            {
              "count": 0,
              "filenames": ["src/lib.rs"],
              "regions": [[20,1,25,1,0,0,0,0]]
            },
            {
              "count": 1,
              "filenames": ["src/lib.rs"],
              "regions": [[20,1,25,1,1,0,0,0]]
            }
          ],
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [20, 1, 1, true, false, false],
                [25, 1, 0, false, false, false]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");
    let totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Function)
        .expect("function totals should exist")
        .get(&PathBuf::from("src/lib.rs"))
        .expect("file totals should exist");

    assert_eq!(totals.covered, 1);
    assert_eq!(totals.total, 1);
}

#[test]
fn keeps_rust_functions_with_different_crate_hashes_as_one_name_based_record() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "functions": [
            {
              "count": 1,
              "name": "_RNvNtCsAAAA_7covgate7metrics22compute_changed_metric",
              "filenames": ["src/lib.rs"],
              "regions": [[20,1,25,1,1,0,0,0]]
            },
            {
              "count": 1,
              "name": "_RNvNtCsBBBB_7covgate7metrics22compute_changed_metric",
              "filenames": ["src/lib.rs"],
              "regions": [[20,1,25,1,1,0,0,0]]
            }
          ],
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [
                [20, 1, 1, true, false, false],
                [25, 1, 0, false, false, false]
              ]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");
    let totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Function)
        .expect("function totals should exist")
        .get(&PathBuf::from("src/lib.rs"))
        .expect("file totals should exist");

    assert_eq!(totals.covered, 1);
    assert_eq!(totals.total, 1);

    let function_opportunities: Vec<_> = report
        .opportunities
        .iter()
        .filter(|op| op.kind == covgate::model::OpportunityKind::Function)
        .collect();
    assert_eq!(function_opportunities.len(), 1);
    assert_eq!(function_opportunities[0].span.start_line, 20);
    assert_eq!(function_opportunities[0].span.end_line, 25);
}

#[test]
fn prefers_longest_suffix_for_function_file_mapping() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::{Path, PathBuf};

    let input = r#"
    {
      "data": [
        {
          "functions": [
            {
              "count": 0,
              "filenames": ["/tmp/build/pkg/src/lib.rs"],
              "regions": [[10,1,10,5,0,0,0,0]]
            }
          ],
          "files": [
            {
              "filename": "src/lib.rs",
              "segments": [[1,1,1,true,false,false],[2,1,0,false,false,false]]
            },
            {
              "filename": "pkg/src/lib.rs",
              "segments": [[1,1,1,true,false,false],[2,1,0,false,false,false]]
            }
          ]
        }
      ]
    }
    "#;

    let report = parse_with_repo_root(input, Path::new("/workspace/covgate"))
        .expect("llvm export should parse");
    let function_totals = report
        .totals_by_file
        .get(&covgate::model::MetricKind::Function)
        .expect("function totals should exist");

    assert!(
        !function_totals.contains_key(&PathBuf::from("src/lib.rs")),
        "function should not map to less specific suffix"
    );
    let mapped = function_totals
        .get(&PathBuf::from("pkg/src/lib.rs"))
        .expect("function should map to longest matching suffix");
    assert_eq!(mapped.covered, 0);
    assert_eq!(mapped.total, 1);
}

#[test]
fn parse_with_repo_root_rejects_invalid_json() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::Path;
    let err = parse_with_repo_root("{", Path::new(".")).expect_err("parse should fail");
    assert!(err.to_string().contains("failed to parse coverage json"));
}

#[test]
fn parse_with_repo_root_rejects_unknown_format() {
    use covgate::coverage::parse_with_repo_root;
    use std::path::Path;
    let err =
        parse_with_repo_root(r#"{"foo":"bar"}"#, Path::new(".")).expect_err("parse should fail");
    assert!(err.to_string().contains("unsupported coverage format"));
}
