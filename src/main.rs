use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = covgate::cli::Cli::parse();

    match cli.command {
        covgate::cli::Command::RecordBase => {
            covgate::git::record_base_ref()?;
            Ok(())
        }
        covgate::cli::Command::Check(args) => {
            let config = covgate::config::Config::try_from(*args)?;
            let code = covgate::run(config)?;
            std::process::exit(code);
        }
    }
}
