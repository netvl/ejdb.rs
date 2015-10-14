use std::ffi::{CStr, CString};
use std::str;
use std::slice;

use libc::{c_int, c_char};

use ejdb_sys;
use bson::{self, oid};

use self::open_mode::OpenMode;
use utils::tclist::TCList;
use ejdb_bson::{EjdbBsonDocument, EjdbObjectId};
use Result;

pub mod query;

pub mod open_mode {
    use ejdb_sys;

    bitflags! {
        flags OpenMode: u32 {
            const JBOREADER  = ejdb_sys::JBOREADER,
            const JBOWRITER  = ejdb_sys::JBOWRITER,
            const JBOCREAT   = ejdb_sys::JBOCREAT,
            const JBOTRUNC   = ejdb_sys::JBOTRUNC,
            const JBONOLCK   = ejdb_sys::JBONOLCK,
            const JBOLCKNB   = ejdb_sys::JBOLCKNB,
            const JBOTSYNC   = ejdb_sys::JBOTSYNC,
        }
    }

    impl Default for OpenMode {
        #[inline]
        fn default() -> OpenMode {
            JBOREADER | JBOWRITER | JBOCREAT
        }
    }
}

#[derive(Debug)]
#[allow(raw_pointer_derive)]
pub struct Database(*mut ejdb_sys::EJDB);

impl Drop for Database {
    fn drop(&mut self) {
        unsafe {
            ejdb_sys::ejdbdel(self.0);
        }
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
    pub fn open<P: Into<Vec<u8>>>(path: P, open_mode: OpenMode) -> Result<Database> {
        let ejdb = unsafe { ejdb_sys::ejdbnew() };
        if ejdb.is_null() {
            return Err("cannot create database".into())
        }

        let p = try!(CString::new(path).map_err(|_| "invalid path specified"));
        unsafe {
            if ejdb_sys::ejdbopen(ejdb, p.as_ptr(), open_mode.bits() as c_int) == 0 {
                return Err(error_code_msg(last_error_code(ejdb)).into());
            }
        }
        Ok(Database(ejdb))
    }

    pub fn last_error_msg(&self) -> Option<&'static str> {
        match last_error_code(self.0) {
            0 => None,
            n => Some(error_code_msg(n))
        }
    }

    pub fn last_error<T>(&self, msg: &'static str) -> Result<T> {
        Err(format!("{}: {}", msg, self.last_error_msg().unwrap_or("unknown error")).into())
    }

    pub fn get_collection_names(&self) -> Result<Vec<String>> {
        let list = unsafe { ejdb_sys::ejdbgetcolls(self.0) };
        if list.is_null() {
            return self.last_error("cannot get collection names");
        }

        let list: TCList<ejdb_sys::EJCOLL> = unsafe { TCList::from_ptr(list) };

        Ok(list.iter()
            .map(|c| Collection { coll: c, db: self })
            .map(|c| c.name())
            .collect())
    }

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

    pub fn get_or_create_collection<S: Into<Vec<u8>>>(&self, name: S, options: CollectionOptions) -> Result<Collection> {
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

    pub fn drop_collection<S: Into<Vec<u8>>>(&self, name: S, prune: bool) -> Result<()> {
        let p = try!(CString::new(name).map_err(|_| "invalid collection name"));
        if unsafe { ejdb_sys::ejdbrmcoll(self.0, p.as_ptr(), prune as u8) } != 0 {
            Ok(())
        } else {
            self.last_error("cannot remove a collection")
        }
    }
}

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
    // TODO: use ejdbmeta
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
        unsafe { str::from_utf8_unchecked(bytes).to_owned() }
    }

    #[inline]
    pub fn begin(&self) -> Result<Transaction> { Transaction::new(self) }

    pub fn transaction_active(&self) -> Result<bool> {
        let mut result = 0;
        if unsafe { ejdb_sys::ejdbtranstatus(self.coll, &mut result) != 0 } {
            Ok(result != 0)
        } else {
            self.db.last_error("error getting transaction status")
        }
    }

    pub fn save(&self, doc: &bson::Document) -> Result<oid::ObjectId> {
        let mut ejdb_doc = try!(EjdbBsonDocument::from_bson(doc));
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

    pub fn save_all<'a, I>(&self, docs: I) -> Result<Vec<oid::ObjectId>>
            where I: IntoIterator<Item=&'a bson::Document> {
        let mut result = Vec::new();
        for doc in docs {
            result.push(try!(self.save(doc)));
        }
        Ok(result)
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
fn test_save() {
    let mut db = Database::open("/tmp/test_database", OpenMode::default()).unwrap();
    let mut coll = db.get_or_create_collection("example_collection", CollectionOptions::default()).unwrap();

    let mut doc = bson::Document::new();
    doc.insert("name".to_owned(), bson::Bson::String("Me".into()));
    doc.insert("age".to_owned(), bson::Bson::FloatingPoint(23.8));
    coll.save(doc).unwrap();
}
