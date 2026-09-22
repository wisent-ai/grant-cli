//! The patterns a good application follows, and the worked examples behind
//! them: the ones this product installs, the ones an operator adds, and the
//! examples attached to either.

use std::fs;

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Row, params};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{Comment, Criterion, Pattern, Requirement};

use super::rows::{pattern_from_row, slugify};
use super::KnowledgeService;

impl<'a> KnowledgeService<'a> {

    pub fn pattern_add(
        &self,
        name: &str,
        category: &str,
        authority: &str,
        structure: Value,
        rationale: &str,
        scope: Value,
        required_inputs: Value,
        anti_patterns: Value,
        source_refs: Value,
        confidence: &str,
    ) -> Result<Pattern> {
        let slug = slugify(name);
        let pattern = Pattern {
            id: prefixed_id("pattern"),
            slug,
            name: name.to_owned(),
            category: category.to_owned(),
            authority: authority.to_owned(),
            status: "active".to_owned(),
            scope,
            structure,
            rationale: rationale.to_owned(),
            required_inputs,
            anti_patterns,
            source_refs,
            confidence: confidence.to_owned(),
            reviewed_at: Some(now()),
            created_at: now(),
        };
        self.db.connection.execute(
            "INSERT INTO patterns(id, slug, name, category, authority, status, scope_json, structure_json, rationale, required_inputs_json, anti_patterns_json, source_refs_json, confidence, reviewed_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15) ON CONFLICT(slug) DO UPDATE SET category = excluded.category, authority = excluded.authority, status = excluded.status, scope_json = excluded.scope_json, structure_json = excluded.structure_json, rationale = excluded.rationale, required_inputs_json = excluded.required_inputs_json, anti_patterns_json = excluded.anti_patterns_json, source_refs_json = excluded.source_refs_json, confidence = excluded.confidence, reviewed_at = excluded.reviewed_at",
            params![pattern.id, pattern.slug, pattern.name, pattern.category, pattern.authority, pattern.status, encode(&pattern.scope)?, encode(&pattern.structure)?, pattern.rationale, encode(&pattern.required_inputs)?, encode(&pattern.anti_patterns)?, encode(&pattern.source_refs)?, pattern.confidence, pattern.reviewed_at, pattern.created_at],
        )?;
        self.db.connection.query_row(
            "SELECT id, slug, name, category, authority, status, scope_json, structure_json, rationale, required_inputs_json, anti_patterns_json, source_refs_json, confidence, reviewed_at, created_at FROM patterns WHERE slug = ?1",
            [pattern.slug], pattern_from_row,
        ).map_err(Into::into)
    }

    pub fn pattern_list(&self, category: Option<&str>) -> Result<Vec<Pattern>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, slug, name, category, authority, status, scope_json, structure_json, rationale, required_inputs_json, anti_patterns_json, source_refs_json, confidence, reviewed_at, created_at FROM patterns WHERE (?1 IS NULL OR category = ?1) ORDER BY category, name",
        )?;
        let rows = statement.query_map([category], pattern_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn install_patterns(&self) -> Result<Vec<Pattern>> {
        let definitions = [
            (
                "Claim evidence chain",
                "evidence",
                json!(["claim", "measure", "source", "citation"]),
                "Every material claim must be traceable to confirmed evidence.",
                json!(["confirmed organization evidence", "official document"]),
                json!(["unsupported superlative", "invented market number"]),
            ),
            (
                "Falsifiable research hypothesis",
                "research",
                json!([
                    "state of knowledge",
                    "uncertainty",
                    "hypothesis",
                    "experiment",
                    "metric",
                    "baseline",
                    "target",
                    "failure consequence"
                ]),
                "Describe research as uncertainty resolved by an experiment, not as implementation work.",
                json!(["state of the art", "baseline", "measurable target"]),
                json!(["feature list", "platform build as research"]),
            ),
            (
                "Measurable milestone",
                "planning",
                json!([
                    "work scope",
                    "result",
                    "metric",
                    "acceptance threshold",
                    "go or no-go decision"
                ]),
                "A milestone must support an objective continuation decision.",
                json!(["task result", "measurement method"]),
                json!(["activity completion as result", "unmeasured deliverable"]),
            ),
            (
                "Cost justification",
                "budget",
                json!([
                    "resource",
                    "task link",
                    "quantity",
                    "unit rate",
                    "calculation",
                    "eligibility basis"
                ]),
                "A cost is defensible only when its necessity, calculation and task relationship are explicit.",
                json!(["scope", "supplier or rate evidence"]),
                json!(["round lump sum", "cost without task"]),
            ),
            (
                "Reviewer objection resolution",
                "review",
                json!([
                    "objection",
                    "application location",
                    "change",
                    "evidence",
                    "verification status"
                ]),
                "Review feedback remains auditable and cannot silently disappear between rounds.",
                json!(["reviewer comment", "current application version"]),
                json!(["generic acknowledgement", "closed without evidence"]),
            ),
        ];
        let mut installed = Vec::new();
        for (name, category, structure, rationale, inputs, anti_patterns) in definitions {
            installed.push(self.pattern_add(
                name,
                category,
                "internal-best-practice",
                structure,
                rationale,
                json!({ "programs": "all" }),
                inputs,
                anti_patterns,
                json!([]),
                "medium",
            )?);
        }
        Ok(installed)
    }

    pub fn example_add(
        &self,
        pattern: Option<&str>,
        application_id: Option<&str>,
        field_code: Option<&str>,
        outcome: &str,
        text: &str,
        evaluator_comment: Option<&str>,
        explanation: Option<&str>,
        source_ref: Option<&str>,
    ) -> Result<Value> {
        let pattern_id = match pattern {
            Some(value) => Some(
                self.db
                    .connection
                    .query_row(
                        "SELECT id FROM patterns WHERE id = ?1 OR slug = ?1",
                        [value],
                        |row| row.get::<_, String>("id"),
                    )
                    .optional()?
                    .context("pattern not found")?,
            ),
            None => None,
        };
        let example_id = prefixed_id("example");
        self.db.connection.execute(
            "INSERT INTO examples(id, pattern_id, application_id, field_code, outcome, text, evaluator_comment, explanation, source_ref, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![example_id, pattern_id, application_id, field_code, outcome, text, evaluator_comment, explanation, source_ref, now()],
        )?;
        Ok(json!({ "id": example_id, "pattern_id": pattern_id, "outcome": outcome }))
    }

    pub fn example_list(&self, pattern: Option<&str>, outcome: Option<&str>) -> Result<Vec<Value>> {
        let mut statement = self.db.connection.prepare(
            "SELECT e.id, e.pattern_id, p.slug AS pattern_slug, e.application_id, e.field_code, e.outcome, e.text, e.evaluator_comment, e.explanation, e.source_ref, e.created_at FROM examples e LEFT JOIN patterns p ON p.id = e.pattern_id WHERE (?1 IS NULL OR e.pattern_id = ?1 OR p.slug = ?1) AND (?2 IS NULL OR e.outcome = ?2) ORDER BY e.created_at DESC",
        )?;
        let rows = statement.query_map(params![pattern, outcome], |row| {
            Ok(json!({
                "id": row.get::<_, String>("id")?,
                "pattern_id": row.get::<_, Option<String>>("pattern_id")?,
                "pattern_slug": row.get::<_, Option<String>>("pattern_slug")?,
                "application_id": row.get::<_, Option<String>>("application_id")?,
                "field_code": row.get::<_, Option<String>>("field_code")?,
                "outcome": row.get::<_, String>("outcome")?,
                "text": row.get::<_, String>("text")?,
                "evaluator_comment": row.get::<_, Option<String>>("evaluator_comment")?,
                "explanation": row.get::<_, Option<String>>("explanation")?,
                "source_ref": row.get::<_, Option<String>>("source_ref")?,
                "created_at": row.get::<_, String>("created_at")?,
            }))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}
