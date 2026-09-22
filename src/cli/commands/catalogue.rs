//! The catalogue commands: the sources an opportunity is imported from, the
//! opportunities themselves, the organization applying, and its eligibility.

use anyhow::Result;
use serde_json::Value;

use crate::db::Database;
use crate::model::OpportunityInput;
use crate::opportunity::OpportunityService;
use crate::organization::OrganizationService;
use crate::source::SourceService;

use super::super::args::{
    EligibilityCommand,
    OpportunityCommand,
    OrganizationCommand,
    SourceCommand,
};
use super::super::run::{json_arg, value};

pub(super) fn source_command(db: &Database, command: SourceCommand) -> Result<Value> {
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

pub(super) fn opportunity_command(db: &Database, command: OpportunityCommand) -> Result<Value> {
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

pub(super) fn organization_command(db: &Database, command: OrganizationCommand) -> Result<Value> {
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

pub(super) fn eligibility_command(db: &Database, command: EligibilityCommand) -> Result<Value> {
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

