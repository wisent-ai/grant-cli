//! One row the fleet database answered, read by column name.

use std::fmt::Display;

use postgres::row::RowIndex;
use postgres::types::{FromSql, Type};
use serde_json::{Map, Value};

use super::{Error, Result};

pub struct Row<'a>(&'a postgres::Row);

impl<'a> Row<'a> {
    pub(super) fn new(row: &'a postgres::Row) -> Self {
        Self(row)
    }

    /// One column as `T`; a NULL reads as `None` when `T` is an `Option`.
    pub fn get<I, T>(&self, index: I) -> Result<T>
    where
        I: RowIndex + Display,
        T: FromSql<'a>,
    {
        Ok(self.0.try_get(index)?)
    }

    /// Every column as a JSON object keyed by column name, for the exported
    /// application package, which carries whole rows.
    pub fn json(&self) -> Result<Value> {
        let mut object = Map::new();
        for (index, column) in self.0.columns().iter().enumerate() {
            let value = match *column.type_() {
                Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => {
                    Value::from(self.0.try_get::<_, Option<String>>(index)?)
                }
                Type::INT8 => Value::from(self.0.try_get::<_, Option<i64>>(index)?),
                Type::INT4 => Value::from(self.0.try_get::<_, Option<i32>>(index)?),
                Type::INT2 => Value::from(self.0.try_get::<_, Option<i16>>(index)?),
                Type::FLOAT8 => Value::from(self.0.try_get::<_, Option<f64>>(index)?),
                Type::FLOAT4 => Value::from(self.0.try_get::<_, Option<f32>>(index)?),
                Type::BOOL => Value::from(self.0.try_get::<_, Option<bool>>(index)?),
                ref other => {
                    return Err(Error::Conversion(format!(
                        "column {} has type {other}, which the exported package does not carry",
                        column.name()
                    )));
                }
            };
            object.insert(column.name().to_owned(), value);
        }
        Ok(Value::Object(object))
    }
}
