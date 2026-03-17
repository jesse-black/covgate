use clap::{CommandFactory, Parser, error::ErrorKind};

fn main() -> anyhow::Result<()> {
    let cli = covgate::cli::Cli::parse();

    if matches!(cli.command, Some(covgate::cli::Command::RecordBase)) {
        covgate::git::record_base_ref()?;
        return Ok(());
    }

    if cli.args.coverage_json.is_none() {
        covgate::cli::Cli::command()
            .error(
                ErrorKind::MissingRequiredArgument,
                "the following required arguments were not provided:
  --coverage-json <COVERAGE_JSON>",
            )
            .exit();
    }

    let config = covgate::config::Config::try_from(cli.args)?;
    let code = covgate::run(config)?;
    std::process::exit(code);
}
