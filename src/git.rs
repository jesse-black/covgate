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
    let commit_ref = format!("{reference}^{{commit}}");
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", &commit_ref])
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

    let show_ref = Command::new("git")
        .args(["show-ref", "--verify", "--hash", reference])
        .output()
        .with_context(|| format!("failed to run git show-ref for {reference}"))?;

    if !show_ref.status.success() {
        return Ok(None);
    }

    Ok(Some(
        String::from_utf8(show_ref.stdout)
            .context("git show-ref output was not valid utf-8")?
            .trim()
            .to_string(),
    ))
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

#[cfg(test)]
mod tests {
    use std::{env, fs, sync::Mutex};

    use tempfile::tempdir;

    use super::{RECORDED_BASE_REF, discover_base_ref, record_base_ref, resolve_ref_sha};

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn run_git(path: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(path)
            .output()
            .expect("git command should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn with_temp_git_repo<F>(f: F)
    where
        F: FnOnce(&std::path::Path),
    {
        let _lock = CWD_LOCK.lock().expect("cwd lock should be available");

        struct CwdGuard(std::path::PathBuf);
        impl Drop for CwdGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }

        let temp = tempdir().expect("tempdir should exist");
        let repo = temp.path();
        fs::write(repo.join("README.md"), "initial\n").expect("fixture file should be written");
        run_git(repo, &["init"]);
        run_git(repo, &["config", "user.email", "covgate@example.com"]);
        run_git(repo, &["config", "user.name", "Covgate Tests"]);
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "initial"]);

        let previous = env::current_dir().expect("cwd should resolve");
        let _guard = CwdGuard(previous);
        env::set_current_dir(repo).expect("should chdir into repo");
        f(repo);
    }

    #[test]
    fn record_base_creates_ref_when_missing() {
        with_temp_git_repo(|_| {
            let head_before = resolve_ref_sha("HEAD").expect("head resolve should work");
            let _ = record_base_ref().expect("record-base should succeed");
            let recorded = resolve_ref_sha(RECORDED_BASE_REF).expect("recorded ref should resolve");
            assert_eq!(recorded, head_before);
        });
    }

    #[test]
    fn record_base_is_idempotent() {
        with_temp_git_repo(|repo| {
            let first = record_base_ref().expect("first record should work");
            fs::write(repo.join("next.txt"), "next\n").expect("next file should write");
            run_git(repo, &["add", "."]);
            run_git(repo, &["commit", "-m", "next"]);

            let second = record_base_ref().expect("second record should work");
            assert_eq!(second, first);
            let recorded = resolve_ref_sha(RECORDED_BASE_REF).expect("recorded ref should resolve");
            assert_eq!(recorded.as_deref(), Some(first.as_str()));
        });
    }

    #[test]
    fn auto_base_prefers_recorded_worktree_ref() {
        with_temp_git_repo(|repo| {
            let main_sha = resolve_ref_sha("HEAD")
                .expect("main sha query should work")
                .expect("head should resolve");
            run_git(repo, &["branch", "-M", "main"]);
            run_git(repo, &["branch", "origin/main", &main_sha]);
            let _ = record_base_ref().expect("record-base should succeed");

            fs::write(repo.join("next.txt"), "next\n").expect("next file should write");
            run_git(repo, &["add", "."]);
            run_git(repo, &["commit", "-m", "next"]);

            let discovered = discover_base_ref().expect("discovery should succeed");
            assert_eq!(discovered.as_deref(), Some(RECORDED_BASE_REF));
        });
    }
}
