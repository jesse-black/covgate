use clap::{Args as ClapArgs, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "covgate",
    about = "Diff-focused coverage gate",
    after_help = "Repository-local defaults may be read from ./covgate.toml.\nCLI flags override config values. Supported defaults in v1:\n  base = \"origin/main\"\n  [gates]\n  fail_under_regions = 90\n  fail_uncovered_regions = 1\n\nAgent workflow:\n  covgate record-base\n  covgate check <coverage-report>"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run coverage gates against a coverage report.
    Check(Box<Args>),

    /// Record HEAD into refs/worktree/covgate/base when it is not already set.
    RecordBase,
}

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Coverage report path (LLVM/Coverlet/Istanbul auto-detected)
    pub coverage_report: PathBuf,

    #[arg(long, conflicts_with = "diff_file")]
    pub base: Option<String>,

    #[arg(long, conflicts_with = "base")]
    pub diff_file: Option<PathBuf>,

    #[arg(long = "fail-under-regions", value_name = "MIN")]
    pub fail_under_regions: Option<f64>,

    #[arg(long = "fail-under-lines", value_name = "MIN")]
    pub fail_under_lines: Option<f64>,

    #[arg(long = "fail-under-branches", value_name = "MIN")]
    pub fail_under_branches: Option<f64>,

    #[arg(long = "fail-under-functions", value_name = "MIN")]
    pub fail_under_functions: Option<f64>,

    #[arg(long = "fail-uncovered-regions", value_name = "MAX")]
    pub fail_uncovered_regions: Option<usize>,

    #[arg(long = "fail-uncovered-lines", value_name = "MAX")]
    pub fail_uncovered_lines: Option<usize>,

    #[arg(long = "fail-uncovered-branches", value_name = "MAX")]
    pub fail_uncovered_branches: Option<usize>,

    #[arg(long = "fail-uncovered-functions", value_name = "MAX")]
    pub fail_uncovered_functions: Option<usize>,

    #[arg(long)]
    pub markdown_output: Option<PathBuf>,
}
