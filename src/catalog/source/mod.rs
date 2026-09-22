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
    pub(super) db: &'a Database,
    pub(super) client: Client,
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


mod rows;
mod sync;

use rows::{canonical_url, empty_input, source_from_row};
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

}
