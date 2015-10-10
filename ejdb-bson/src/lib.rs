extern crate libc;
extern crate ejdb_sys;

use std::mem;
use std::ffi::CString;

use libc::{c_int, c_uint, c_char};

pub struct Bson(ejdb_sys::bson);

impl Drop for Bson {
    fn drop(&mut self) {
        unsafe {
            ejdb_sys::bson_destroy(&mut self.0);
        }
    }
}

impl Bson {
    pub fn new() -> Bson {
        let mut bson = unsafe { mem::uninitialized() };
        unsafe { ejdb_sys::bson_init(&mut bson); }
        Bson(bson)
    }

    pub fn new_as_query() -> Bson {
        let mut bson = unsafe { mem::uninitialized() };
        unsafe { ejdb_sys::bson_init_as_query(&mut bson); }
        Bson(bson)
    }

    pub fn is_query(&self) -> bool {
        self.0.flags as c_uint & ejdb_sys::BSON_FLAG_QUERY_MODE != 0
    }

    pub unsafe fn as_raw(&self) -> *const ejdb_sys::bson {
        &self.0
    }

    pub unsafe fn as_raw_mut(&mut self) -> *mut ejdb_sys::bson {
        &mut self.0
    }

    pub fn finish(mut self) -> Bson {
        // TODO: handle error
        unsafe { ejdb_sys::bson_finish(&mut self.0); }
        self
    }

    pub fn start_object(mut self, name: &[u8]) -> Bson {
        // TODO: handle error
        unsafe {
            ejdb_sys::bson_append_start_object2(
                &mut self.0, name.as_ptr() as *const _, name.len() as c_int
            );
        }
        self
    }

    pub fn finish_object(mut self) -> Bson {
        // TODO: handle error
        unsafe {
            ejdb_sys::bson_append_finish_object(&mut self.0);
        }
        self
    }

    pub fn start_array(mut self, name: &[u8]) -> Bson {
        // TODO: handle error
        unsafe {
            ejdb_sys::bson_append_start_array2(
                &mut self.0, name.as_ptr() as *const _, name.len() as c_int
            );
        }
        self
    }

    pub fn finish_array(mut self) -> Bson {
        // TODO: handle error
        unsafe {
            ejdb_sys::bson_append_finish_array(&mut self.0);
        }
        self
    }

    pub fn check_duplicate_keys(&self) -> bool {
        unsafe {
            ejdb_sys::bson_check_duplicate_keys(&self.0) != 0
        }
    }

    pub fn fix_duplicate_keys(&self) -> Bson {
        let mut out = if self.is_query() { Bson::new_as_query() } else { Bson::new() };
        // TODO: handle errors
        unsafe {
            ejdb_sys::bson_fix_duplicate_keys(&self.0, &mut out.0);
        }
        out
    }

    pub fn merge(&self, other: &Bson, overwrite: bool) -> Bson {
        let mut out = if self.is_query() || other.is_query() {
            Bson::new_as_query()
        } else {
            Bson::new()
        };

        // TODO: handle errors
        unsafe {
            ejdb_sys::bson_merge(&self.0, &other.0, overwrite as ejdb_sys::bson_bool_t, &mut out.0);
        }

        out
    }

    pub fn merge_recursive(&self, other: &Bson, overwrite: bool) -> Bson {
        let mut out = if self.is_query() || other.is_query() {
            Bson::new_as_query()
        } else {
            Bson::new()
        };

        // TODO: handle errors
        unsafe {
            ejdb_sys::bson_merge_recursive(&self.0, &other.0, overwrite as ejdb_sys::bson_bool_t, &mut out.0);
        }

        out
    }
}

macro_rules! gen_append_method {
    ($method_name:ident (|$($arg:ident : $arg_t:ty),+| $ffi_fn:ident ($($e:expr),+))) => {
        pub fn $method_name<K: Into<Vec<u8>>>(mut self, key: K, $($arg: $arg_t),+) -> Bson {
            let key_cstr = CString::new(key).unwrap();

            // TODO: check for errors
            unsafe {
                ejdb_sys::$ffi_fn(&mut self.0, key_cstr.as_ptr(), $($e),+);
            }

            self
        }
    };
    ($method_name:ident (|| $ffi_fn:ident ())) => {
        pub fn $method_name<K: Into<Vec<u8>>>(mut self, key: K) -> Bson {
            let key_cstr = CString::new(key).unwrap();

            // TODO: check for errors
            unsafe {
                ejdb_sys::$ffi_fn(&mut self.0, key_cstr.as_ptr());
            }

            self
        }
    };
    ($method_name:ident (like_string, $ffi_fn:ident)) => {
        gen_append_method! {
            $method_name (|value: &[u8]| $ffi_fn(value.as_ptr() as *const _, value.len() as c_int))
        }
    }
}

macro_rules! gen_append_methods {
    ($($method_name:ident ($($internals:tt)*)),+) => {
        $(gen_append_method! { $method_name ($($internals)*) })+
    }
}

macro_rules! gen_appends {
    ($tpe:ident, $($decl:tt)*) => {
        impl $tpe {
            gen_append_methods! { $($decl)* }
        }
    };
}

gen_appends! { Bson,
    append_string (like_string, bson_append_string_n),
    append_code (like_string, bson_append_code_n),
    append_code_with_scope (|value: &[u8], scope: &Bson| bson_append_code_w_scope_n(
        value.as_ptr() as *const _, value.len() as c_int, scope.as_raw()
    )),
    append_symbol (like_string, bson_append_symbol_n),
    append_binary (|binary_type: u8, value: &[u8]| bson_append_binary(
        binary_type as c_char, value.as_ptr() as *const _, value.len() as c_int
    )),
    append_bool(|value: bool| bson_append_bool(value as ejdb_sys::bson_bool_t)),
    append_null(|| bson_append_null()),
    append_undefined(|| bson_append_undefined()),
    append_bson(|bson: &Bson| bson_append_bson(bson.as_raw()))
    // TODO: bson_append_regex, bson_append_element (?), bson_append_timestamp, bson_append_date
}
