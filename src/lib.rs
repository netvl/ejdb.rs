#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate error_type;
extern crate bson;
extern crate ejdb_sys;
extern crate libc;

pub use database::{Database, Collection, CollectionOptions};
pub use database::open_mode::{self, OpenMode};
pub use ejdb_bson::EjdbBsonDocument;
pub use types::*;

mod database;
mod ejdb_bson;
mod types;
mod utils;
