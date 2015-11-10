use std::ffi::CString;

use libc::{c_uint, c_int};

use ejdb_sys;

use super::Collection;
use Result;

impl<'db> Collection<'db> {
    pub fn index<S: Into<String>>(&self, key: S) -> Index {
        Index {
            coll: self,
            key: key.into(),
            flags: None
        }
    }
}

/// A builder for an operation on an index of a certain field of an EJDB collection.
///
/// In EJDB every collection can have an index on the fields of its records. Indices can be
/// of one of three types:
///
/// * string (possibly case insensitive);
/// * number;
/// * array.
///
/// Indices can be set, dropped, optimized or rebuilt. Indices are stored in a separate file
/// from their collections and can speed up certain access patterns in queries. Naturally,
/// indices are specified for some field in collection records, so this structure is used to
/// configure indices for one specific field.
///
/// Index manipulation is done with this structure which provides builder-like interface
/// to create, change properties or drop an index on one field. Since an index can't exist
/// separately from a collection, this structure is linked via a lifetime to its corresponding
/// collection object. An instance of this structure is obtained with `Collection::index()` method.
///
/// # Example
///
/// ```no_run
/// # use ejdb::Database;
/// let db = Database::open("/path/to/db").unwrap();
/// let coll = db.collection("some_collection").unwrap();
///
/// // create a string index on `name` field
/// coll.index("name").string(true).set().unwrap();
///
/// // create multiple indices on `coords` field
/// coll.index("coords").number().array().set().unwrap();
/// ```
pub struct Index<'coll, 'db: 'coll> {
    coll: &'coll Collection<'db>,
    key: String,
    flags: Option<c_uint>
}

impl<'coll, 'db: 'coll> Index<'coll, 'db> {
    fn add_flags(self, flags: c_uint) -> Self {
        Index {
            coll: self.coll,
            key: self.key,
            flags: Some(self.flags.unwrap_or(0) | flags)
        }
    }

    /// Specifies that this index must be built over string values of this field.
    ///
    /// `case_sensitive` argument determines whether this index must take string case into account,
    /// `true` for case sensitive matching, `false` for the opposite.
    pub fn string(self, case_sensitive: bool) -> Self {
        self.add_flags(if case_sensitive { ejdb_sys::JBIDXSTR } else { ejdb_sys::JBIDXISTR })
    }

    /// Specifies that this index must be built over numeric values of this field.
    pub fn number(self) -> Self {
        self.add_flags(ejdb_sys::JBIDXNUM)
    }

    /// Specifies that this index must be built over array values of this field.
    pub fn array(self) -> Self {
        self.add_flags(ejdb_sys::JBIDXARR)
    }

    /// Creates one or more indices of specified types on this field.
    ///
    /// Panics if no types were specified before calling this method.
    pub fn set(self) -> Result<()> {
        self.check_type().execute()
    }

    /// Drops all indices on this field.
    pub fn drop_all(mut self) -> Result<()> {
        self.flags = Some(ejdb_sys::JBIDXDROPALL);
        self.execute()
    }

    /// Drops indices of the previously specified types on this field.
    ///
    /// Panics if no type has been set prior to calling this method.
    pub fn drop(self) -> Result<()> {
        self.add_flags(ejdb_sys::JBIDXDROP).check_type().execute()
    }

    /// Rebuilds indices of the previously specified types on this field from scratch.
    ///
    /// Panics if no type has been set prior to calling this method.
    pub fn rebuild(self) -> Result<()> {
        self.add_flags(ejdb_sys::JBIDXREBLD).check_type().execute()
    }

    /// Optimizes indices of the previously specified types on this field.
    ///
    /// Panics if no type has been set prior to calling this method.
    pub fn optimize(self) -> Result<()> {
        self.add_flags(ejdb_sys::JBIDXOP).check_type().execute()
    }

    fn check_type(self) -> Self {
        let flags = self.flags.expect("index type is not specified");
        assert!([
            ejdb_sys::JBIDXSTR, ejdb_sys::JBIDXISTR,
            ejdb_sys::JBIDXNUM, ejdb_sys::JBIDXARR
        ].iter().any(|&f| flags & f != 0), "index type is not specified");
        self
    }

    fn execute(self) -> Result<()> {
        let flags = self.flags.expect("index flags are not defined");  // should always unwrap
        let key = try!(CString::new(self.key).map_err(|_| "invalid key"));
        let result = unsafe {
            ejdb_sys::ejdbsetindex(self.coll.coll, key.as_ptr(), flags as c_int)
        };
        if result != 0 {
            Ok(())
        } else {
            self.coll.db.last_error("cannot update index")
        }
    }
}
