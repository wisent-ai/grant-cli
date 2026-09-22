//! Reading one source from a page: fetching it, snapshotting exactly what
//! came back, and turning the listing into the opportunities this store
//! keeps, with a changed one recorded as a change rather than overwritten.

use std::fs;

use anyhow::{Context, Result, anyhow};
use quick_xml::de::from_str;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use rusqlite::{OptionalExtension, Row, params};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{OpportunityInput, Source};
use crate::opportunity::{OpportunityService, Upsert};

use super::rows::{canonical_url, empty_input, source_from_row};
use super::{SourceService, SyncReport};

impl<'a> SourceService<'a> {
    pub fn sync(&self, source_id: &str) -> Result<SyncReport> {
        let source = self.db.connection.query_row(
            "SELECT id, name, kind, url, authority, enabled, config_json, last_synced_at FROM sources WHERE id = ?1 OR name = ?1",
            [source_id], source_from_row,
        ).optional()?.context("source not found")?;
        if !source.enabled {
            return Err(anyhow!("source is disabled: {}", source.name));
        }
        let response = self.client.get(&source.url).send()?.error_for_status()?;
        let media_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = response.bytes()?.to_vec();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let object_path = self.db.object_path(&digest);
        if !object_path.exists() {
            fs::write(&object_path, &bytes)?;
        }
        let retrieved_at = now();
        let snapshot_id = self.db.connection.query_row(
            "SELECT id FROM source_snapshots WHERE source_id = ?1 AND url = ?2 AND content_hash = ?3",
            params![source.id, source.url, digest], |row| row.get::<_, String>("id"),
        ).optional()?.unwrap_or_else(|| prefixed_id("snap"));
        self.db.connection.execute(
            "INSERT OR IGNORE INTO source_snapshots(id, source_id, url, content_hash, media_type, object_path, metadata_json, retrieved_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', ?7)",
            params![snapshot_id, source.id, source.url, digest, media_type, object_path.to_string_lossy(), retrieved_at],
        )?;
        let body = String::from_utf8_lossy(&bytes);
        let inputs = self.parse(&source, &body)?;
        let opportunities = OpportunityService::new(self.db);
        let mut report = SyncReport {
            source_id: source.id.clone(),
            snapshot_id,
            discovered: inputs.len(),
            created: usize::default(),
            changed: usize::default(),
            unchanged: usize::default(),
        };
        for input in inputs {
            match opportunities.upsert(Some(&source.id), input)? {
                Upsert::Created => report.created += usize::from(true),
                Upsert::Changed => report.changed += usize::from(true),
                Upsert::Unchanged => report.unchanged += usize::from(true),
            }
        }
        self.db.connection.execute(
            "UPDATE sources SET last_synced_at = ?1 WHERE id = ?2",
            params![now(), source.id],
        )?;
        self.db.activity("source", &source.id, "synced", &report)?;
        Ok(report)
    }

    fn parse(&self, source: &Source, body: &str) -> Result<Vec<OpportunityInput>> {
        match source.kind.as_str() {
            "rss" => self.parse_rss(body),
            "atom" => self.parse_atom(body),
            "json" => self.parse_json(body),
            "html" => self.parse_html(&source.url, body),
            kind => Err(anyhow!("unsupported source kind: {kind}")),
        }
    }

    fn parse_rss(&self, body: &str) -> Result<Vec<OpportunityInput>> {
        let feed: Rss = from_str(body)?;
        Ok(feed
            .channel
            .items
            .into_iter()
            .map(|item| {
                empty_input(
                    item.title,
                    item.link,
                    item.guid,
                    item.description,
                    item.published_at,
                )
            })
            .collect())
    }

    fn parse_atom(&self, body: &str) -> Result<Vec<OpportunityInput>> {
        let feed: AtomFeed = from_str(body)?;
        Ok(feed
            .entries
            .into_iter()
            .filter_map(|entry| {
                let url = entry
                    .links
                    .iter()
                    .find(|link| link.rel.as_deref().unwrap_or("alternate") == "alternate")
                    .or_else(|| entry.links.first())?
                    .href
                    .clone();
                Some(empty_input(
                    entry.title,
                    url,
                    entry.id,
                    entry.summary.or(entry.content),
                    entry.updated,
                ))
            })
            .collect())
    }

    fn parse_json(&self, body: &str) -> Result<Vec<OpportunityInput>> {
        let value: Value = serde_json::from_str(body)?;
        let records = value
            .as_array()
            .cloned()
            .or_else(|| value.get("items").and_then(Value::as_array).cloned())
            .or_else(|| {
                value
                    .get("opportunities")
                    .and_then(Value::as_array)
                    .cloned()
            })
            .context("JSON source must be an array or contain items/opportunities")?;
        records
            .into_iter()
            .map(serde_json::from_value)
            .collect::<serde_json::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn parse_html(&self, base_url: &str, body: &str) -> Result<Vec<OpportunityInput>> {
        let document = Html::parse_document(body);
        let selector =
            Selector::parse("a[href]").map_err(|error| anyhow!("invalid selector: {error}"))?;
        let base = Url::parse(base_url)?;
        let keywords = [
            "nabór", "nabor", "konkurs", "funding", "grant", "call", "dotacj", "wsparci",
        ];
        let mut seen = std::collections::HashSet::new();
        let mut inputs = Vec::new();
        for anchor in document.select(&selector) {
            let title = anchor
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let Some(href) = anchor.value().attr("href") else {
                continue;
            };
            let searchable = format!("{} {href}", title.to_lowercase());
            if title.is_empty() || !keywords.iter().any(|keyword| searchable.contains(keyword)) {
                continue;
            }
            let Ok(url) = base.join(href) else { continue };
            let canonical = canonical_url(url);
            if seen.insert(canonical.clone()) {
                inputs.push(empty_input(title, canonical, None, None, None));
            }
        }
        Ok(inputs)
    }
}
}
