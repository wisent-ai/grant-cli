//! Reading one row back as the thing it stands for, and deciding whether a
//! declared eligibility rule holds: the context an organization and its
//! evidence make, and the comparison each clause performs.

use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Map, Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{EligibilityFinding, EligibilityRule, Evidence, FitAssessment, Organization};


pub(super) fn organization_from_row(row: &Row<'_>) -> rusqlite::Result<Organization> {
    let profile: String = row.get("profile_json")?;
    Ok(Organization {
        id: row.get("id")?,
        slug: row.get("slug")?,
        name: row.get("name")?,
        profile: serde_json::from_str(&profile).unwrap_or(Value::Null),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub(super) fn evidence_from_row(row: &Row<'_>) -> rusqlite::Result<Evidence> {
    let value: String = row.get("value_json")?;
    Ok(Evidence {
        id: row.get("id")?,
        organization_id: row.get("organization_id")?,
        kind: row.get("kind")?,
        title: row.get("title")?,
        value: serde_json::from_str(&value).unwrap_or(Value::Null),
        source: row.get("source")?,
        valid_from: row.get("valid_from")?,
        valid_until: row.get("valid_until")?,
        confidence: row.get("confidence")?,
        created_at: row.get("created_at")?,
    })
}

pub(super) fn rule_from_row(row: &Row<'_>) -> rusqlite::Result<EligibilityRule> {
    let expression: String = row.get("expression_json")?;
    Ok(EligibilityRule {
        id: row.get("id")?,
        opportunity_id: row.get("opportunity_id")?,
        name: row.get("name")?,
        expression: serde_json::from_str(&expression).unwrap_or(Value::Null),
        hard_gate: row.get("hard_gate")?,
        citation: row.get("citation")?,
    })
}

pub(super) fn build_context(profile: &Value, evidence: &[Evidence]) -> Value {
    let mut context = profile.as_object().cloned().unwrap_or_else(Map::new);
    let mut evidence_map = Map::new();
    for item in evidence {
        evidence_map.insert(item.kind.clone(), item.value.clone());
        evidence_map.insert(item.title.clone(), item.value.clone());
    }
    context.insert("evidence".to_owned(), Value::Object(evidence_map));
    Value::Object(context)
}

pub(super) fn evaluate_expression(expression: &Value, context: &Value) -> Result<(Option<bool>, String)> {
    if let Some(all) = expression.get("all").and_then(Value::as_array) {
        let results = all
            .iter()
            .map(|entry| evaluate_expression(entry, context))
            .collect::<Result<Vec<_>>>()?;
        if results.iter().any(|result| result.0 == Some(false)) {
            return Ok((
                Some(false),
                "at least one required condition failed".to_owned(),
            ));
        }
        if results.iter().any(|result| result.0.is_none()) {
            return Ok((None, "at least one required value is unknown".to_owned()));
        }
        return Ok((Some(true), "all required conditions passed".to_owned()));
    }
    if let Some(any) = expression.get("any").and_then(Value::as_array) {
        let results = any
            .iter()
            .map(|entry| evaluate_expression(entry, context))
            .collect::<Result<Vec<_>>>()?;
        if results.iter().any(|result| result.0 == Some(true)) {
            return Ok((Some(true), "at least one alternative passed".to_owned()));
        }
        if results.iter().any(|result| result.0.is_none()) {
            return Ok((None, "alternative values are incomplete".to_owned()));
        }
        return Ok((Some(false), "no alternative passed".to_owned()));
    }
    let path = expression
        .get("path")
        .and_then(Value::as_str)
        .context("rule expression requires path")?;
    let operator = expression.get("op").and_then(Value::as_str).unwrap_or("eq");
    let expected = expression.get("value").unwrap_or(&Value::Null);
    let actual = value_at(context, path);
    let Some(actual) = actual else {
        return Ok((None, format!("missing value at {path}")));
    };
    let passed = match operator {
        "eq" => actual == expected,
        "neq" => actual != expected,
        "in" => expected
            .as_array()
            .is_some_and(|values| values.contains(actual)),
        "contains" => match (actual, expected) {
            (Value::Array(values), value) => values.contains(value),
            (Value::String(text), Value::String(fragment)) => text.contains(fragment),
            _ => false,
        },
        "gte" => compare_numbers(actual, expected, |left, right| left >= right)?,
        "lte" => compare_numbers(actual, expected, |left, right| left <= right)?,
        "exists" => !actual.is_null(),
        value => return Err(anyhow!("unsupported rule operator: {value}")),
    };
    Ok((
        Some(passed),
        format!("{path} {operator} {expected}; actual: {actual}"),
    ))
}

fn value_at<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(value, |current, segment| current.get(segment))
}

pub(super) fn compare_numbers(
    actual: &Value,
    expected: &Value,
    predicate: impl FnOnce(f64, f64) -> bool,
) -> Result<bool> {
    let left = actual
        .as_f64()
        .context("actual rule value is not numeric")?;
    let right = expected
        .as_f64()
        .context("expected rule value is not numeric")?;
    Ok(predicate(left, right))
}
