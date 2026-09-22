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
