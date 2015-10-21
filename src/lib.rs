#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate error_type;
extern crate bson;
extern crate itertools;
extern crate ejdb_sys;
extern crate libc;

pub use database::{Database, Collection, CollectionOptions, Transaction, Query, QueryResult};
pub use database::open_mode::{self, OpenMode};
pub use database::query;
pub use ejdb_bson::EjdbBsonDocument;
pub use types::*;
pub use utils::bson::DocumentBuilder;

#[macro_use]
mod macros;
mod database;
mod ejdb_bson;
mod types;
mod utils;
