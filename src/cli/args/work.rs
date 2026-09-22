//! What can be done with an application in flight: starting one, the tasks
//! under it, the documents it reads, and the guidance and patterns read out
//! of those documents.

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum ApplicationCommand {
    Init {
        opportunity: String,
        organization: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        internal_deadline: Option<String>,
    },
    Show {
        application: String,
    },
    List {
        #[arg(long)]
        stage: Option<String>,
    },
    Stage {
        application: String,
        stage: String,
        #[arg(long)]
        submission_reference: Option<String>,
    },
    Dashboard,
}

#[derive(Subcommand)]
pub enum TaskCommand {
    Add {
        application: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        due_at: Option<String>,
        #[arg(long)]
        depends_on: Option<String>,
    },
    List {
        application: String,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        overdue: bool,
    },
    Complete {
        task: String,
    },
}

#[derive(Subcommand)]
pub enum DocumentCommand {
    Ingest {
        target: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        kind: String,
        #[arg(long, default_value = "working")]
        authority: String,
        #[arg(long)]
        application: Option<String>,
        #[arg(long)]
        opportunity: Option<String>,
        #[arg(long)]
        organization: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        effective_at: Option<String>,
    },
    List {
        #[arg(long)]
        application: Option<String>,
        #[arg(long)]
        authority: Option<String>,
    },
    Text {
        document: String,
    },
}

#[derive(Subcommand)]
pub enum GuideCommand {
    Extract {
        application: String,
        document: String,
    },
    RequirementsImport {
        application: String,
        input: String,
        #[arg(long)]
        document: Option<String>,
    },
    CriteriaImport {
        application: String,
        input: String,
        #[arg(long)]
        document: Option<String>,
    },
    Requirements {
        application: String,
    },
    Criteria {
        application: String,
    },
}

#[derive(Subcommand)]
pub enum PatternCommand {
    Install,
    List {
        #[arg(long)]
        category: Option<String>,
    },
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        category: String,
        #[arg(long)]
        authority: String,
        #[arg(long)]
        structure: String,
        #[arg(long)]
        rationale: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        required_inputs: Option<String>,
        #[arg(long)]
        anti_patterns: Option<String>,
        #[arg(long)]
        source_refs: Option<String>,
        #[arg(long, default_value = "medium")]
        confidence: String,
    },
    ExampleAdd {
        #[arg(long)]
        pattern: Option<String>,
        #[arg(long)]
        application: Option<String>,
        #[arg(long)]
        field_code: Option<String>,
        #[arg(long)]
        outcome: String,
        #[arg(long)]
        text: String,
        #[arg(long)]
        evaluator_comment: Option<String>,
        #[arg(long)]
        explanation: Option<String>,
        #[arg(long)]
        source_ref: Option<String>,
    },
    ExampleList {
        #[arg(long)]
        pattern: Option<String>,
        #[arg(long)]
        outcome: Option<String>,
    },
}

