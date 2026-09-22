//! The budget under an application: what it is denominated in, the lines in
//! it, and the checks a line and a total have to pass.

use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{ApplicationField, BudgetLine, Claim, Finding, ReviewReport};


use super::AuthoringService;
use super::rows::{budget_line_from_row, finding};

impl<'a> AuthoringService<'a> {

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
}
