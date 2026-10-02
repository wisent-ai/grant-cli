//! Taking back what `field link-requirement`, `field link-criterion` and
//! `claim link` recorded. Each refuses a link that does not exist, so a
//! mistyped id is never a silent success.

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::db::sql::{OptionalExtension, params};

use super::authoring::AuthoringService;

impl<'a> AuthoringService<'a> {
    pub fn field_unlink_requirement(&self, application_id: &str, code: &str, requirement_id: &str) -> Result<Value> {
        let field = self.field_get(application_id, code)?;
        let removed = self.db.connection.execute(
            "DELETE FROM field_requirements WHERE field_id = $1 AND requirement_id = $2",
            params![field.id, requirement_id],
        )?;
        if removed == 0 {
            return Err(anyhow!("field {code} is not linked to requirement {requirement_id}"));
        }
        Ok(json!({ "field_id": field.id, "requirement_id": requirement_id, "linked": false }))
    }

    pub fn field_unlink_criterion(&self, application_id: &str, code: &str, criterion_id: &str) -> Result<Value> {
        let field = self.field_get(application_id, code)?;
        let removed = self.db.connection.execute(
            "DELETE FROM field_criteria WHERE field_id = $1 AND criterion_id = $2",
            params![field.id, criterion_id],
        )?;
        if removed == 0 {
            return Err(anyhow!("field {code} is not linked to criterion {criterion_id}"));
        }
        Ok(json!({ "field_id": field.id, "criterion_id": criterion_id, "linked": false }))
    }

    /// Remove one evidence link by the id `claim link` printed. A claim left
    /// with no link goes back from `supported` to `unverified`, the status
    /// it had before it was linked.
    pub fn claim_unlink(&self, link_id: &str) -> Result<Value> {
        let claim_id: String = self.db.connection.query_row(
            "SELECT claim_id FROM evidence_links WHERE id = $1", [link_id], |row| row.get("claim_id"),
        ).optional()?.ok_or_else(|| anyhow!("evidence link {link_id} not found"))?;
        self.db.connection.execute("DELETE FROM evidence_links WHERE id = $1", [link_id])?;
        self.db.connection.execute(
            "UPDATE field_claims SET status = 'unverified' WHERE id = $1 AND status = 'supported' AND NOT EXISTS (SELECT 1 FROM evidence_links WHERE claim_id = $1)",
            [&claim_id],
        )?;
        let status: String = self.db.connection.query_row(
            "SELECT status FROM field_claims WHERE id = $1", [&claim_id], |row| row.get("status"),
        )?;
        Ok(json!({ "id": link_id, "claim_id": claim_id, "claim_status": status }))
    }
}
