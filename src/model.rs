use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub url: String,
    pub authority: String,
    pub enabled: bool,
    pub config: Value,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub id: String,
    pub source_id: String,
    pub url: String,
    pub content_hash: String,
    pub media_type: Option<String>,
    pub object_path: String,
    pub retrieved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityInput {
    pub external_id: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub url: String,
    pub status: Option<String>,
    pub opens_at: Option<String>,
    pub deadline_at: Option<String>,
    pub funding_min: Option<f64>,
    pub funding_max: Option<f64>,
    pub currency: Option<String>,
    pub funding_rate: Option<f64>,
    #[serde(default)]
    pub regions: Vec<String>,
    #[serde(default)]
    pub applicant_types: Vec<String>,
    #[serde(default)]
    pub technologies: Vec<String>,
    pub trl_min: Option<f64>,
    pub trl_max: Option<f64>,
    pub consortium_required: Option<bool>,
    #[serde(default)]
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: String,
    pub source_id: Option<String>,
    pub external_id: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub url: String,
    pub status: String,
    pub opens_at: Option<String>,
    pub deadline_at: Option<String>,
    pub funding_min: Option<f64>,
    pub funding_max: Option<f64>,
    pub currency: Option<String>,
    pub funding_rate: Option<f64>,
    pub regions: Vec<String>,
    pub applicant_types: Vec<String>,
    pub technologies: Vec<String>,
    pub trl_min: Option<f64>,
    pub trl_max: Option<f64>,
    pub consortium_required: Option<bool>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub changed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityChange {
    pub id: String,
    pub opportunity_id: String,
    pub changed_fields: Vec<String>,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub profile: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub organization_id: String,
    pub kind: String,
    pub title: String,
    pub value: Value,
    pub source: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub confidence: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityRule {
    pub id: String,
    pub opportunity_id: String,
    pub name: String,
    pub expression: Value,
    pub hard_gate: bool,
    pub citation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityFinding {
    pub rule_id: String,
    pub name: String,
    pub passed: Option<bool>,
    pub hard_gate: bool,
    pub reason: String,
    pub citation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitAssessment {
    pub id: String,
    pub opportunity_id: String,
    pub organization_id: String,
    pub eligibility: String,
    pub score: Option<f64>,
    pub dimensions: Value,
    pub findings: Vec<EligibilityFinding>,
    pub assessed_at: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetLine {
    pub id: String,
    pub budget_id: String,
    pub task_code: Option<String>,
    pub category: String,
    pub research_type: Option<String>,
    pub description: String,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub unit_cost: Option<f64>,
    pub eligible_cost: f64,
    pub aid_rate: Option<f64>,
    pub requested_funding: Option<f64>,
    pub source_ref: Option<String>,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub finding_type: String,
    pub severity: String,
    pub field_id: Option<String>,
    pub message: String,
    pub basis_ref: Option<String>,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    pub id: String,
    pub application_id: String,
    pub status: String,
    pub findings: Vec<Finding>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub id: String,
    pub application_id: String,
    pub result: String,
    pub decided_at: Option<String>,
    pub awarded_amount: Option<f64>,
    pub score: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analytics {
    pub applications: i64,
    pub awarded: i64,
    pub rejected: i64,
    pub submitted: i64,
    pub requested_funding: f64,
    pub awarded_funding: f64,
    pub open_findings: i64,
    pub overdue_tasks: i64,
}
