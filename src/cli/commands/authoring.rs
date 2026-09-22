//! The commands over the text of an application: its fields and claims, the
//! budget under them, and the outcome finally recorded against it.

use anyhow::Result;
use serde_json::Value;

use crate::authoring::AuthoringService;
use crate::db::Database;
use crate::delivery::DeliveryService;

use super::super::args::{BudgetCommand, ClaimCommand, FieldCommand, OutcomeCommand};
use super::super::run::{json_arg, value};

pub(super) fn field_command(db: &Database, command: FieldCommand) -> Result<Value> {
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

pub(super) fn claim_command(db: &Database, command: ClaimCommand) -> Result<Value> {
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

pub(super) fn budget_command(db: &Database, command: BudgetCommand) -> Result<Value> {
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

pub(super) fn outcome_command(db: &Database, command: OutcomeCommand) -> Result<Value> {
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

