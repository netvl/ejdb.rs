//! Safe and idiomatic Rust bindings for EJDB, a MongoDB-like embedded database library.
//!
//! ejdb.rs provides an interface for [EJDB], an embeddable JSON-based document database
//! library. Think of it as SQLite-like MongoDB. EJDB attempts to be compatible (to some extent)
//! with MongoDB basic concepts and query language, so if you have any experience with MongoDB,
//! learning EJDB will be easy.
//!
//! EJDB uses BSON internally, just like MongoDB, so ejdb.rs uses [bson-rs] crate, which is
//! reexported as `ejdb::bson` module. Please note that it is important to use types and
//! functions from `ejdb::bson` module, not loading `bson` with `extern crate`, because of
//! possible version incompatibilities. `bson!` macro provided by this crate also uses
//! types from `ejdb::bson`.
//!
//! The central type in this library is `Database` structure. It represents an opened
//! EJDB database. An EJDB database usually consists of several files: the database itself,
//! a file for each collection and a file for each index. Therefore, it makes sense to
//! dedicate a whole directory for EJDB database files. However, `Database::open()` method
//! and its comrades accepts a path to the database file itself, not the directory.
//!
//! ```no_run
//! use ejdb::Database;
//!
//! let db = Database::open("/path/to/db").unwrap();
//! ```
//!
//! `Database`'s `Drop` implementation closes the database automatically according to RAII pattern.
//!
//! The database can be opened in various modes; see `DatabaseOpenMode` structure for
//! more information.
//!
//! After the database is opened, you can obtain collections out of it. It is done
//! primarily with `Database::collection()` method:
//!
//! ```no_run
//! # use ejdb::Database;
//! # let db = Database::open("/path/to/db").unwrap();
//! let coll = db.collection("some_collection").unwrap();
//! ```
//!
//! `Database::collection()` method returns an existing collection or creates a new one
//! with the default options. See `CollectionOptions` structure for more information about
//! which options collections have.
//!
//! A collection may be used to perform queries, initiate transactions or save/load BSON
//! documents by their identifiers directly, without using queries. Collection objects
//! can also be used to manage indices.
//!
//! ## Saving/loading BSON documents
//!
//! You can use `Collection::save()` or `Collection::save_all()` methods to store BSON documents
//! directly into the collection, and `Collection::load()` to load a document by its id:
//!
//! ```no_run
//! # #[macro_use] extern crate ejdb;
//! # use ejdb::Database;
//! # fn main() {
//! # let db = Database::open("/path/to/db").unwrap();
//! # let coll = db.collection("some_collection").unwrap();
//! let mut d = bson! {
//!     "name" => "Foo Bar",
//!     "count" => 10
//! };
//! let inserted_id = coll.save(&d).unwrap();
//!
//! d.insert("_id", inserted_id.clone());
//! let d2 = coll.load(&inserted_id).unwrap().unwrap();
//! assert_eq!(d, d2);
//! # }
//! ```
//!
//! If the `_id` field is not present in the BSON document, it will be generated and added
//! automatically.
//!
//! `Collection::save_all()` method is implemented over `Collection::save()` and returns a
//! special kind of error which contains information about errors for each save operation,
//! if any.
//!
//! ## Performing queries
//!
//! EJDB supports a pretty large subset of operations provided by MongoDB, and even has
//! its own unique queries, like joins.
//!
//! Queries are perfomed with `Collection::query()` method which accepts anything which can be
//! borrowed into a `Query`. `Query` consists of two parts: the actual query, i.e. constraints
//! on the data in a collection, and query hints which alter the way the query is processed
//! and returned. `Query` contains both of these parts, 
//!
//!   [EJDB]: http://ejdb.org/
//!   [bson-rs]: https://crates.io/crates/bson

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate quick_error;
extern crate bson as bson_crate;
extern crate itertools;
extern crate ejdb_sys;
extern crate libc;

/// A reexport of `bson` crate used by this crate in public interface.
pub use bson_crate as bson;

pub use database::{Database, Collection, CollectionOptions, PreparedQuery, QueryResult};
pub use database::open_mode::{self, DatabaseOpenMode};
pub use database::query;
pub use database::meta;
pub use database::tx::Transaction;
pub use database::indices::Index;
pub use types::{Result, Error};

#[macro_use]
mod macros;
mod database;
mod utils;

pub mod ejdb_bson;
pub mod types;
