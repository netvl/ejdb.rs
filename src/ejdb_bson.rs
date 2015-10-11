use std::slice;

use ejdb_sys;
use bson::{self, Document, EncoderResult, DecoderResult};

pub struct EjdbBsonDocument(*mut ejdb_sys::bson);

impl EjdbBsonDocument {
    #[inline]
    pub unsafe fn from_ptr(ptr: &mut ejdb_sys::bson) -> EjdbBsonDocument {
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
