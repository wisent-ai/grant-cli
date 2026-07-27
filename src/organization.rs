use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Map, Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{EligibilityFinding, EligibilityRule, Evidence, FitAssessment, Organization};

pub struct OrganizationService<'a> {
    db: &'a Database,
}

impl<'a> OrganizationService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn upsert(&self, slug: &str, name: &str, profile: Value) -> Result<Organization> {
        let timestamp = now();
        let existing: Option<String> = self
            .db
            .connection
            .query_row(
                "SELECT id FROM organizations WHERE slug = ?1",
                [slug],
                |row| row.get("id"),
            )
            .optional()?;
        let organization_id = existing.unwrap_or_else(|| prefixed_id("org"));
        self.db.connection.execute(
            "INSERT INTO organizations(id, slug, name, profile_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5) ON CONFLICT(slug) DO UPDATE SET name = excluded.name, profile_json = excluded.profile_json, updated_at = excluded.updated_at",
            params![organization_id, slug, name, encode(&profile)?, timestamp],
        )?;
        let organization = self.get(slug)?;
        self.db.activity(
            "organization",
            &organization.id,
            "profile-updated",
            &organization,
        )?;
        Ok(organization)
    }

    pub fn get(&self, value: &str) -> Result<Organization> {
        self.db.connection.query_row(
            "SELECT id, slug, name, profile_json, created_at, updated_at FROM organizations WHERE id = ?1 OR slug = ?1",
            [value], organization_from_row,
        ).optional()?.context("organization not found")
    }

    pub fn evidence_add(
        &self,
        organization: &str,
        kind: &str,
        title: &str,
        value: Value,
        source: Option<&str>,
        valid_from: Option<&str>,
        valid_until: Option<&str>,
        confidence: &str,
    ) -> Result<Evidence> {
        let organization_id = self.get(organization)?.id;
        let evidence = Evidence {
            id: prefixed_id("ev"),
            organization_id,
            kind: kind.to_owned(),
            title: title.to_owned(),
            value,
            source: source.map(str::to_owned),
            valid_from: valid_from.map(str::to_owned),
            valid_until: valid_until.map(str::to_owned),
            confidence: confidence.to_owned(),
            created_at: now(),
        };
        self.db.connection.execute(
            "INSERT INTO organization_evidence(id, organization_id, kind, title, value_json, source, valid_from, valid_until, confidence, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![evidence.id, evidence.organization_id, evidence.kind, evidence.title, encode(&evidence.value)?, evidence.source, evidence.valid_from, evidence.valid_until, evidence.confidence, evidence.created_at],
        )?;
        self.db.activity(
            "organization",
            &evidence.organization_id,
            "evidence-added",
            &evidence,
        )?;
        Ok(evidence)
    }

    pub fn evidence_list(&self, organization: &str) -> Result<Vec<Evidence>> {
        let organization_id = self.get(organization)?.id;
        let mut statement = self.db.connection.prepare(
            "SELECT id, organization_id, kind, title, value_json, source, valid_from, valid_until, confidence, created_at FROM organization_evidence WHERE organization_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([organization_id], evidence_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn rule_add(
        &self,
        opportunity_id: &str,
        name: &str,
        expression: Value,
        hard_gate: bool,
        citation: Option<&str>,
    ) -> Result<EligibilityRule> {
        let rule = EligibilityRule {
            id: prefixed_id("rule"),
            opportunity_id: opportunity_id.to_owned(),
            name: name.to_owned(),
            expression,
            hard_gate,
            citation: citation.map(str::to_owned),
        };
        self.db.connection.execute(
            "INSERT INTO eligibility_rules(id, opportunity_id, name, expression_json, hard_gate, citation, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![rule.id, rule.opportunity_id, rule.name, encode(&rule.expression)?, rule.hard_gate, rule.citation, now()],
        )?;
        self.db.activity(
            "opportunity",
            opportunity_id,
            "eligibility-rule-added",
            &rule,
        )?;
        Ok(rule)
    }

    pub fn assess(&self, opportunity_id: &str, organization: &str) -> Result<FitAssessment> {
        let organization = self.get(organization)?;
        let evidence = self.evidence_list(&organization.id)?;
        let context = build_context(&organization.profile, &evidence);
        let mut statement = self.db.connection.prepare(
            "SELECT id, opportunity_id, name, expression_json, hard_gate, citation FROM eligibility_rules WHERE opportunity_id = ?1 ORDER BY hard_gate DESC, name",
        )?;
        let rules = statement
            .query_map([opportunity_id], rule_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut findings = Vec::new();
        for rule in rules {
            let (passed, reason) = evaluate_expression(&rule.expression, &context)?;
            findings.push(EligibilityFinding {
                rule_id: rule.id,
                name: rule.name,
                passed,
                hard_gate: rule.hard_gate,
                reason,
                citation: rule.citation,
            });
        }
        let hard_failed = findings
            .iter()
            .any(|finding| finding.hard_gate && finding.passed == Some(false));
        let unknown_hard = findings
            .iter()
            .any(|finding| finding.hard_gate && finding.passed.is_none());
        let eligible = if hard_failed {
            "ineligible"
        } else if unknown_hard {
            "unknown"
        } else {
            "eligible"
        };
        let evaluated = findings
            .iter()
            .filter(|finding| finding.passed.is_some())
            .count();
        let passed = findings
            .iter()
            .filter(|finding| finding.passed == Some(true))
            .count();
        let score = if evaluated == usize::default() {
            None
        } else {
            Some(passed as f64 / evaluated as f64)
        };
        let dimensions = json!({ "rules_passed": passed, "rules_evaluated": evaluated, "hard_gate_failed": hard_failed });
        let assessment = FitAssessment {
            id: prefixed_id("fit"),
            opportunity_id: opportunity_id.to_owned(),
            organization_id: organization.id,
            eligibility: eligible.to_owned(),
            score,
            dimensions,
            findings,
            assessed_at: now(),
        };
        self.db.connection.execute(
            "INSERT INTO fit_assessments(id, opportunity_id, organization_id, eligibility, score, dimensions_json, findings_json, assessed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(opportunity_id, organization_id) DO UPDATE SET id = excluded.id, eligibility = excluded.eligibility, score = excluded.score, dimensions_json = excluded.dimensions_json, findings_json = excluded.findings_json, assessed_at = excluded.assessed_at",
            params![assessment.id, assessment.opportunity_id, assessment.organization_id, assessment.eligibility, assessment.score, encode(&assessment.dimensions)?, encode(&assessment.findings)?, assessment.assessed_at],
        )?;
        self.db
            .activity("opportunity", opportunity_id, "fit-assessed", &assessment)?;
        Ok(assessment)
    }
}

fn organization_from_row(row: &Row<'_>) -> rusqlite::Result<Organization> {
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

fn evidence_from_row(row: &Row<'_>) -> rusqlite::Result<Evidence> {
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

fn rule_from_row(row: &Row<'_>) -> rusqlite::Result<EligibilityRule> {
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

fn build_context(profile: &Value, evidence: &[Evidence]) -> Value {
    let mut context = profile.as_object().cloned().unwrap_or_else(Map::new);
    let mut evidence_map = Map::new();
    for item in evidence {
        evidence_map.insert(item.kind.clone(), item.value.clone());
        evidence_map.insert(item.title.clone(), item.value.clone());
    }
    context.insert("evidence".to_owned(), Value::Object(evidence_map));
    Value::Object(context)
}

fn evaluate_expression(expression: &Value, context: &Value) -> Result<(Option<bool>, String)> {
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

fn compare_numbers(
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
