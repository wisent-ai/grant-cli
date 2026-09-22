//! The commands over an application in flight: starting one, its tasks, the
//! documents it reads, and the guidance, patterns and comments read out of
//! them.

use anyhow::Result;
use serde_json::Value;

use crate::application::ApplicationService;
use crate::db::Database;
use crate::document::{DocumentService, IngestOptions};
use crate::knowledge::{CriterionInput, KnowledgeService, RequirementInput};

use super::super::args::{
    ApplicationCommand,
    CommentCommand,
    DocumentCommand,
    GuideCommand,
    PatternCommand,
    TaskCommand,
};
use super::super::run::{json_arg, value};

pub(super) fn application_command(db: &Database, command: ApplicationCommand) -> Result<Value> {
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

pub(super) fn task_command(db: &Database, command: TaskCommand) -> Result<Value> {
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

pub(super) fn document_command(db: &Database, command: DocumentCommand) -> Result<Value> {
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

pub(super) fn guide_command(db: &Database, command: GuideCommand) -> Result<Value> {
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

pub(super) fn pattern_command(db: &Database, command: PatternCommand) -> Result<Value> {
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

pub(super) fn comment_command(db: &Database, command: CommentCommand) -> Result<Value> {
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

