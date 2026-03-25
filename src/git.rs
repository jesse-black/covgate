use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

use anyhow::{Context, Result, bail};

pub const RECORDED_BASE_REF: &str = "refs/worktree/covgate/base";
const RECORDED_BASE_BRANCH_MARKER: &str = "covgate/base.branch";
const STANDARD_BASE_REFS: &[&str] = &[
    "origin/HEAD",
    "origin/main",
    "origin/master",
    "main",
    "master",
];

struct GitOutput(Output);

fn git_output(args: &[&str], context: &'static str) -> Result<GitOutput> {
    Command::new("git")
        .args(args)
        .output()
        .context(context)
        .map(GitOutput)
}

impl GitOutput {
    fn require_success(self, failure_message: impl FnOnce(&Self) -> String) -> Result<Self> {
        if self.0.status.success() {
            return Ok(self);
        }

        bail!("{}", failure_message(&self))
    }

    fn optional_on_nonzero(self) -> Option<Self> {
        if self.0.status.success() {
            Some(self)
        } else {
            None
        }
    }

    fn stdout_utf8(self, context: &'static str) -> Result<String> {
        String::from_utf8(self.0.stdout)
            .context(context)
            .map(|text| text.trim().to_string())
    }

    fn ignore_stdout(self) {}

    fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.0.stderr).trim().to_string()
    }

    fn status_code(&self) -> Option<i32> {
        self.0.status.code()
    }

    fn status(&self) -> &std::process::ExitStatus {
        &self.0.status
    }
}

pub fn ensure_available() -> Result<()> {
    git_output(
        &["--version"],
        "git is required to run covgate but was not found in PATH",
    )?
    .require_success(|output| {
        let stderr = output.stderr_text();
        if stderr.is_empty() {
            "git is required to run covgate but `git --version` failed".to_string()
        } else {
            format!("git is required to run covgate but `git --version` failed: {stderr}")
        }
    })?
    .ignore_stdout();

    Ok(())
}

pub fn resolve_head_sha() -> Result<String> {
    git_output(
        &["rev-parse", "--verify", "HEAD^{commit}"],
        "failed to run git rev-parse for HEAD",
    )?
    .require_success(|output| format!("failed to resolve HEAD commit: {}", output.stderr_text()))?
    .stdout_utf8("git rev-parse output was not valid utf-8")
}

pub fn resolve_ref_sha(reference: &str) -> Result<Option<String>> {
    git_output(
        &["rev-parse", "--verify", "--quiet", reference],
        "failed to run git rev-parse for reference",
    )?
    .optional_on_nonzero()
    .map(|output| output.stdout_utf8("git rev-parse output was not valid utf-8"))
    .transpose()
}

pub fn resolve_repo_root() -> Result<Option<PathBuf>> {
    git_output(
        &["rev-parse", "--show-toplevel"],
        "failed to run git rev-parse for repository root",
    )?
    .optional_on_nonzero()
    .map(|output| output.stdout_utf8("git rev-parse output was not valid utf-8"))
    .transpose()
    .map(|root| root.and_then(|root| (!root.is_empty()).then(|| PathBuf::from(root))))
}

pub fn merge_base(base: &str, head: &str) -> Result<String> {
    git_output(&["merge-base", base, head], "failed to run git merge-base")?
        .require_success(|output| format!("git merge-base failed with status {}", output.status()))?
        .stdout_utf8("git merge-base output was not valid utf-8")
}

pub fn diff_with_unified_zero(base: &str) -> Result<String> {
    git_output(
        &["diff", "--unified=0", "--no-ext-diff", base],
        "failed to run git diff",
    )?
    .require_success(|output| format!("git diff failed with status {}", output.status()))?
    .stdout_utf8("git diff output was not valid utf-8")
}

pub fn list_untracked_files() -> Result<Vec<String>> {
    let stdout = git_output(
        &["ls-files", "--others", "--exclude-standard"],
        "failed to run git ls-files for untracked files",
    )?
    .require_success(|output| format!("failed to list untracked files: {}", output.stderr_text()))?
    .stdout_utf8("git ls-files output was not valid utf-8")?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    Ok(trimmed.lines().map(str::to_string).collect())
}

pub fn create_ref(reference: &str, target: &str) -> Result<()> {
    git_output(
        &["update-ref", reference, target],
        "failed to run git update-ref",
    )?
    .require_success(|output| {
        format!(
            "failed to update git ref {reference}: {}",
            output.stderr_text()
        )
    })?
    .ignore_stdout();

    Ok(())
}

fn resolve_git_path(path: &str) -> Result<PathBuf> {
    git_output(
        &["rev-parse", "--git-path", path],
        "failed to run git rev-parse for requested git path",
    )?
    .require_success(|output| {
        format!(
            "failed to resolve git path {path}: {}",
            output.stderr_text()
        )
    })?
    .stdout_utf8("git rev-parse output was not valid utf-8")
    .map(PathBuf::from)
}

fn resolve_current_branch() -> Result<Option<String>> {
    let output = git_output(
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
        "failed to run git symbolic-ref for HEAD",
    )?;

    if output.status_code() == Some(1) {
        return Ok(None);
    }

    output
        .require_success(|output| {
            format!("failed to resolve current branch: {}", output.stderr_text())
        })?
        .stdout_utf8("git symbolic-ref output was not valid utf-8")
        .map(Some)
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
    Ok(git_output(
        &["merge-base", "--is-ancestor", ancestor, descendant],
        "failed to run git merge-base --is-ancestor",
    )?
    .optional_on_nonzero()
    .is_some())
}

pub fn discover_base_ref() -> Result<Option<String>> {
    for candidate in STANDARD_BASE_REFS
        .iter()
        .copied()
        .chain([RECORDED_BASE_REF].into_iter())
    {
        if resolve_ref_sha(candidate)?.is_some() {
            return Ok(Some(candidate.to_string()));
        }
    }

    Ok(None)
}

fn find_first_resolved_base_ref(
    resolve_ref: fn(&str) -> Result<Option<String>>,
) -> Result<Option<(&'static str, String)>> {
    for candidate in STANDARD_BASE_REFS {
        if let Some(sha) = resolve_ref(candidate)? {
            return Ok(Some((candidate, sha)));
        }
    }

    Ok(None)
}

fn discover_standard_base_ref() -> Result<Option<(&'static str, String)>> {
    find_first_resolved_base_ref(resolve_ref_sha)
}

pub fn record_base_ref() -> Result<String> {
    if let Some((base_ref, sha)) = discover_standard_base_ref()? {
        println!(
            "Base ref `{base_ref}` is available; `record-base` is unnecessary in this environment."
        );
        return Ok(sha);
    }

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
    use std::process::{ExitStatus, Output};

    use super::{GitOutput, find_first_resolved_base_ref};

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }

    fn mock_output(code: i32, stdout: &str, stderr: &str) -> GitOutput {
        GitOutput(Output {
            status: exit_status(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        })
    }

    #[test]
    fn require_success_returns_error_for_nonzero_status() {
        let err =
            match mock_output(1, "", "nope").require_success(|_| "expected failure".to_string()) {
                Ok(_) => panic!("nonzero git output should fail"),
                Err(err) => err,
            };

        assert!(err.to_string().contains("expected failure"));
    }

    #[test]
    fn optional_on_nonzero_returns_none_for_nonzero_status() {
        assert!(mock_output(1, "", "").optional_on_nonzero().is_none());
    }

    #[test]
    fn stdout_utf8_trims_whitespace() {
        let stdout = mock_output(0, "  some-sha-123  \n", "")
            .stdout_utf8("utf8 should decode")
            .expect("stdout should decode");
        assert_eq!(stdout, "some-sha-123");
    }

    #[test]
    fn first_resolved_base_ref_returns_first_match() {
        let resolved = find_first_resolved_base_ref(|candidate| {
            Ok(match candidate {
                "origin/HEAD" => None,
                "origin/main" => None,
                _ => Some("later".to_string()),
            })
        })
        .expect("base-ref scan should succeed");

        assert_eq!(resolved, Some(("origin/master", "later".to_string())));
    }

    #[test]
    fn first_resolved_base_ref_returns_none_when_nothing_resolves() {
        let resolved =
            find_first_resolved_base_ref(|_| Ok(None)).expect("empty base-ref scan should succeed");

        assert_eq!(resolved, None);
    }
}
