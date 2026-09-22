use std::fs;

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Row, params};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{Comment, Criterion, Pattern, Requirement};

pub struct KnowledgeService<'a> {
    pub(super) db: &'a Database,
}

#[derive(Debug, Deserialize)]
pub struct RequirementInput {
    pub authority: String,
    pub kind: String,
    pub code: Option<String>,
    pub title: String,
    pub text: String,
    pub citation: Option<String>,
    #[serde(default = "default_true")]
    pub mandatory: bool,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Deserialize)]
pub struct CriterionInput {
    pub code: Option<String>,
    pub title: String,
    pub text: String,
    #[serde(default)]
    pub gate: bool,
    pub weight: Option<f64>,
    pub citation: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct GuideExtraction {
    pub document_id: String,
    pub requirements: Vec<Requirement>,
    pub criteria: Vec<Criterion>,
}


mod comments;
mod guide;
mod patterns;
mod rows;


