//! Reading one SQLite row back as the thing it stands for, and the small
//! text rules a requirement and a slug are derived with.

use std::fs;

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Row, params};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{Comment, Criterion, Pattern, Requirement};


pub(super) fn default_true() -> bool {
    true
}
pub(super) fn first_line(value: &str) -> String {
    value
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Extracted item")
        .trim()
        .to_owned()
}
pub(super) fn classify_requirement(value: &str) -> &'static str {
    if value.contains("limit") || value.contains("znak") {
        "field-limit"
    } else if value.contains("załącz") || value.contains("attachment") {
        "attachment"
    } else if value.contains("deadline") || value.contains("termin") {
        "deadline"
    } else {
        "instruction"
    }
}
pub(super) fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
        .collect()
}
pub(super) fn slugify(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub(super) fn requirement_from_row(row: &Row<'_>) -> rusqlite::Result<Requirement> {
    let metadata: String = row.get("metadata_json")?;
    Ok(Requirement {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        document_id: row.get("document_id")?,
        authority: row.get("authority")?,
        kind: row.get("kind")?,
        code: row.get("code")?,
        title: row.get("title")?,
        text: row.get("text")?,
        citation: row.get("citation")?,
        mandatory: row.get("mandatory")?,
        metadata: serde_json::from_str(&metadata).unwrap_or(Value::Null),
        created_at: row.get("created_at")?,
    })
}
pub(super) fn criterion_from_row(row: &Row<'_>) -> rusqlite::Result<Criterion> {
    Ok(Criterion {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        document_id: row.get("document_id")?,
        code: row.get("code")?,
        title: row.get("title")?,
        text: row.get("text")?,
        gate: row.get("gate")?,
        weight: row.get("weight")?,
        citation: row.get("citation")?,
        created_at: row.get("created_at")?,
    })
}
pub(super) fn pattern_from_row(row: &Row<'_>) -> rusqlite::Result<Pattern> {
    let scope: String = row.get("scope_json")?;
    let structure: String = row.get("structure_json")?;
    let required_inputs: String = row.get("required_inputs_json")?;
    let anti_patterns: String = row.get("anti_patterns_json")?;
    let source_refs: String = row.get("source_refs_json")?;
    Ok(Pattern {
        id: row.get("id")?,
        slug: row.get("slug")?,
        name: row.get("name")?,
        category: row.get("category")?,
        authority: row.get("authority")?,
        status: row.get("status")?,
        scope: serde_json::from_str(&scope).unwrap_or(Value::Null),
        structure: serde_json::from_str(&structure).unwrap_or(Value::Null),
        rationale: row.get("rationale")?,
        required_inputs: serde_json::from_str(&required_inputs).unwrap_or(Value::Null),
        anti_patterns: serde_json::from_str(&anti_patterns).unwrap_or(Value::Null),
        source_refs: serde_json::from_str(&source_refs).unwrap_or(Value::Null),
        confidence: row.get("confidence")?,
        reviewed_at: row.get("reviewed_at")?,
        created_at: row.get("created_at")?,
    })
}
pub(super) fn comment_from_row(row: &Row<'_>) -> rusqlite::Result<Comment> {
    let actions: String = row.get("suggested_actions_json")?;
    Ok(Comment {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        field_id: row.get("field_id")?,
        comment_type: row.get("type")?,
        severity: row.get("severity")?,
        body: row.get("body")?,
        basis_kind: row.get("basis_kind")?,
        basis_ref: row.get("basis_ref")?,
        suggested_actions: serde_json::from_str(&actions).unwrap_or(Value::Null),
        status: row.get("status")?,
        owner: row.get("owner")?,
        resolution: row.get("resolution")?,
        created_at: row.get("created_at")?,
        resolved_at: row.get("resolved_at")?,
    })
}
