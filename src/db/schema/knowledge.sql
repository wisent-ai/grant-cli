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
