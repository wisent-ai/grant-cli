use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{ApplicationField, BudgetLine, Claim, Finding, ReviewReport};

pub struct AuthoringService<'a> {
    db: &'a Database,
}

impl<'a> AuthoringService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn field_add(
        &self,
        application_id: &str,
        code: &str,
        title: &str,
        instruction: Option<&str>,
        char_limit: Option<usize>,
        metadata: Value,
    ) -> Result<ApplicationField> {
        let timestamp = now();
        let field = ApplicationField {
            id: prefixed_id("field"),
            application_id: application_id.to_owned(),
            code: code.to_owned(),
            title: title.to_owned(),
            instruction: instruction.map(str::to_owned),
            char_limit,
            value: None,
            status: "empty".to_owned(),
            metadata,
            updated_at: timestamp,
        };
        self.db.connection.execute(
            "INSERT INTO application_fields(id, application_id, code, title, instruction, char_limit, metadata_json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(application_id, code) DO UPDATE SET title = excluded.title, instruction = excluded.instruction, char_limit = excluded.char_limit, metadata_json = excluded.metadata_json, updated_at = excluded.updated_at",
            params![field.id, field.application_id, field.code, field.title, field.instruction, field.char_limit, encode(&field.metadata)?, field.updated_at],
        )?;
        self.field_get(application_id, code)
    }

    pub fn field_get(&self, application_id: &str, code: &str) -> Result<ApplicationField> {
        self.db.connection.query_row(
            "SELECT id, application_id, code, title, instruction, char_limit, value, status, metadata_json, updated_at FROM application_fields WHERE application_id = ?1 AND (code = ?2 OR id = ?2)",
            params![application_id, code], field_from_row,
        ).optional()?.context("application field not found")
    }

    pub fn field_list(&self, application_id: &str) -> Result<Vec<ApplicationField>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, application_id, code, title, instruction, char_limit, value, status, metadata_json, updated_at FROM application_fields WHERE application_id = ?1 ORDER BY code",
        )?;
        let rows = statement.query_map([application_id], field_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn field_draft(
        &self,
        application_id: &str,
        code: &str,
        value: &str,
        status: &str,
    ) -> Result<ApplicationField> {
        if !["draft", "needs-evidence", "ready", "approved"].contains(&status) {
            return Err(anyhow!("unsupported field status: {status}"));
        }
        let field = self.field_get(application_id, code)?;
        self.db.connection.execute(
            "UPDATE application_fields SET value = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
            params![value, status, now(), field.id],
        )?;
        self.db.activity(
            "application",
            application_id,
            "field-drafted",
            &json!({ "field_id": field.id, "code": field.code, "status": status }),
        )?;
        self.field_get(application_id, code)
    }

    pub fn field_link_requirement(
        &self,
        application_id: &str,
        code: &str,
        requirement_id: &str,
    ) -> Result<Value> {
        let field = self.field_get(application_id, code)?;
        self.db.connection.execute(
            "INSERT OR IGNORE INTO field_requirements(field_id, requirement_id) VALUES (?1, ?2)",
            params![field.id, requirement_id],
        )?;
        Ok(json!({ "field_id": field.id, "requirement_id": requirement_id }))
    }

    pub fn field_link_criterion(
        &self,
        application_id: &str,
        code: &str,
        criterion_id: &str,
    ) -> Result<Value> {
        let field = self.field_get(application_id, code)?;
        self.db.connection.execute(
            "INSERT OR IGNORE INTO field_criteria(field_id, criterion_id) VALUES (?1, ?2)",
            params![field.id, criterion_id],
        )?;
        Ok(json!({ "field_id": field.id, "criterion_id": criterion_id }))
    }

    pub fn claim_add(&self, application_id: &str, code: &str, text: &str) -> Result<Claim> {
        let field = self.field_get(application_id, code)?;
        let claim = Claim {
            id: prefixed_id("claim"),
            field_id: field.id,
            claim: text.to_owned(),
            status: "unverified".to_owned(),
            created_at: now(),
        };
        self.db.connection.execute(
            "INSERT INTO field_claims(id, field_id, claim, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![claim.id, claim.field_id, claim.claim, claim.status, claim.created_at],
        )?;
        Ok(claim)
    }

    pub fn claim_list(&self, application_id: &str, code: Option<&str>) -> Result<Vec<Claim>> {
        let mut statement = self.db.connection.prepare(
            "SELECT c.id, c.field_id, c.claim, c.status, c.created_at FROM field_claims c JOIN application_fields f ON f.id = c.field_id WHERE f.application_id = ?1 AND (?2 IS NULL OR f.code = ?2 OR f.id = ?2) ORDER BY c.created_at",
        )?;
        let rows = statement.query_map(params![application_id, code], |row| {
            Ok(Claim {
                id: row.get("id")?,
                field_id: row.get("field_id")?,
                claim: row.get("claim")?,
                status: row.get("status")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn claim_link(
        &self,
        claim_id: &str,
        organization_evidence_id: Option<&str>,
        document_id: Option<&str>,
        citation: Option<&str>,
        note: Option<&str>,
    ) -> Result<Value> {
        if organization_evidence_id.is_none() && document_id.is_none() {
            return Err(anyhow!("an evidence or document reference is required"));
        }
        let link_id = prefixed_id("link");
        self.db.connection.execute(
            "INSERT INTO evidence_links(id, claim_id, organization_evidence_id, document_id, citation, note, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![link_id, claim_id, organization_evidence_id, document_id, citation, note, now()],
        )?;
        self.db.connection.execute(
            "UPDATE field_claims SET status = 'supported' WHERE id = ?1",
            [claim_id],
        )?;
        Ok(
            json!({ "id": link_id, "claim_id": claim_id, "organization_evidence_id": organization_evidence_id, "document_id": document_id }),
        )
    }

    pub fn lint(&self, application_id: &str) -> Result<Vec<Finding>> {
        let fields = self.field_list(application_id)?;
        let mut findings = Vec::new();
        let mut normalized: HashMap<String, Vec<String>> = HashMap::new();
        for field in &fields {
            match field.value.as_deref() {
                None | Some("") => findings.push(finding(
                    "missing-field",
                    "error",
                    Some(&field.id),
                    format!("Field {} is empty", field.code),
                    None,
                    Some("Draft the field from verified inputs"),
                )),
                Some(value) => {
                    if field
                        .char_limit
                        .is_some_and(|limit| value.chars().count() > limit)
                    {
                        findings.push(finding(
                            "character-limit",
                            "error",
                            Some(&field.id),
                            format!("Field {} exceeds its character limit", field.code),
                            field.char_limit.map(|limit| format!("limit:{limit}")),
                            Some("Reduce the field without removing required evidence"),
                        ));
                    }
                    let key = value
                        .split_whitespace()
                        .map(str::to_lowercase)
                        .collect::<Vec<_>>()
                        .join(" ");
                    normalized.entry(key).or_default().push(field.id.clone());
                }
            }
            let unmapped_requirements: i64 = self.db.connection.query_row(
                "SELECT COUNT(*) AS value FROM field_requirements WHERE field_id = ?1",
                [&field.id],
                |row| row.get("value"),
            )?;
            let unmapped_criteria: i64 = self.db.connection.query_row(
                "SELECT COUNT(*) AS value FROM field_criteria WHERE field_id = ?1",
                [&field.id],
                |row| row.get("value"),
            )?;
            if unmapped_requirements == i64::default() && unmapped_criteria == i64::default() {
                findings.push(finding(
                    "unmapped-field",
                    "warning",
                    Some(&field.id),
                    format!(
                        "Field {} is not mapped to a requirement or criterion",
                        field.code
                    ),
                    None,
                    Some("Link the field to the official instruction or evaluation criterion"),
                ));
            }
        }
        for field_ids in normalized
            .values()
            .filter(|values| values.len() > usize::from(true))
        {
            for field_id in field_ids {
                findings.push(finding(
                    "duplicate-answer",
                    "warning",
                    Some(field_id),
                    "The same answer appears in multiple fields".to_owned(),
                    None,
                    Some("Tailor each answer to its own criterion"),
                ));
            }
        }
        let unsupported = self.db.connection.prepare(
            "SELECT c.id, c.field_id, c.claim FROM field_claims c JOIN application_fields f ON f.id = c.field_id WHERE f.application_id = ?1 AND c.status != 'supported'",
        )?.query_map([application_id], |row| Ok((row.get::<_, String>("id")?, row.get::<_, String>("field_id")?, row.get::<_, String>("claim")?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (claim_id, field_id, claim) in unsupported {
            findings.push(finding(
                "unsupported-claim",
                "error",
                Some(&field_id),
                format!("Unsupported claim: {claim}"),
                Some(claim_id),
                Some("Link confirmed organization evidence or an authoritative document"),
            ));
        }
        let open_comments = self.db.connection.prepare(
            "SELECT id, field_id, severity, body FROM comments WHERE application_id = ?1 AND status = 'open'",
        )?.query_map([application_id], |row| Ok((row.get::<_, String>("id")?, row.get::<_, Option<String>>("field_id")?, row.get::<_, String>("severity")?, row.get::<_, String>("body")?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (comment_id, field_id, severity, body) in open_comments {
            findings.push(finding(
                "open-comment",
                &severity,
                field_id.as_deref(),
                body,
                Some(comment_id),
                Some("Resolve the comment and record the resolution"),
            ));
        }
        Ok(findings)
    }

    pub fn budget_init(
        &self,
        application_id: &str,
        currency: &str,
        indirect_method: Option<&str>,
        indirect_rate: Option<f64>,
        private_financing: Value,
    ) -> Result<Value> {
        let budget_id = prefixed_id("budget");
        self.db.connection.execute(
            "INSERT INTO budgets(id, application_id, currency, indirect_method, indirect_rate, private_financing_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7) ON CONFLICT(application_id) DO UPDATE SET currency = excluded.currency, indirect_method = excluded.indirect_method, indirect_rate = excluded.indirect_rate, private_financing_json = excluded.private_financing_json, updated_at = excluded.updated_at",
            params![budget_id, application_id, currency, indirect_method, indirect_rate, encode(&private_financing)?, now()],
        )?;
        let resolved: String = self.db.connection.query_row(
            "SELECT id FROM budgets WHERE application_id = ?1",
            [application_id],
            |row| row.get("id"),
        )?;
        Ok(
            json!({ "id": resolved, "application_id": application_id, "currency": currency, "indirect_method": indirect_method, "indirect_rate": indirect_rate, "private_financing": private_financing }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn budget_line_add(
        &self,
        application_id: &str,
        task_code: Option<&str>,
        category: &str,
        research_type: Option<&str>,
        description: &str,
        quantity: Option<f64>,
        unit: Option<&str>,
        unit_cost: Option<f64>,
        eligible_cost: f64,
        aid_rate: Option<f64>,
        requested_funding: Option<f64>,
        source_ref: Option<&str>,
        metadata: Value,
    ) -> Result<BudgetLine> {
        let budget_id: String = self
            .db
            .connection
            .query_row(
                "SELECT id FROM budgets WHERE application_id = ?1",
                [application_id],
                |row| row.get("id"),
            )
            .optional()?
            .context("budget not initialized")?;
        let calculated_funding =
            requested_funding.or_else(|| aid_rate.map(|rate| eligible_cost * rate));
        let line = BudgetLine {
            id: prefixed_id("cost"),
            budget_id,
            task_code: task_code.map(str::to_owned),
            category: category.to_owned(),
            research_type: research_type.map(str::to_owned),
            description: description.to_owned(),
            quantity,
            unit: unit.map(str::to_owned),
            unit_cost,
            eligible_cost,
            aid_rate,
            requested_funding: calculated_funding,
            source_ref: source_ref.map(str::to_owned),
            metadata,
            created_at: now(),
        };
        self.db.connection.execute(
            "INSERT INTO budget_lines(id, budget_id, task_code, category, research_type, description, quantity, unit, unit_cost, eligible_cost, aid_rate, requested_funding, source_ref, metadata_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![line.id, line.budget_id, line.task_code, line.category, line.research_type, line.description, line.quantity, line.unit, line.unit_cost, line.eligible_cost, line.aid_rate, line.requested_funding, line.source_ref, encode(&line.metadata)?, line.created_at],
        )?;
        Ok(line)
    }

    pub fn budget_check(&self, application_id: &str) -> Result<Vec<Finding>> {
        let budget = self.db.connection.query_row(
            "SELECT id, indirect_method, indirect_rate, private_financing_json FROM budgets WHERE application_id = ?1", [application_id],
            |row| Ok((row.get::<_, String>("id")?, row.get::<_, Option<String>>("indirect_method")?, row.get::<_, Option<f64>>("indirect_rate")?, row.get::<_, String>("private_financing_json")?)),
        ).optional()?.context("budget not initialized")?;
        let (budget_id, indirect_method, indirect_rate, private_json) = budget;
        let mut statement = self.db.connection.prepare(
            "SELECT id, budget_id, task_code, category, research_type, description, quantity, unit, unit_cost, eligible_cost, aid_rate, requested_funding, source_ref, metadata_json, created_at FROM budget_lines WHERE budget_id = ?1 ORDER BY category, task_code",
        )?;
        let lines = statement
            .query_map([budget_id], budget_line_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut findings = Vec::new();
        for line in &lines {
            if line.eligible_cost.is_sign_negative() {
                findings.push(finding(
                    "negative-cost",
                    "error",
                    None,
                    format!("Negative eligible cost in {}", line.id),
                    Some(line.id.clone()),
                    Some("Correct the cost calculation"),
                ));
            }
            if line.task_code.as_deref().unwrap_or("").is_empty() {
                findings.push(finding(
                    "cost-without-task",
                    "error",
                    None,
                    format!("Cost {} is not linked to a task", line.id),
                    Some(line.id.clone()),
                    Some("Assign the cost to a project task"),
                ));
            }
            if let (Some(quantity), Some(unit_cost)) = (line.quantity, line.unit_cost) {
                let calculated = quantity * unit_cost;
                if (calculated - line.eligible_cost).abs() > f64::EPSILON {
                    findings.push(finding(
                        "cost-calculation",
                        "error",
                        None,
                        format!("Cost {} does not equal quantity times unit cost", line.id),
                        Some(line.id.clone()),
                        Some("Align quantity, rate and eligible cost"),
                    ));
                }
            }
            if let (Some(rate), Some(requested)) = (line.aid_rate, line.requested_funding) {
                if (line.eligible_cost * rate - requested).abs() > f64::EPSILON {
                    findings.push(finding(
                        "aid-intensity",
                        "error",
                        None,
                        format!(
                            "Requested funding for {} does not match eligible cost times aid rate",
                            line.id
                        ),
                        Some(line.id.clone()),
                        Some("Correct the aid-rate calculation"),
                    ));
                }
            }
            if line.source_ref.is_none() {
                findings.push(finding(
                    "cost-source",
                    "warning",
                    None,
                    format!("Cost {} has no rate or eligibility source", line.id),
                    Some(line.id.clone()),
                    Some("Attach a quote, rate basis or instruction citation"),
                ));
            }
        }
        if indirect_method.is_some() != indirect_rate.is_some() {
            findings.push(finding(
                "indirect-cost-method",
                "error",
                None,
                "Indirect cost method and rate must be defined together".to_owned(),
                None,
                Some("Complete both indirect cost fields"),
            ));
        }
        let private: Value = serde_json::from_str(&private_json).unwrap_or(Value::Null);
        if private.is_null() || private.as_object().is_some_and(|value| value.is_empty()) {
            findings.push(finding(
                "private-financing",
                "warning",
                None,
                "Private financing sources are not defined".to_owned(),
                None,
                Some("Record equity, loan or other private financing explicitly"),
            ));
        }
        Ok(findings)
    }

    pub fn review(&self, application_id: &str) -> Result<ReviewReport> {
        let mut findings = self.lint(application_id)?;
        if self.db.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM budgets WHERE application_id = ?1) AS value",
            [application_id],
            |row| row.get::<_, bool>("value"),
        )? {
            findings.extend(self.budget_check(application_id)?);
        } else {
            findings.push(finding(
                "missing-budget",
                "error",
                None,
                "Application has no budget".to_owned(),
                None,
                Some("Initialize and complete the application budget"),
            ));
        }
        let hard_fit: Option<String> = self.db.connection.query_row(
            "SELECT eligibility FROM fit_assessments f JOIN applications a ON a.opportunity_id = f.opportunity_id AND a.organization_id = f.organization_id WHERE a.id = ?1 ORDER BY assessed_at DESC LIMIT 1",
            [application_id], |row| row.get("eligibility"),
        ).optional()?;
        match hard_fit.as_deref() {
            Some("ineligible") => findings.push(finding(
                "eligibility",
                "blocker",
                None,
                "Organization fails a hard eligibility gate".to_owned(),
                None,
                Some("Stop the application or correct verified eligibility data"),
            )),
            Some("unknown") | None => findings.push(finding(
                "eligibility",
                "error",
                None,
                "Eligibility has unresolved hard gates".to_owned(),
                None,
                Some("Run eligibility assessment with complete evidence"),
            )),
            _ => {}
        }
        let status = if findings
            .iter()
            .any(|entry| ["blocker", "error"].contains(&entry.severity.as_str()))
        {
            "failed"
        } else {
            "passed"
        };
        let report = ReviewReport {
            id: prefixed_id("review"),
            application_id: application_id.to_owned(),
            status: status.to_owned(),
            findings,
            created_at: now(),
        };
        self.db.connection.execute(
            "INSERT INTO reviews(id, application_id, kind, status, summary_json, created_at) VALUES (?1, ?2, 'full', ?3, ?4, ?5)",
            params![report.id, report.application_id, report.status, encode(&json!({ "findings": report.findings.len() }))?, report.created_at],
        )?;
        for entry in &report.findings {
            self.db.connection.execute(
                "INSERT INTO review_findings(id, review_id, field_id, type, severity, message, basis_ref, suggested_action, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![prefixed_id("finding"), report.id, entry.field_id, entry.finding_type, entry.severity, entry.message, entry.basis_ref, entry.suggested_action, now()],
            )?;
        }
        self.db
            .activity("application", application_id, "reviewed", &report)?;
        Ok(report)
    }

    pub fn export(&self, application_id: &str, output: Option<&str>) -> Result<Value> {
        let application = row_json(
            &self.db.connection,
            "SELECT * FROM applications WHERE id = ?1",
            application_id,
        )?
        .context("application not found")?;
        let fields = rows_json(
            &self.db.connection,
            "SELECT * FROM application_fields WHERE application_id = ?1 ORDER BY code",
            application_id,
        )?;
        let requirements = rows_json(
            &self.db.connection,
            "SELECT * FROM requirements WHERE application_id = ?1 ORDER BY authority, kind, code",
            application_id,
        )?;
        let criteria = rows_json(
            &self.db.connection,
            "SELECT * FROM criteria WHERE application_id = ?1 ORDER BY gate DESC, code",
            application_id,
        )?;
        let comments = rows_json(
            &self.db.connection,
            "SELECT * FROM comments WHERE application_id = ?1 ORDER BY created_at",
            application_id,
        )?;
        let tasks = rows_json(
            &self.db.connection,
            "SELECT * FROM application_tasks WHERE application_id = ?1 ORDER BY due_at",
            application_id,
        )?;
        let budget = row_json(
            &self.db.connection,
            "SELECT * FROM budgets WHERE application_id = ?1",
            application_id,
        )?;
        let budget_lines = rows_json(
            &self.db.connection,
            "SELECT l.* FROM budget_lines l JOIN budgets b ON b.id = l.budget_id WHERE b.application_id = ?1 ORDER BY l.category, l.task_code",
            application_id,
        )?;
        let package = json!({
            "schema": "grant-cli.application-package.v1", "exported_at": now(), "application": application,
            "fields": fields, "requirements": requirements, "criteria": criteria, "comments": comments, "tasks": tasks,
            "budget": budget, "budget_lines": budget_lines,
        });
        let path = output.map(ToOwned::to_owned).unwrap_or_else(|| {
            self.db
                .export_path(&format!("{application_id}.json"))
                .to_string_lossy()
                .into_owned()
        });
        fs::write(&path, serde_json::to_vec_pretty(&package)?)?;
        self.db.activity(
            "application",
            application_id,
            "exported",
            &json!({ "path": path }),
        )?;
        Ok(json!({ "path": path, "package": package }))
    }
}

fn finding(
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

fn field_from_row(row: &Row<'_>) -> rusqlite::Result<ApplicationField> {
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
fn budget_line_from_row(row: &Row<'_>) -> rusqlite::Result<BudgetLine> {
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
fn row_json(connection: &rusqlite::Connection, sql: &str, id: &str) -> Result<Option<Value>> {
    connection
        .query_row(sql, [id], sqlite_row_json)
        .optional()
        .map_err(Into::into)
}
fn rows_json(connection: &rusqlite::Connection, sql: &str, id: &str) -> Result<Vec<Value>> {
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
