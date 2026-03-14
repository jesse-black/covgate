use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(task) = args.next() else {
        bail!(
            "usage: cargo xtask <task>\n\n  validate\n  regen-fixture-coverage <language>/<scenario>\n  regen-fixture-coverage-all"
        );
    };

    match task.as_str() {
        "validate" => validate(),
        "regen-fixture-coverage" => {
            let Some(fixture_id) = args.next() else {
                bail!("usage: cargo xtask regen-fixture-coverage <language>/<scenario>");
            };
            regen_fixture_coverage(&fixture_id)
        }
        "regen-fixture-coverage-all" => regen_fixture_coverage_all(),
        _ => bail!("unknown xtask `{task}`"),
    }
}

fn validate() -> Result<()> {
    run("cargo", &["fmt", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    let coverage_json = coverage_path();
    let coverage_json_str = coverage_json
        .to_str()
        .context("coverage output path contained non-utf8 characters")?;

    run(
        "cargo",
        &[
            "llvm-cov",
            "--json",
            "--output-path",
            coverage_json_str,
            "--fail-under-regions=88",
        ],
    )?;

    let base_ref = resolve_base_ref()?;
    run(
        "cargo",
        &[
            "run",
            "--bin",
            "covgate",
            "--",
            "--coverage-json",
            coverage_json_str,
            "--base",
            &base_ref,
        ],
    )?;

    run("cargo-machete", &["."])?;
    run("cargo-deny", &["check"])?;

    std::fs::remove_file(&coverage_json).ok();
    Ok(())
}

#[derive(Clone, Copy)]
struct FixtureCoverageSpec {
    id: &'static str,
    source_file: &'static str,
    include_branches: bool,
    branch_column: u32,
    covered_overlay_line: bool,
}

const FIXTURE_COVERAGE_SPECS: &[FixtureCoverageSpec] = &[
    FixtureCoverageSpec {
        id: "rust/basic-fail",
        source_file: "src/lib.rs",
        include_branches: false,
        branch_column: 1,
        covered_overlay_line: false,
    },
    FixtureCoverageSpec {
        id: "rust/basic-pass",
        source_file: "src/lib.rs",
        include_branches: false,
        branch_column: 1,
        covered_overlay_line: true,
    },
    FixtureCoverageSpec {
        id: "cpp/basic-fail",
        source_file: "src/lib.cpp",
        include_branches: true,
        branch_column: 5,
        covered_overlay_line: false,
    },
    FixtureCoverageSpec {
        id: "cpp/basic-pass",
        source_file: "src/lib.cpp",
        include_branches: true,
        branch_column: 5,
        covered_overlay_line: true,
    },
    FixtureCoverageSpec {
        id: "swift/basic-fail",
        source_file: "Sources/CovgateDemo/CovgateDemo.swift",
        include_branches: true,
        branch_column: 8,
        covered_overlay_line: false,
    },
    FixtureCoverageSpec {
        id: "swift/basic-pass",
        source_file: "Sources/CovgateDemo/CovgateDemo.swift",
        include_branches: true,
        branch_column: 8,
        covered_overlay_line: true,
    },
];

fn regen_fixture_coverage(fixture_id: &str) -> Result<()> {
    let spec = FIXTURE_COVERAGE_SPECS
        .iter()
        .find(|spec| spec.id == fixture_id)
        .ok_or_else(|| anyhow::anyhow!("unknown fixture `{fixture_id}`"))?;
    write_fixture_coverage(spec)
}

fn regen_fixture_coverage_all() -> Result<()> {
    for spec in FIXTURE_COVERAGE_SPECS {
        write_fixture_coverage(spec)?;
    }
    Ok(())
}

fn write_fixture_coverage(spec: &FixtureCoverageSpec) -> Result<()> {
    let repo_root = project_root()?;
    let coverage_path = repo_root
        .join("tests")
        .join("fixtures")
        .join(spec.id)
        .join("coverage.json");

    let overlay_count = if spec.covered_overlay_line { 1 } else { 0 };
    let line_two_col = if spec.include_branches {
        spec.branch_column
    } else {
        1
    };

    let branches = if spec.include_branches {
        format!(
            ",\n          \"branches\": [\n            [1, {line_two_col}, 1, true],\n            [3, {line_two_col}, {overlay_count}, true]\n          ]"
        )
    } else {
        String::new()
    };

    let json = format!(
        "{{\n  \"data\": [\n    {{\n      \"files\": [\n        {{\n          \"filename\": \"{source_file}\",\n          \"segments\": [\n            [1, 1, 1, true, false, false],\n            [3, 1, {overlay_count}, true, false, false],\n            [5, 1, 0, true, false, false],\n            [6, 1, 0, false, false, false]\n          ]{branches}\n        }}\n      ]\n    }}\n  ]\n}}\n",
        source_file = spec.source_file,
        overlay_count = overlay_count,
        branches = branches
    );

    std::fs::write(&coverage_path, json).with_context(|| {
        format!(
            "failed to write fixture coverage: {}",
            coverage_path.display()
        )
    })?;

    eprintln!("updated {}", coverage_path.display());
    Ok(())
}

fn project_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .context("xtask manifest should live under the repository root")?;
    Ok(root.to_path_buf())
}

fn resolve_base_ref() -> Result<String> {
    for candidate in ["origin/main", "origin/master", "main", "master", "HEAD~1"] {
        if git_ref_exists(candidate) {
            return Ok(candidate.to_owned());
        }
    }

    bail!("unable to resolve a usable git base reference for covgate dogfooding")
}

fn git_ref_exists(reference: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn coverage_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "covgate-xtask-validate-{}-{}.json",
        std::process::id(),
        chrono_like_timestamp()
    ));
    path
}

fn chrono_like_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    eprintln!("> {} {}", program, args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute `{program}`"))?;

    if !status.success() {
        bail!(
            "command `{program} {}` failed with status {status}",
            args.join(" ")
        );
    }

    Ok(())
}
