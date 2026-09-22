//! The catalogue an application is written against: a source, a snapshot of
//! it, an opportunity, the organization applying, and whether it qualifies.

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

