pub mod cli;
pub mod config;
pub mod coverage;
pub mod diff;
pub mod gate;
pub mod git;
pub mod metrics;
pub mod model;
pub mod render;

use anyhow::Result;

use crate::{config::Config, diff::DiffSource};

pub fn run(config: Config) -> Result<i32> {
    emit_untracked_files_warning(&config)?;

    let report = coverage::parse_path(&config.coverage_report)?;
    let diff = diff::load_changed_lines(&config.diff_source)?;

    let mut metrics = Vec::new();
    let mut requested_metrics = config.rules.iter().map(|r| r.metric()).collect::<Vec<_>>();
    requested_metrics.sort();
    requested_metrics.dedup();

    let available_metrics = report
        .totals_by_file
        .iter()
        .filter(|(_, totals)| totals.values().any(|file_totals| file_totals.total > 0))
        .map(|(metric, _)| *metric)
        .filter(|metric| !requested_metrics.contains(metric))
        .collect::<Vec<_>>();

    requested_metrics.extend(available_metrics);

    for metric_kind in requested_metrics {
        let metric = metrics::compute_changed_metric(&report, &diff, metric_kind)?;
        metrics.push(metric);
    }

    let gate_result = gate::evaluate(metrics, &config.rules)?;

    let console = render::console::render(&gate_result, &config.diff_source.describe());
    println!("{console}");

    if let Some(path) = &config.markdown_output {
        let markdown = render::markdown::render(&gate_result, &config.diff_source.describe());
        std::fs::write(path, markdown)?;
    }

    Ok(if gate_result.passed { 0 } else { 1 })
}

fn emit_untracked_files_warning(config: &Config) -> Result<()> {
    let DiffSource::GitBase(_) = &config.diff_source else {
        return Ok(());
    };

    let untracked_files = git::list_untracked_files()?;
    if untracked_files.is_empty() {
        return Ok(());
    }

    let add_command = format_git_add_command(&untracked_files);
    eprintln!(
        "⚠️ Untracked-files warning: untracked files are not included in diff gating and can produce a false pass. Add them with: `{add_command}`."
    );

    Ok(())
}

fn format_git_add_command(paths: &[String]) -> String {
    let escaped_paths = paths
        .iter()
        .map(|path| {
            if path
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
            {
                path.clone()
            } else {
                format!("'{}'", path.replace('\'', "'\''"))
            }
        })
        .collect::<Vec<_>>();

    format!("git add -N {}", escaped_paths.join(" "))
}

#[cfg(test)]
pub mod test_support {
    use std::sync::Mutex;

    pub static CWD_LOCK: Mutex<()> = Mutex::new(());
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use tempfile::tempdir;

    use super::{emit_untracked_files_warning, format_git_add_command};
    use crate::{config::Config, diff::DiffSource, test_support::CWD_LOCK};

    struct CwdGuard(std::path::PathBuf);
    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }

    fn base_config(diff_source: DiffSource) -> Config {
        Config {
            coverage_report: "coverage.json".into(),
            diff_source,
            rules: Vec::new(),
            markdown_output: None,
        }
    }

    #[test]
    fn format_git_add_command_quotes_only_when_needed() {
        assert_eq!(
            format_git_add_command(&["new_untracked.rs".to_string(), "dir/extra.rs".to_string()]),
            "git add -N new_untracked.rs dir/extra.rs"
        );
        assert!(format_git_add_command(&["space name.rs".to_string()]).contains("'space name.rs'"));
    }

    #[test]
    fn untracked_warning_skips_diff_file_mode() {
        emit_untracked_files_warning(&base_config(DiffSource::DiffFile("scenario.diff".into())))
            .expect("diff-file mode should not query git");
    }

    #[test]
    fn untracked_warning_lists_git_base_untracked_paths() {
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
        fs::write("new_untracked.rs", "pub fn pending() {}\n").expect("file should write");

        emit_untracked_files_warning(&base_config(DiffSource::GitBase("HEAD".to_string())))
            .expect("git-base mode should warn successfully");
    }
}
