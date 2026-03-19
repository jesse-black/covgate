use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

use anyhow::{Context, Result, bail};

pub const RECORDED_BASE_REF: &str = "refs/worktree/covgate/base";
const RECORDED_BASE_BRANCH_MARKER: &str = "refs/worktree/covgate/base.branch";

fn git_output(args: &[&str], context: &'static str) -> Result<Output> {
    Command::new("git").args(args).output().context(context)
}

fn stdout_utf8(output: Output, context: &'static str) -> Result<String> {
    String::from_utf8(output.stdout)
        .context(context)
        .map(|text| text.trim().to_string())
}

pub fn resolve_head_sha() -> Result<String> {
    let output = git_output(
        &["rev-parse", "--verify", "HEAD^{commit}"],
        "failed to run git rev-parse for HEAD",
    )?;

    if !output.status.success() {
        bail!(
            "failed to resolve HEAD commit: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    stdout_utf8(output, "git rev-parse output was not valid utf-8")
}

pub fn resolve_ref_sha(reference: &str) -> Result<Option<String>> {
    let output = git_output(
        &["rev-parse", "--verify", "--quiet", reference],
        "failed to run git rev-parse for reference",
    )?;

    if output.status.success() {
        return stdout_utf8(output, "git rev-parse output was not valid utf-8").map(Some);
    }

    Ok(None)
}

pub fn create_ref(reference: &str, target: &str) -> Result<()> {
    let output = git_output(
        &["update-ref", reference, target],
        "failed to run git update-ref",
    )?;

    if !output.status.success() {
        bail!(
            "failed to update git ref {reference}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

fn resolve_git_path(path: &str) -> Result<PathBuf> {
    let output = git_output(
        &["rev-parse", "--git-path", path],
        "failed to run git rev-parse for requested git path",
    )?;

    if !output.status.success() {
        bail!(
            "failed to resolve git path {path}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    stdout_utf8(output, "git rev-parse output was not valid utf-8").map(PathBuf::from)
}

fn resolve_current_branch() -> Result<Option<String>> {
    let output = git_output(
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
        "failed to run git symbolic-ref for HEAD",
    )?;

    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Ok(None);
        }

        bail!(
            "failed to resolve current branch: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    stdout_utf8(output, "git symbolic-ref output was not valid utf-8").map(Some)
}

fn read_recorded_branch_marker() -> Result<Option<String>> {
    let marker_path = resolve_git_path(RECORDED_BASE_BRANCH_MARKER)?;
    if !marker_path.exists() {
        return Ok(None);
    }

    let branch =
        fs::read_to_string(&marker_path).context("failed to read recorded base branch marker")?;
    let branch = branch.trim();
    if branch.is_empty() {
        return Ok(None);
    }

    Ok(Some(branch.to_string()))
}

fn write_recorded_branch_marker(branch: &str) -> Result<()> {
    let marker_path = resolve_git_path(RECORDED_BASE_BRANCH_MARKER)?;
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent).context("failed to create recorded branch marker directory")?;
    }
    fs::write(&marker_path, format!("{branch}\n"))
        .context("failed to write recorded base branch marker")
}

fn is_ancestor(ancestor: &str, descendant: &str) -> Result<bool> {
    let output = git_output(
        &["merge-base", "--is-ancestor", ancestor, descendant],
        "failed to run git merge-base --is-ancestor",
    )?;

    Ok(output.status.success())
}

pub fn list_untracked_files() -> Result<Vec<String>> {
    let output = git_output(
        &["ls-files", "--others", "--exclude-standard"],
        "failed to run git ls-files for untracked files",
    )?;

    if !output.status.success() {
        bail!(
            "failed to list untracked files: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout = stdout_utf8(output, "git ls-files output was not valid utf-8")?;
    if stdout.is_empty() {
        return Ok(Vec::new());
    }

    Ok(stdout.lines().map(ToString::to_string).collect())
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

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use tempfile::tempdir;

    use super::list_untracked_files;
    use crate::test_support::CWD_LOCK;

    struct CwdGuard(std::path::PathBuf);
    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }

    #[test]
    fn list_untracked_files_reports_empty_and_non_empty_results() {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let temp = tempdir().expect("tempdir should exist");
        let previous = env::current_dir().expect("cwd should resolve");
        let _guard = CwdGuard(previous);
        env::set_current_dir(temp.path()).expect("should chdir into tempdir");
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .args(args)
                .output()
                .expect("git command should run");
            assert!(output.status.success(), "git {:?} failed", args);
        };
        git(&["init"]);
        git(&["config", "user.email", "covgate@example.com"]);
        git(&["config", "user.name", "Covgate Tests"]);
        fs::write("tracked.txt", "tracked\n").expect("tracked file should write");
        git(&["add", "."]);
        git(&["commit", "-m", "baseline"]);

        assert!(
            list_untracked_files()
                .expect("listing should succeed")
                .is_empty()
        );
        fs::write("new_untracked.rs", "pub fn pending() {}\n").expect("file should write");
        assert_eq!(
            list_untracked_files().expect("listing should succeed"),
            vec!["new_untracked.rs"]
        );
    }
}
