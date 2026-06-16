//! `loon` CLI binary entry point.

use clap::Parser;

use loon::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    loon::run(cli)
}
