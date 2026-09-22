//! The command line: what it takes, and what each command does.

mod args;
mod commands;
mod run;

pub use args::Cli;
pub use run::run;
