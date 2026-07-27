use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

pub struct Database {
    pub connection: Connection,
    pub home: PathBuf,
}

impl Database {
    pub fn open(home: Option<&Path>) -> Result<Self> {
        let root = match home {
            Some(path) => path.to_path_buf(),
            None => dirs::data_local_dir()
                .context("cannot determine local data directory")?
                .join("grant-cli"),
        };
        fs::create_dir_all(root.join("objects"))?;
        fs::create_dir_all(root.join("exports"))?;
        let connection = Connection::open(root.join("grant.db"))?;
        connection.execute_batch(SCHEMA)?;
        Ok(Self {
            connection,
            home: root,
        })
    }

    pub fn object_path(&self, digest: &str) -> PathBuf {
        self.home.join("objects").join(digest)
    }

    pub fn export_path(&self, name: &str) -> PathBuf {
        self.home.join("exports").join(name)
    }

    pub fn activity<T: Serialize>(
        &self,
        entity_type: &str,
        entity_id: &str,
        action: &str,
        data: &T,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO activity(id, entity_type, entity_id, action, data_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![prefixed_id("act"), entity_type, entity_id, action, serde_json::to_string(data)?, now()],
        )?;
        Ok(())
    }
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn prefixed_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4())
}

pub fn encode<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

pub fn decode(value: String) -> Result<Value> {
    Ok(serde_json::from_str(&value)?)
}

const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sources (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    url TEXT NOT NULL,
    authority TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    config_json TEXT NOT NULL DEFAULT '{}',
    last_synced_at TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS source_snapshots (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    media_type TEXT,
    object_path TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    retrieved_at TEXT NOT NULL,
    UNIQUE(source_id, url, content_hash)
);
CREATE TABLE IF NOT EXISTS opportunities (
    id TEXT PRIMARY KEY,
    source_id TEXT REFERENCES sources(id) ON DELETE SET NULL,
    external_id TEXT,
    title TEXT NOT NULL,
    summary TEXT,
    url TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'discovered',
    opens_at TEXT,
    deadline_at TEXT,
    funding_min REAL,
    funding_max REAL,
    currency TEXT,
    funding_rate REAL,
    regions_json TEXT NOT NULL DEFAULT '[]',
    applicant_types_json TEXT NOT NULL DEFAULT '[]',
    technologies_json TEXT NOT NULL DEFAULT '[]',
    trl_min REAL,
    trl_max REAL,
    consortium_required INTEGER,
    fingerprint TEXT NOT NULL,
    raw_json TEXT NOT NULL DEFAULT '{}',
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    UNIQUE(source_id, external_id)
);
CREATE INDEX IF NOT EXISTS opportunities_deadline_idx ON opportunities(deadline_at);
CREATE INDEX IF NOT EXISTS opportunities_status_idx ON opportunities(status);
CREATE TABLE IF NOT EXISTS opportunity_changes (
    id TEXT PRIMARY KEY,
    opportunity_id TEXT NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    old_fingerprint TEXT,
    new_fingerprint TEXT NOT NULL,
    changed_fields_json TEXT NOT NULL,
    observed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS watches (
    id TEXT PRIMARY KEY,
    opportunity_id TEXT NOT NULL UNIQUE REFERENCES opportunities(id) ON DELETE CASCADE,
    label TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS organizations (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    profile_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS organization_evidence (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    value_json TEXT NOT NULL,
    source TEXT,
    valid_from TEXT,
    valid_until TEXT,
    confidence TEXT NOT NULL DEFAULT 'confirmed',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS eligibility_rules (
    id TEXT PRIMARY KEY,
    opportunity_id TEXT NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    expression_json TEXT NOT NULL,
    hard_gate INTEGER NOT NULL DEFAULT 1,
    source_snapshot_id TEXT REFERENCES source_snapshots(id) ON DELETE SET NULL,
    citation TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS fit_assessments (
    id TEXT PRIMARY KEY,
    opportunity_id TEXT NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    eligibility TEXT NOT NULL,
    score REAL,
    dimensions_json TEXT NOT NULL,
    findings_json TEXT NOT NULL,
    assessed_at TEXT NOT NULL,
    UNIQUE(opportunity_id, organization_id)
);
CREATE TABLE IF NOT EXISTS applications (
    id TEXT PRIMARY KEY,
    opportunity_id TEXT NOT NULL REFERENCES opportunities(id) ON DELETE RESTRICT,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    stage TEXT NOT NULL DEFAULT 'preparing',
    owner TEXT,
    internal_deadline_at TEXT,
    submitted_at TEXT,
    submission_reference TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS application_tasks (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    owner TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    due_at TEXT,
    depends_on_id TEXT REFERENCES application_tasks(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    completed_at TEXT
);
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    application_id TEXT REFERENCES applications(id) ON DELETE CASCADE,
    opportunity_id TEXT REFERENCES opportunities(id) ON DELETE CASCADE,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    authority TEXT NOT NULL DEFAULT 'working',
    title TEXT NOT NULL,
    source_uri TEXT NOT NULL,
    version_label TEXT,
    effective_at TEXT,
    content_hash TEXT NOT NULL,
    media_type TEXT,
    object_path TEXT NOT NULL,
    text_path TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS requirements (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    document_id TEXT REFERENCES documents(id) ON DELETE SET NULL,
    authority TEXT NOT NULL,
    kind TEXT NOT NULL,
    code TEXT,
    title TEXT NOT NULL,
    text TEXT NOT NULL,
    citation TEXT,
    mandatory INTEGER NOT NULL DEFAULT 1,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS criteria (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    document_id TEXT REFERENCES documents(id) ON DELETE SET NULL,
    code TEXT,
    title TEXT NOT NULL,
    text TEXT NOT NULL,
    gate INTEGER NOT NULL DEFAULT 0,
    weight REAL,
    citation TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS application_fields (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    title TEXT NOT NULL,
    instruction TEXT,
    char_limit INTEGER,
    value TEXT,
    status TEXT NOT NULL DEFAULT 'empty',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL,
    UNIQUE(application_id, code)
);
CREATE TABLE IF NOT EXISTS field_requirements (
    field_id TEXT NOT NULL REFERENCES application_fields(id) ON DELETE CASCADE,
    requirement_id TEXT NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    PRIMARY KEY(field_id, requirement_id)
);
CREATE TABLE IF NOT EXISTS field_criteria (
    field_id TEXT NOT NULL REFERENCES application_fields(id) ON DELETE CASCADE,
    criterion_id TEXT NOT NULL REFERENCES criteria(id) ON DELETE CASCADE,
    PRIMARY KEY(field_id, criterion_id)
);
CREATE TABLE IF NOT EXISTS field_claims (
    id TEXT PRIMARY KEY,
    field_id TEXT NOT NULL REFERENCES application_fields(id) ON DELETE CASCADE,
    claim TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unverified',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS evidence_links (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL REFERENCES field_claims(id) ON DELETE CASCADE,
    organization_evidence_id TEXT REFERENCES organization_evidence(id) ON DELETE CASCADE,
    document_id TEXT REFERENCES documents(id) ON DELETE CASCADE,
    citation TEXT,
    note TEXT,
    created_at TEXT NOT NULL,
    CHECK (organization_evidence_id IS NOT NULL OR document_id IS NOT NULL)
);
CREATE TABLE IF NOT EXISTS patterns (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    authority TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'candidate',
    scope_json TEXT NOT NULL DEFAULT '{}',
    structure_json TEXT NOT NULL,
    rationale TEXT NOT NULL,
    required_inputs_json TEXT NOT NULL DEFAULT '[]',
    anti_patterns_json TEXT NOT NULL DEFAULT '[]',
    source_refs_json TEXT NOT NULL DEFAULT '[]',
    confidence TEXT NOT NULL DEFAULT 'medium',
    reviewed_at TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS examples (
    id TEXT PRIMARY KEY,
    pattern_id TEXT REFERENCES patterns(id) ON DELETE SET NULL,
    application_id TEXT REFERENCES applications(id) ON DELETE SET NULL,
    field_code TEXT,
    outcome TEXT NOT NULL,
    text TEXT NOT NULL,
    evaluator_comment TEXT,
    explanation TEXT,
    source_ref TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    field_id TEXT REFERENCES application_fields(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    severity TEXT NOT NULL,
    body TEXT NOT NULL,
    basis_kind TEXT,
    basis_ref TEXT,
    suggested_actions_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'open',
    owner TEXT,
    resolution TEXT,
    created_at TEXT NOT NULL,
    resolved_at TEXT
);
CREATE TABLE IF NOT EXISTS budgets (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL UNIQUE REFERENCES applications(id) ON DELETE CASCADE,
    currency TEXT NOT NULL,
    indirect_method TEXT,
    indirect_rate REAL,
    private_financing_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS budget_lines (
    id TEXT PRIMARY KEY,
    budget_id TEXT NOT NULL REFERENCES budgets(id) ON DELETE CASCADE,
    task_code TEXT,
    category TEXT NOT NULL,
    research_type TEXT,
    description TEXT NOT NULL,
    quantity REAL,
    unit TEXT,
    unit_cost REAL,
    eligible_cost REAL NOT NULL,
    aid_rate REAL,
    requested_funding REAL,
    source_ref TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS reviews (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    summary_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS review_findings (
    id TEXT PRIMARY KEY,
    review_id TEXT NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,
    field_id TEXT REFERENCES application_fields(id) ON DELETE SET NULL,
    type TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    basis_ref TEXT,
    suggested_action TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS outcomes (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL UNIQUE REFERENCES applications(id) ON DELETE CASCADE,
    result TEXT NOT NULL,
    decided_at TEXT,
    awarded_amount REAL,
    score REAL,
    feedback_document_id TEXT REFERENCES documents(id) ON DELETE SET NULL,
    notes TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS activity (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    data_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);
INSERT OR REPLACE INTO metadata(key, value) VALUES ('schema', 'initial');
"#;
