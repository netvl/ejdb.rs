//! Contains low-level utilities for conversion between Rust and EJDB BSON representations.
//!
//! This module is only public to facilitate direct usage of `ejdb-sys` library, if such
//! need arises. The types provided here are useful for converting Rust BSON values
//! to EJDB ones and vice versa.
//!
//! Types from this module should not be used unless absolutely necessary.

use std::slice;

use bson::oid;
use bson::{self, DecoderResult, Document, EncoderResult};
use ejdb_sys;

pub struct EjdbBsonDocument(*mut ejdb_sys::bson);

impl EjdbBsonDocument {
    pub fn empty() -> EjdbBsonDocument {
        unsafe {
            // TODO: check for alloc errors properly
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
            EjdbBsonDocument(ejdb_sys::bson_create_from_buffer(
                buf.as_ptr() as *const _,
                buf.len() as i32,
            ))
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
    pub fn as_raw(&self) -> *const ejdb_sys::bson {
        self.0 as *const _
    }

    #[inline]
    pub fn as_raw_mut(&mut self) -> *mut ejdb_sys::bson {
        self.0 as *mut _
    }
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
        let empty_arr: [i8; 12] = [0; 12];
        EjdbObjectId(ejdb_sys::bson_oid_t { bytes: empty_arr })
    }

    #[inline]
    pub fn to_rust(self) -> oid::ObjectId {
        let bytes: [i8; 12];
        unsafe {
            bytes = (self.0).bytes;
        }
        oid::ObjectId::with_bytes(to_u(bytes))
    }

    #[inline]
    pub fn from_rust(oid: oid::ObjectId) -> EjdbObjectId {
        EjdbObjectId(ejdb_sys::bson_oid_t {
            bytes: to_i(oid.bytes()),
        })
    }

    #[inline]
    pub fn to_ejdb(self) -> ejdb_sys::bson_oid_t {
        self.0
    }

    #[inline]
    pub fn as_raw(&self) -> *const ejdb_sys::bson_oid_t {
        &self.0
    }

    #[inline]
    pub fn as_raw_mut(&mut self) -> *mut ejdb_sys::bson_oid_t {
        &mut self.0
    }
}

impl From<ejdb_sys::bson_oid_t> for EjdbObjectId {
    #[inline]
    fn from(oid: ejdb_sys::bson_oid_t) -> EjdbObjectId {
        EjdbObjectId(oid)
    }
}

impl From<oid::ObjectId> for EjdbObjectId {
    #[inline]
    fn from(oid: oid::ObjectId) -> EjdbObjectId {
        EjdbObjectId::from_rust(oid)
    }
}

impl Into<ejdb_sys::bson_oid_t> for EjdbObjectId {
    #[inline]
    fn into(self) -> ejdb_sys::bson_oid_t {
        self.to_ejdb()
    }
}

impl Into<oid::ObjectId> for EjdbObjectId {
    #[inline]
    fn into(self) -> oid::ObjectId {
        self.to_rust()
    }
}

fn to_i(arr: [u8; 12]) -> [i8; 12] {
    let mut result: [i8; 12] = [0; 12];
    for i in 0..arr.len() {
        result[i] = arr[i] as i8;
    }
    return result;
}

fn to_u(arr: [i8; 12]) -> [u8; 12] {
    let mut result: [u8; 12] = [0; 12];
    for i in 0..arr.len() {
        result[i] = arr[i] as u8;
    }
    return result;
}
