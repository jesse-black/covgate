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

#[test]
fn load_changed_lines_uses_git_base_helpers() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let fake_git = temp.path().join("git");
    fs::write(
        &fake_git,
        r#"#!/usr/bin/env bash
if [ "$1" = "merge-base" ]; then
  printf 'abc123\n'
  exit 0
fi
if [ "$1" = "diff" ] && [ "$4" = "abc123" ]; then
  printf 'diff --git a/src/lib.rs b/src/lib.rs\n'
  printf '+++ b/src/lib.rs\n'
  printf '@@ -1,0 +2,2 @@\n'
  exit 0
fi
printf 'unexpected args: %s\n' "$*" >&2
exit 99
"#,
    )
    .expect("fake git script should be written");
    make_executable(&fake_git);

    let original_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}:{}", temp.path().to_string_lossy(), original_path);
    let original_cwd = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(original_cwd);
    std::env::set_current_dir(temp.path()).expect("should chdir into temp");

    let original = std::env::var("PATH").ok();
    unsafe { std::env::set_var("PATH", &path) };
    let changed =
        load_changed_lines(&DiffSource::GitBase("main".to_string())).expect("diff should load");
    match original {
        Some(value) => unsafe { std::env::set_var("PATH", value) },
        None => unsafe { std::env::remove_var("PATH") },
    }

    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].path, std::path::PathBuf::from("src/lib.rs"));
    assert_eq!(changed[0].changed_lines[0].start, 2);
    assert_eq!(changed[0].changed_lines[0].end, 3);
}
