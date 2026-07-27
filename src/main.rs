mod application;
mod authoring;
mod cli;
mod db;
mod delivery;
mod document;
mod knowledge;
mod model;
mod opportunity;
mod organization;
mod source;

use clap::Parser;

fn main() {
    if let Err(error) = cli::run(cli::Cli::parse()) {
        eprintln!("grant: {error:#}");
        std::process::exit(i32::from(true));
    }
}
