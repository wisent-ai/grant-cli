//! What a call document says an application must contain: the requirements
//! and criteria imported from it, and the extraction that reads both out of
//! a document this store already holds.

use std::fs;

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Row, params};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{Comment, Criterion, Pattern, Requirement};

use super::rows::{criterion_from_row, requirement_from_row, classify_requirement, first_line, slugify, split_paragraphs};
use super::{CriterionInput, GuideExtraction, KnowledgeService, RequirementInput};

impl<'a> KnowledgeService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn import_requirements(
        &self,
        application_id: &str,
        document_id: Option<&str>,
        inputs: Vec<RequirementInput>,
    ) -> Result<Vec<Requirement>> {
        let mut requirements = Vec::new();
        for input in inputs {
            let requirement = Requirement {
                id: prefixed_id("req"),
                application_id: application_id.to_owned(),
                document_id: document_id.map(str::to_owned),
                authority: input.authority,
                kind: input.kind,
                code: input.code,
                title: input.title,
                text: input.text,
                citation: input.citation,
                mandatory: input.mandatory,
                metadata: input.metadata,
                created_at: now(),
            };
            self.db.connection.execute(
                "INSERT INTO requirements(id, application_id, document_id, authority, kind, code, title, text, citation, mandatory, metadata_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![requirement.id, requirement.application_id, requirement.document_id, requirement.authority, requirement.kind, requirement.code, requirement.title, requirement.text, requirement.citation, requirement.mandatory, encode(&requirement.metadata)?, requirement.created_at],
            )?;
            requirements.push(requirement);
        }
        self.db.activity(
            "application",
            application_id,
            "requirements-imported",
            &json!({ "count": requirements.len(), "document_id": document_id }),
        )?;
        Ok(requirements)
    }

    pub fn import_criteria(
        &self,
        application_id: &str,
        document_id: Option<&str>,
        inputs: Vec<CriterionInput>,
    ) -> Result<Vec<Criterion>> {
        let mut criteria = Vec::new();
        for input in inputs {
            let criterion = Criterion {
                id: prefixed_id("criterion"),
                application_id: application_id.to_owned(),
                document_id: document_id.map(str::to_owned),
                code: input.code,
                title: input.title,
                text: input.text,
                gate: input.gate,
                weight: input.weight,
                citation: input.citation,
                created_at: now(),
            };
            self.db.connection.execute(
                "INSERT INTO criteria(id, application_id, document_id, code, title, text, gate, weight, citation, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![criterion.id, criterion.application_id, criterion.document_id, criterion.code, criterion.title, criterion.text, criterion.gate, criterion.weight, criterion.citation, criterion.created_at],
            )?;
            criteria.push(criterion);
        }
        self.db.activity(
            "application",
            application_id,
            "criteria-imported",
            &json!({ "count": criteria.len(), "document_id": document_id }),
        )?;
        Ok(criteria)
    }

    pub fn extract_guide(
        &self,
        application_id: &str,
        document_id: &str,
    ) -> Result<GuideExtraction> {
        let (authority, text_path): (String, String) = self.db.connection.query_row(
            "SELECT authority, text_path FROM documents WHERE id = ?1 AND (application_id = ?2 OR application_id IS NULL)",
            params![document_id, application_id], |row| Ok((row.get("authority")?, row.get("text_path")?)),
        ).optional()?.context("document not found for application")?;
        let text = fs::read_to_string(text_path)?;
        let paragraphs = split_paragraphs(&text);
        let criterion_words = ["kryter", "criterion", "punkt", "score", "ocen"];
        let requirement_words = [
            "należy", "musi", "wymaga", "limit", "deadline", "załącz", "required", "shall", "must",
        ];
        let mut requirement_inputs = Vec::new();
        let mut criterion_inputs = Vec::new();
        for paragraph in paragraphs {
            let lowered = paragraph.to_lowercase();
            if criterion_words.iter().any(|word| lowered.contains(word)) {
                criterion_inputs.push(CriterionInput {
                    code: None,
                    title: first_line(&paragraph),
                    text: paragraph,
                    gate: lowered.contains("zero-jedynk") || lowered.contains("obligatory"),
                    weight: None,
                    citation: Some(format!("document:{document_id}")),
                });
            } else if requirement_words.iter().any(|word| lowered.contains(word)) {
                requirement_inputs.push(RequirementInput {
                    authority: authority.clone(),
                    kind: classify_requirement(&lowered).to_owned(),
                    code: None,
                    title: first_line(&paragraph),
                    text: paragraph,
                    citation: Some(format!("document:{document_id}")),
                    mandatory: true,
                    metadata: Value::Null,
                });
            }
        }
        let requirements =
            self.import_requirements(application_id, Some(document_id), requirement_inputs)?;
        let criteria = self.import_criteria(application_id, Some(document_id), criterion_inputs)?;
        Ok(GuideExtraction {
            document_id: document_id.to_owned(),
            requirements,
            criteria,
        })
    }

    pub fn requirements(&self, application_id: &str) -> Result<Vec<Requirement>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, application_id, document_id, authority, kind, code, title, text, citation, mandatory, metadata_json, created_at FROM requirements WHERE application_id = ?1 ORDER BY authority, kind, code, title",
        )?;
        let rows = statement.query_map([application_id], requirement_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn criteria(&self, application_id: &str) -> Result<Vec<Criterion>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, application_id, document_id, code, title, text, gate, weight, citation, created_at FROM criteria WHERE application_id = ?1 ORDER BY gate DESC, code, title",
        )?;
        let rows = statement.query_map([application_id], criterion_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}
