use std::fs;
use std::sync::Mutex;

use covgate::diff::{DiffSource, load_changed_lines};
use tempfile::tempdir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn make_executable(path: &std::path::Path) {
    let mut perms = fs::metadata(path)
        .expect("metadata should exist")
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
    }
    fs::set_permissions(path, perms).expect("permissions should be updated");
}

fn with_fake_git(script_body: &str, f: impl FnOnce()) {
    let temp = tempdir().expect("tempdir should exist");
    let fake_git = temp.path().join("git");
    fs::write(&fake_git, script_body).expect("fake git script should be written");
    make_executable(&fake_git);

    let original_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}:{}", temp.path().to_string_lossy(), original_path);
    let original_cwd = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(original_cwd);
    std::env::set_current_dir(temp.path()).expect("should chdir into temp");

    let original = std::env::var("PATH").ok();
    unsafe { std::env::set_var("PATH", &path) };
    f();
    match original {
        Some(value) => unsafe { std::env::set_var("PATH", value) },
        None => unsafe { std::env::remove_var("PATH") },
    }
}

#[test]
fn load_changed_lines_reports_merge_base_spawn_failure() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let original = std::env::var("PATH").ok();
    unsafe { std::env::set_var("PATH", "") };
    let err = load_changed_lines(&DiffSource::GitBase("main".to_string()))
        .expect_err("missing git should fail spawn");
    assert!(
        err.to_string().contains("failed to run git merge-base"),
        "err={err:#}"
    );
    match original {
        Some(value) => unsafe { std::env::set_var("PATH", value) },
        None => unsafe { std::env::remove_var("PATH") },
    }
}

#[test]
fn load_changed_lines_reports_merge_base_failure() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    with_fake_git(
        r#"#!/usr/bin/env bash
if [ "$1" = "merge-base" ]; then
  exit 2
fi
printf 'unexpected args: %s\n' "$*" >&2
exit 99
"#,
        || {
            let err = load_changed_lines(&DiffSource::GitBase("main".to_string()))
                .expect_err("merge-base failure should surface");
            assert!(
                err.to_string()
                    .contains("git merge-base failed with status"),
                "err={err:#}"
            );
        },
    );
}

#[test]
fn load_changed_lines_reports_non_utf8_merge_base_output() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    with_fake_git(
        r#"#!/usr/bin/env bash
if [ "$1" = "merge-base" ]; then
  printf '\377'
  exit 0
fi
printf 'unexpected args: %s\n' "$*" >&2
exit 99
"#,
        || {
            let err = load_changed_lines(&DiffSource::GitBase("main".to_string()))
                .expect_err("non-utf8 merge-base should surface");
            assert!(
                err.to_string()
                    .contains("git merge-base output was not valid utf-8"),
                "err={err:#}"
            );
        },
    );
}
