use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = covgate::cli::Cli::parse();

    if matches!(cli.command, Some(covgate::cli::Command::RecordBase)) {
        covgate::git::record_base_ref()?;
        return Ok(());
    }

    let config = covgate::config::Config::try_from(cli.args)?;
    let code = covgate::run(config)?;
    std::process::exit(code);
}
