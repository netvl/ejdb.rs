#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate quick_error;
extern crate bson as bson_crate;
extern crate itertools;
extern crate ejdb_sys;
extern crate libc;

pub use bson_crate as bson;

pub use database::{Database, Collection, CollectionOptions, Transaction, Query, QueryResult};
pub use database::open_mode::{self, DatabaseOpenMode};
pub use database::query;
pub use database::indices::Index;
pub use database::meta;
pub use types::{Result, Error};

#[macro_use]
mod macros;
mod database;
mod utils;

pub mod ejdb_bson;
pub mod types;
