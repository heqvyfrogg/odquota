//! odquota: Open Design のアカウントクオータを JSON で返すネイティブ CLI。
//!
//! Two jobs: keep credentials encrypted on disk, and summarize the billing API
//! for every stored account in one JSON document.

use clap::Parser;
use odquota::{app, cli};

fn main() {
    let cli = cli::Cli::parse();
    let code = match app::run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("odquota: {}", err.message());
            err.exit_code()
        }
    };
    std::process::exit(code);
}
