use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use crate::db::{Database, now, prefixed_id};
use crate::model::{Analytics, Outcome};

pub struct DeliveryService<'a> {
    db: &'a Database,
}

impl<'a> DeliveryService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn record_outcome(
        &self,
        application_id: &str,
        result: &str,
        decided_at: Option<&str>,
        awarded_amount: Option<f64>,
        score: Option<f64>,
        feedback_document_id: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Outcome> {
        let allowed = [
            "awarded",
            "rejected",
            "withdrawn",
            "clarification",
            "pending",
        ];
        if !allowed.contains(&result) {
            anyhow::bail!("unsupported outcome: {result}");
        }
        let outcome = Outcome {
            id: prefixed_id("outcome"),
            application_id: application_id.to_owned(),
            result: result.to_owned(),
            decided_at: decided_at.map(str::to_owned),
            awarded_amount,
            score,
            notes: notes.map(str::to_owned),
            created_at: now(),
        };
        self.db.connection.execute(
            "INSERT INTO outcomes(id, application_id, result, decided_at, awarded_amount, score, feedback_document_id, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) ON CONFLICT(application_id) DO UPDATE SET id = excluded.id, result = excluded.result, decided_at = excluded.decided_at, awarded_amount = excluded.awarded_amount, score = excluded.score, feedback_document_id = excluded.feedback_document_id, notes = excluded.notes, created_at = excluded.created_at",
            params![outcome.id, outcome.application_id, outcome.result, outcome.decided_at, outcome.awarded_amount, outcome.score, feedback_document_id, outcome.notes, outcome.created_at],
        )?;
        if ["awarded", "rejected", "withdrawn"].contains(&result) {
            self.db.connection.execute(
                "UPDATE applications SET stage = ?1, updated_at = ?2 WHERE id = ?3",
                params![result, now(), application_id],
            )?;
        }
        self.db.activity(
            "application",
            application_id,
            "outcome-recorded",
            &json!({ "outcome": outcome, "feedback_document_id": feedback_document_id }),
        )?;
        Ok(outcome)
    }

    pub fn ingest_feedback(&self, application_id: &str, document_id: &str) -> Result<Value> {
        let text_path: String = self
            .db
            .connection
            .query_row(
                "SELECT text_path FROM documents WHERE id = ?1",
                [document_id],
                |row| row.get("text_path"),
            )
            .optional()?
            .context("feedback document not found")?;
        let text = std::fs::read_to_string(text_path)?;
        let comment_words = [
            "uwaga",
            "brak",
            "nie wykazano",
            "niewystarcz",
            "recommend",
            "weakness",
            "missing",
        ];
        let mut comments = Vec::new();
        for paragraph in text
            .split("\n\n")
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let lowered = paragraph.to_lowercase();
            if comment_words.iter().any(|word| lowered.contains(word)) {
                let comment_id = prefixed_id("comment");
                self.db.connection.execute(
                    "INSERT INTO comments(id, application_id, type, severity, body, basis_kind, basis_ref, suggested_actions_json, status, created_at) VALUES (?1, ?2, 'reviewer-risk', 'warning', ?3, 'evaluator-feedback', ?4, '[]', 'open', ?5)",
                    params![comment_id, application_id, paragraph, document_id, now()],
                )?;
                comments.push(json!({ "id": comment_id, "body": paragraph }));
            }
        }
        self.db.activity(
            "application",
            application_id,
            "feedback-ingested",
            &json!({ "document_id": document_id, "comments": comments.len() }),
        )?;
        Ok(
            json!({ "application_id": application_id, "document_id": document_id, "comments": comments }),
        )
    }

    pub fn analytics(&self) -> Result<Analytics> {
        let applications = scalar_i64(self.db, "SELECT COUNT(*) AS value FROM applications")?;
        let awarded = scalar_i64(
            self.db,
            "SELECT COUNT(*) AS value FROM outcomes WHERE result = 'awarded'",
        )?;
        let rejected = scalar_i64(
            self.db,
            "SELECT COUNT(*) AS value FROM outcomes WHERE result = 'rejected'",
        )?;
        let submitted = scalar_i64(
            self.db,
            "SELECT COUNT(*) AS value FROM applications WHERE submitted_at IS NOT NULL",
        )?;
        let requested_funding = scalar_f64(
            self.db,
            "SELECT COALESCE(SUM(requested_funding), 0) AS value FROM budget_lines",
        )?;
        let awarded_funding = scalar_f64(
            self.db,
            "SELECT COALESCE(SUM(awarded_amount), 0) AS value FROM outcomes WHERE result = 'awarded'",
        )?;
        let open_findings = scalar_i64(
            self.db,
            "SELECT COUNT(*) AS value FROM comments WHERE status = 'open'",
        )?;
        let overdue_tasks = scalar_i64(
            self.db,
            "SELECT COUNT(*) AS value FROM application_tasks WHERE status != 'done' AND due_at IS NOT NULL AND due_at < datetime('now')",
        )?;
        Ok(Analytics {
            applications,
            awarded,
            rejected,
            submitted,
            requested_funding,
            awarded_funding,
            open_findings,
            overdue_tasks,
        })
    }

    pub fn lessons(&self, application_id: &str) -> Result<Value> {
        let application = self
            .db
            .connection
            .query_row(
                "SELECT name, stage FROM applications WHERE id = ?1",
                [application_id],
                |row| {
                    Ok((
                        row.get::<_, String>("name")?,
                        row.get::<_, String>("stage")?,
                    ))
                },
            )
            .optional()?
            .context("application not found")?;
        let (name, stage) = application;
        let outcome: Option<Value> = self.db.connection.query_row(
            "SELECT result, score, awarded_amount, notes FROM outcomes WHERE application_id = ?1", [application_id],
            |row| Ok(json!({ "result": row.get::<_, String>("result")?, "score": row.get::<_, Option<f64>>("score")?, "awarded_amount": row.get::<_, Option<f64>>("awarded_amount")?, "notes": row.get::<_, Option<String>>("notes")? })),
        ).optional()?;
        let comments = self.db.connection.prepare(
            "SELECT type, severity, body, basis_ref, status, resolution FROM comments WHERE application_id = ?1 ORDER BY created_at",
        )?.query_map([application_id], |row| Ok(json!({ "type": row.get::<_, String>("type")?, "severity": row.get::<_, String>("severity")?, "body": row.get::<_, String>("body")?, "basis_ref": row.get::<_, Option<String>>("basis_ref")?, "status": row.get::<_, String>("status")?, "resolution": row.get::<_, Option<String>>("resolution")? })))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(
            json!({ "application_id": application_id, "name": name, "stage": stage, "outcome": outcome, "comments": comments }),
        )
    }
}

fn scalar_i64(db: &Database, sql: &str) -> Result<i64> {
    Ok(db.connection.query_row(sql, [], |row| row.get("value"))?)
}
fn scalar_f64(db: &Database, sql: &str) -> Result<f64> {
    Ok(db.connection.query_row(sql, [], |row| row.get("value"))?)
}
