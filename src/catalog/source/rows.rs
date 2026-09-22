//! Reading one row back as a source, the empty opportunity a bare listing
//! entry becomes, and the canonical form of a URL, with the tracking
//! parameters that never identify a page removed.

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


pub(super) fn source_from_row(row: &Row<'_>) -> rusqlite::Result<Source> {
    let config: String = row.get("config_json")?;
    Ok(Source {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        url: row.get("url")?,
        authority: row.get("authority")?,
        enabled: row.get("enabled")?,
        config: serde_json::from_str(&config).unwrap_or(Value::Null),
        last_synced_at: row.get("last_synced_at")?,
    })
}

pub(super) fn empty_input(
    title: String,
    url: String,
    external_id: Option<String>,
    summary: Option<String>,
    opens_at: Option<String>,
) -> OpportunityInput {
    OpportunityInput {
        external_id,
        title,
        summary,
        url,
        status: Some("discovered".to_owned()),
        opens_at,
        deadline_at: None,
        funding_min: None,
        funding_max: None,
        currency: None,
        funding_rate: None,
        regions: Vec::new(),
        applicant_types: Vec::new(),
        technologies: Vec::new(),
        trl_min: None,
        trl_max: None,
        consortium_required: None,
        raw: Value::Null,
    }
}

pub(super) fn canonical_url(mut url: Url) -> String {
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
