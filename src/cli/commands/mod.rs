//! Every command this CLI answers, grouped by what the caller is doing:
//! the catalogue, the application in flight, and its text.

mod authoring;
mod catalogue;
mod work;

pub use authoring::*;
pub use catalogue::*;
pub use work::*;
