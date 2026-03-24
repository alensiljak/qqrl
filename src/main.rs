use clap::Parser;

mod cli;
mod commands;
mod config;

use cli::Args;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        cli::Command::Balance(opts) => commands::balance::run(opts)?,
        cli::Command::Register(opts) => commands::register::run(opts)?,
        cli::Command::Query(opts) => commands::query::run(opts)?,
        cli::Command::Lots(opts) => commands::lots::run(opts)?,
        cli::Command::Assert(opts) => commands::assert::run(opts)?,
        cli::Command::Price(opts) => commands::price::run(opts)?,
    }

    Ok(())
}
