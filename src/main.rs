use clap::Parser;

fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("record-base") {
        covgate::git::record_base_ref()?;
        return Ok(());
    }

    let args = covgate::cli::Args::parse();
    let config = covgate::config::Config::try_from(args)?;
    let code = covgate::run(config)?;
    std::process::exit(code);
}
