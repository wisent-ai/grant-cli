//! Running one command: opening the store, dispatching to the family that
//! owns the command, and printing what it answered.

use std::fs;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Value, json};

use crate::authoring::AuthoringService;
use crate::db::Database;
use crate::delivery::DeliveryService;
use crate::knowledge::KnowledgeService;
use crate::source::SourceService;

use super::args::{Cli, Command, ReviewCommand};
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
    let mut output = io::stdout().lock();
    if compact {
        serde_json::to_writer(&mut output, value)?;
        writeln!(output)?;
    } else {
        write_text(value, &mut String::new(), &mut output)?;
    }
    Ok(())
}

fn write_text<W: Write>(value: &Value, path: &mut String, output: &mut W) -> Result<()> {
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            for (key, field) in fields {
                let length = path.len();
                if length != 0 {
                    path.push('.');
                }
                path.push_str(key);
                write_text(field, path, output)?;
                path.truncate(length);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                let length = path.len();
                write!(path, "[{index}]")?;
                write_text(item, path, output)?;
                path.truncate(length);
            }
        }
        _ => {
            if !path.is_empty() {
                write!(output, "{path}: ")?;
            }
            serde_json::to_writer(&mut *output, value)?;
            writeln!(output)?;
        }
    }
    Ok(())
}
