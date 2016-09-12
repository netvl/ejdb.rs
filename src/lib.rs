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
//! Queries are perfomed with `Collection::query()` method which accepts, two arguments:
//! anything which can be borrowed into a `Query` and anything which can be borrowed into
//! a `QueryHints`. `Query` is the actual query, i.e. constraints on the data in a collection,
//! and `QueryHints` alter the way the query is processed and returned.
//!
//! Both query and query hints are just BSON documents of [special format][ejdb-ql], therefore
//! ejdb.rs provides the respective `From<bson::Document>`/`Into<bson::Document>` for both
//! `Query` and `QueryHints`; however, it is recommended to use the builder API instead
//! of constructing queries manually because this way it is much harder to create invalid queries.
//! Naturally, invalid queries are by no means unsafe in Rust sense - if such a query is passed
//! for execution, an error will be returned.
//!
//! Query builder API provides two entry points, `ejdb::query::Q` and `ejdb::query::QH`, which
//! are kind of aliases for `Query::new()` and `QueryHints::new()` but look arguably nicer.
//! To run a query, pass an instance of `Query` and `QueryHints` to `Collection::query()` method.
//! The latter returns a `ejdb::PreparedQuery` instance which can be used to execute the query
//! in various ways.
//!
//! ```no_run
//! # #[macro_use] extern crate ejdb;
//! # use ejdb::Database;
//! use ejdb::query::{Q, QH};
//! use ejdb::bson;
//! use ejdb::Result;
//! # fn main() {
//! # let db = Database::open("/path/to/db").unwrap();
//! # let coll = db.collection("some_collection").unwrap();
//!
//! let n = coll.query(Q.field("name").eq("Foo").set("count", 10), QH.empty()).update().unwrap();
//! // `n` is the number of affected rows
//!
//! let names = ["foo", "bar", "baz"];
//! let items = coll.query(Q.field("name").contained_in(names.iter().cloned()), QH.max(12))
//!     .find().unwrap();
//! // `items` is an iterator which contains at maximum 12 records whose `name`
//! // field is either "foo", "bar" or "baz"
//! let items: Result<Vec<bson::Document>> = items.collect();  // collect them into a vector
//!
//! let item = coll.query(Q.field("count").between(-10, 10.2), QH.field("name").include())
//!     .find_one().unwrap();
//! // `item` is an `Option<bson::Document>` which contains a record whose `count` field
//! // is between -10 and 10.2, inclusive, if there is one, and this document will only contain
//! // `name` field.
//!
//! let n = coll.query(Q.field("name").exists(true), QH.empty()).count().unwrap();
//! // `n` is the number of records which contain `name` field
//! # }
//! ```
//!
//! ## Transactions
//!
//! You can use `Collection::begin_transaction()` method which will start a transaction over
//! this collection. Citing the official documentation:
//!
//! > EJDB provides atomic and durable non parallel and read-uncommited collection level
//! > transactions, i.e., There is only one transaction for collection is active for a single
//! > point in a time. The data written in a transaction is visible for other non transactional
//! > readers. EJDB transaction system utilizes write ahead logging to provide consistent
//! > transaction rollbacks.
//!
//! Transactions in ejdb.rs are implemented with RAII pattern: a transaction is represented
//! by a guard object. When this object is dropped, the transaction is committed or aborted.
//! By default it is aborted; but you can change the default behavior with corresponding methods.
//! Alternatively, you can explicitly commit or abort the transaction with `Transaction::commit()`
//! or `Transaction::abort()`, respectively. Additionally, these methods return a `Result<()>`
//! which can be used to track erros; when the transaction is closed on its drop, the result
//! is ignored.
//!
//! ```no_run
//! # use ejdb::Database;
//! # let db = Database::open("/path/to/db").unwrap();
//! # let coll = db.collection("some_collection").unwrap();
//! loop {
//!     let tx = coll.begin_transaction().unwrap();
//!     // execute queries and other operations
//!     // if some error happens and the loop exists prematurely, e.g. through unwinding,
//!     // the transaction will be aborted automatically
//!
//!     // try to commit the transaction and try again if there is an error
//!     if let Ok(_) = tx.commit() {
//!         break;
//!     }
//! }
//! ```
//!
//! ## Indices
//!
//! It is also possible to use `Collection::index()` method to configure indices in the collection.
//! `index()` accepts the name of the field on which the user needs to configure indices; it
//! returns a builder-like object which can be used to tweak indices on this field.
//!
//! In EJDB a field can have several associated indices of different types, which is important
//! for heterogeneous fields. It is also possible to rebuild and optimize indices. This can
//! be done with the respective methods on `Index` structure returned by `Collection::index()`.
//!
//! ```no_run
//! # use ejdb::Database;
//! # let db = Database::open("/path/to/db").unwrap();
//! # let coll = db.collection("some_collection").unwrap();
//! // create a case-sensitive string index on field `name`
//! coll.index("name").string(true).set().unwrap();
//!
//! // create case-insensitive string and numeric indices on field `title`
//! coll.index("title").number().string(false).set().unwrap();
//!
//! // remove number and array indices from field `items`
//! coll.index("items").number().array().drop().unwrap();
//!
//! // optimize string index on field `name`
//! coll.index("name").string(true).optimize().unwrap();
//!
//! // drop all indices on field `properties`
//! coll.index("properties").drop_all();
//! ```
//!
//! All consuming methods except for `Index::drop_all()` will panic if index type is not
//! specified before their invocation:
//!
//! ```no_run
//! # use ejdb::Database;
//! # let db = Database::open("/path/to/db").unwrap();
//! # let coll = db.collection("some_collection").unwrap();
//! coll.index("name").set();  // will panic
//! ```
//!
//!   [EJDB]: http://ejdb.org/
//!   [bson-rs]: https://crates.io/crates/bson
//!   [ejdb-ql]: http://ejdb.org/doc/ql/ql.html

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate quick_error;
pub extern crate bson as bson_crate;
extern crate itertools;
pub extern crate ejdb_sys;
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
