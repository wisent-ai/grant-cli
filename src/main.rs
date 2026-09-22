//! Grant: the opportunities an organization can apply for, the applications
//! written against them, and the store both live in.

mod catalog;
mod cli;
mod db;
mod model;
mod work;

/// The module paths every caller already uses. The two families above group
/// the modules by what they are; these keep `crate::source`, `crate::
/// application` and the rest resolving exactly as they did.
pub(crate) use catalog::{opportunity, organization, source};
pub(crate) use work::{application, authoring, delivery, document, knowledge};

use clap::Parser;

fn main() {
    if let Err(error) = cli::run(cli::Cli::parse()) {
        eprintln!("grant: {error:#}");
        std::process::exit(i32::from(true));
    }
}
