use anyhow::{Context, Result, anyhow};
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::db::{Database, now, prefixed_id};
use crate::model::{Application, ApplicationTask};

pub struct ApplicationService<'a> {
    db: &'a Database,
}

impl<'a> ApplicationService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        opportunity: &str,
        organization: &str,
        name: &str,
        owner: Option<&str>,
        internal_deadline: Option<&str>,
    ) -> Result<Application> {
        let opportunity_id: String = self
            .db
            .connection
            .query_row(
                "SELECT id FROM opportunities WHERE id = ?1 OR external_id = ?1 OR url = ?1",
                [opportunity],
                |row| row.get("id"),
            )
            .optional()?
            .context("opportunity not found")?;
        let organization_id: String = self
            .db
            .connection
            .query_row(
                "SELECT id FROM organizations WHERE id = ?1 OR slug = ?1",
                [organization],
                |row| row.get("id"),
            )
            .optional()?
            .context("organization not found")?;
        let timestamp = now();
        let application = Application {
            id: prefixed_id("app"),
            opportunity_id,
            organization_id,
            name: name.to_owned(),
            stage: "preparing".to_owned(),
            owner: owner.map(str::to_owned),
            internal_deadline_at: internal_deadline.map(str::to_owned),
            submitted_at: None,
            submission_reference: None,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        self.db.connection.execute(
            "INSERT INTO applications(id, opportunity_id, organization_id, name, stage, owner, internal_deadline_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![application.id, application.opportunity_id, application.organization_id, application.name, application.stage, application.owner, application.internal_deadline_at, application.created_at],
        )?;
        self.db
            .activity("application", &application.id, "created", &application)?;
        Ok(application)
    }

    pub fn get(&self, application_id: &str) -> Result<Application> {
        self.db.connection.query_row(
            "SELECT id, opportunity_id, organization_id, name, stage, owner, internal_deadline_at, submitted_at, submission_reference, created_at, updated_at FROM applications WHERE id = ?1 OR name = ?1",
            [application_id], application_from_row,
        ).optional()?.context("application not found")
    }

    pub fn list(&self, stage: Option<&str>) -> Result<Vec<Application>> {
        let mut statement = self.db.connection.prepare(
            "SELECT id, opportunity_id, organization_id, name, stage, owner, internal_deadline_at, submitted_at, submission_reference, created_at, updated_at FROM applications WHERE (?1 IS NULL OR stage = ?1) ORDER BY COALESCE(internal_deadline_at, 'Z'), updated_at DESC",
        )?;
        let rows = statement.query_map([stage], application_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn set_stage(
        &self,
        application_id: &str,
        stage: &str,
        submission_reference: Option<&str>,
    ) -> Result<Application> {
        let stages = [
            "preparing",
            "internal-review",
            "external-review",
            "ready",
            "submitted",
            "clarification",
            "awarded",
            "rejected",
            "withdrawn",
            "archived",
        ];
        if !stages.contains(&stage) {
            return Err(anyhow!("unsupported application stage: {stage}"));
        }
        let application = self.get(application_id)?;
        let submitted_at = if stage == "submitted" {
            Some(now())
        } else {
            application.submitted_at
        };
        self.db.connection.execute(
            "UPDATE applications SET stage = ?1, submitted_at = ?2, submission_reference = COALESCE(?3, submission_reference), updated_at = ?4 WHERE id = ?5",
            params![stage, submitted_at, submission_reference, now(), application.id],
        )?;
        self.db.activity("application", &application.id, "stage-changed", &json!({ "from": application.stage, "to": stage, "submission_reference": submission_reference }))?;
        self.get(&application.id)
    }

    pub fn task_add(
        &self,
        application_id: &str,
        title: &str,
        description: Option<&str>,
        owner: Option<&str>,
        due_at: Option<&str>,
        depends_on: Option<&str>,
    ) -> Result<ApplicationTask> {
        let application = self.get(application_id)?;
        if let Some(dependency) = depends_on {
            self.db
                .connection
                .query_row(
                    "SELECT id FROM application_tasks WHERE id = ?1 AND application_id = ?2",
                    params![dependency, application.id],
                    |row| row.get::<_, String>("id"),
                )
                .optional()?
                .context("dependency task not found in application")?;
        }
        let task = ApplicationTask {
            id: prefixed_id("task"),
            application_id: application.id,
            title: title.to_owned(),
            description: description.map(str::to_owned),
            owner: owner.map(str::to_owned),
            status: "open".to_owned(),
            due_at: due_at.map(str::to_owned),
            depends_on_id: depends_on.map(str::to_owned),
            created_at: now(),
            completed_at: None,
        };
        self.db.connection.execute(
            "INSERT INTO application_tasks(id, application_id, title, description, owner, status, due_at, depends_on_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![task.id, task.application_id, task.title, task.description, task.owner, task.status, task.due_at, task.depends_on_id, task.created_at],
        )?;
        self.db
            .activity("application", &task.application_id, "task-added", &task)?;
        Ok(task)
    }

    pub fn task_list(
        &self,
        application_id: &str,
        status: Option<&str>,
        overdue: bool,
    ) -> Result<Vec<ApplicationTask>> {
        let application = self.get(application_id)?;
        let mut statement = self.db.connection.prepare(
            "SELECT id, application_id, title, description, owner, status, due_at, depends_on_id, created_at, completed_at FROM application_tasks WHERE application_id = ?1 AND (?2 IS NULL OR status = ?2) AND (?3 = 0 OR (status != 'done' AND due_at IS NOT NULL AND due_at < datetime('now'))) ORDER BY COALESCE(due_at, 'Z'), created_at",
        )?;
        let rows = statement.query_map(params![application.id, status, overdue], task_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn task_complete(&self, task_id: &str) -> Result<ApplicationTask> {
        let task = self.db.connection.query_row(
            "SELECT id, application_id, title, description, owner, status, due_at, depends_on_id, created_at, completed_at FROM application_tasks WHERE id = ?1",
            [task_id], task_from_row,
        ).optional()?.context("task not found")?;
        if let Some(dependency) = &task.depends_on_id {
            let dependency_status: String = self.db.connection.query_row(
                "SELECT status FROM application_tasks WHERE id = ?1",
                [dependency],
                |row| row.get("status"),
            )?;
            if dependency_status != "done" {
                return Err(anyhow!("dependency task is not complete: {dependency}"));
            }
        }
        self.db.connection.execute(
            "UPDATE application_tasks SET status = 'done', completed_at = ?1 WHERE id = ?2",
            params![now(), task.id],
        )?;
        self.db.activity(
            "application",
            &task.application_id,
            "task-completed",
            &json!({ "task_id": task.id }),
        )?;
        self.db.connection.query_row(
            "SELECT id, application_id, title, description, owner, status, due_at, depends_on_id, created_at, completed_at FROM application_tasks WHERE id = ?1",
            [task_id], task_from_row,
        ).map_err(Into::into)
    }

    pub fn dashboard(&self) -> Result<Value> {
        let applications: i64 = self.db.connection.query_row("SELECT COUNT(*) AS value FROM applications WHERE stage NOT IN ('archived', 'rejected', 'withdrawn')", [], |row| row.get("value"))?;
        let overdue_tasks: i64 = self.db.connection.query_row("SELECT COUNT(*) AS value FROM application_tasks WHERE status != 'done' AND due_at IS NOT NULL AND due_at < datetime('now')", [], |row| row.get("value"))?;
        let upcoming = self.db.connection.prepare(
            "SELECT id, name, stage, internal_deadline_at FROM applications WHERE internal_deadline_at IS NOT NULL AND stage NOT IN ('archived', 'rejected', 'withdrawn') ORDER BY internal_deadline_at LIMIT 10",
        )?.query_map([], |row| Ok(json!({ "id": row.get::<_, String>("id")?, "name": row.get::<_, String>("name")?, "stage": row.get::<_, String>("stage")?, "deadline": row.get::<_, String>("internal_deadline_at")? })))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(
            json!({ "active_applications": applications, "overdue_tasks": overdue_tasks, "upcoming_deadlines": upcoming }),
        )
    }
}

fn application_from_row(row: &Row<'_>) -> rusqlite::Result<Application> {
    Ok(Application {
        id: row.get("id")?,
        opportunity_id: row.get("opportunity_id")?,
        organization_id: row.get("organization_id")?,
        name: row.get("name")?,
        stage: row.get("stage")?,
        owner: row.get("owner")?,
        internal_deadline_at: row.get("internal_deadline_at")?,
        submitted_at: row.get("submitted_at")?,
        submission_reference: row.get("submission_reference")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn task_from_row(row: &Row<'_>) -> rusqlite::Result<ApplicationTask> {
    Ok(ApplicationTask {
        id: row.get("id")?,
        application_id: row.get("application_id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        owner: row.get("owner")?,
        status: row.get("status")?,
        due_at: row.get("due_at")?,
        depends_on_id: row.get("depends_on_id")?,
        created_at: row.get("created_at")?,
        completed_at: row.get("completed_at")?,
    })
}
