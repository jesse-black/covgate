use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(task) = args.next() else {
        bail!("usage: cargo xtask <task>");
    };

    match task.as_str() {
        "validate" => validate(),
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
