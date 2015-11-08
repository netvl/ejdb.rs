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

    pub fn string(self, case_sensitive: bool) -> Self {
        self.add_flags(if case_sensitive { ejdb_sys::JBIDXSTR } else { ejdb_sys::JBIDXISTR })
    }

    pub fn number(self) -> Self {
        self.add_flags(ejdb_sys::JBIDXNUM)
    }

    pub fn array(self) -> Self {
        self.add_flags(ejdb_sys::JBIDXARR)
    }

    pub fn set(self) -> Result<()> {
        self.check_type().execute()
    }

    pub fn drop_all(mut self) -> Result<()> {
        self.flags = Some(ejdb_sys::JBIDXDROPALL);
        self.execute()
    }

    pub fn drop(self) -> Result<()> {
        self.add_flags(ejdb_sys::JBIDXDROP).check_type().execute()
    }

    pub fn rebuild(self) -> Result<()> {
        self.add_flags(ejdb_sys::JBIDXREBLD).check_type().execute()
    }

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
