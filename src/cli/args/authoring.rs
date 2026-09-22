//! What can be done with the text of an application: the comments on it,
//! the fields and claims it is made of, its budget, the review it passes,
//! and the outcome finally recorded against it.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum CommentCommand {
    Add {
        application: String,
        #[arg(long)]
        field: Option<String>,
        #[arg(long = "type")]
        comment_type: String,
        #[arg(long)]
        severity: String,
        #[arg(long)]
        body: String,
        #[arg(long)]
        basis_kind: Option<String>,
        #[arg(long)]
        basis_ref: Option<String>,
        #[arg(long)]
        actions: Option<String>,
        #[arg(long)]
        owner: Option<String>,
    },
    List {
        application: String,
        #[arg(long)]
        status: Option<String>,
    },
    Resolve {
        comment: String,
        #[arg(long)]
        resolution: String,
    },
}

#[derive(Subcommand)]
pub enum FieldCommand {
    Add {
        application: String,
        #[arg(long)]
        code: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        instruction: Option<String>,
        #[arg(long)]
        char_limit: Option<usize>,
        #[arg(long)]
        metadata: Option<String>,
    },
    List {
        application: String,
    },
    Draft {
        application: String,
        field: String,
        #[arg(long)]
        value: String,
        #[arg(long, default_value = "draft")]
        status: String,
    },
    LinkRequirement {
        application: String,
        field: String,
        requirement: String,
    },
    LinkCriterion {
        application: String,
        field: String,
        criterion: String,
    },
    Lint {
        application: String,
    },
}

#[derive(Subcommand)]
pub enum ClaimCommand {
    Add {
        application: String,
        field: String,
        #[arg(long)]
        text: String,
    },
    List {
        application: String,
        #[arg(long)]
        field: Option<String>,
    },
    Link {
        claim: String,
        #[arg(long)]
        organization_evidence: Option<String>,
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        citation: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum BudgetCommand {
    Init {
        application: String,
        #[arg(long)]
        currency: String,
        #[arg(long)]
        indirect_method: Option<String>,
        #[arg(long)]
        indirect_rate: Option<f64>,
        #[arg(long)]
        private_financing: Option<String>,
    },
    LineAdd {
        application: String,
        #[arg(long)]
        task_code: Option<String>,
        #[arg(long)]
        category: String,
        #[arg(long)]
        research_type: Option<String>,
        #[arg(long)]
        description: String,
        #[arg(long)]
        quantity: Option<f64>,
        #[arg(long)]
        unit: Option<String>,
        #[arg(long)]
        unit_cost: Option<f64>,
        #[arg(long)]
        eligible_cost: f64,
        #[arg(long)]
        aid_rate: Option<f64>,
        #[arg(long)]
        requested_funding: Option<f64>,
        #[arg(long)]
        source_ref: Option<String>,
        #[arg(long)]
        metadata: Option<String>,
    },
    Check {
        application: String,
    },
}

#[derive(Subcommand)]
pub enum ReviewCommand {
    Run { application: String },
}

#[derive(Subcommand)]
pub enum OutcomeCommand {
    Record {
        application: String,
        result: String,
        #[arg(long)]
        decided_at: Option<String>,
        #[arg(long)]
        awarded_amount: Option<f64>,
        #[arg(long)]
        score: Option<f64>,
        #[arg(long)]
        feedback_document: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    IngestFeedback {
        application: String,
        document: String,
    },
    Lessons {
        application: String,
    },
}

