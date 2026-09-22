//! What this product keeps, as the shapes every module reads and writes:
//! the catalogue it applies against, the application in flight, and what
//! came of it.

mod catalogue;
mod delivery;
mod work;

pub use catalogue::*;
pub use delivery::*;
pub use work::*;
