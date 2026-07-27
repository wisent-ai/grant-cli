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

pub fn run(cli: Cli) -> Result<()> {
    let db = Database::open(cli.home.as_deref())?;
    let value = execute(&db, cli.command)?;
    print_value(&value, cli.json)?;
    Ok(())
}

fn execute(db: &Database, command: Command) -> Result<Value> {
    match command {
        Command::Init => {
            let sources = SourceService::new(db)?.install_catalog()?;
            let patterns = KnowledgeService::new(db).install_patterns()?;
            Ok(json!({ "home": db.home, "sources": sources, "patterns": patterns }))
        }
        Command::Source { command } => source_command(db, command),
        Command::Opportunity { command } => opportunity_command(db, command),
        Command::Organization { command } => organization_command(db, command),
        Command::Eligibility { command } => eligibility_command(db, command),
        Command::Application { command } => application_command(db, command),
        Command::Task { command } => task_command(db, command),
        Command::Document { command } => document_command(db, command),
        Command::Guide { command } => guide_command(db, command),
        Command::Pattern { command } => pattern_command(db, command),
        Command::Comment { command } => comment_command(db, command),
        Command::Field { command } => field_command(db, command),
        Command::Claim { command } => claim_command(db, command),
        Command::Budget { command } => budget_command(db, command),
        Command::Review {
            command: ReviewCommand::Run { application },
        } => value(AuthoringService::new(db).review(&application)?),
        Command::Outcome { command } => outcome_command(db, command),
        Command::Analytics => value(DeliveryService::new(db).analytics()?),
        Command::Export {
            application,
            output,
        } => AuthoringService::new(db).export(&application, output.as_deref()),
    }
}

fn source_command(db: &Database, command: SourceCommand) -> Result<Value> {
    let service = SourceService::new(db)?;
    match command {
        SourceCommand::Add {
            name,
            kind,
            url,
            authority,
            config,
        } => {
            value(service.register(&name, &kind, &url, &authority, json_arg(config.as_deref())?)?)
        }
        SourceCommand::List => value(service.list()?),
        SourceCommand::Snapshots { source } => value(service.snapshots(source.as_deref())?),
        SourceCommand::InstallCatalog => value(service.install_catalog()?),
        SourceCommand::Sync { source } => {
            let targets = match source {
                Some(value) => vec![value],
                None => service
                    .list()?
                    .into_iter()
                    .filter(|entry| entry.enabled)
                    .map(|entry| entry.id)
                    .collect(),
            };
            let mut reports = Vec::new();
            for target in targets {
                reports.push(service.sync(&target)?);
            }
            value(reports)
        }
    }
}

fn opportunity_command(db: &Database, command: OpportunityCommand) -> Result<Value> {
    let service = OpportunityService::new(db);
    match command {
        OpportunityCommand::Import(args) => {
            let input = OpportunityInput {
                external_id: args.external_id,
                title: args.title,
                summary: args.summary,
                url: args.url,
                status: args.status,
                opens_at: args.opens_at,
                deadline_at: args.deadline_at,
                funding_min: args.funding_min,
                funding_max: args.funding_max,
                currency: args.currency,
                funding_rate: args.funding_rate,
                regions: args.regions,
                applicant_types: args.applicant_types,
                technologies: args.technologies,
                trl_min: args.trl_min,
                trl_max: args.trl_max,
                consortium_required: args.consortium_required,
                raw: json_arg(args.raw.as_deref())?,
            };
            value(service.import(args.source.as_deref(), input)?)
        }
        OpportunityCommand::List(args) | OpportunityCommand::Search(args) => {
            value(service.search(
                args.query.as_deref(),
                args.status.as_deref(),
                args.region.as_deref(),
                args.technology.as_deref(),
                args.deadline_before.as_deref(),
                args.watched,
            )?)
        }
        OpportunityCommand::Watch { opportunity, label } => {
            service.watch(&opportunity, label.as_deref())
        }
        OpportunityCommand::Changes { opportunity } => value(service.changes(&opportunity)?),
    }
}

fn organization_command(db: &Database, command: OrganizationCommand) -> Result<Value> {
    let service = OrganizationService::new(db);
    match command {
        OrganizationCommand::Set {
            slug,
            name,
            profile,
        } => value(service.upsert(&slug, &name, json_arg(Some(&profile))?)?),
        OrganizationCommand::Show { organization } => value(service.get(&organization)?),
        OrganizationCommand::EvidenceAdd {
            organization,
            kind,
            title,
            value: raw,
            source,
            valid_from,
            valid_until,
            confidence,
        } => value(service.evidence_add(
            &organization,
            &kind,
            &title,
            json_arg(Some(&raw))?,
            source.as_deref(),
            valid_from.as_deref(),
            valid_until.as_deref(),
            &confidence,
        )?),
        OrganizationCommand::EvidenceList { organization } => {
            value(service.evidence_list(&organization)?)
        }
    }
}

fn eligibility_command(db: &Database, command: EligibilityCommand) -> Result<Value> {
    let service = OrganizationService::new(db);
    match command {
        EligibilityCommand::RuleAdd {
            opportunity,
            name,
            expression,
            hard_gate,
            citation,
        } => value(service.rule_add(
            &opportunity,
            &name,
            json_arg(Some(&expression))?,
            hard_gate,
            citation.as_deref(),
        )?),
        EligibilityCommand::Assess {
            opportunity,
            organization,
        } => value(service.assess(&opportunity, &organization)?),
    }
}

fn application_command(db: &Database, command: ApplicationCommand) -> Result<Value> {
    let service = ApplicationService::new(db);
    match command {
        ApplicationCommand::Init {
            opportunity,
            organization,
            name,
            owner,
            internal_deadline,
        } => value(service.create(
            &opportunity,
            &organization,
            &name,
            owner.as_deref(),
            internal_deadline.as_deref(),
        )?),
        ApplicationCommand::Show { application } => value(service.get(&application)?),
        ApplicationCommand::List { stage } => value(service.list(stage.as_deref())?),
        ApplicationCommand::Stage {
            application,
            stage,
            submission_reference,
        } => value(service.set_stage(&application, &stage, submission_reference.as_deref())?),
        ApplicationCommand::Dashboard => service.dashboard(),
    }
}

fn task_command(db: &Database, command: TaskCommand) -> Result<Value> {
    let service = ApplicationService::new(db);
    match command {
        TaskCommand::Add {
            application,
            title,
            description,
            owner,
            due_at,
            depends_on,
        } => value(service.task_add(
            &application,
            &title,
            description.as_deref(),
            owner.as_deref(),
            due_at.as_deref(),
            depends_on.as_deref(),
        )?),
        TaskCommand::List {
            application,
            status,
            overdue,
        } => value(service.task_list(&application, status.as_deref(), overdue)?),
        TaskCommand::Complete { task } => value(service.task_complete(&task)?),
    }
}

fn document_command(db: &Database, command: DocumentCommand) -> Result<Value> {
    let service = DocumentService::new(db)?;
    match command {
        DocumentCommand::Ingest {
            target,
            title,
            kind,
            authority,
            application,
            opportunity,
            organization,
            version,
            effective_at,
        } => value(service.ingest(IngestOptions {
            target,
            title,
            kind,
            authority,
            application_id: application,
            opportunity_id: opportunity,
            organization_id: organization,
            version_label: version,
            effective_at,
        })?),
        DocumentCommand::List {
            application,
            authority,
        } => value(service.list(application.as_deref(), authority.as_deref())?),
        DocumentCommand::Text { document } => Ok(Value::String(service.text(&document)?)),
    }
}

fn guide_command(db: &Database, command: GuideCommand) -> Result<Value> {
    let service = KnowledgeService::new(db);
    match command {
        GuideCommand::Extract {
            application,
            document,
        } => value(service.extract_guide(&application, &document)?),
        GuideCommand::RequirementsImport {
            application,
            input,
            document,
        } => {
            let inputs: Vec<RequirementInput> = serde_json::from_value(json_arg(Some(&input))?)?;
            value(service.import_requirements(&application, document.as_deref(), inputs)?)
        }
        GuideCommand::CriteriaImport {
            application,
            input,
            document,
        } => {
            let inputs: Vec<CriterionInput> = serde_json::from_value(json_arg(Some(&input))?)?;
            value(service.import_criteria(&application, document.as_deref(), inputs)?)
        }
        GuideCommand::Requirements { application } => value(service.requirements(&application)?),
        GuideCommand::Criteria { application } => value(service.criteria(&application)?),
    }
}

fn pattern_command(db: &Database, command: PatternCommand) -> Result<Value> {
    let service = KnowledgeService::new(db);
    match command {
        PatternCommand::Install => value(service.install_patterns()?),
        PatternCommand::List { category } => value(service.pattern_list(category.as_deref())?),
        PatternCommand::Add {
            name,
            category,
            authority,
            structure,
            rationale,
            scope,
            required_inputs,
            anti_patterns,
            source_refs,
            confidence,
        } => value(service.pattern_add(
            &name,
            &category,
            &authority,
            json_arg(Some(&structure))?,
            &rationale,
            json_arg(scope.as_deref())?,
            json_arg(required_inputs.as_deref())?,
            json_arg(anti_patterns.as_deref())?,
            json_arg(source_refs.as_deref())?,
            &confidence,
        )?),
        PatternCommand::ExampleAdd {
            pattern,
            application,
            field_code,
            outcome,
            text,
            evaluator_comment,
            explanation,
            source_ref,
        } => service.example_add(
            pattern.as_deref(),
            application.as_deref(),
            field_code.as_deref(),
            &outcome,
            &text,
            evaluator_comment.as_deref(),
            explanation.as_deref(),
            source_ref.as_deref(),
        ),
        PatternCommand::ExampleList { pattern, outcome } => {
            value(service.example_list(pattern.as_deref(), outcome.as_deref())?)
        }
    }
}

fn comment_command(db: &Database, command: CommentCommand) -> Result<Value> {
    let service = KnowledgeService::new(db);
    match command {
        CommentCommand::Add {
            application,
            field,
            comment_type,
            severity,
            body,
            basis_kind,
            basis_ref,
            actions,
            owner,
        } => value(service.comment_add(
            &application,
            field.as_deref(),
            &comment_type,
            &severity,
            &body,
            basis_kind.as_deref(),
            basis_ref.as_deref(),
            json_arg(actions.as_deref())?,
            owner.as_deref(),
        )?),
        CommentCommand::List {
            application,
            status,
        } => value(service.comment_list(&application, status.as_deref())?),
        CommentCommand::Resolve {
            comment,
            resolution,
        } => value(service.comment_resolve(&comment, &resolution)?),
    }
}

fn field_command(db: &Database, command: FieldCommand) -> Result<Value> {
    let service = AuthoringService::new(db);
    match command {
        FieldCommand::Add {
            application,
            code,
            title,
            instruction,
            char_limit,
            metadata,
        } => value(service.field_add(
            &application,
            &code,
            &title,
            instruction.as_deref(),
            char_limit,
            json_arg(metadata.as_deref())?,
        )?),
        FieldCommand::List { application } => value(service.field_list(&application)?),
        FieldCommand::Draft {
            application,
            field,
            value: text,
            status,
        } => value(service.field_draft(&application, &field, &text, &status)?),
        FieldCommand::LinkRequirement {
            application,
            field,
            requirement,
        } => service.field_link_requirement(&application, &field, &requirement),
        FieldCommand::LinkCriterion {
            application,
            field,
            criterion,
        } => service.field_link_criterion(&application, &field, &criterion),
        FieldCommand::Lint { application } => value(service.lint(&application)?),
    }
}

fn claim_command(db: &Database, command: ClaimCommand) -> Result<Value> {
    let service = AuthoringService::new(db);
    match command {
        ClaimCommand::Add {
            application,
            field,
            text,
        } => value(service.claim_add(&application, &field, &text)?),
        ClaimCommand::List { application, field } => {
            value(service.claim_list(&application, field.as_deref())?)
        }
        ClaimCommand::Link {
            claim,
            organization_evidence,
            document,
            citation,
            note,
        } => service.claim_link(
            &claim,
            organization_evidence.as_deref(),
            document.as_deref(),
            citation.as_deref(),
            note.as_deref(),
        ),
    }
}

fn budget_command(db: &Database, command: BudgetCommand) -> Result<Value> {
    let service = AuthoringService::new(db);
    match command {
        BudgetCommand::Init {
            application,
            currency,
            indirect_method,
            indirect_rate,
            private_financing,
        } => service.budget_init(
            &application,
            &currency,
            indirect_method.as_deref(),
            indirect_rate,
            json_arg(private_financing.as_deref())?,
        ),
        BudgetCommand::LineAdd {
            application,
            task_code,
            category,
            research_type,
            description,
            quantity,
            unit,
            unit_cost,
            eligible_cost,
            aid_rate,
            requested_funding,
            source_ref,
            metadata,
        } => value(service.budget_line_add(
            &application,
            task_code.as_deref(),
            &category,
            research_type.as_deref(),
            &description,
            quantity,
            unit.as_deref(),
            unit_cost,
            eligible_cost,
            aid_rate,
            requested_funding,
            source_ref.as_deref(),
            json_arg(metadata.as_deref())?,
        )?),
        BudgetCommand::Check { application } => value(service.budget_check(&application)?),
    }
}

fn outcome_command(db: &Database, command: OutcomeCommand) -> Result<Value> {
    let service = DeliveryService::new(db);
    match command {
        OutcomeCommand::Record {
            application,
            result,
            decided_at,
            awarded_amount,
            score,
            feedback_document,
            notes,
        } => value(service.record_outcome(
            &application,
            &result,
            decided_at.as_deref(),
            awarded_amount,
            score,
            feedback_document.as_deref(),
            notes.as_deref(),
        )?),
        OutcomeCommand::IngestFeedback {
            application,
            document,
        } => service.ingest_feedback(&application, &document),
        OutcomeCommand::Lessons { application } => service.lessons(&application),
    }
}

fn json_arg(input: Option<&str>) -> Result<Value> {
    let Some(input) = input else {
        return Ok(json!({}));
    };
    let content = match input.strip_prefix('@') {
        Some(path) => {
            fs::read_to_string(path).with_context(|| format!("cannot read JSON from {path}"))?
        }
        None => input.to_owned(),
    };
    serde_json::from_str(&content).with_context(|| "invalid JSON")
}

fn value<T: Serialize>(input: T) -> Result<Value> {
    Ok(serde_json::to_value(input)?)
}
fn print_value(value: &Value, compact: bool) -> Result<()> {
    if compact {
        println!("{}", serde_json::to_string(value)?);
    } else if let Value::String(text) = value {
        println!("{text}");
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}
