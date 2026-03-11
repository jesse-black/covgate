use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "covgate", about = "Diff-focused coverage gate")]
pub struct Args {
    #[arg(long)]
    pub coverage_json: PathBuf,

    #[arg(long, conflicts_with = "diff_file")]
    pub base: Option<String>,

    #[arg(long, conflicts_with = "base")]
    pub diff_file: Option<PathBuf>,

    #[arg(long = "fail-under", value_name = "METRIC=PERCENT")]
    pub fail_under: String,

    #[arg(long)]
    pub markdown_output: Option<PathBuf>,
}
