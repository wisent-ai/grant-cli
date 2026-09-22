//! Running one command: opening the store, dispatching to the family that
//! owns the command, and printing what it answered.

use anyhow::Result;
use serde::Serialize;
use serde_json::{Value, json};

use crate::db::Database;
use crate::source::SourceService;

use super::args::{Cli, Command};
use super::commands::{
    application_command,
    budget_command,
    claim_command,
    comment_command,
    document_command,
    eligibility_command,
    field_command,
    guide_command,
    opportunity_command,
    organization_command,
    outcome_command,
    pattern_command,
    source_command,
    task_command,
};

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

pub(super) fn json_arg(input: Option<&str>) -> Result<Value> {
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

pub(super) fn value<T: Serialize>(input: T) -> Result<Value> {
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
