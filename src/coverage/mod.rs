use std::{env, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::model::CoverageReport;

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
    let format = detect_format(&parsed)?;

    let repo_root = env::current_dir()
        .context("failed to determine current directory for coverage path normalization")?;

    match format {
        CoverageFormat::Llvm => llvm_json::parse_str_with_repo_root(input, &repo_root),
        CoverageFormat::Coverlet => coverlet_json::parse_str_with_repo_root(input, &repo_root),
        CoverageFormat::Istanbul => istanbul_json::parse_str_with_repo_root(input, &repo_root),
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
    use std::fs;

    use tempfile::tempdir;

    use super::{CoverageFormat, detect_format, parse_path, parse_str};

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
    fn parse_path_reads_file_and_dispatches() {
        let temp = tempdir().expect("temp dir should exist");
        let path = temp.path().join("coverage.json");
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
}
