//! The fields an application is made of and the claims written into them:
//! adding one, drafting its text, linking it to the requirement or criterion
//! it answers, and the evidence behind each claim.

use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{ApplicationField, BudgetLine, Claim, Finding, ReviewReport};


use super::AuthoringService;
use super::rows::{field_from_row, finding};

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
}
