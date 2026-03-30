use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::{git, model::CoverageReport};

pub mod coverlet_json;
pub mod istanbul_json;
pub mod llvm_json;

pub fn parse_path(path: &Path) -> Result<CoverageReport> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read coverage json: {}", path.display()))?;
    parse_str(&text)
}

pub fn parse_str(input: &str) -> Result<CoverageReport> {
    let parsed: Value = serde_json::from_str(input).context("failed to parse coverage json")?;
    let _format = detect_format(&parsed)?;

    let repo_root = git::resolve_repo_root()
        .context("failed to determine repository root for coverage path normalization")?
        .ok_or_else(|| anyhow::anyhow!("coverage path normalization requires a git repository"))?;

    parse_with_repo_root(input, &repo_root)
}

fn parse_with_repo_root(input: &str, repo_root: &Path) -> Result<CoverageReport> {
    let parsed: Value = serde_json::from_str(input).context("failed to parse coverage json")?;
    let format = detect_format(&parsed)?;

    match format {
        CoverageFormat::Llvm => llvm_json::parse_str_with_repo_root(input, repo_root),
        CoverageFormat::Coverlet => coverlet_json::parse_str_with_repo_root(input, repo_root),
        CoverageFormat::Istanbul => istanbul_json::parse_str_with_repo_root(input, repo_root),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CoverageFormat {
    Llvm,
    Coverlet,
    Istanbul,
}

fn detect_format(value: &Value) -> Result<CoverageFormat> {
    let llvm = value
        .as_object()
        .and_then(|obj| obj.get("data"))
        .is_some_and(Value::is_array);
    let coverlet = contains_coverlet_markers(value);
    let istanbul = contains_istanbul_markers(value);

    match (llvm, coverlet, istanbul) {
        (true, false, false) => Ok(CoverageFormat::Llvm),
        (false, true, false) => Ok(CoverageFormat::Coverlet),
        (false, false, true) => Ok(CoverageFormat::Istanbul),
        (false, false, false) => {
            bail!(
                "unsupported coverage format: expected LLVM JSON export, Coverlet native JSON, or Istanbul native JSON"
            )
        }
        _ => bail!(
            "coverage format is ambiguous; multiple supported coverage format markers were detected"
        ),
    }
}

fn contains_coverlet_markers(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, nested)| {
            ((key == "Lines" && nested.is_object()) || (key == "Branches" && nested.is_array()))
                || contains_coverlet_markers(nested)
        }),
        Value::Array(values) => values.iter().any(contains_coverlet_markers),
        _ => false,
    }
}

fn contains_istanbul_markers(value: &Value) -> bool {
    let Some(files) = value.as_object() else {
        return false;
    };

    files.values().any(|entry| {
        entry.as_object().is_some_and(|object| {
            object.contains_key("statementMap")
                && object.contains_key("fnMap")
                && object.contains_key("branchMap")
                && object.contains_key("s")
                && object.contains_key("f")
                && object.contains_key("b")
        })
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, sync::Mutex};

    use tempfile::tempdir;

    use crate::model::MetricKind;

    use super::{CoverageFormat, detect_format, parse_path, parse_str};

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
    fn detects_llvm_json() {
        let value = serde_json::json!({"data": []});
        assert_eq!(
            detect_format(&value).expect("format should detect"),
            CoverageFormat::Llvm
        );
    }

    #[test]
    fn detects_coverlet_json() {
        let value = serde_json::json!({
            "Demo.dll": {
                "src/lib.cs": {
                    "Demo.Math": {
                        "System.Int32 Demo.Math::Add()": {
                            "Lines": {"1": 1},
                            "Branches": []
                        }
                    }
                }
            }
        });
        assert_eq!(
            detect_format(&value).expect("format should detect"),
            CoverageFormat::Coverlet
        );
    }

    #[test]
    fn detects_istanbul_json() {
        let value = serde_json::json!({
            "src/math.js": {
                "statementMap": {},
                "fnMap": {},
                "branchMap": {},
                "s": {},
                "f": {},
                "b": {}
            }
        });
        assert_eq!(
            detect_format(&value).expect("format should detect"),
            CoverageFormat::Istanbul
        );
    }

    #[test]
    fn rejects_ambiguous_format() {
        let value = serde_json::json!({
            "data": [],
            "Demo.dll": {
                "src/lib.cs": {
                    "Demo.Math": {
                        "m": {
                            "Lines": {"1": 1}
                        }
                    }
                }
            }
        });

        let err = detect_format(&value).expect_err("format should be ambiguous");
        assert!(err.to_string().contains("ambiguous"));
    }

    #[test]
    fn rejects_ambiguous_coverlet_and_istanbul_format() {
        let value = serde_json::json!({
            "Demo.dll": {
                "src/lib.cs": {
                    "Demo.Math": {
                        "m": {
                            "Lines": {"1": 1},
                            "Branches": []
                        }
                    }
                }
            },
            "src/math.js": {
                "statementMap": {},
                "fnMap": {},
                "branchMap": {},
                "s": {},
                "f": {},
                "b": {}
            }
        });

        let err = detect_format(&value).expect_err("format should be ambiguous");
        assert!(err.to_string().contains("ambiguous"));
    }

    #[test]
    fn rejects_array_json_for_istanbul_detection() {
        let value = serde_json::json!([{"statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {}}]);
        let err = detect_format(&value).expect_err("array root should be unsupported");
        assert!(err.to_string().contains("unsupported coverage format"));
    }

    #[test]
    fn rejects_unknown_format() {
        let value = serde_json::json!({"foo": "bar"});
        let err = detect_format(&value).expect_err("format should be unsupported");
        assert!(err.to_string().contains("unsupported coverage format"));
    }

    #[test]
    fn parse_str_rejects_invalid_json() {
        let err = parse_str("{").expect_err("parse should fail");
        assert!(err.to_string().contains("failed to parse coverage json"));
    }

    #[test]
    fn parse_str_rejects_unknown_format_before_git_lookup() {
        let _lock = CWD_LOCK.lock().expect("cwd lock should not be poisoned");

        let temp = tempdir().expect("temp dir should exist");
        let previous = std::env::current_dir().expect("cwd should resolve");
        let _guard = CwdGuard(previous);
        std::env::set_current_dir(temp.path()).expect("should chdir into temp workspace");

        let err = parse_str(r#"{"foo":"bar"}"#).expect_err("parse should fail");
        assert!(err.to_string().contains("unsupported coverage format"));
    }

    #[test]
    fn parse_path_reads_file_and_dispatches() {
        let _lock = CWD_LOCK.lock().expect("cwd lock should not be poisoned");

        let temp = tempdir().expect("temp dir should exist");
        let repo = temp.path();
        fs::write(repo.join("README.md"), "initial\n").expect("readme should write");

        run_git(repo, &["init"]);
        run_git(repo, &["config", "user.email", "covgate@example.com"]);
        run_git(repo, &["config", "user.name", "Covgate Tests"]);
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "initial"]);

        let previous = std::env::current_dir().expect("cwd should resolve");
        let _guard = CwdGuard(previous);
        std::env::set_current_dir(repo).expect("should chdir into temp repo");

        let path = repo.join("coverage.json");
        fs::write(
            &path,
            r#"{
              "Demo.dll": {
                "src/lib.cs": {
                  "Demo.Math": {
                    "System.Int32 Demo.Math::Add()": {
                      "Lines": {"1": 1},
                    "Branches": []
                    }
                }
              }
              }
            }"#,
        )
        .expect("coverage file should be written");

        let report = parse_path(&path).expect("coverage file should parse");
        assert!(!report.opportunities.is_empty());
    }

    #[test]
    fn parse_path_prefers_git_repo_root_for_absolute_coverlet_paths() {
        let _lock = CWD_LOCK.lock().expect("cwd lock should not be poisoned");

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
        .expect("coverage file should be written");

        let report = parse_path(&coverage_path).expect("coverage file should parse");
        assert!(
            report
                .totals_by_file
                .get(&MetricKind::Line)
                .expect("line totals should exist")
                .contains_key(&PathBuf::from("src/lib.cs"))
        );
    }

    #[test]
    fn parse_path_reads_istanbul_file_and_dispatches() {
        let _lock = CWD_LOCK.lock().expect("cwd lock should not be poisoned");

        let temp = tempdir().expect("temp dir should exist");
        let repo = temp.path();
        fs::write(repo.join("README.md"), "initial\n").expect("readme should write");

        run_git(repo, &["init"]);
        run_git(repo, &["config", "user.email", "covgate@example.com"]);
        run_git(repo, &["config", "user.name", "Covgate Tests"]);
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "initial"]);

        let previous = std::env::current_dir().expect("cwd should resolve");
        let _guard = CwdGuard(previous);
        std::env::set_current_dir(repo).expect("should chdir into temp repo");

        let path = repo.join("coverage.json");
        fs::write(
            &path,
            r#"{
              "src/math.js": {
                "statementMap": {
                  "0": {"start": {"line": 1}, "end": {"line": 1}}
                },
                "fnMap": {
                  "0": {"loc": {"start": {"line": 1}, "end": {"line": 1}}}
                },
                "branchMap": {},
                "s": {"0": 1},
                "f": {"0": 1},
                "b": {}
              }
            }"#,
        )
        .expect("coverage file should be written");

        let report = parse_path(&path).expect("coverage file should parse");
        assert!(!report.opportunities.is_empty());
    }

    #[test]
    fn parse_str_uses_git_repo_root_for_absolute_coverlet_paths_from_subdir() {
        let _lock = CWD_LOCK.lock().expect("cwd lock should not be poisoned");

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
        let input = format!(
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
        );

        let report = parse_str(&input).expect("coverage should parse");
        assert!(
            report
                .totals_by_file
                .get(&MetricKind::Line)
                .expect("line totals should exist")
                .contains_key(&PathBuf::from("src/lib.cs"))
        );
    }

    #[test]
    fn parse_str_requires_git_repo_for_path_normalization() {
        let _lock = CWD_LOCK.lock().expect("cwd lock should not be poisoned");

        let temp = tempdir().expect("temp dir should exist");
        let workspace = temp.path();
        fs::create_dir_all(workspace.join("src")).expect("src dir should exist");

        let previous = std::env::current_dir().expect("cwd should resolve");
        let _guard = CwdGuard(previous);
        std::env::set_current_dir(workspace).expect("should chdir into temp workspace");

        let absolute_source = workspace.join("src").join("lib.cs");
        let input = format!(
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
        );

        let err = parse_str(&input).expect_err("parse should require a git repo");
        assert!(
            err.to_string()
                .contains("coverage path normalization requires a git repository")
        );
    }

    #[test]
    fn parse_str_reports_git_repo_lookup_failure_when_git_is_missing() {
        let _lock = CWD_LOCK.lock().expect("cwd lock should not be poisoned");

        let temp = tempdir().expect("temp dir should exist");
        let previous = std::env::current_dir().expect("cwd should resolve");
        let _guard = CwdGuard(previous);
        std::env::set_current_dir(temp.path()).expect("should chdir into temp workspace");

        with_path_override("", || {
            let err = parse_str(
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
            .expect_err("parse should fail when git lookup cannot run");

            assert!(
                err.to_string().contains(
                    "failed to determine repository root for coverage path normalization"
                )
            );
        });
    }
}
