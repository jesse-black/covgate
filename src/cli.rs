use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "covgate",
    about = "Diff-focused coverage gate",
    after_help = "Repository-local defaults may be read from ./covgate.toml.\nCLI flags override config values. Supported defaults in v1:\n  base = \"origin/main\"\n  [gates]\n  fail_under_regions = 90"
)]
pub struct Args {
    #[arg(long)]
    pub coverage_json: PathBuf,

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

    #[arg(long)]
    pub markdown_output: Option<PathBuf>,
}
