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

pub struct SourceService<'a> {
    db: &'a Database,
    client: Client,
}

#[derive(Debug, serde::Serialize)]
pub struct SyncReport {
    pub source_id: String,
    pub snapshot_id: String,
    pub discovered: usize,
    pub created: usize,
    pub changed: usize,
    pub unchanged: usize,
}

#[derive(Debug, Deserialize)]
struct Rss {
    channel: RssChannel,
}
#[derive(Debug, Deserialize)]
struct RssChannel {
    #[serde(rename = "item", default)]
    items: Vec<RssItem>,
}
#[derive(Debug, Deserialize)]
struct RssItem {
    title: String,
    link: String,
    description: Option<String>,
    guid: Option<String>,
    #[serde(rename = "pubDate")]
    published_at: Option<String>,
}
#[derive(Debug, Deserialize)]
struct AtomFeed {
    #[serde(rename = "entry", default)]
    entries: Vec<AtomEntry>,
}
#[derive(Debug, Deserialize)]
struct AtomEntry {
    title: String,
    id: Option<String>,
    summary: Option<String>,
    content: Option<String>,
    updated: Option<String>,
    #[serde(rename = "link", default)]
    links: Vec<AtomLink>,
}
#[derive(Debug, Deserialize)]
struct AtomLink {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@rel")]
    rel: Option<String>,
}

impl<'a> SourceService<'a> {
    pub fn new(db: &'a Database) -> Result<Self> {
        let client = Client::builder()
            .user_agent("grant-cli/0.1 (+https://github.com/wisent-ai/grant-cli)")
            .build()?;
        Ok(Self { db, client })
    }

    pub fn register(
        &self,
        name: &str,
        kind: &str,
        url: &str,
        authority: &str,
        config: Value,
    ) -> Result<Source> {
        Url::parse(url).with_context(|| format!("invalid source URL: {url}"))?;
        if !["rss", "atom", "json", "html"].contains(&kind) {
            return Err(anyhow!("unsupported source kind: {kind}"));
        }
        let source = Source {
            id: prefixed_id("src"),
            name: name.to_owned(),
            kind: kind.to_owned(),
            url: url.to_owned(),
            authority: authority.to_owned(),
            enabled: true,
            config,
            last_synced_at: None,
        };
        self.db.connection.execute(
            "INSERT INTO sources(id, name, kind, url, authority, enabled, config_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![source.id, source.name, source.kind, source.url, source.authority, source.enabled, encode(&source.config)?, now()],
        )?;
        self.db
            .activity("source", &source.id, "registered", &source)?;
        Ok(source)
    }

    pub fn list(&self) -> Result<Vec<Source>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, name, kind, url, authority, enabled, config_json, last_synced_at FROM sources ORDER BY name",
        )?;
        let rows = statement.query_map([], source_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn snapshots(&self, source: Option<&str>) -> Result<Vec<Value>> {
        let mut statement = self.db.connection.prepare(
            "SELECT s.id, s.source_id, r.name AS source_name, s.url, s.content_hash, s.media_type, s.object_path, s.metadata_json, s.retrieved_at FROM source_snapshots s JOIN sources r ON r.id = s.source_id WHERE (?1 IS NULL OR s.source_id = ?1 OR r.name = ?1) ORDER BY s.retrieved_at DESC",
        )?;
        let rows = statement.query_map([source], |row| {
            let metadata: String = row.get("metadata_json")?;
            Ok(json!({
                "id": row.get::<_, String>("id")?,
                "source_id": row.get::<_, String>("source_id")?,
                "source_name": row.get::<_, String>("source_name")?,
                "url": row.get::<_, String>("url")?,
                "content_hash": row.get::<_, String>("content_hash")?,
                "media_type": row.get::<_, Option<String>>("media_type")?,
                "object_path": row.get::<_, String>("object_path")?,
                "metadata": serde_json::from_str::<Value>(&metadata).unwrap_or(Value::Null),
                "retrieved_at": row.get::<_, String>("retrieved_at")?,
            }))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn install_catalog(&self) -> Result<Vec<Source>> {
        let catalog = [
            (
                "fundusze-europejskie",
                "html",
                "https://funduszeeuropejskie.gov.pl/nabory-wnioskow/",
                "Ministerstwo Funduszy i Polityki Regionalnej",
            ),
            (
                "parp",
                "html",
                "https://www.parp.gov.pl/component/grants/grants",
                "Polska Agencja Rozwoju Przedsiębiorczości",
            ),
            (
                "ncbr",
                "html",
                "https://www.gov.pl/web/ncbr/aktualne-konkursy",
                "Narodowe Centrum Badań i Rozwoju",
            ),
            (
                "eic",
                "html",
                "https://eic.ec.europa.eu/eic-funding-opportunities_en",
                "European Innovation Council",
            ),
            (
                "eu-funding-tenders",
                "html",
                "https://ec.europa.eu/info/funding-tenders/opportunities/portal/screen/opportunities/calls-for-proposals",
                "European Commission",
            ),
        ];
        let mut installed = Vec::new();
        for (name, kind, url, authority) in catalog {
            let existing = self.db.connection.query_row(
                "SELECT id, name, kind, url, authority, enabled, config_json, last_synced_at FROM sources WHERE name = ?1",
                [name], source_from_row,
            ).optional()?;
            installed.push(match existing {
                Some(source) => source,
                None => self.register(name, kind, url, authority, json!({ "official": true }))?,
            });
        }
        Ok(installed)
    }

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

fn source_from_row(row: &Row<'_>) -> rusqlite::Result<Source> {
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

fn empty_input(
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
