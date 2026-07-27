use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use rusqlite::{OptionalExtension, Row, params};
use scraper::Html;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;
use zip::ZipArchive;

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::Document;

pub struct DocumentService<'a> {
    db: &'a Database,
    client: Client,
}

#[derive(Debug, Clone)]
pub struct IngestOptions {
    pub target: String,
    pub title: Option<String>,
    pub kind: String,
    pub authority: String,
    pub application_id: Option<String>,
    pub opportunity_id: Option<String>,
    pub organization_id: Option<String>,
    pub version_label: Option<String>,
    pub effective_at: Option<String>,
}

impl<'a> DocumentService<'a> {
    pub fn new(db: &'a Database) -> Result<Self> {
        Ok(Self {
            db,
            client: Client::builder()
                .user_agent("grant-cli/0.1 (+https://github.com/wisent-ai/grant-cli)")
                .build()?,
        })
    }

    pub fn ingest(&self, options: IngestOptions) -> Result<Document> {
        let (bytes, media_type, source_uri, inferred_title) =
            if options.target.starts_with("http://") || options.target.starts_with("https://") {
                let url = Url::parse(&options.target)?;
                let response = self.client.get(url.clone()).send()?.error_for_status()?;
                let media_type = response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                let bytes = response.bytes()?.to_vec();
                let title = url
                    .path_segments()
                    .and_then(Iterator::last)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("document")
                    .to_owned();
                (bytes, media_type, url.to_string(), title)
            } else {
                let path = PathBuf::from(&options.target)
                    .canonicalize()
                    .with_context(|| format!("cannot read {}", options.target))?;
                let bytes = fs::read(&path)?;
                let media_type = infer_media_type(&path);
                let title = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("document")
                    .to_owned();
                (
                    bytes,
                    media_type,
                    path.to_string_lossy().into_owned(),
                    title,
                )
            };
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let object_path = self.db.object_path(&digest);
        if !object_path.exists() {
            fs::write(&object_path, &bytes)?;
        }
        let text = extract_text(&bytes, media_type.as_deref(), &source_uri)?;
        let text_digest = format!("{:x}", Sha256::digest(text.as_bytes()));
        let text_path = self.db.object_path(&text_digest);
        if !text_path.exists() {
            fs::write(&text_path, text.as_bytes())?;
        }
        let existing = self.db.connection.query_row(
            "SELECT id, application_id, opportunity_id, organization_id, kind, authority, title, source_uri, version_label, effective_at, content_hash, media_type, object_path, text_path, metadata_json, created_at FROM documents WHERE content_hash = ?1 AND kind = ?2 AND COALESCE(application_id, '') = COALESCE(?3, '') AND COALESCE(opportunity_id, '') = COALESCE(?4, '') AND COALESCE(organization_id, '') = COALESCE(?5, '')",
            params![digest, options.kind, options.application_id, options.opportunity_id, options.organization_id], document_from_row,
        ).optional()?;
        if let Some(document) = existing {
            return Ok(document);
        }
        let document = Document {
            id: prefixed_id("doc"),
            application_id: options.application_id,
            opportunity_id: options.opportunity_id,
            organization_id: options.organization_id,
            kind: options.kind,
            authority: options.authority,
            title: options.title.unwrap_or(inferred_title),
            source_uri,
            version_label: options.version_label,
            effective_at: options.effective_at,
            content_hash: digest,
            media_type,
            object_path: object_path.to_string_lossy().into_owned(),
            text_path: text_path.to_string_lossy().into_owned(),
            metadata: json!({ "text_hash": text_digest }),
            created_at: now(),
        };
        self.db.connection.execute(
            "INSERT INTO documents(id, application_id, opportunity_id, organization_id, kind, authority, title, source_uri, version_label, effective_at, content_hash, media_type, object_path, text_path, metadata_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![document.id, document.application_id, document.opportunity_id, document.organization_id, document.kind, document.authority, document.title, document.source_uri, document.version_label, document.effective_at, document.content_hash, document.media_type, document.object_path, document.text_path, encode(&document.metadata)?, document.created_at],
        )?;
        self.db
            .activity("document", &document.id, "ingested", &document)?;
        Ok(document)
    }

    pub fn list(
        &self,
        application_id: Option<&str>,
        authority: Option<&str>,
    ) -> Result<Vec<Document>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, application_id, opportunity_id, organization_id, kind, authority, title, source_uri, version_label, effective_at, content_hash, media_type, object_path, text_path, metadata_json, created_at FROM documents WHERE (?1 IS NULL OR application_id = ?1) AND (?2 IS NULL OR authority = ?2) ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map(params![application_id, authority], document_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn text(&self, document_id: &str) -> Result<String> {
        let path: String = self
            .db
            .connection
            .query_row(
                "SELECT text_path FROM documents WHERE id = ?1",
                [document_id],
                |row| row.get("text_path"),
            )
            .optional()?
            .context("document not found")?;
        Ok(fs::read_to_string(path)?)
    }
}

fn infer_media_type(path: &Path) -> Option<String> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("pdf") => Some("application/pdf".to_owned()),
        Some("docx") => Some(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_owned(),
        ),
        Some("html") | Some("htm") => Some("text/html".to_owned()),
        Some("json") => Some("application/json".to_owned()),
        Some("xml") => Some("application/xml".to_owned()),
        Some("md") => Some("text/markdown".to_owned()),
        Some("txt") => Some("text/plain".to_owned()),
        _ => None,
    }
}

fn extract_text(bytes: &[u8], media_type: Option<&str>, source: &str) -> Result<String> {
    let kind = media_type.unwrap_or("").to_lowercase();
    if kind.contains("pdf") || source.to_lowercase().ends_with(".pdf") {
        return pdf_extract::extract_text_from_mem(bytes)
            .map_err(|error| anyhow!("PDF extraction failed: {error}"));
    }
    if kind.contains("wordprocessingml") || source.to_lowercase().ends_with(".docx") {
        let mut archive = ZipArchive::new(Cursor::new(bytes))?;
        let mut document = archive.by_name("word/document.xml")?;
        let mut xml = String::new();
        document.read_to_string(&mut xml)?;
        let paragraph = Regex::new(r"</w:p>")?;
        let tags = Regex::new(r"<[^>]+>")?;
        let separated = paragraph.replace_all(&xml, "\n");
        let plain = tags.replace_all(&separated, "");
        return Ok(quick_xml::escape::unescape(&plain)?.into_owned());
    }
    let raw = String::from_utf8_lossy(bytes);
    if kind.contains("html") || source.ends_with(".html") || source.ends_with(".htm") {
        let html = Html::parse_document(&raw);
        return Ok(html.root_element().text().collect::<Vec<_>>().join(" "));
    }
    Ok(raw.into_owned())
}

fn document_from_row(row: &Row<'_>) -> rusqlite::Result<Document> {
    let metadata: String = row.get("metadata_json")?;
    Ok(Document {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        opportunity_id: row.get("opportunity_id")?,
        organization_id: row.get("organization_id")?,
        kind: row.get("kind")?,
        authority: row.get("authority")?,
        title: row.get("title")?,
        source_uri: row.get("source_uri")?,
        version_label: row.get("version_label")?,
        effective_at: row.get("effective_at")?,
        content_hash: row.get("content_hash")?,
        media_type: row.get("media_type")?,
        object_path: row.get("object_path")?,
        text_path: row.get("text_path")?,
        metadata: serde_json::from_str(&metadata).unwrap_or(Value::Null),
        created_at: row.get("created_at")?,
    })
}
