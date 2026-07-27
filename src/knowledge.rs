use std::fs;

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Row, params};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{Comment, Criterion, Pattern, Requirement};

pub struct KnowledgeService<'a> {
    db: &'a Database,
}

#[derive(Debug, Deserialize)]
pub struct RequirementInput {
    pub authority: String,
    pub kind: String,
    pub code: Option<String>,
    pub title: String,
    pub text: String,
    pub citation: Option<String>,
    #[serde(default = "default_true")]
    pub mandatory: bool,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Deserialize)]
pub struct CriterionInput {
    pub code: Option<String>,
    pub title: String,
    pub text: String,
    #[serde(default)]
    pub gate: bool,
    pub weight: Option<f64>,
    pub citation: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct GuideExtraction {
    pub document_id: String,
    pub requirements: Vec<Requirement>,
    pub criteria: Vec<Criterion>,
}

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

    pub fn comment_add(
        &self,
        application_id: &str,
        field_id: Option<&str>,
        comment_type: &str,
        severity: &str,
        body: &str,
        basis_kind: Option<&str>,
        basis_ref: Option<&str>,
        actions: Value,
        owner: Option<&str>,
    ) -> Result<Comment> {
        let comment = Comment {
            id: prefixed_id("comment"),
            application_id: application_id.to_owned(),
            field_id: field_id.map(str::to_owned),
            comment_type: comment_type.to_owned(),
            severity: severity.to_owned(),
            body: body.to_owned(),
            basis_kind: basis_kind.map(str::to_owned),
            basis_ref: basis_ref.map(str::to_owned),
            suggested_actions: actions,
            status: "open".to_owned(),
            owner: owner.map(str::to_owned),
            resolution: None,
            created_at: now(),
            resolved_at: None,
        };
        self.db.connection.execute(
            "INSERT INTO comments(id, application_id, field_id, type, severity, body, basis_kind, basis_ref, suggested_actions_json, status, owner, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![comment.id, comment.application_id, comment.field_id, comment.comment_type, comment.severity, comment.body, comment.basis_kind, comment.basis_ref, encode(&comment.suggested_actions)?, comment.status, comment.owner, comment.created_at],
        )?;
        self.db
            .activity("application", application_id, "comment-added", &comment)?;
        Ok(comment)
    }

    pub fn comment_list(&self, application_id: &str, status: Option<&str>) -> Result<Vec<Comment>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, application_id, field_id, type, severity, body, basis_kind, basis_ref, suggested_actions_json, status, owner, resolution, created_at, resolved_at FROM comments WHERE application_id = ?1 AND (?2 IS NULL OR status = ?2) ORDER BY CASE severity WHEN 'blocker' THEN 0 WHEN 'error' THEN 1 WHEN 'warning' THEN 2 ELSE 3 END, created_at",
        )?;
        let rows = statement.query_map(params![application_id, status], comment_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn comment_resolve(&self, comment_id: &str, resolution: &str) -> Result<Comment> {
        self.db.connection.execute(
            "UPDATE comments SET status = 'resolved', resolution = ?1, resolved_at = ?2 WHERE id = ?3", params![resolution, now(), comment_id],
        )?;
        self.db.connection.query_row(
            "SELECT id, application_id, field_id, type, severity, body, basis_kind, basis_ref, suggested_actions_json, status, owner, resolution, created_at, resolved_at FROM comments WHERE id = ?1",
            [comment_id], comment_from_row,
        ).optional()?.context("comment not found")
    }
}

fn default_true() -> bool {
    true
}
fn first_line(value: &str) -> String {
    value
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Extracted item")
        .trim()
        .to_owned()
}
fn classify_requirement(value: &str) -> &'static str {
    if value.contains("limit") || value.contains("znak") {
        "field-limit"
    } else if value.contains("załącz") || value.contains("attachment") {
        "attachment"
    } else if value.contains("deadline") || value.contains("termin") {
        "deadline"
    } else {
        "instruction"
    }
}
fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
        .collect()
}
fn slugify(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn requirement_from_row(row: &Row<'_>) -> rusqlite::Result<Requirement> {
    let metadata: String = row.get("metadata_json")?;
    Ok(Requirement {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        document_id: row.get("document_id")?,
        authority: row.get("authority")?,
        kind: row.get("kind")?,
        code: row.get("code")?,
        title: row.get("title")?,
        text: row.get("text")?,
        citation: row.get("citation")?,
        mandatory: row.get("mandatory")?,
        metadata: serde_json::from_str(&metadata).unwrap_or(Value::Null),
        created_at: row.get("created_at")?,
    })
}
fn criterion_from_row(row: &Row<'_>) -> rusqlite::Result<Criterion> {
    Ok(Criterion {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        document_id: row.get("document_id")?,
        code: row.get("code")?,
        title: row.get("title")?,
        text: row.get("text")?,
        gate: row.get("gate")?,
        weight: row.get("weight")?,
        citation: row.get("citation")?,
        created_at: row.get("created_at")?,
    })
}
fn pattern_from_row(row: &Row<'_>) -> rusqlite::Result<Pattern> {
    let scope: String = row.get("scope_json")?;
    let structure: String = row.get("structure_json")?;
    let required_inputs: String = row.get("required_inputs_json")?;
    let anti_patterns: String = row.get("anti_patterns_json")?;
    let source_refs: String = row.get("source_refs_json")?;
    Ok(Pattern {
        id: row.get("id")?,
        slug: row.get("slug")?,
        name: row.get("name")?,
        category: row.get("category")?,
        authority: row.get("authority")?,
        status: row.get("status")?,
        scope: serde_json::from_str(&scope).unwrap_or(Value::Null),
        structure: serde_json::from_str(&structure).unwrap_or(Value::Null),
        rationale: row.get("rationale")?,
        required_inputs: serde_json::from_str(&required_inputs).unwrap_or(Value::Null),
        anti_patterns: serde_json::from_str(&anti_patterns).unwrap_or(Value::Null),
        source_refs: serde_json::from_str(&source_refs).unwrap_or(Value::Null),
        confidence: row.get("confidence")?,
        reviewed_at: row.get("reviewed_at")?,
        created_at: row.get("created_at")?,
    })
}
fn comment_from_row(row: &Row<'_>) -> rusqlite::Result<Comment> {
    let actions: String = row.get("suggested_actions_json")?;
    Ok(Comment {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        field_id: row.get("field_id")?,
        comment_type: row.get("type")?,
        severity: row.get("severity")?,
        body: row.get("body")?,
        basis_kind: row.get("basis_kind")?,
        basis_ref: row.get("basis_ref")?,
        suggested_actions: serde_json::from_str(&actions).unwrap_or(Value::Null),
        status: row.get("status")?,
        owner: row.get("owner")?,
        resolution: row.get("resolution")?,
        created_at: row.get("created_at")?,
        resolved_at: row.get("resolved_at")?,
    })
}
