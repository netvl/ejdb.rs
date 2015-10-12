use std::slice;

use ejdb_sys;
use bson::{self, Document, EncoderResult, DecoderResult};
use bson::oid;

pub struct EjdbBsonDocument(*mut ejdb_sys::bson);

impl EjdbBsonDocument {
    pub fn empty() -> EjdbBsonDocument {
        unsafe {
            // TODO: check for alloc errors
            let bson_ptr = ejdb_sys::bson_create();
            if bson_ptr.is_null() {
                panic!("Cannot allocate new BSON document");
            }
            ejdb_sys::bson_init(bson_ptr);
            EjdbBsonDocument::from_ptr(bson_ptr)
        }
    }

    #[inline]
    pub unsafe fn from_ptr(ptr: *mut ejdb_sys::bson) -> EjdbBsonDocument {
        EjdbBsonDocument(ptr)
    }

    #[inline]
    pub fn from_buffer(buf: &[u8]) -> EjdbBsonDocument {
        unsafe {
            EjdbBsonDocument(
                ejdb_sys::bson_create_from_buffer(buf.as_ptr() as *const _, buf.len() as i32)
            )
        }
    }

    pub fn from_bson(bson: &Document) -> EncoderResult<EjdbBsonDocument> {
        let mut buffer = Vec::new();
        bson::encode_document(&mut buffer, bson).map(|_| EjdbBsonDocument::from_buffer(&buffer))
    }

    pub fn to_bson(&self) -> DecoderResult<Document> {
        let buf_ptr = unsafe { ejdb_sys::bson_data(self.0 as *const _) as *const u8 };
        let buf_size = unsafe { ejdb_sys::bson_size(self.0 as *const _) };

        let mut buf = unsafe { slice::from_raw_parts(buf_ptr, buf_size as usize) };
        bson::decode_document(&mut buf)
    }

    #[inline]
    pub fn as_raw(&self) -> *const ejdb_sys::bson { self.0 as *const _ }

    #[inline]
    pub fn as_raw_mut(&mut self) -> *mut ejdb_sys::bson { self.0 as *mut _ }
}

impl Drop for EjdbBsonDocument {
    fn drop(&mut self) {
        unsafe {
            ejdb_sys::bson_del(self.0);
        }
    }
}

#[derive(Copy, Clone)]
pub struct EjdbObjectId(ejdb_sys::bson_oid_t);

impl EjdbObjectId {
    #[inline]
    pub fn empty() -> EjdbObjectId {
        EjdbObjectId(ejdb_sys::bson_oid_t::default())
    }

    #[inline]
    pub fn to_rust(self) -> oid::ObjectId {
        oid::ObjectId::with_bytes((self.0)._bindgen_data_)
    }

    #[inline]
    pub fn from_rust(oid: oid::ObjectId) -> EjdbObjectId {
        EjdbObjectId(ejdb_sys::bson_oid_t { _bindgen_data_: oid.bytes() })
    }

    #[inline]
    pub fn to_ejdb(self) -> ejdb_sys::bson_oid_t { self.0 }

    #[inline]
    pub fn as_raw(&self)-> *const ejdb_sys::bson_oid_t { &self.0 }

    #[inline]
    pub fn as_raw_mut(&mut self) -> *mut ejdb_sys::bson_oid_t { &mut self.0 }
}

impl From<ejdb_sys::bson_oid_t> for EjdbObjectId {
    #[inline]
    fn from(oid: ejdb_sys::bson_oid_t) -> EjdbObjectId { EjdbObjectId(oid) }
}

impl From<oid::ObjectId> for EjdbObjectId {
    #[inline]
    fn from(oid: oid::ObjectId) -> EjdbObjectId { EjdbObjectId::from_rust(oid) }
}

impl Into<ejdb_sys::bson_oid_t> for EjdbObjectId {
    #[inline]
    fn into(self) -> ejdb_sys::bson_oid_t { self.to_ejdb() }
}

impl Into<oid::ObjectId> for EjdbObjectId {
    #[inline]
    fn into(self) -> oid::ObjectId { self.to_rust() }
}
