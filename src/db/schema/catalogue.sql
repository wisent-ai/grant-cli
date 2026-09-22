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
