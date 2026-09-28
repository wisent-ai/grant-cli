//! grant-cli's services talk to the fleet database through this one client,
//! a SeaORM connection `stado_database::connect` opens. The services run one
//! command on one thread and read synchronously, so the connection carries a
//! current-thread runtime and each statement blocks on it.

mod bind;
mod row;

use std::fmt;

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement as SeaStatement};
use tokio::runtime::Runtime;

pub use bind::Bind;
pub use row::Row;

#[derive(Debug)]
pub enum Error {
    /// A statement that must answer one row answered none.
    NoRows,
    Database(DbErr),
    /// A stored value does not fit the Rust type the command reads it as.
    Conversion(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRows => formatter.write_str("the fleet database answered no row"),
            Self::Database(error) => write!(formatter, "the fleet database refused: {error}"),
            Self::Conversion(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DbErr> for Error {
    fn from(error: DbErr) -> Self {
        Self::Database(error)
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(error: std::num::TryFromIntError) -> Self {
        Self::Conversion(format!("a stored integer does not fit: {error}"))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Turns "no row" into `None` for statements whose row may be absent.
pub trait OptionalExtension<T> {
    fn optional(self) -> Result<Option<T>>;
}

impl<T> OptionalExtension<T> for Result<T> {
    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(Error::NoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

/// Parameters of mixed types, built by `params!`.
pub struct Values(pub Vec<sea_orm::Value>);

/// What a statement can be bound with: `params![...]` or an array of one type.
pub trait Params {
    fn values(&self) -> Vec<sea_orm::Value>;
}

impl Params for Values {
    fn values(&self) -> Vec<sea_orm::Value> {
        self.0.clone()
    }
}

impl<T: Bind, const N: usize> Params for [T; N] {
    fn values(&self) -> Vec<sea_orm::Value> {
        self.iter().map(Bind::bind).collect()
    }
}

/// Binds values of mixed types: `params![id, name, now()]`.
macro_rules! params {
    ($($value:expr),* $(,)?) => {
        $crate::db::sql::Values(vec![$($crate::db::sql::Bind::bind(&$value)),*])
    };
}
pub(crate) use params;

pub struct Connection {
    runtime: Runtime,
    database: DatabaseConnection,
}

impl Connection {
    /// The fleet database `grant-cli`, resolved through Stado and Skarbiec.
    /// A refusal names the step that failed and what Stado answered.
    pub fn open() -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let fleet = stado_database::FleetDatabase::for_product("grant-cli", "GRANT_FLEET_HOME")?;
        let database = runtime.block_on(stado_database::connect(&fleet))?;
        Ok(Self { runtime, database })
    }

    fn statement(sql: &str, params: impl Params) -> SeaStatement {
        SeaStatement::from_sql_and_values(DbBackend::Postgres, sql, params.values())
    }

    /// Several statements without parameters, as the schema files hold them.
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.runtime.block_on(self.database.execute_unprepared(sql))?;
        Ok(())
    }

    pub fn execute(&self, sql: &str, params: impl Params) -> Result<u64> {
        let done = self
            .runtime
            .block_on(self.database.execute(Self::statement(sql, params)))?;
        Ok(done.rows_affected())
    }

    fn query(&self, sql: &str, params: impl Params) -> Result<Vec<sea_orm::QueryResult>> {
        Ok(self
            .runtime
            .block_on(self.database.query_all(Self::statement(sql, params)))?)
    }

    /// The first row the statement answers, mapped; `Error::NoRows` if none.
    pub fn query_row<T, F>(&self, sql: &str, params: impl Params, map: F) -> Result<T>
    where
        F: FnOnce(&Row<'_>) -> Result<T>,
    {
        let rows = self.query(sql, params)?;
        let first = rows.first().ok_or(Error::NoRows)?;
        map(&Row::new(first))
    }

    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>> {
        Ok(Statement {
            connection: self,
            sql: sql.to_owned(),
        })
    }
}

/// A statement answering many rows.
pub struct Statement<'c> {
    connection: &'c Connection,
    sql: String,
}

impl Statement<'_> {
    pub fn query_map<T, F>(
        &mut self,
        params: impl Params,
        mut map: F,
    ) -> Result<std::vec::IntoIter<Result<T>>>
    where
        F: FnMut(&Row<'_>) -> Result<T>,
    {
        let rows = self.connection.query(self.sql.as_str(), params)?;
        let mapped: Vec<Result<T>> = rows.iter().map(|row| map(&Row::new(row))).collect();
        Ok(mapped.into_iter())
    }
}
