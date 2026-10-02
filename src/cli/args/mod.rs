use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::{Value, json};

use crate::application::ApplicationService;
use crate::authoring::AuthoringService;
use crate::db::Database;
use crate::delivery::DeliveryService;
use crate::document::{DocumentService, IngestOptions};
use crate::knowledge::{CriterionInput, KnowledgeService, RequirementInput};
use crate::model::OpportunityInput;
use crate::opportunity::OpportunityService;
use crate::organization::OrganizationService;
use crate::source::SourceService;


mod authoring;
mod catalogue;
mod work;

pub use authoring::*;
pub use catalogue::*;
pub use work::*;

#[derive(Parser)]
#[command(
    name = "grant",
    version,
    about = "Grant discovery, qualification, authoring and tracking on the Wisent fleet database",
    after_help = "Results print as path: value text, or with --json as one compact JSON line. Exit 2: the invocation is wrong; exit 1: the command was refused or failed, with the reason on stderr."
)]
pub struct Cli {
    /// Directory for fetched source objects and exports.
    #[arg(long, global = true, env = "GRANT_HOME")]
    pub home: Option<PathBuf>,
    /// Print the result as one compact JSON line.
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install the official sources and the shipped writing patterns.
    Init,
    /// The feeds and pages opportunities are read from.
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    /// Funding opportunities: import, search, watch and their changes.
    Opportunity {
        #[command(subcommand)]
        command: OpportunityCommand,
    },
    /// The organization applying and the evidence about it.
    Organization {
        #[command(subcommand)]
        command: OrganizationCommand,
    },
    /// Eligibility rules of an opportunity and the fit assessment.
    Eligibility {
        #[command(subcommand)]
        command: EligibilityCommand,
    },
    /// Applications and the stage each one is in.
    Application {
        #[command(subcommand)]
        command: ApplicationCommand,
    },
    /// The tasks an application needs done.
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    /// Documents read into an application.
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    /// Requirements and criteria read out of a call's guide.
    Guide {
        #[command(subcommand)]
        command: GuideCommand,
    },
    /// Writing patterns and the worked examples behind them.
    Pattern {
        #[command(subcommand)]
        command: PatternCommand,
    },
    /// Comments on an application: add, list, resolve, reopen.
    Comment {
        #[command(subcommand)]
        command: CommentCommand,
    },
    /// The fields an application form is made of.
    Field {
        #[command(subcommand)]
        command: FieldCommand,
    },
    /// Claims written into fields and the evidence behind them.
    Claim {
        #[command(subcommand)]
        command: ClaimCommand,
    },
    /// The budget of an application and its lines.
    Budget {
        #[command(subcommand)]
        command: BudgetCommand,
    },
    /// Review an application before submission.
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    /// The outcome of a submitted application and its lessons.
    Outcome {
        #[command(subcommand)]
        command: OutcomeCommand,
    },
    /// Counts of applications, submissions and outcomes, and the funding requested.
    Analytics,
    /// Write an application's submission package.
    Export {
        application: String,
        /// Where the package is written; `<application>.json` in the GRANT_HOME exports directory when omitted.
        #[arg(long)]
        output: Option<String>,
    },
}

