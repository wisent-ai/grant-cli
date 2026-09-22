//! What can be done with the catalogue an application is written against:
//! the sources it is imported from, the opportunities themselves, the
//! organization applying, and the eligibility rules it is held to.

use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum SourceCommand {
    Add {
        name: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        authority: String,
        #[arg(long)]
        config: Option<String>,
    },
    List,
    Snapshots {
        source: Option<String>,
    },
    Sync {
        source: Option<String>,
    },
    InstallCatalog,
}

#[derive(Subcommand)]
pub enum OpportunityCommand {
    Import(OpportunityArgs),
    List(SearchArgs),
    Search(SearchArgs),
    Watch {
        opportunity: String,
        #[arg(long)]
        label: Option<String>,
    },
    Changes {
        opportunity: String,
    },
}

#[derive(Args)]
pub struct OpportunityArgs {
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    external_id: Option<String>,
    #[arg(long)]
    title: String,
    #[arg(long)]
    summary: Option<String>,
    #[arg(long)]
    url: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    opens_at: Option<String>,
    #[arg(long)]
    deadline_at: Option<String>,
    #[arg(long)]
    funding_min: Option<f64>,
    #[arg(long)]
    funding_max: Option<f64>,
    #[arg(long)]
    currency: Option<String>,
    #[arg(long)]
    funding_rate: Option<f64>,
    #[arg(long, value_delimiter = ',')]
    regions: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    applicant_types: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    technologies: Vec<String>,
    #[arg(long)]
    trl_min: Option<f64>,
    #[arg(long)]
    trl_max: Option<f64>,
    #[arg(long)]
    consortium_required: Option<bool>,
    #[arg(long)]
    raw: Option<String>,
}

#[derive(Args)]
pub struct SearchArgs {
    pub query: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    region: Option<String>,
    #[arg(long)]
    technology: Option<String>,
    #[arg(long)]
    deadline_before: Option<String>,
    #[arg(long)]
    watched: bool,
}

#[derive(Subcommand)]
pub enum OrganizationCommand {
    Set {
        slug: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        profile: String,
    },
    Show {
        organization: String,
    },
    EvidenceAdd {
        organization: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        value: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        valid_from: Option<String>,
        #[arg(long)]
        valid_until: Option<String>,
        #[arg(long, default_value = "confirmed")]
        confidence: String,
    },
    EvidenceList {
        organization: String,
    },
}

#[derive(Subcommand)]
pub enum EligibilityCommand {
    RuleAdd {
        opportunity: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        expression: String,
        #[arg(long, default_value_t = true)]
        hard_gate: bool,
        #[arg(long)]
        citation: Option<String>,
    },
    Assess {
        opportunity: String,
        organization: String,
    },
}

