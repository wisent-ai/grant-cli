//! grant-cli's services talk to the fleet database through this one client.
//! Every service borrows the `Database` immutably while a command runs, and a
//! Postgres client writes through `&mut`, so the client sits in a `RefCell`;
//! a command is one thread and never holds two borrows at once.

mod row;

use std::cell::RefCell;
use std::fmt;

use postgres::Client;
use postgres::types::ToSql;

pub use row::Row;

/// One bound parameter of a statement.
pub type Value<'a> = &'a (dyn ToSql + Sync);

#[derive(Debug)]
pub enum Error {
    /// A statement that must answer one row answered none.
    NoRows,
    Postgres(postgres::Error),
    /// A stored value does not fit the Rust type the command reads it as.
    Conversion(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRows => formatter.write_str("the fleet database answered no row"),
            Self::Postgres(error) => write!(formatter, "the fleet database refused: {error}"),
            Self::Conversion(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Postgres(error) => Some(error),
            _ => None,
        }
    }
}

impl From<postgres::Error> for Error {
    fn from(error: postgres::Error) -> Self {
        Self::Postgres(error)
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
pub struct Values<'a>(pub Vec<Value<'a>>);

/// What a statement can be bound with: `params![...]` or an array of one type.
pub trait Params {
    fn values(&self) -> Vec<Value<'_>>;
}

impl Params for Values<'_> {
    fn values(&self) -> Vec<Value<'_>> {
        self.0.clone()
    }
}

impl<T: ToSql + Sync, const N: usize> Params for [T; N] {
    fn values(&self) -> Vec<Value<'_>> {
        self.iter().map(|value| value as Value<'_>).collect()
    }
}

/// Binds values of mixed types: `params![id, name, now()]`.
macro_rules! params {
    ($($value:expr),* $(,)?) => {
        $crate::db::sql::Values(vec![$(&$value as &(dyn ::postgres::types::ToSql + Sync)),*])
    };
}
pub(crate) use params;

pub struct Connection {
    client: RefCell<Client>,
}

impl Connection {
    pub fn new(client: Client) -> Self {
        Self {
            client: RefCell::new(client),
        }
    }

    /// Several statements without parameters, as the schema files hold them.
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.client.borrow_mut().batch_execute(sql)?;
        Ok(())
    }

    pub fn execute(&self, sql: &str, params: impl Params) -> Result<u64> {
        Ok(self.client.borrow_mut().execute(sql, &params.values())?)
    }

    /// The first row the statement answers, mapped; `Error::NoRows` if none.
    pub fn query_row<T, F>(&self, sql: &str, params: impl Params, map: F) -> Result<T>
    where
        F: FnOnce(&Row<'_>) -> Result<T>,
    {
        let rows = self.client.borrow_mut().query(sql, &params.values())?;
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
        let rows = self
            .connection
            .client
            .borrow_mut()
            .query(self.sql.as_str(), &params.values())?;
        let mapped: Vec<Result<T>> = rows.iter().map(|row| map(&Row::new(row))).collect();
        Ok(mapped.into_iter())
    }
}
