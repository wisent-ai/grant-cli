use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{Opportunity, OpportunityChange, OpportunityInput};

pub struct OpportunityService<'a> {
    db: &'a Database,
}

pub enum Upsert {
    Created,
    Changed,
    Unchanged,
}

impl<'a> OpportunityService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn import(&self, source_id: Option<&str>, input: OpportunityInput) -> Result<Opportunity> {
        let resolved_source = match source_id {
            Some(value) => Some(
                self.db
                    .connection
                    .query_row(
                        "SELECT id FROM sources WHERE id = ?1 OR name = ?1",
                        [value],
                        |row| row.get::<_, String>("id"),
                    )
                    .optional()?
                    .context("source not found")?,
            ),
            None => None,
        };
        let url = canonical_url(Url::parse(&input.url)?);
        self.upsert(resolved_source.as_deref(), input)?;
        self.db.connection.query_row(
            "SELECT id, source_id, external_id, title, summary, url, status, opens_at, deadline_at, funding_min, funding_max, currency, funding_rate, regions_json, applicant_types_json, technologies_json, trl_min, trl_max, consortium_required, first_seen_at, last_seen_at, changed_at FROM opportunities WHERE url = ?1",
            [url], opportunity_from_row,
        ).map_err(Into::into)
    }

    pub fn upsert(&self, source_id: Option<&str>, input: OpportunityInput) -> Result<Upsert> {
        let canonical = canonical_url(
            Url::parse(&input.url)
                .with_context(|| format!("invalid opportunity URL: {}", input.url))?,
        );
        let normalized = json!({
            "title": input.title, "summary": input.summary, "status": input.status,
            "opens_at": input.opens_at, "deadline_at": input.deadline_at,
            "funding_min": input.funding_min, "funding_max": input.funding_max,
            "currency": input.currency, "funding_rate": input.funding_rate,
            "regions": input.regions, "applicant_types": input.applicant_types,
            "technologies": input.technologies, "trl_min": input.trl_min, "trl_max": input.trl_max,
            "consortium_required": input.consortium_required,
        });
        let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&normalized)?));
        let stored = json!({ "normalized": normalized, "raw": input.raw });
        let existing = self.db.connection.query_row(
            "SELECT id, fingerprint, raw_json FROM opportunities WHERE url = ?1 OR (source_id = ?2 AND external_id = ?3 AND external_id IS NOT NULL)",
            params![canonical, source_id, input.external_id],
            |row| Ok((row.get::<_, String>("id")?, row.get::<_, String>("fingerprint")?, row.get::<_, String>("raw_json")?)),
        ).optional()?;
        let timestamp = now();
        match existing {
            None => {
                let opportunity_id = prefixed_id("opp");
                self.db.connection.execute(
                    "INSERT INTO opportunities(id, source_id, external_id, title, summary, url, status, opens_at, deadline_at, funding_min, funding_max, currency, funding_rate, regions_json, applicant_types_json, technologies_json, trl_min, trl_max, consortium_required, fingerprint, raw_json, first_seen_at, last_seen_at, changed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?22, ?22)",
                    params![opportunity_id, source_id, input.external_id, input.title, input.summary, canonical, input.status.unwrap_or_else(|| "discovered".to_owned()), input.opens_at, input.deadline_at, input.funding_min, input.funding_max, input.currency, input.funding_rate, encode(&input.regions)?, encode(&input.applicant_types)?, encode(&input.technologies)?, input.trl_min, input.trl_max, input.consortium_required, fingerprint, encode(&stored)?, timestamp],
                )?;
                self.db
                    .activity("opportunity", &opportunity_id, "discovered", &stored)?;
                Ok(Upsert::Created)
            }
            Some((opportunity_id, old_fingerprint, old_json)) if old_fingerprint != fingerprint => {
                let old: Value = serde_json::from_str(&old_json).unwrap_or(Value::Null);
                let changed_fields =
                    changed_fields(old.get("normalized"), stored.get("normalized"));
                self.db.connection.execute(
                    "UPDATE opportunities SET external_id = ?1, title = ?2, summary = ?3, url = ?4, status = ?5, opens_at = ?6, deadline_at = ?7, funding_min = ?8, funding_max = ?9, currency = ?10, funding_rate = ?11, regions_json = ?12, applicant_types_json = ?13, technologies_json = ?14, trl_min = ?15, trl_max = ?16, consortium_required = ?17, fingerprint = ?18, raw_json = ?19, last_seen_at = ?20, changed_at = ?20 WHERE id = ?21",
                    params![input.external_id, input.title, input.summary, canonical, input.status.unwrap_or_else(|| "discovered".to_owned()), input.opens_at, input.deadline_at, input.funding_min, input.funding_max, input.currency, input.funding_rate, encode(&input.regions)?, encode(&input.applicant_types)?, encode(&input.technologies)?, input.trl_min, input.trl_max, input.consortium_required, fingerprint, encode(&stored)?, timestamp, opportunity_id],
                )?;
                self.db.connection.execute(
                    "INSERT INTO opportunity_changes(id, opportunity_id, old_fingerprint, new_fingerprint, changed_fields_json, observed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![prefixed_id("chg"), opportunity_id, old_fingerprint, fingerprint, encode(&changed_fields)?, timestamp],
                )?;
                self.db
                    .activity("opportunity", &opportunity_id, "changed", &changed_fields)?;
                Ok(Upsert::Changed)
            }
            Some((opportunity_id, _, _)) => {
                self.db.connection.execute(
                    "UPDATE opportunities SET last_seen_at = ?1 WHERE id = ?2",
                    params![timestamp, opportunity_id],
                )?;
                Ok(Upsert::Unchanged)
            }
        }
    }

    pub fn search(
        &self,
        query: Option<&str>,
        status: Option<&str>,
        region: Option<&str>,
        technology: Option<&str>,
        deadline_before: Option<&str>,
        watched: bool,
    ) -> Result<Vec<Opportunity>> {
        let needle = query.map(|value| format!("%{value}%"));
        let region_needle = region.map(|value| format!("%\"{value}\"%"));
        let technology_needle = technology.map(|value| format!("%\"{value}\"%"));
        let mut statement = self.db.connection.prepare(
            "SELECT o.id, o.source_id, o.external_id, o.title, o.summary, o.url, o.status, o.opens_at, o.deadline_at, o.funding_min, o.funding_max, o.currency, o.funding_rate, o.regions_json, o.applicant_types_json, o.technologies_json, o.trl_min, o.trl_max, o.consortium_required, o.first_seen_at, o.last_seen_at, o.changed_at FROM opportunities o LEFT JOIN watches w ON w.opportunity_id = o.id WHERE (?1 IS NULL OR o.title LIKE ?1 OR o.summary LIKE ?1) AND (?2 IS NULL OR o.status = ?2) AND (?3 IS NULL OR o.regions_json LIKE ?3) AND (?4 IS NULL OR o.technologies_json LIKE ?4) AND (?5 IS NULL OR o.deadline_at <= ?5) AND (?6 = 0 OR w.id IS NOT NULL) ORDER BY COALESCE(o.deadline_at, 'Z') ASC, o.changed_at DESC",
        )?;
        let rows = statement.query_map(
            params![
                needle,
                status,
                region_needle,
                technology_needle,
                deadline_before,
                watched
            ],
            opportunity_from_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn watch(&self, opportunity_id: &str, label: Option<&str>) -> Result<Value> {
        let resolved = self.resolve(opportunity_id)?;
        self.db.connection.execute(
            "INSERT INTO watches(id, opportunity_id, label, created_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(opportunity_id) DO UPDATE SET label = excluded.label",
            params![prefixed_id("watch"), resolved, label, now()],
        )?;
        self.db.activity(
            "opportunity",
            &resolved,
            "watched",
            &json!({ "label": label }),
        )?;
        Ok(json!({ "opportunity_id": resolved, "watched": true, "label": label }))
    }

    pub fn changes(&self, opportunity_id: &str) -> Result<Vec<OpportunityChange>> {
        let resolved = self.resolve(opportunity_id)?;
        let mut statement = self.db.connection.prepare(
            "SELECT id, opportunity_id, changed_fields_json, observed_at FROM opportunity_changes WHERE opportunity_id = ?1 ORDER BY observed_at DESC",
        )?;
        let rows = statement.query_map([resolved], |row| {
            let changed_fields: String = row.get("changed_fields_json")?;
            Ok(OpportunityChange {
                id: row.get("id")?,
                opportunity_id: row.get("opportunity_id")?,
                changed_fields: serde_json::from_str(&changed_fields).unwrap_or_default(),
                observed_at: row.get("observed_at")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn resolve(&self, value: &str) -> Result<String> {
        self.db
            .connection
            .query_row(
                "SELECT id FROM opportunities WHERE id = ?1 OR external_id = ?1 OR url = ?1",
                [value],
                |row| row.get("id"),
            )
            .optional()?
            .context("opportunity not found")
    }
}

fn opportunity_from_row(row: &Row<'_>) -> rusqlite::Result<Opportunity> {
    let regions: String = row.get("regions_json")?;
    let applicant_types: String = row.get("applicant_types_json")?;
    let technologies: String = row.get("technologies_json")?;
    Ok(Opportunity {
        id: row.get("id")?,
        source_id: row.get("source_id")?,
        external_id: row.get("external_id")?,
        title: row.get("title")?,
        summary: row.get("summary")?,
        url: row.get("url")?,
        status: row.get("status")?,
        opens_at: row.get("opens_at")?,
        deadline_at: row.get("deadline_at")?,
        funding_min: row.get("funding_min")?,
        funding_max: row.get("funding_max")?,
        currency: row.get("currency")?,
        funding_rate: row.get("funding_rate")?,
        regions: serde_json::from_str(&regions).unwrap_or_default(),
        applicant_types: serde_json::from_str(&applicant_types).unwrap_or_default(),
        technologies: serde_json::from_str(&technologies).unwrap_or_default(),
        trl_min: row.get("trl_min")?,
        trl_max: row.get("trl_max")?,
        consortium_required: row.get("consortium_required")?,
        first_seen_at: row.get("first_seen_at")?,
        last_seen_at: row.get("last_seen_at")?,
        changed_at: row.get("changed_at")?,
    })
}

fn canonical_url(mut url: Url) -> String {
    url.set_fragment(None);
    let tracking = ["fbclid", "gclid"];
    let retained: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| !key.starts_with("utm_") && !tracking.contains(&key.as_ref()))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.query_pairs_mut().clear().extend_pairs(retained);
    url.to_string()
}

fn changed_fields(old: Option<&Value>, new: Option<&Value>) -> Vec<String> {
    let mut fields = Vec::new();
    if let Some(values) = new.and_then(Value::as_object) {
        for (key, value) in values {
            if old
                .and_then(Value::as_object)
                .and_then(|entries| entries.get(key))
                != Some(value)
            {
                fields.push(key.clone());
            }
        }
    }
    if fields.is_empty() {
        fields.push("normalized_fields".to_owned());
    }
    fields
}
