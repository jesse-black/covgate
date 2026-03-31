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
