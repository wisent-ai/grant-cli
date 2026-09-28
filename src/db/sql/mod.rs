//! grant-cli's services talk to the fleet database `grant-cli` through
//! stado-database's synchronous client: one SeaORM connection, resolved
//! through Stado and Skarbiec, with no client of grant-cli's own.

pub use stado_database::params;
pub use stado_database::sync::{Client as Connection, Error, OptionalExtension, Result, Row};

/// The fleet database `grant-cli`. The account's home is `GRANT_FLEET_HOME`
/// when set, else HOME; a refusal names the step that failed.
pub fn open() -> anyhow::Result<Connection> {
    let fleet = stado_database::FleetDatabase::for_product("grant-cli", "GRANT_FLEET_HOME")?;
    Ok(Connection::connect(&fleet)?)
}
