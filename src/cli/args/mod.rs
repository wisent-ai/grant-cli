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
    about = "Local-first grant discovery, qualification, authoring and tracking"
)]
pub struct Cli {
    #[arg(long, global = true, env = "GRANT_HOME")]
    pub home: Option<PathBuf>,
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Init,
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    Opportunity {
        #[command(subcommand)]
        command: OpportunityCommand,
    },
    Organization {
        #[command(subcommand)]
        command: OrganizationCommand,
    },
    Eligibility {
        #[command(subcommand)]
        command: EligibilityCommand,
    },
    Application {
        #[command(subcommand)]
        command: ApplicationCommand,
    },
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    Guide {
        #[command(subcommand)]
        command: GuideCommand,
    },
    Pattern {
        #[command(subcommand)]
        command: PatternCommand,
    },
    Comment {
        #[command(subcommand)]
        command: CommentCommand,
    },
    Field {
        #[command(subcommand)]
        command: FieldCommand,
    },
    Claim {
        #[command(subcommand)]
        command: ClaimCommand,
    },
    Budget {
        #[command(subcommand)]
        command: BudgetCommand,
    },
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    Outcome {
        #[command(subcommand)]
        command: OutcomeCommand,
    },
    Analytics,
    Export {
        application: String,
        #[arg(long)]
        output: Option<String>,
    },
}

