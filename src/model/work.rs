//! The application in flight: the application itself, its tasks and
//! documents, what the call requires of it, and the text written into it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub opportunity_id: String,
    pub organization_id: String,
    pub name: String,
    pub stage: String,
    pub owner: Option<String>,
    pub internal_deadline_at: Option<String>,
    pub submitted_at: Option<String>,
    pub submission_reference: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationTask {
    pub id: String,
    pub application_id: String,
    pub title: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub status: String,
    pub due_at: Option<String>,
    pub depends_on_id: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub application_id: Option<String>,
    pub opportunity_id: Option<String>,
    pub organization_id: Option<String>,
    pub kind: String,
    pub authority: String,
    pub title: String,
    pub source_uri: String,
    pub version_label: Option<String>,
    pub effective_at: Option<String>,
    pub content_hash: String,
    pub media_type: Option<String>,
    pub object_path: String,
    pub text_path: String,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub application_id: String,
    pub document_id: Option<String>,
    pub authority: String,
    pub kind: String,
    pub code: Option<String>,
    pub title: String,
    pub text: String,
    pub citation: Option<String>,
    pub mandatory: bool,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criterion {
    pub id: String,
    pub application_id: String,
    pub document_id: Option<String>,
    pub code: Option<String>,
    pub title: String,
    pub text: String,
    pub gate: bool,
    pub weight: Option<f64>,
    pub citation: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationField {
    pub id: String,
    pub application_id: String,
    pub code: String,
    pub title: String,
    pub instruction: Option<String>,
    pub char_limit: Option<usize>,
    pub value: Option<String>,
    pub status: String,
    pub metadata: Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub field_id: String,
    pub claim: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub category: String,
    pub authority: String,
    pub status: String,
    pub scope: Value,
    pub structure: Value,
    pub rationale: String,
    pub required_inputs: Value,
    pub anti_patterns: Value,
    pub source_refs: Value,
    pub confidence: String,
    pub reviewed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub application_id: String,
    pub field_id: Option<String>,
    pub comment_type: String,
    pub severity: String,
    pub body: String,
    pub basis_kind: Option<String>,
    pub basis_ref: Option<String>,
    pub suggested_actions: Value,
    pub status: String,
    pub owner: Option<String>,
    pub resolution: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

