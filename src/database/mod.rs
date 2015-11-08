use std::ffi::{CStr, CString};
use std::str;
use std::slice;
use std::io;
use std::ptr;
use std::borrow::Borrow;
use std::fmt;

use libc::{c_int, c_char};

use ejdb_sys;
use bson::{self, oid};

use self::open_mode::DatabaseOpenMode;
use utils::tcxstr::TCXString;
use ejdb_bson::{EjdbBsonDocument, EjdbObjectId};
use types::PartialSave;
use {Error, Result};

pub mod indices;
pub mod query;
pub mod meta;

pub mod open_mode {
    use ejdb_sys;

    bitflags! {
        /// Several bit flags defining the open mode for EJDB database.
        ///
        /// This type has `Default` implementation which returns the most common set of open
        /// mode flags:
        ///
        /// ```
        /// # use ejdb::{DatabaseOpenMode, open_mode};
        /// assert_eq!(
        ///     DatabaseOpenMode::default(),
        ///     open_mode::READ | open_mode::WRITE | open_mode::CREATE
        /// );
        /// ```
        ///
        /// This default set of flags is used by `Database::open()` static method.
        flags DatabaseOpenMode: u32 {
            /// Open the database in read-only mode.
            const READ                    = ejdb_sys::JBOREADER,
            /// Open the database in write-only mode.
            const WRITE                   = ejdb_sys::JBOWRITER,
            /// Create the database file if it does not exist.
            const CREATE                  = ejdb_sys::JBOCREAT,
            /// Truncate the database after opening it.
            const TRUNCATE                = ejdb_sys::JBOTRUNC,
            /// Open the database without locking.
            const NO_LOCK                 = ejdb_sys::JBONOLCK,
            /// Lock the database without blocking.
            const LOCK_WITHOUT_BLOCKING   = ejdb_sys::JBOLCKNB,
            /// Synchronize every transaction.
            const SYNC                    = ejdb_sys::JBOTSYNC,
        }
    }

    impl Default for DatabaseOpenMode {
        #[inline]
        fn default() -> DatabaseOpenMode {
            READ | WRITE | CREATE
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
        /// let db = (DatabaseOpenMode::default() | open_mode::TRUNCATE).open("path/to/db");
        /// // equivalent to
        /// let db = Database::open_with_mode(
        ///     "path/to/db", DatabaseOpenMode::default() | open_mode::TRUNCATE
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
    pub fn open_with_mode<P: Into<Vec<u8>>>(path: P, open_mode: DatabaseOpenMode) -> Result<Database> {
        let ejdb = unsafe { ejdb_sys::ejdbnew() };
        if ejdb.is_null() {
            return Err("cannot create database".into())
        }

        let p = try!(CString::new(path).map_err(|_| "invalid path specified"));
        unsafe {
            if ejdb_sys::ejdbopen(ejdb, p.as_ptr(), open_mode.bits() as c_int) == 0 {
                return Err(format!("cannot open database: {}", error_code_msg(last_error_code(ejdb))).into());
            }
        }
        Ok(Database(ejdb))
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
            n => Some(error_code_msg(n))
        }
    }

    fn last_error<T>(&self, msg: &'static str) -> Result<T> {
        Err(format!("{}: {}", msg, self.last_error_msg().unwrap_or("unknown error")).into())
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
                Some(msg) => Err(msg.into())
            }
        } else {
            Ok(Some(Collection { coll: coll, db: self }))
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
    pub fn collection_with_options<S: Into<Vec<u8>>>(&self, name: S, options: CollectionOptions) -> Result<Collection> {
        let p = try!(CString::new(name).map_err(|_| "invalid collection name"));
        let mut ejcollopts = ejdb_sys::EJCOLLOPTS {
            large: options.large as u8,
            compressed: options.compressed as u8,
            records: options.records,
            cachedrecords: options.cached_records as c_int
        };
        let coll = unsafe { ejdb_sys::ejdbcreatecoll(self.0, p.as_ptr(), &mut ejcollopts) };
        if coll.is_null() {
            self.last_error("cannot create or open a collection")
        } else {
            Ok(Collection { coll: coll, db: self })
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
        if unsafe { ejdb_sys::ejdbrmcoll(self.0, p.as_ptr(), prune as u8) } != 0 {
            Ok(())
        } else {
            self.last_error("cannot remove a collection")
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct CollectionOptions {
    pub large: bool,
    pub compressed: bool,
    pub records: i64,
    pub cached_records: i32
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
            cached_records: 0
        }
    }
}

pub struct Collection<'db> {
    coll: *mut ejdb_sys::EJCOLL,
    db: &'db Database
}

impl<'db> Collection<'db> {
    pub fn name(&self) -> String {
        fn get_coll_name(coll: *mut ejdb_sys::EJCOLL) -> (*const u8, usize) {
            #[repr(C)]
            struct EjcollInternal {
                cname: *const c_char,
                cnamesz: c_int
            }

            let coll_internal = coll as *const _ as *const EjcollInternal;
            unsafe {
                ((*coll_internal).cname as *const u8, (*coll_internal).cnamesz as usize)
            }
        }

        let (data, size) = get_coll_name(self.coll);
        let bytes = unsafe { slice::from_raw_parts(data, size) };
        // XXX: should be safe, but need to check
        unsafe { str::from_utf8_unchecked(bytes).into() }
    }

    #[inline]
    pub fn begin_transaction(&self) -> Result<Transaction> { Transaction::new(self) }

    pub fn transaction_active(&self) -> Result<bool> {
        let mut result = 0;
        if unsafe { ejdb_sys::ejdbtranstatus(self.coll, &mut result) != 0 } {
            Ok(result != 0)
        } else {
            self.db.last_error("error getting transaction status")
        }
    }

    pub fn save<D: Borrow<bson::Document>>(&self, doc: D) -> Result<oid::ObjectId> {
        let mut ejdb_doc = try!(EjdbBsonDocument::from_bson(doc.borrow()));
        let mut out_id = EjdbObjectId::empty();

        if unsafe { ejdb_sys::ejdbsavebson(self.coll, ejdb_doc.as_raw_mut(), out_id.as_raw_mut()) != 0 } {
            Ok(out_id.into())
        } else {
            self.db.last_error("error saving BSON document")
        }
    }

    pub fn load(&self, id: oid::ObjectId) -> Result<Option<bson::Document>> {
        let ejdb_oid: EjdbObjectId = id.into();
        let result = unsafe { ejdb_sys::ejdbloadbson(self.coll, ejdb_oid.as_raw()) };
        if result.is_null() {
            if self.db.last_error_msg().is_none() { Ok(None) }
            else { self.db.last_error("error loading BSON document") }
        } else {
            unsafe {
                EjdbBsonDocument::from_ptr(result).to_bson().map(Some).map_err(|e| e.into())
            }
        }
    }

    pub fn save_all<I>(&self, docs: I) -> Result<Vec<oid::ObjectId>>
            where I: IntoIterator,
                  I::Item: Borrow<bson::Document> {
        let mut result = Vec::new();
        for doc in docs {
            match self.save(doc.borrow()) {
                Ok(id) => result.push(id),
                Err(e) => return Err(Error::PartialSave(PartialSave {
                    cause: Box::new(e),
                    successful_ids: result
                }))
            }
        }
        Ok(result)
    }

    #[inline]
    pub fn query<Q: Borrow<query::Query>>(&self, query: Q) -> Query<Q> {
        Query {
            coll: self,
            query: query,
            log_out: None
        }
    }
}

pub struct Query<'coll, 'db: 'coll, 'out, Q> {
    coll: &'coll Collection<'db>,
    query: Q,
    log_out: Option<&'out mut io::Write>
}

impl<'coll, 'db, 'out, Q: Borrow<query::Query>> Query<'coll, 'db, 'out, Q> {
    pub fn log_output<'o>(self, target: &'o mut (io::Write + 'o)) -> Query<'coll, 'db, 'o, Q> {
        Query {
            coll: self.coll,
            query: self.query,
            log_out: Some(target)
        }
    }

    #[inline]
    pub fn count(self) -> Result<u32> {
        self.execute(ejdb_sys::JBQRYCOUNT).map(|(_, n)| n)
    }

    #[inline]
    pub fn update(self) -> Result<u32> {
        self.execute(ejdb_sys::JBQRYCOUNT).map(|(_, n)| n)
    }

    pub fn find_one(self) -> Result<Option<bson::Document>> {
        self.execute(ejdb_sys::JBQRYFINDONE)
            .map(|(r, n)| QueryResult { result: r, current: 0, total: n })
            .and_then(|qr| match qr.into_iter().next() {
                Some(r) => r.map(Some),
                None => Ok(None)
            })
    }

    pub fn find(self) -> Result<QueryResult> {
        self.execute(0).map(|(r, n)| QueryResult { result: r, current: 0, total: n })
    }

    fn execute(self, flags: u32) -> Result<(ejdb_sys::EJQRESULT, u32)> {
        let (hints, query) = self.query.borrow().build_ref();

        let mut query_doc = Vec::new();
        try!(bson::encode_document(&mut query_doc, query));

        let query = unsafe {
            ejdb_sys::ejdbcreatequery2(self.coll.db.0, query_doc.as_ptr() as *const _)
        };
        if query.is_null() {
            return self.coll.db.last_error("error creating query object");
        }

        struct QueryGuard(*mut ejdb_sys::EJQ);
        impl Drop for QueryGuard {
            fn drop(&mut self) {
                unsafe { ejdb_sys::ejdbquerydel(self.0); }
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

        // TODO: actually use log output
        let mut log = if self.log_out.is_some() { Some(TCXString::new()) } else { None };
        let log_ptr = log.as_mut().map(|e| e.as_raw()).unwrap_or(ptr::null_mut());

        let mut count = 0;
        let result = unsafe {
            ejdb_sys::ejdbqryexecute(self.coll.coll, query.0, &mut count, flags as c_int, log_ptr)
        };
        if result.is_null() && (flags & ejdb_sys::JBQRYCOUNT) == 0 {
            return self.coll.db.last_error("error executing query");
        }

        Ok((result, count))
    }
}

pub struct QueryResult {
    result: ejdb_sys::EJQRESULT,
    current: c_int,
    total: u32
}

impl QueryResult {
    #[inline]
    pub fn count(&self) -> u32 { self.total }
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
        if item.is_null() { return None; }
        self.current += 1;

        let mut data = unsafe { slice::from_raw_parts(item, item_size as usize) };
        Some(bson::decode_document(&mut data).map_err(|e| e.into()))
    }
}

pub struct Transaction<'coll, 'db: 'coll> {
    coll: &'coll Collection<'db>,
    commit: bool,
    finished: bool
}

impl<'coll, 'db> Drop for Transaction<'coll, 'db> {
    fn drop(&mut self) {
        let _ = self.finish_mut();  // ignore the result
    }
}

impl<'coll, 'db> Transaction<'coll, 'db> {
    fn new(coll: &'coll Collection<'db>) -> Result<Transaction<'coll, 'db>> {
        if unsafe { ejdb_sys::ejdbtranbegin(coll.coll) != 0 } {
            coll.db.last_error("error opening transaction")
        } else {
            Ok(Transaction { coll: coll, commit: false, finished: false })
        }
    }

    #[inline]
    pub fn will_commit(&self) -> bool { self.commit }

    #[inline]
    pub fn will_abort(&self) -> bool { !self.commit }

    #[inline]
    pub fn set_commit(&mut self) { self.commit = true; }

    #[inline]
    pub fn set_abort(&mut self) { self.commit = false; }

    #[inline]
    pub fn finish(mut self) -> Result<()> { self.finish_mut() }

    #[inline]
    pub fn commit(mut self) -> Result<()> { self.commit_mut() }

    #[inline]
    pub fn abort(mut self) -> Result<()> { self.abort_mut() }

    fn finish_mut(&mut self) -> Result<()> {
        if self.finished { Ok(()) }
        else { if self.commit { self.commit_mut() } else { self.abort_mut() } }
    }

    fn commit_mut(&mut self) -> Result<()> {
        self.finished = true;
        if unsafe { ejdb_sys::ejdbtrancommit(self.coll.coll) != 0 } { Ok(()) }
        else { self.coll.db.last_error("error commiting transaction") }
    }

    fn abort_mut(&mut self) -> Result<()> {
        self.finished = true;
        if unsafe { ejdb_sys::ejdbtranabort(self.coll.coll) != 0 } { Ok(()) }
        else { self.coll.db.last_error("error aborting transaction") }
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
    use query::Q;

    let db = Database::open("/tmp/test_database").unwrap();
    let coll = db.collection("example_collection").unwrap();

    let items = (0..10).map(|i| bson! {
        "name" => (format!("Me #{}", i)),
        "age" => (23.8 + i as f64)
    });
    coll.save_all(items).unwrap();

    let q = Q.field("age").gte(25);

    for item in coll.query(&q).find().unwrap() {
        println!("{}", item.unwrap());
    }

    let count = coll.query(&q).count().unwrap();
    println!("Count: {}", count);

    let one = coll.query(&q).find_one().unwrap();
    println!("One: {}", one.unwrap());
}
