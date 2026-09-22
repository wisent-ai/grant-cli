use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{ApplicationField, BudgetLine, Claim, Finding, ReviewReport};

pub struct AuthoringService<'a> {
    pub(super) db: &'a Database,
}


mod budget;
mod fields;
mod review;
mod rows;


