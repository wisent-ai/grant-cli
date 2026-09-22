//! What an application looks like when it is read as a whole: the review
//! that gathers every finding, and the export that writes the application
//! and everything under it as one document.

use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{ApplicationField, BudgetLine, Claim, Finding, ReviewReport};


use super::AuthoringService;
use super::rows::{finding, row_json, rows_json};

impl<'a> AuthoringService<'a> {

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
}
