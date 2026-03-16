use std::process::Command;

use anyhow::{Context, Result, bail};

pub const RECORDED_BASE_REF: &str = "refs/worktree/covgate/base";

pub fn resolve_head_sha() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD^{commit}"])
        .output()
        .context("failed to run git rev-parse for HEAD")?;

    if !output.status.success() {
        bail!(
            "failed to resolve HEAD commit: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8(output.stdout)
        .context("git rev-parse output was not valid utf-8")?
        .trim()
        .to_string())
}

pub fn resolve_ref_sha(reference: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .output()
        .with_context(|| format!("failed to run git rev-parse for {reference}"))?;

    if output.status.success() {
        return Ok(Some(
            String::from_utf8(output.stdout)
                .context("git rev-parse output was not valid utf-8")?
                .trim()
                .to_string(),
        ));
    }

    Ok(None)
}

pub fn create_ref(reference: &str, target: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["update-ref", reference, target])
        .output()
        .with_context(|| format!("failed to run git update-ref for {reference}"))?;

    if !output.status.success() {
        bail!(
            "failed to update git ref {reference}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

pub fn discover_base_ref() -> Result<Option<String>> {
    for candidate in [
        RECORDED_BASE_REF,
        "origin/HEAD",
        "origin/main",
        "origin/master",
        "main",
        "master",
    ] {
        if resolve_ref_sha(candidate)?.is_some() {
            return Ok(Some(candidate.to_string()));
        }
    }

    Ok(None)
}

pub fn record_base_ref() -> Result<String> {
    let head_sha = resolve_head_sha()?;

    if let Some(existing) = resolve_ref_sha(RECORDED_BASE_REF)? {
        println!("Base already recorded at {RECORDED_BASE_REF} -> {existing}");
        return Ok(existing);
    }

    create_ref(RECORDED_BASE_REF, "HEAD")?;
    println!("Recorded base commit {head_sha} at {RECORDED_BASE_REF}");
    Ok(head_sha)
}
