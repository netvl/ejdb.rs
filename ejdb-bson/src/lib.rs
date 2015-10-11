extern crate libc;
extern crate ejdb_sys;

use std::mem;
use std::ptr;
use std::ffi::{CString, CStr};
use std::marker::PhantomData;
use std::slice;
use std::fmt;
use std::str;

use libc::{c_int, c_uint};

pub type BsonDate = i64;

#[derive(Copy, Clone)]
pub struct BsonTimestamp(ejdb_sys::bson_timestamp_t);

impl fmt::Debug for BsonTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BsonTimestamp {{ increment: {}, timestamp: {} }}", self.increment(), self.timestamp())
    }
}

impl PartialEq for BsonTimestamp {
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        self.increment() == rhs.increment() && self.timestamp() == rhs.timestamp()
    }
}

impl Eq for BsonTimestamp {}

impl BsonTimestamp {
    #[inline]
    pub fn increment(self) -> i32 { self.0.i }

    #[inline]
    pub fn timestamp(self) -> i32 { self.0.t }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BsonBinaryType {
    Binary,
    Func,
    BinaryOld,
    Uuid,
    Md5,
    User
}

impl BsonBinaryType {
    pub fn from_i8(value: i8) -> Option<BsonBinaryType> {
        match value as c_uint {
            ejdb_sys::BSON_BIN_BINARY     => Some(BsonBinaryType::Binary),
            ejdb_sys::BSON_BIN_FUNC       => Some(BsonBinaryType::Func),
            ejdb_sys::BSON_BIN_BINARY_OLD => Some(BsonBinaryType::BinaryOld),
            ejdb_sys::BSON_BIN_UUID       => Some(BsonBinaryType::Uuid),
            ejdb_sys::BSON_BIN_MD5        => Some(BsonBinaryType::Md5),
            ejdb_sys::BSON_BIN_USER       => Some(BsonBinaryType::User),
            _                             => None
        }
    }

    pub fn to_i8(self) -> i8 {
        (match self {
            BsonBinaryType::Binary => ejdb_sys::BSON_BIN_BINARY,
            BsonBinaryType::Func => ejdb_sys::BSON_BIN_FUNC,
            BsonBinaryType::BinaryOld => ejdb_sys::BSON_BIN_BINARY_OLD,
            BsonBinaryType::Uuid => ejdb_sys::BSON_BIN_UUID,
            BsonBinaryType::Md5 => ejdb_sys::BSON_BIN_MD5,
            BsonBinaryType::User => ejdb_sys::BSON_BIN_USER
        }) as i8
    }
}

// TODO: check packing for bson_oid_t, there's something with #pragma pack in the original code
#[derive(Copy, Clone)]
pub struct BsonOid(ejdb_sys::bson_oid_t);

impl fmt::Display for BsonOid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; 25];
        unsafe {
            ejdb_sys::bson_oid_to_string(&self.0, buf.as_mut_ptr() as *mut _);
        }
        write!(f, "{}", unsafe { str::from_utf8_unchecked(&buf[..buf.len()-1]) })
    }
}

impl fmt::Debug for BsonOid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BsonOid({})", self)
    }
}

pub enum Bson {
    #[doc(hidden)]
    Value(ejdb_sys::bson),
    #[doc(hidden)]
    Pointer(*mut ejdb_sys::bson)
}

impl fmt::Debug for Bson {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Bson::Value(_) => write!(f, "Bson::Value"),
            Bson::Pointer(p) => write!(f, "Bson::Pointer({:p})", p)
        }
    }
}

impl Drop for Bson {
    fn drop(&mut self) {
        match *self {
            Bson::Value(ref mut value) => unsafe { ejdb_sys::bson_destroy(value as *mut _); },
            Bson::Pointer(ptr) => unsafe { ejdb_sys::bson_del(ptr); }
        }
    }
}

impl Clone for Bson {
    fn clone(&self) -> Bson {
        let mut result = self.new_like_this();
        unsafe {
            ejdb_sys::bson_copy(result.as_raw_mut(), self.as_raw());
        }
        result
    }
}

impl Bson {
    pub fn new() -> Bson {
        let mut bson = unsafe { mem::uninitialized() };
        unsafe { ejdb_sys::bson_init(&mut bson); }
        Bson::Value(bson)
    }

    pub fn new_as_query() -> Bson {
        let mut bson = unsafe { mem::uninitialized() };
        unsafe { ejdb_sys::bson_init_as_query(&mut bson); }
        Bson::Value(bson)
    }

    #[inline]
    pub fn new_like_this(&self) -> Bson {
        if self.is_query() { Bson::new_as_query() } else { Bson::new() }
    }

    #[inline]
    pub fn from_json<S: Into<Vec<u8>>>(json: S) -> Option<Bson> {
        let json = CString::new(json).unwrap();
        match unsafe { ejdb_sys::json2bson(json.as_ptr()) } {
            p if p.is_null() => None,
            p => Some(Bson::Pointer(p))
        }
    }

    #[inline]
    pub fn to_json(&self) -> Option<String> {
        let mut out_str = ptr::null_mut();
        let mut out_len = 0;
        let result = unsafe {
            ejdb_sys::bson2json((*self.as_raw()).data, &mut out_str, &mut out_len)
        };

        let result = if !out_str.is_null() && result == ejdb_sys::BSON_OK {
            let slice = unsafe { slice::from_raw_parts(out_str as *const _, out_len as usize) };
              // should be OK as it is valid json
            Some(unsafe { String::from_utf8_unchecked(slice.to_owned()) })
        } else {
            None
        };

        // need to free with malloc
        if !out_str.is_null() {
            unsafe { libc::free(out_str as *mut _); }
        }

        result
    }

    #[inline]
    pub fn is_query(&self) -> bool {
        unsafe { (*self.as_raw()).flags as c_uint & ejdb_sys::BSON_FLAG_QUERY_MODE != 0 }
    }

    #[inline]
    pub unsafe fn as_raw(&self) -> *const ejdb_sys::bson {
        match *self {
            Bson::Value(ref value) => value as *const _,
            Bson::Pointer(ptr) => ptr as *const _
        }
    }

    pub unsafe fn as_raw_mut(&mut self) -> *mut ejdb_sys::bson {
        match *self {
            Bson::Value(ref mut value) => value as *mut _,
            Bson::Pointer(ptr) => ptr
        }
    }

    pub fn finish(mut self) -> Bson {
        // TODO: handle error
        unsafe { ejdb_sys::bson_finish(self.as_raw_mut()); }
        self
    }

    pub fn start_object(mut self, name: &[u8]) -> Bson {
        // TODO: handle error
        unsafe {
            ejdb_sys::bson_append_start_object2(
                self.as_raw_mut(), name.as_ptr() as *const _, name.len() as c_int
            );
        }
        self
    }

    pub fn finish_object(mut self) -> Bson {
        // TODO: handle error
        unsafe {
            ejdb_sys::bson_append_finish_object(self.as_raw_mut());
        }
        self
    }

    pub fn start_array(mut self, name: &[u8]) -> Bson {
        // TODO: handle error
        unsafe {
            ejdb_sys::bson_append_start_array2(
                self.as_raw_mut(), name.as_ptr() as *const _, name.len() as c_int
            );
        }
        self
    }

    pub fn finish_array(mut self) -> Bson {
        // TODO: handle error
        unsafe {
            ejdb_sys::bson_append_finish_array(self.as_raw_mut());
        }
        self
    }

    pub fn check_duplicate_keys(&self) -> bool {
        unsafe {
            ejdb_sys::bson_check_duplicate_keys(self.as_raw()) != 0
        }
    }

    pub fn fix_duplicate_keys(&self) -> Bson {
        let mut out = self.new_like_this();
        // TODO: handle errors
        unsafe {
            ejdb_sys::bson_fix_duplicate_keys(self.as_raw(), out.as_raw_mut());
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
            ejdb_sys::bson_merge(
                self.as_raw(), other.as_raw(),
                overwrite as ejdb_sys::bson_bool_t,
                out.as_raw_mut()
            );
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
            ejdb_sys::bson_merge_recursive(
                self.as_raw(), other.as_raw(),
                overwrite as ejdb_sys::bson_bool_t,
                out.as_raw_mut()
            );
        }

        out
    }

    pub fn iter(&self) -> BsonObjectIterator {
        let mut iterator = unsafe { mem::uninitialized() };
        unsafe {
            ejdb_sys::bson_iterator_init(&mut iterator, self.as_raw());
        }
        BsonObjectIterator(iterator, PhantomData)
    }

    pub fn append_all_from_iter(&mut self, source: &mut BsonObjectIterator) {
        while source.has_more() {
            unsafe {
                ejdb_sys::bson_append_element(self.as_raw_mut(), ptr::null(), &source.0);
            }
            source.advance();
        }
    }

    pub fn append_from_iter<K: Into<Vec<u8>>>(&mut self, custom_key: Option<K>, source: &BsonObjectIterator) {
        let data;
        let key = match custom_key.map(CString::new) {
            Some(cstr) => {
                data = cstr.unwrap();
                data.as_ptr()
            }
            None => ptr::null()
        };

        unsafe {
            ejdb_sys::bson_append_element(self.as_raw_mut(), key, &source.0);
        }
    }

    pub fn validate(&mut self, check_dots: bool, check_dollar: bool) -> bool {
        unsafe {
            ejdb_sys::bson_validate(self.as_raw_mut(), check_dots as u8, check_dollar as u8) ==
                ejdb_sys::BSON_OK
        }
    }
}

pub struct BsonObjectIterator<'bson>(ejdb_sys::bson_iterator, PhantomData<&'bson ejdb_sys::bson>);

impl<'bson> BsonObjectIterator<'bson> {
    #[inline]
    pub fn has_more(&self) -> bool {
        unsafe { ejdb_sys::bson_iterator_more(&self.0) != 0 }
    }

    #[inline]
    pub fn advance(&mut self) {
        unsafe { ejdb_sys::bson_iterator_next(&mut self.0); }
    }
}

impl<'bson> Iterator for BsonObjectIterator<'bson> {
    type Item = (&'bson [u8], BsonIteratorItem<'bson>);

    fn next(&mut self) -> Option<(&'bson [u8], BsonIteratorItem<'bson>)> {
        match unsafe { ejdb_sys::bson_iterator_next(&mut self.0) } {
            ejdb_sys::BSON_EOO => None,
            _ => {
                let key_ptr = unsafe { ejdb_sys::bson_iterator_key(&self.0) };
                let key_cstr = unsafe { CStr::from_ptr(key_ptr) };
                Some((key_cstr.to_bytes(), BsonIteratorItem::from_iterator(&self.0)))
            }
        }
    }
}

impl<'bson> Clone for BsonObjectIterator<'bson> {
    fn clone(&self) -> BsonObjectIterator<'bson> {
        BsonObjectIterator(self.0, self.1)
    }
}

impl<'bson> fmt::Debug for BsonObjectIterator<'bson> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BsonObjectIterator {{ cur: {:p}, first: {} }}", self.0.cur, self.0.first)
    }
}

pub struct BsonArrayIterator<'bson>(ejdb_sys::bson_iterator, PhantomData<&'bson ejdb_sys::bson>);

impl<'bson> Iterator for BsonArrayIterator<'bson> {
    type Item = BsonIteratorItem<'bson>;

    fn next(&mut self) -> Option<BsonIteratorItem<'bson>> {
        match unsafe { ejdb_sys::bson_iterator_next(&mut self.0) } {
            ejdb_sys::BSON_EOO => None,
            _ => Some(BsonIteratorItem::from_iterator(&self.0))
        }
    }
}

impl<'bson> Clone for BsonArrayIterator<'bson> {
    fn clone(&self) -> BsonArrayIterator<'bson> {
        BsonArrayIterator(self.0, self.1)
    }
}

impl<'bson> fmt::Debug for BsonArrayIterator<'bson> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BsonArrayIterator {{ cur: {:p}, first: {} }}", self.0.cur, self.0.first)
    }
}

#[derive(Clone, Debug)]
pub enum BsonIteratorItem<'bson> {
    Double(f64),
    Int(i32),
    Long(i64),
    Timestamp(BsonTimestamp),
    Bool(bool),
    Oid(BsonOid),
    String(&'bson [u8]),
    Code(&'bson [u8], Option<Bson>),
    Date(BsonDate),
    Binary(BsonBinaryType, &'bson [u8]),
    // TODO: regex
    Object(BsonObjectIterator<'bson>),
    Array(BsonArrayIterator<'bson>)
}

impl<'bson> BsonIteratorItem<'bson> {
    fn from_iterator(iterator: &ejdb_sys::bson_iterator) -> BsonIteratorItem<'bson> {
        let iterator = iterator as *const _;
        match unsafe { ejdb_sys::bson_iterator_type(iterator) } {
            ejdb_sys::BSON_DOUBLE => BsonIteratorItem::Double(unsafe {
                ejdb_sys::bson_iterator_double_raw(iterator)
            }),
            ejdb_sys::BSON_INT => BsonIteratorItem::Int(unsafe {
                ejdb_sys::bson_iterator_int_raw(iterator) as i32
            }),
            ejdb_sys::BSON_LONG => BsonIteratorItem::Long(unsafe {
                ejdb_sys::bson_iterator_long_raw(iterator) as i64
            }),
            ejdb_sys::BSON_TIMESTAMP => BsonIteratorItem::Timestamp(unsafe {
                BsonTimestamp(ejdb_sys::bson_iterator_timestamp(iterator))
            }),
            ejdb_sys::BSON_BOOL => BsonIteratorItem::Bool(unsafe {
                ejdb_sys::bson_iterator_bool_raw(iterator) != 0
            }),
            ejdb_sys::BSON_OID => BsonIteratorItem::Oid(unsafe {
                BsonOid(*ejdb_sys::bson_iterator_oid(iterator))
            }),
            ejdb_sys::BSON_STRING => BsonIteratorItem::String(unsafe {
                let data = ejdb_sys::bson_iterator_string(iterator) as *const _;
                let len = ejdb_sys::bson_iterator_string_len(iterator) as usize - 1;  // ignore zero byte
                slice::from_raw_parts(data, len)
            }),
            ejdb_sys::BSON_CODE => BsonIteratorItem::Code(unsafe {
                let data = ejdb_sys::bson_iterator_code(iterator);
                let data_cstr = CStr::from_ptr(data);
                data_cstr.to_bytes()
            }, None),
            ejdb_sys::BSON_CODEWSCOPE => BsonIteratorItem::Code(unsafe {
                let data = ejdb_sys::bson_iterator_code(iterator);
                let data_cstr = CStr::from_ptr(data);
                data_cstr.to_bytes()
            }, unsafe {
                let mut bson = Bson::new();
                ejdb_sys::bson_iterator_code_scope(iterator, bson.as_raw_mut());
                Some(bson)
            }),
            ejdb_sys::BSON_DATE => BsonIteratorItem::Date(unsafe {
                ejdb_sys::bson_iterator_date(iterator)
            }),
            ejdb_sys::BSON_BINDATA => BsonIteratorItem::Binary(unsafe {
                BsonBinaryType::from_i8(ejdb_sys::bson_iterator_bin_type(iterator)).unwrap()
            }, unsafe {
                let data = ejdb_sys::bson_iterator_bin_data(iterator) as *const _;
                let len = ejdb_sys::bson_iterator_bin_len(iterator) as usize;
                slice::from_raw_parts(data, len)
            }),
            ejdb_sys::BSON_OBJECT => BsonIteratorItem::Object(unsafe {
                let mut sub_iterator = mem::uninitialized();
                ejdb_sys::bson_iterator_subiterator(iterator, &mut sub_iterator);
                BsonObjectIterator(sub_iterator, PhantomData)
            }),
            ejdb_sys::BSON_ARRAY => BsonIteratorItem::Array(unsafe {
                let mut sub_iterator = mem::uninitialized();
                ejdb_sys::bson_iterator_subiterator(iterator, &mut sub_iterator);
                BsonArrayIterator(sub_iterator, PhantomData)
            }),
            tpe => panic!("Unsupported BSON type: {}", tpe)
        }
    }
}

macro_rules! gen_append_method {
    ($method_name:ident (|$($arg:ident : $arg_t:ty),+| $ffi_fn:ident ($($e:expr),+))) => {
        pub fn $method_name<K: Into<Vec<u8>>>(mut self, key: K, $($arg: $arg_t),+) -> Bson {
            let key_cstr = CString::new(key).unwrap();

            // TODO: check for errors
            unsafe {
                ejdb_sys::$ffi_fn(self.as_raw_mut(), key_cstr.as_ptr(), $($e),+);
            }

            self
        }
    };
    ($method_name:ident (|| $ffi_fn:ident ())) => {
        pub fn $method_name<K: Into<Vec<u8>>>(mut self, key: K) -> Bson {
            let key_cstr = CString::new(key).unwrap();

            // TODO: check for errors
            unsafe {
                ejdb_sys::$ffi_fn(self.as_raw_mut(), key_cstr.as_ptr());
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
    ($($tpe:ident),+; $($decl:tt)*) => {
        $(
            impl $tpe {
                gen_append_methods! { $($decl)* }
            }
        )+
    };
}

gen_appends! { Bson, BsonArrayBuilder;
    append_oid (|value: &BsonOid| bson_append_oid(&value.0)),
    append_new_oid (|| bson_append_new_oid()),
    append_int (|value: i32| bson_append_int(value as c_int)),
    append_long (|value: i64| bson_append_long(value)),
    append_double (|value: f64| bson_append_double(value)),
    append_string (like_string, bson_append_string_n),
    append_code (like_string, bson_append_code_n),
    append_code_with_scope (|value: &[u8], scope: &Bson| bson_append_code_w_scope_n(
        value.as_ptr() as *const _, value.len() as c_int, scope.as_raw()
    )),
    append_symbol (like_string, bson_append_symbol_n),
    append_binary (|binary_type: BsonBinaryType, value: &[u8]| bson_append_binary(
        binary_type.to_i8(), value.as_ptr() as *const _, value.len() as c_int
    )),
    append_bool(|value: bool| bson_append_bool(value as ejdb_sys::bson_bool_t)),
    append_null(|| bson_append_null()),
    append_undefined(|| bson_append_undefined()),
    append_bson(|value: &Bson| bson_append_bson(value.as_raw())),
    append_timestamp(|value: BsonTimestamp| bson_append_timestamp2(value.timestamp(), value.increment())),
    append_date(|value: BsonDate| bson_append_date(value))
    // TODO: bson_append_regex
}
