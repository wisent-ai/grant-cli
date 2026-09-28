//! Reading one database row back as the thing it stands for, and the finding
//! shape every check reports in.

use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result, anyhow};
use crate::db::sql::{Connection, OptionalExtension, Row, params};
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

pub(super) fn field_from_row(row: &Row<'_>) -> crate::db::sql::Result<ApplicationField> {
    let metadata: String = row.get("metadata_json")?;
    Ok(ApplicationField {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        code: row.get("code")?,
        title: row.get("title")?,
        instruction: row.get("instruction")?,
        char_limit: row.get::<_, Option<i64>>("char_limit")?.map(usize::try_from).transpose()?,
        value: row.get("value")?,
        status: row.get("status")?,
        metadata: serde_json::from_str(&metadata).unwrap_or(Value::Null),
        updated_at: row.get("updated_at")?,
    })
}
pub(super) fn budget_line_from_row(row: &Row<'_>) -> crate::db::sql::Result<BudgetLine> {
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
pub(super) fn row_json(connection: &Connection, sql: &str, id: &str) -> Result<Option<Value>> {
    connection
        .query_row(sql, [id], |row| row.json())
        .optional()
        .map_err(Into::into)
}
pub(super) fn rows_json(connection: &Connection, sql: &str, id: &str) -> Result<Vec<Value>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([id], |row| row.json())?;
    rows.collect::<crate::db::sql::Result<Vec<_>>>()
        .map_err(Into::into)
}
