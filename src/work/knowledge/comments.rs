//! The comments recorded against an application: adding one, reading them
//! in the order their severity demands, and resolving one.

use std::fs;

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Row, params};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::{Database, encode, now, prefixed_id};
use crate::model::{Comment, Criterion, Pattern, Requirement};

use super::rows::comment_from_row;
use super::KnowledgeService;

impl<'a> KnowledgeService<'a> {

    pub fn comment_add(
        &self,
        application_id: &str,
        field_id: Option<&str>,
        comment_type: &str,
        severity: &str,
        body: &str,
        basis_kind: Option<&str>,
        basis_ref: Option<&str>,
        actions: Value,
        owner: Option<&str>,
    ) -> Result<Comment> {
        let comment = Comment {
            id: prefixed_id("comment"),
            application_id: application_id.to_owned(),
            field_id: field_id.map(str::to_owned),
            comment_type: comment_type.to_owned(),
            severity: severity.to_owned(),
            body: body.to_owned(),
            basis_kind: basis_kind.map(str::to_owned),
            basis_ref: basis_ref.map(str::to_owned),
            suggested_actions: actions,
            status: "open".to_owned(),
            owner: owner.map(str::to_owned),
            resolution: None,
            created_at: now(),
            resolved_at: None,
        };
        self.db.connection.execute(
            "INSERT INTO comments(id, application_id, field_id, type, severity, body, basis_kind, basis_ref, suggested_actions_json, status, owner, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![comment.id, comment.application_id, comment.field_id, comment.comment_type, comment.severity, comment.body, comment.basis_kind, comment.basis_ref, encode(&comment.suggested_actions)?, comment.status, comment.owner, comment.created_at],
        )?;
        self.db
            .activity("application", application_id, "comment-added", &comment)?;
        Ok(comment)
    }

    pub fn comment_list(&self, application_id: &str, status: Option<&str>) -> Result<Vec<Comment>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, application_id, field_id, type, severity, body, basis_kind, basis_ref, suggested_actions_json, status, owner, resolution, created_at, resolved_at FROM comments WHERE application_id = ?1 AND (?2 IS NULL OR status = ?2) ORDER BY CASE severity WHEN 'blocker' THEN 0 WHEN 'error' THEN 1 WHEN 'warning' THEN 2 ELSE 3 END, created_at",
        )?;
        let rows = statement.query_map(params![application_id, status], comment_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn comment_resolve(&self, comment_id: &str, resolution: &str) -> Result<Comment> {
        self.db.connection.execute(
            "UPDATE comments SET status = 'resolved', resolution = ?1, resolved_at = ?2 WHERE id = ?3", params![resolution, now(), comment_id],
        )?;
        self.db.connection.query_row(
            "SELECT id, application_id, field_id, type, severity, body, basis_kind, basis_ref, suggested_actions_json, status, owner, resolution, created_at, resolved_at FROM comments WHERE id = ?1",
            [comment_id], comment_from_row,
        ).optional()?.context("comment not found")
    }
}
}
