use clap::{Args as ClapArgs, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "covgate", about = "Diff-focused coverage gate")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run coverage gates against a coverage report.
    Check(Box<Args>),

    #[command(
        about = "Record a stable task-start base for constrained cloud-agent worktrees",
        long_about = r#"Record a stable task-start base for constrained cloud-agent worktrees.

Use this when a cloud agent or sandboxed worktree cannot rely on normal base
branches such as main or origin/main. Run it once at the start of a task before
making Git changes, then run `covgate check <coverage-report>` without
`--base`."#
    )]
    RecordBase,
}

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Coverage report path
    pub coverage_report: PathBuf,

    /// Git base reference to diff against, such as `origin/main`
    #[arg(long, conflicts_with = "diff_file")]
    pub base: Option<String>,

    /// Precomputed unified diff file to use instead of Git base discovery
    #[arg(long, conflicts_with = "base")]
    pub diff_file: Option<PathBuf>,

    /// Minimum changed-region coverage percentage required to pass
    #[arg(long = "fail-under-regions", value_name = "MIN")]
    pub fail_under_regions: Option<f64>,

    /// Minimum changed-line coverage percentage required to pass
    #[arg(long = "fail-under-lines", value_name = "MIN")]
    pub fail_under_lines: Option<f64>,

    /// Minimum changed-branch coverage percentage required to pass
    #[arg(long = "fail-under-branches", value_name = "MIN")]
    pub fail_under_branches: Option<f64>,

    /// Minimum changed-function coverage percentage required to pass
    #[arg(long = "fail-under-functions", value_name = "MIN")]
    pub fail_under_functions: Option<f64>,

    /// Maximum uncovered changed-region count allowed before failing
    #[arg(long = "fail-uncovered-regions", value_name = "MAX")]
    pub fail_uncovered_regions: Option<usize>,

    /// Maximum uncovered changed-line count allowed before failing
    #[arg(long = "fail-uncovered-lines", value_name = "MAX")]
    pub fail_uncovered_lines: Option<usize>,

    /// Maximum uncovered changed-branch count allowed before failing
    #[arg(long = "fail-uncovered-branches", value_name = "MAX")]
    pub fail_uncovered_branches: Option<usize>,

    /// Maximum uncovered changed-function count allowed before failing
    #[arg(long = "fail-uncovered-functions", value_name = "MAX")]
    pub fail_uncovered_functions: Option<usize>,

    /// Write a Markdown summary to this file
    #[arg(long)]
    pub markdown_output: Option<PathBuf>,
}
