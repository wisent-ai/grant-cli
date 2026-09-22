//! What came of an application: the budget lines behind its figures, the
//! review findings, the outcome recorded against it, and the numbers those
//! outcomes add up to.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
