//! Reading one SQLite row back as the thing it stands for, and the finding
//! shape every check reports in.

use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{ApplicationField, BudgetLine, Claim, Finding, ReviewReport};



pub(super) fn finding(
    finding_type: &str,
    severity: &str,
    field_id: Option<&str>,
    message: String,
    basis_ref: Option<String>,
    suggested_action: Option<&str>,
) -> Finding {
    Finding {
        finding_type: finding_type.to_owned(),
        severity: severity.to_owned(),
        field_id: field_id.map(str::to_owned),
        message,
        basis_ref,
        suggested_action: suggested_action.map(str::to_owned),
    }
}

pub(super) fn field_from_row(row: &Row<'_>) -> rusqlite::Result<ApplicationField> {
    let metadata: String = row.get("metadata_json")?;
    Ok(ApplicationField {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        code: row.get("code")?,
        title: row.get("title")?,
        instruction: row.get("instruction")?,
        char_limit: row.get("char_limit")?,
        value: row.get("value")?,
        status: row.get("status")?,
        metadata: serde_json::from_str(&metadata).unwrap_or(Value::Null),
        updated_at: row.get("updated_at")?,
    })
}
pub(super) fn budget_line_from_row(row: &Row<'_>) -> rusqlite::Result<BudgetLine> {
    let metadata: String = row.get("metadata_json")?;
    Ok(BudgetLine {
        id: row.get("id")?,
        budget_id: row.get("budget_id")?,
        task_code: row.get("task_code")?,
        category: row.get("category")?,
        research_type: row.get("research_type")?,
        description: row.get("description")?,
        quantity: row.get("quantity")?,
        unit: row.get("unit")?,
        unit_cost: row.get("unit_cost")?,
        eligible_cost: row.get("eligible_cost")?,
        aid_rate: row.get("aid_rate")?,
        requested_funding: row.get("requested_funding")?,
        source_ref: row.get("source_ref")?,
        metadata: serde_json::from_str(&metadata).unwrap_or(Value::Null),
        created_at: row.get("created_at")?,
    })
}
pub(super) fn row_json(connection: &rusqlite::Connection, sql: &str, id: &str) -> Result<Option<Value>> {
    connection
        .query_row(sql, [id], sqlite_row_json)
        .optional()
        .map_err(Into::into)
}
pub(super) fn rows_json(connection: &rusqlite::Connection, sql: &str, id: &str) -> Result<Vec<Value>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([id], sqlite_row_json)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
fn sqlite_row_json(row: &Row<'_>) -> rusqlite::Result<Value> {
    let mut object = serde_json::Map::new();
    let statement = row.as_ref();
    for name in statement.column_names() {
        let value = match row.get_ref(name)? {
            rusqlite::types::ValueRef::Null => Value::Null,
            rusqlite::types::ValueRef::Integer(value) => Value::from(value),
            rusqlite::types::ValueRef::Real(value) => Value::from(value),
            rusqlite::types::ValueRef::Text(value) => {
                Value::String(String::from_utf8_lossy(value).into_owned())
            }
            rusqlite::types::ValueRef::Blob(value) => {
                Value::String(format!("<binary:{}>", value.len()))
            }
        };
        object.insert(name.to_owned(), value);
    }
    Ok(Value::Object(object))
}
