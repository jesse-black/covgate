use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};

pub const RECORDED_BASE_REF: &str = "refs/worktree/covgate/base";
const RECORDED_BASE_BRANCH_MARKER: &str = "refs/worktree/covgate/base.branch";

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

fn resolve_git_path(path: &str) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-path", path])
        .output()
        .with_context(|| format!("failed to run git rev-parse for git path {path}"))?;

    if !output.status.success() {
        bail!(
            "failed to resolve git path {path}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let raw =
        String::from_utf8(output.stdout).context("git rev-parse output was not valid utf-8")?;
    Ok(PathBuf::from(raw.trim()))
}

fn resolve_current_branch() -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .output()
        .context("failed to run git symbolic-ref for HEAD")?;

    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Ok(None);
        }

        bail!(
            "failed to resolve current branch: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(Some(
        String::from_utf8(output.stdout)
            .context("git symbolic-ref output was not valid utf-8")?
            .trim()
            .to_string(),
    ))
}

fn read_recorded_branch_marker() -> Result<Option<String>> {
    let marker_path = resolve_git_path(RECORDED_BASE_BRANCH_MARKER)?;
    if !marker_path.exists() {
        return Ok(None);
    }

    let branch = fs::read_to_string(&marker_path).with_context(|| {
        format!(
            "failed to read branch marker from {}",
            marker_path.display()
        )
    })?;
    let branch = branch.trim();
    if branch.is_empty() {
        return Ok(None);
    }

    Ok(Some(branch.to_string()))
}

fn write_recorded_branch_marker(branch: &str) -> Result<()> {
    let marker_path = resolve_git_path(RECORDED_BASE_BRANCH_MARKER)?;
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create marker directory {}", parent.display()))?;
    }
    fs::write(&marker_path, format!("{branch}\n"))
        .with_context(|| format!("failed to write branch marker {}", marker_path.display()))
}

fn is_ancestor(ancestor: &str, descendant: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .output()
        .context("failed to run git merge-base --is-ancestor")?;

    Ok(output.status.success())
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
    let current_branch = resolve_current_branch()?;

    if let Some(existing) = resolve_ref_sha(RECORDED_BASE_REF)? {
        let recorded_branch = read_recorded_branch_marker()?;
        let should_refresh = match (&current_branch, recorded_branch.as_deref()) {
            (Some(current), Some(recorded)) => current != recorded,
            (_, _) => !is_ancestor(&existing, &head_sha)?,
        };

        if should_refresh {
            create_ref(RECORDED_BASE_REF, "HEAD")?;
            if let Some(branch) = current_branch.as_deref() {
                write_recorded_branch_marker(branch)?;
                println!(
                    "Refreshed base commit {head_sha} at {RECORDED_BASE_REF} for branch {branch}"
                );
            } else {
                println!("Refreshed base commit {head_sha} at {RECORDED_BASE_REF}");
            }
            return Ok(head_sha);
        }

        if recorded_branch.is_none()
            && let Some(branch) = current_branch.as_deref()
        {
            write_recorded_branch_marker(branch)?;
        }
        println!("Base already recorded at {RECORDED_BASE_REF} -> {existing}");
        return Ok(existing);
    }

    create_ref(RECORDED_BASE_REF, "HEAD")?;
    if let Some(branch) = current_branch.as_deref() {
        write_recorded_branch_marker(branch)?;
    }
    println!("Recorded base commit {head_sha} at {RECORDED_BASE_REF}");
    Ok(head_sha)
}
