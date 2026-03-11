use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = covgate::cli::Args::parse();
    let config = covgate::config::Config::try_from(args)?;
    let code = covgate::run(config)?;
    std::process::exit(code);
}
