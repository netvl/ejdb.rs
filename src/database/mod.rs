use std::borrow::Borrow;
use std::ffi::{CStr, CString};
use std::fmt;
use std::io;
use std::ptr;
use std::slice;
use std::str;

use libc::{c_char, c_int};

use bson::{self, oid};
use ejdb_sys;

use self::open_mode::DatabaseOpenMode;
use ejdb_bson::{EjdbBsonDocument, EjdbObjectId};
use types::PartialSave;
use utils::tcxstr::TCXString;
use {Error, Result};

pub mod indices;
pub mod meta;
pub mod query;
pub mod tx;

/// Database open mode constants.
///
/// See `DatabaseOpenMode` for more information.
pub mod open_mode {
    use ejdb_sys;

    bitflags! {
        /// Several bit flags defining how an EJDB database should be opened.
        ///
        /// This type has `Default` implementation which returns the most common set of open
        /// mode flags:
        ///
        /// ```
        /// # use ejdb::{DatabaseOpenMode, open_mode};
        /// assert_eq!(
        ///     DatabaseOpenMode::default(),
        ///     DatabaseOpenMode::READ | DatabaseOpenMode::WRITE | DatabaseOpenMode::CREATE
        /// );
        /// ```
        ///
        /// This default set of flags is used by `Database::open()` static method.
        pub struct DatabaseOpenMode: u32 {
            /// Open the database in read-only mode.
            const READ                    = ejdb_sys::JBOREADER;
            /// Open the database in write-only mode.
            const WRITE                   = ejdb_sys::JBOWRITER;
            /// Create the database file if it does not exist.
            const CREATE                  = ejdb_sys::JBOCREAT;
            /// Truncate the database after opening it.
            const TRUNCATE                = ejdb_sys::JBOTRUNC;
            /// Open the database without locking.
            const NO_LOCK                 = ejdb_sys::JBONOLCK;
            /// Lock the database without blocking.
            const LOCK_WITHOUT_BLOCKING   = ejdb_sys::JBOLCKNB;
            /// Synchronize every transaction.
            const SYNC                    = ejdb_sys::JBOTSYNC;
        }
    }

    impl Default for DatabaseOpenMode {
        #[inline]
        fn default() -> DatabaseOpenMode {
            DatabaseOpenMode::READ | DatabaseOpenMode::WRITE | DatabaseOpenMode::CREATE
        }
    }

    impl DatabaseOpenMode {
        /// Invokes `Database::open_with_mode()` with this mode and the provided path as arguments.
        ///
        /// This is a convenient shortcut for creating database with non-default options.
        ///
        /// # Example
        ///
        /// ```no_run
        /// # use ejdb::{Database, DatabaseOpenMode, open_mode};
        /// let db = (DatabaseOpenMode::default() | DatabaseOpenMode::TRUNCATE).open("path/to/db");
        /// // equivalent to
        /// let db = Database::open_with_mode(
        ///     "path/to/db", DatabaseOpenMode::default() | DatabaseOpenMode::TRUNCATE
        /// );
        /// ```
        #[inline]
        pub fn open<P: Into<Vec<u8>>>(self, path: P) -> ::Result<super::Database> {
            super::Database::open_with_mode(path, self)
        }
    }
}

/// An EJDB database handle.
///
/// This type represents an access point for an EJDB database. An object of this type can be
/// created by `open()` or `open_with_mode()` methods or with `DatabaseOpenMode::open()`
/// method. When a value of this type is dropped, the database will be closed automatically.
///
/// This type has methods to access EJDB database metadata as well as methods for manipulating
/// collections.
pub struct Database(*mut ejdb_sys::EJDB);

// Database is not tied to a thread, so it is sendable.
unsafe impl Send for Database {}

impl Drop for Database {
    fn drop(&mut self) {
        unsafe {
            ejdb_sys::ejdbdel(self.0);
        }
    }
}

impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Database({:p})", self.0)
    }
}

#[inline]
fn last_error_code(ejdb: *mut ejdb_sys::EJDB) -> i32 {
    unsafe { ejdb_sys::ejdbecode(ejdb) }
}

fn error_code_msg(code: i32) -> &'static str {
    unsafe {
        let msg = ejdb_sys::ejdberrmsg(code);
        let msg_cstr = CStr::from_ptr(msg);
        str::from_utf8_unchecked(msg_cstr.to_bytes())
    }
}

impl Database {
    /// Opens the specified database with the provided open mode.
    ///
    /// The `path` argument may be anything convertible to a vector of bytes. Strings, string
    /// slices, bytes, bytes slices will all do.
    ///
    /// See also `DatabaseOpenMode::open()` method for a possibly more convenient alternative.
    ///
    /// # Failures
    ///
    /// Returns an error when the database can't be accessed, or if `path` contains zero bytes
    /// and probably in other cases when EJDB library can't open the database.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::{Database, DatabaseOpenMode};
    /// let db = Database::open_with_mode("/path/to/db", DatabaseOpenMode::default()).unwrap();
    /// // work with the database
    /// ```
    pub fn open_with_mode<P: Into<Vec<u8>>>(
        path: P,
        open_mode: DatabaseOpenMode,
    ) -> Result<Database> {
        let ejdb = unsafe { ejdb_sys::ejdbnew() };
        if ejdb.is_null() {
            return Err("cannot create database".into());
        }

        let p = try!(CString::new(path).map_err(|_| "invalid path specified"));

        if unsafe { ejdb_sys::ejdbopen(ejdb, p.as_ptr(), open_mode.bits() as c_int) } {
            Ok(Database(ejdb))
        } else {
            Err(format!(
                "cannot open database: {}",
                error_code_msg(last_error_code(ejdb))
            ).into())
        }
    }

    /// A shortcut for `Database::open_with_mode(path, DatabaseOpenMode::default())`.
    ///
    /// This method is used in most cases when one needs to open a database.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::{Database, DatabaseOpenMode};
    /// let db = Database::open("/path/to/database").unwrap();
    /// // work with the database
    /// ```
    #[inline]
    pub fn open<P: Into<Vec<u8>>>(path: P) -> Result<Database> {
        DatabaseOpenMode::default().open(path)
    }

    fn last_error_msg(&self) -> Option<&'static str> {
        match last_error_code(self.0) {
            0 => None,
            n => Some(error_code_msg(n)),
        }
    }

    fn last_error<T>(&self, msg: &'static str) -> Result<T> {
        Err(format!(
            "{}: {}",
            msg,
            self.last_error_msg().unwrap_or("unknown error")
        ).into())
    }

    /// Returns the given collection by its name, if it exists.
    ///
    /// This method will only return a collection if it already exists in the database; it
    /// won't create a new collection. See `Database::collection_with_options()` and
    /// `Database::collection()` methods if you need to create new collections.
    ///
    /// `path` argument may be of any type convertible to a vector of bytes, like strings or
    /// byte arrays.
    ///
    /// # Failures
    ///
    /// Fails if `name` contains zero bytes or in other cases when the corresponding EJDB
    /// operation can't be completed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// let db = Database::open("/path/to/db").unwrap();
    /// match db.get_collection("some_collection").unwrap() {
    ///     Some(coll) => { /* work with the collection */ }
    ///     None => { /* do something else */ }
    /// }
    /// ```
    pub fn get_collection<S: Into<Vec<u8>>>(&self, name: S) -> Result<Option<Collection>> {
        let p = try!(CString::new(name).map_err(|_| "invalid collection name"));
        let coll = unsafe { ejdb_sys::ejdbgetcoll(self.0, p.as_ptr()) };
        if coll.is_null() {
            match self.last_error_msg() {
                None => Ok(None),
                Some(msg) => Err(msg.into()),
            }
        } else {
            Ok(Some(Collection {
                coll: coll,
                db: self,
            }))
        }
    }

    /// Returns a collection by its name or creates one with the given options if it doesn't exist.
    ///
    /// `name` argument may be of any type convertible to a byte vector, for example, strings
    /// or byte slices. `CollectionOptions` specify which options the collection will have
    /// if it doesn't exist; if it does exist, this argument is ignored.
    ///
    /// See also `CollectionOptions::get_or_create()` method for a possibly more convenient
    /// alternative.
    ///
    /// # Failures
    ///
    /// Returns an error when `name` argument contains zero bytes in it or when the corresponding
    /// EJDB operation cannot be completed successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::{Database, CollectionOptions};
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection_with_options("some_collection", CollectionOptions::default()).unwrap();
    /// // work with the collection
    /// ```
    pub fn collection_with_options<S: Into<Vec<u8>>>(
        &self,
        name: S,
        options: CollectionOptions,
    ) -> Result<Collection> {
        let p = try!(CString::new(name).map_err(|_| "invalid collection name"));
        let mut ejcollopts = ejdb_sys::EJCOLLOPTS {
            large: options.large,
            compressed: options.compressed,
            records: options.records,
            cachedrecords: options.cached_records as c_int,
        };
        let coll = unsafe { ejdb_sys::ejdbcreatecoll(self.0, p.as_ptr(), &mut ejcollopts) };
        if coll.is_null() {
            self.last_error("cannot create or open a collection")
        } else {
            Ok(Collection {
                coll: coll,
                db: self,
            })
        }
    }

    /// A shortcut for `Database::collection_with_options(&db, name, CollectionOptions::default())`.
    ///
    /// This method is used in most cases when access to a collection is needed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// // work with the collection
    /// ```
    #[inline]
    pub fn collection<S: Into<Vec<u8>>>(&self, name: S) -> Result<Collection> {
        CollectionOptions::default().get_or_create(self, name)
    }

    /// Removes the specified collection from the database, possibly dropping all the data in it.
    ///
    /// This method removes a collection from the database. Its second argument, `prune`,
    /// determines whether all the data files for the collection should be removed as well
    /// (`true` for removing, naturally). `name` may be of any type convertible to a byte vector,
    /// for example, a string or a byte slice.
    ///
    /// # Failures
    ///
    /// Returns an error if `name` argument contains zero bytes inside it or if the
    /// corresponding EJDB operation cannot be completed successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// let db = Database::open("/path/to/db").unwrap();
    /// db.drop_collection("some_collection", true).unwrap();
    /// ```
    pub fn drop_collection<S: Into<Vec<u8>>>(&self, name: S, prune: bool) -> Result<()> {
        let p = try!(CString::new(name).map_err(|_| "invalid collection name"));
        if unsafe { ejdb_sys::ejdbrmcoll(self.0, p.as_ptr(), prune) } {
            Ok(())
        } else {
            self.last_error("cannot remove a collection")
        }
    }
}

/// Represents a set of options of an EJDB collection.
///
/// Used when new collections are created. It is not possible to change options of a created
/// collection.
///
/// This is a builder object, so you can chain method calls to set various options. Finally,
/// you can create a collection with these options with `get_or_create()` method.
///
/// # Example
///
/// ```no_run
/// # use ejdb::CollectionOptions;
/// let options = CollectionOptions::default()
///     .large(true)
///     .compressed(true)
///     .records(1_024_000)
///     .cached_records(1024);
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct CollectionOptions {
    /// Make the collection "large", i.e. able to hold more than 2GB of data. Default is false.
    pub large: bool,
    /// Compress records in the collection with DEFLATE. Default is false.
    pub compressed: bool,
    /// Expected number of records in the collection. Default is 128 000.
    pub records: i64,
    /// Maximum number of records cached in memory. Default is 0.
    pub cached_records: i32,
}

impl CollectionOptions {
    pub fn large(mut self, large: bool) -> CollectionOptions {
        self.large = large;
        self
    }

    pub fn compressed(mut self, compressed: bool) -> CollectionOptions {
        self.compressed = compressed;
        self
    }

    pub fn records(mut self, records: i64) -> CollectionOptions {
        self.records = records;
        self
    }

    pub fn cached_records(mut self, cached_records: i32) -> CollectionOptions {
        self.cached_records = cached_records;
        self
    }

    /// Invokes `db.collection_with_options(name, options)` with this object as an argument.
    ///
    /// This is a convenience method which allows setting options and creating a collection
    /// in one go. Remember that if collection with the specified name already exists,
    /// it will be returned and options will be ignored.
    ///
    /// `name` argument can be of any type which is convertible to a vector of bytes, like
    /// string or byte slice.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::{Database, CollectionOptions};
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = CollectionOptions::default()
    ///     .large(true).compressed(true)
    ///     .records(1_024_000).cached_records(1024)
    ///     .get_or_create(&db, "new_collection").unwrap();
    /// // work with the collection
    /// ```
    pub fn get_or_create<S: Into<Vec<u8>>>(self, db: &Database, name: S) -> Result<Collection> {
        db.collection_with_options(name, self)
    }
}

impl Default for CollectionOptions {
    fn default() -> CollectionOptions {
        CollectionOptions {
            large: false,
            compressed: false,
            records: 128_000,
            cached_records: 0,
        }
    }
}

/// A handle to an EJDB collection.
///
/// This structure is connected via a lifetime to the corresponding database object,
/// so it is not possible for collections to outlive their database.
///
/// Most of the work with EJDB databases goes through this structure. This includes the
/// following operations:
///
/// * Executing queries.
/// * Creating transactions.
/// * Saving and loading objects by their identifier.
///
/// Dropping and creating collections is performed through `Database` object.
///
/// `Collection` instances can be created with `Database::get_collection()`,
/// `Database::collection()`, `Database::collection_with_options()` or
/// `CollectionOptions::get_or_create()` methods.
pub struct Collection<'db> {
    coll: *mut ejdb_sys::EJCOLL,
    db: &'db Database,
}

impl<'db> Collection<'db> {
    /// Returns the name of the collection.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// assert_eq!("some_collection", coll.name());
    /// ```
    pub fn name(&self) -> &str {
        fn get_coll_name(coll: *mut ejdb_sys::EJCOLL) -> (*const u8, usize) {
            #[repr(C)]
            struct EjcollInternal {
                cname: *const c_char,
                cnamesz: c_int,
            }

            let coll_internal = coll as *const _ as *const EjcollInternal;
            unsafe {
                (
                    (*coll_internal).cname as *const u8,
                    (*coll_internal).cnamesz as usize,
                )
            }
        }

        let (data, size) = get_coll_name(self.coll);
        let bytes = unsafe { slice::from_raw_parts(data, size) };
        // XXX: should be safe, but need to check
        unsafe { str::from_utf8_unchecked(bytes) }
    }

    /// Saves the given BSON document to this collection, assigning it a fresh id, if needed.
    ///
    /// This is a convenient way to store a single object into the database. If the document
    /// contains an `_id` field of type `bson::oid::ObjectId`, then it will be used as
    /// an identifier for the new record; otherwise, a fresh identifier is generated. The
    /// actual identifier of the record, be it the provided one or the generated one,
    /// is returned if this call completed successfully.
    ///
    /// If a document with such id is already present in the collection, it will be replaced
    /// with the provided one entirely.
    ///
    /// # Failures
    ///
    /// Returns an error if the provided document can't be converted to the EJDB one or
    /// if some error occurs which prevents the corresponding EJDB operation from successful
    /// completion.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[macro_use] extern crate ejdb;
    /// # use ejdb::Database;
    /// # fn main() {
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// coll.save(bson! {
    ///     "name" => "FooBar",
    ///     "count" => 12345
    /// }).unwrap();
    /// # }
    /// ```
    pub fn save<D: Borrow<bson::Document>>(&self, doc: D) -> Result<oid::ObjectId> {
        let mut ejdb_doc = try!(EjdbBsonDocument::from_bson(doc.borrow()));
        let mut out_id = EjdbObjectId::empty();

        if unsafe { ejdb_sys::ejdbsavebson(self.coll, ejdb_doc.as_raw_mut(), out_id.as_raw_mut()) }
        {
            Ok(out_id.into())
        } else {
            self.db.last_error("error saving BSON document")
        }
    }

    /// Attempts to load a BSON document from this collection by its id.
    ///
    /// This is a convenient way to find a single object by its identifier without resorting
    /// to queries. If the object with the specified id is present in the collection,
    /// returns it, otherwise returns `None`.
    ///
    /// # Failures
    ///
    /// Returns an error if there are problems in converting the document from EJDB BSON
    /// representation or if the corresponding EJDB operation can't be completed successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// # use ejdb::bson::oid::ObjectId;
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// let value = coll.load(&ObjectId::with_string("1234567890abcdef0987feab").unwrap()).unwrap();
    /// // value is ejdb::bson::Document
    /// ```
    pub fn load(&self, id: &oid::ObjectId) -> Result<Option<bson::Document>> {
        let ejdb_oid: EjdbObjectId = id.clone().into();
        let result = unsafe { ejdb_sys::ejdbloadbson(self.coll, ejdb_oid.as_raw()) };
        if result.is_null() {
            if self.db.last_error_msg().is_none() {
                Ok(None)
            } else {
                self.db.last_error("error loading BSON document")
            }
        } else {
            unsafe {
                EjdbBsonDocument::from_ptr(result)
                    .to_bson()
                    .map(Some)
                    .map_err(|e| e.into())
            }
        }
    }

    /// Saves all BSON documents in the provided iterable to this collection.
    ///
    /// Every BSON document from the provided iterable will be saved to this collection as if
    /// they all have been passed one by one to `Collection::save()`. Returns a vector
    /// of identifiers of each created record. Any BSON document may contain an `_id` field
    /// of `bson::oid::ObjectId` type, it will then be used as a record id; otherwise,
    /// a fresh identifier will be generated.
    ///
    /// # Failures
    ///
    /// Returns an error if saving of any of the provided documents has failed. As documents
    /// are processed one by one, none of the documents after the failed one will be saved.
    /// The error will contain a vector of identifiers of documents which has been saved
    /// successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[macro_use] extern crate ejdb;
    /// # use ejdb::Database;
    /// # fn main() {
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// coll.save_all(&[
    ///     bson!{ "name" => "Foo", "count" => 123 },
    ///     bson!{ "name" => "Bar", "items" => [4, 5, 6] }
    /// ]).unwrap();
    /// # }
    /// ```
    pub fn save_all<I>(&self, docs: I) -> Result<Vec<oid::ObjectId>>
    where
        I: IntoIterator,
        I::Item: Borrow<bson::Document>,
    {
        let mut result = Vec::new();
        for doc in docs {
            match self.save(doc.borrow()) {
                Ok(id) => result.push(id),
                Err(e) => {
                    return Err(Error::PartialSave(PartialSave {
                        cause: Box::new(e),
                        successful_ids: result,
                    }))
                }
            }
        }
        Ok(result)
    }

    /// Prepares the provided query for execution.
    ///
    /// This method accepts a query object and returns a prepared query object which can
    /// then be used to execute the query in various ways.
    ///
    /// The `query` argument may be of any type which can be borrowed into a `Query`, which
    /// means that `query::Query` instances may be passed by value and by reference.
    ///
    /// # Examples
    ///
    /// Using the query API:
    /// ```no_run
    /// # #[macro_use] extern crate ejdb;
    /// # use ejdb::Database;
    /// use ejdb::query::{Q, QH};
    ///
    /// # fn main() {
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// let query = coll.query(Q.field("name").eq("Foo"), QH.empty());
    /// // work with the query object
    /// # }
    /// ```
    #[inline]
    pub fn query<Q, H>(&self, query: Q, hints: H) -> PreparedQuery<Q, H>
    where
        Q: Borrow<query::Query>,
        H: Borrow<query::QueryHints>,
    {
        PreparedQuery {
            coll: self,
            query: query,
            hints: hints,
            log_out: None,
        }
    }
}

/// Represents a query which is ready to be executed.
///
/// This structure is created out of `query::Query` object and provides methods to perform
/// queries in various ways. It is tied with a lifetime parameter to the collection this
/// query is executed on and therefore cannot outlive it.
///
/// `PreparedQuery` is created using `Collection::query()` method.
pub struct PreparedQuery<'coll, 'db: 'coll, 'out, Q, H> {
    coll: &'coll Collection<'db>,
    query: Q,
    hints: H,
    log_out: Option<&'out mut io::Write>,
}

impl<'coll, 'db, 'out, Q, H> PreparedQuery<'coll, 'db, 'out, Q, H>
where
    Q: Borrow<query::Query>,
    H: Borrow<query::QueryHints>,
{
    /// Sets the provided writer as a logging target for this query.
    ///
    /// This method can be used to analyze how the query is executed. It is needed mostly for
    /// debug purposes.
    ///
    /// Unfortunately, due to EJDB API design, the data will be written to the provided target
    /// only after the query is executed entirely (and *if* it is executed at all).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[macro_use] extern crate ejdb;
    /// # use ejdb::Database;
    /// use ejdb::query::{Q, QH};
    /// use std::io::Write;
    ///
    /// # fn main() {
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    ///
    /// let mut log_data = Vec::new();
    /// let query = coll.query(Q.field("name").eq("Foo"), QH.empty())
    ///     .log_output(&mut log_data);
    /// // the query now will log to `log_data` vector when executed
    /// # }
    /// ```
    pub fn log_output<'o>(
        self,
        target: &'o mut (io::Write + 'o),
    ) -> PreparedQuery<'coll, 'db, 'o, Q, H> {
        PreparedQuery {
            coll: self.coll,
            query: self.query,
            hints: self.hints,
            log_out: Some(target),
        }
    }

    /// Executes the query, returning the number of affected records.
    ///
    /// This method is equivalent to `find().map(|r| r.len())` but more efficient because
    /// it does not load the actual data from the database.
    ///
    /// Note that due to EJDB API structure this method is exactly equivalent to
    /// `PreparedQuery::update()`, but it has its own name for semantic purposes.
    ///
    /// # Failures
    ///
    /// Returns an error if the query document can't be serialized to EJDB representation,
    /// if writing to the output log has failed or if any of the underlying EJDB operations
    /// can't be completed successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// use ejdb::query::{Q, QH};
    ///
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// let query = coll.query(Q.field("name").eq("Foo"), QH.empty());
    /// let n = query.count().unwrap();
    /// // n is the number of records with "name" field equal to "Foo"
    /// ```
    #[inline]
    pub fn count(self) -> Result<u32> {
        self.execute(ejdb_sys::JBQRYCOUNT).map(|(_, n)| n)
    }

    /// Executes the query which does not return results, returning the number of affected records.
    ///
    /// No data is loaded from the database when this method is executed, so it is primarily
    /// needed for updating queries.
    ///
    /// Note that due to EJDB API structure this method is exactly equivalent to
    /// `PreparedQuery::count()`, but it has its own name for semantic purposes.
    ///
    /// # Failures
    ///
    /// Returns an error if the query document can't be serialized to EJDB representation,
    /// if writing to the output log has failed or if any of the underlying EJDB operations
    /// can't be completed successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// use ejdb::query::{Q, QH};
    ///
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// let n = coll.query(Q.field("name").eq("Foo").set("count", 42), QH.empty())
    ///     .update().unwrap();
    /// // n is the number of records affected by the update
    /// ```
    #[inline]
    pub fn update(self) -> Result<u32> {
        self.execute(ejdb_sys::JBQRYCOUNT).map(|(_, n)| n)
    }

    /// Executes the query, returning the first matched element if it is available.
    ///
    /// This method executes the prepared query, returning only one element matching the query.
    /// This is more efficient than `PreparedQuery::find()` method because only one object
    /// is actually loaded from the database.
    ///
    /// # Failures
    ///
    /// Returns an error if the query document can't be serialized to EJDB representation,
    /// if the returned document can't be deserialized from EJDB representation, if writing
    /// to the output log has failed or if any of the underlying EJDB operations can't
    /// be completed successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// use ejdb::query::{Q, QH};
    ///
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// match coll.query(Q.field("name").eq("Foo"), QH.empty()).find_one().unwrap() {
    ///     Some(doc) => { /* `doc` is the first record with "name" field equal to "Foo" */ }
    ///     None => { /* no document with "name" equal to "Foo" has been found */ }
    /// }
    /// ```
    pub fn find_one(self) -> Result<Option<bson::Document>> {
        self.execute(ejdb_sys::JBQRYFINDONE)
            .map(|(r, n)| QueryResult {
                result: r,
                current: 0,
                total: n,
            }).and_then(|qr| match qr.into_iter().next() {
                Some(r) => r.map(Some),
                None => Ok(None),
            })
    }

    /// Executes the query, returning an iterator of all documents matching the query.
    ///
    /// This method executes the prepared query and returns an iterator of all records which match
    /// it. This is the main method to use if you need to access multiple elements from the
    /// database.
    ///
    /// # Failures
    ///
    /// Returns an error if the query document can't be serialized to EJDB representation,
    /// if writing to the output log has failed or if any of the underlying EJDB operations
    /// can't be completed successfully. Each document from the query is deserialized from EJDB
    /// representation separately when the iterator is traversed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// use ejdb::query::{Q, QH};
    ///
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// let result = coll.query(Q.field("name").eq("Foo"), QH.empty()).find().unwrap();
    /// let items: Result<Vec<_>, _> = result.collect();  // collect all found records into a vector
    /// ```
    pub fn find(self) -> Result<QueryResult> {
        self.execute(0).map(|(r, n)| QueryResult {
            result: r,
            current: 0,
            total: n,
        })
    }

    fn execute(self, flags: u32) -> Result<(ejdb_sys::EJQRESULT, u32)> {
        let query = self.query.borrow().as_bson();
        let hints = self.hints.borrow().as_bson();

        let mut query_doc = Vec::new();
        try!(bson::encode_document(&mut query_doc, query));

        let query =
            unsafe { ejdb_sys::ejdbcreatequery2(self.coll.db.0, query_doc.as_ptr() as *const _) };
        if query.is_null() {
            return self.coll.db.last_error("error creating query object");
        }

        struct QueryGuard(*mut ejdb_sys::EJQ);
        impl Drop for QueryGuard {
            fn drop(&mut self) {
                unsafe {
                    ejdb_sys::ejdbquerydel(self.0);
                }
            }
        }

        let mut query = QueryGuard(query);

        if !hints.is_empty() {
            query_doc.clear();
            try!(bson::encode_document(&mut query_doc, hints));

            let new_query = unsafe {
                ejdb_sys::ejdbqueryhints(self.coll.db.0, query.0, query_doc.as_ptr() as *const _)
            };
            if new_query.is_null() {
                return self.coll.db.last_error("error setting query hints");
            }

            query.0 = new_query;
        }

        let mut log = if self.log_out.is_some() {
            Some(TCXString::new())
        } else {
            None
        };
        let log_ptr = log.as_mut().map(|e| e.as_raw()).unwrap_or(ptr::null_mut());

        let mut count = 0;
        let result = unsafe {
            ejdb_sys::ejdbqryexecute(self.coll.coll, query.0, &mut count, flags as c_int, log_ptr)
        };
        if result.is_null() && (flags & ejdb_sys::JBQRYCOUNT) == 0 {
            return self.coll.db.last_error("error executing query");
        }

        // dump the log to the output
        match (log, self.log_out) {
            (Some(log), Some(log_out)) => {
                try!(log_out.write(&log));
            }
            _ => {}
        }

        Ok((result, count))
    }
}

/// An iterator over EJDB query results.
///
/// Objects of this structure are returned by `PreparedQuery::find()` method.
pub struct QueryResult {
    result: ejdb_sys::EJQRESULT,
    current: c_int,
    total: u32,
}

impl QueryResult {
    /// Returns the number of records returned by the query.
    ///
    /// This iterator contains exactly `count()` elements.
    #[inline]
    pub fn count(&self) -> u32 {
        self.total
    }
}

impl Drop for QueryResult {
    fn drop(&mut self) {
        unsafe {
            ejdb_sys::ejdbqresultdispose(self.result);
        }
    }
}

impl Iterator for QueryResult {
    type Item = Result<bson::Document>;

    fn next(&mut self) -> Option<Result<bson::Document>> {
        let mut item_size = 0;
        let item: *const u8 = unsafe {
            ejdb_sys::ejdbqresultbsondata(self.result, self.current, &mut item_size) as *const _
        };
        if item.is_null() {
            return None;
        }
        self.current += 1;

        let mut data = unsafe { slice::from_raw_parts(item, item_size as usize) };
        Some(bson::decode_document(&mut data).map_err(|e| e.into()))
    }
}

#[test]
#[ignore]
fn test_save() {
    let db = Database::open("/tmp/test_database").unwrap();
    let coll = db.collection("example_collection").unwrap();

    coll.save(bson! {
        "name" => "Me",
        "age" => 23.8
    }).unwrap();
}

#[test]
#[ignore]
fn test_find() {
    use query::{Q, QH};

    let db = Database::open("/tmp/test_database").unwrap();
    let coll = db.collection("example_collection").unwrap();

    let items = (0..10).map(|i| {
        bson! {
            "name" => (format!("Me #{}", i)),
            "age" => (23.8 + i as f64)
        }
    });
    coll.save_all(items).unwrap();

    let q = Q.field("age").gte(25);

    for item in coll.query(&q, QH.empty()).find().unwrap() {
        println!("{}", item.unwrap());
    }

    let count = coll.query(&q, QH.empty()).count().unwrap();
    println!("Count: {}", count);

    let one = coll.query(&q, QH.empty()).find_one().unwrap();
    println!("One: {}", one.unwrap());
}
