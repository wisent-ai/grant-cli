use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

/// The schema is SQL, so it lives in SQL files: one file per family of
/// tables, applied in this order because each family references the one
/// before it. It is data about shape, not code.
const SCHEMA: [&str; 4] = [
    include_str!("schema/catalogue.sql"),
    include_str!("schema/work.sql"),
    include_str!("schema/knowledge.sql"),
    include_str!("schema/delivery.sql"),
];

pub struct Database {
    pub connection: Connection,
    pub home: PathBuf,
}

impl Database {
    pub fn open(home: Option<&Path>) -> Result<Self> {
        let root = match home {
            Some(path) => path.to_path_buf(),
            None => dirs::data_local_dir()
                .context("cannot determine local data directory")?
                .join("grant-cli"),
        };
        fs::create_dir_all(root.join("objects"))?;
        fs::create_dir_all(root.join("exports"))?;
        let connection = Connection::open(root.join("grant.db"))?;
        for family in SCHEMA {
            connection.execute_batch(family)?;
        }
        Ok(Self {
            connection,
            home: root,
        })
    }

    pub fn object_path(&self, digest: &str) -> PathBuf {
        self.home.join("objects").join(digest)
    }

    pub fn export_path(&self, name: &str) -> PathBuf {
        self.home.join("exports").join(name)
    }

    pub fn activity<T: Serialize>(
        &self,
        entity_type: &str,
        entity_id: &str,
        action: &str,
        data: &T,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO activity(id, entity_type, entity_id, action, data_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![prefixed_id("act"), entity_type, entity_id, action, serde_json::to_string(data)?, now()],
        )?;
        Ok(())
    }
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn prefixed_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4())
}

pub fn encode<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

pub fn decode(value: String) -> Result<Value> {
    Ok(serde_json::from_str(&value)?)
}
