use clap::{Args as ClapArgs, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about)]
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

    /// Write a Markdown summary to this file, or `-` for stdout
    #[arg(long)]
    pub markdown_output: Option<PathBuf>,

    /// Disable automatic GitHub Actions step summary output
    #[arg(long)]
    pub no_github_summary: bool,
}
