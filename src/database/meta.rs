use std::iter;
use std::slice;
use std::ops::Deref;
use std::borrow::Cow;
use std::str::FromStr;
use std::result;

use bson::{Document, Bson, ValueAccessError};
use ejdb_sys;

use super::Database;
use ejdb_bson::EjdbBsonDocument;
use Result;

impl Database {
    pub fn get_metadata(&self) -> Result<DatabaseMetadata> {
        let doc = unsafe { ejdb_sys::ejdbmeta(self.0) };
        if doc.is_null() {
            return self.last_error("cannot load metadata");
        } else {
            let bson_doc = unsafe {
                try!(EjdbBsonDocument::from_ptr(doc).to_bson())
            };
            Ok(DatabaseMetadata(bson_doc))
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct DatabaseMetadata(Document);

impl DatabaseMetadata {
    #[inline]
    pub fn into_inner(self) -> Document { self.0 }

    pub fn file(&self) -> &str {
        self.0.get_str("file").expect("cannot get database file name")
    }

    pub fn collections(&self) -> Collections {
        self.0.get_array("collections").expect("cannot get collections metadata")
            .iter().map(parse_collection_metadata)
    }
}

impl Deref for DatabaseMetadata {
    type Target = Document;

    #[inline]
    fn deref(&self) -> &Document { &self.0 }
}

pub type Collections<'a> = iter::Map<
    slice::Iter<'a, Bson>,
    for<'d> fn(&'d Bson) -> CollectionMetadata<'d>
>;

fn parse_collection_metadata(bson: &Bson) -> CollectionMetadata {
    match *bson {
        Bson::Document(ref doc) => CollectionMetadata(Cow::Borrowed(doc)),
        ref something_else => panic!("invalid collections metadata: {}", something_else.to_json())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct CollectionMetadata<'a>(Cow<'a, Document>);

impl<'a> CollectionMetadata<'a> {
    pub fn name(&self) -> &str {
        self.0.get_str("name").expect("cannot get collection name")
    }

    pub fn file(&self) -> &str {
        self.0.get_str("file").expect("cannot get collection file name")
    }

    pub fn records(&self) -> u64 {
        self.0.get_i64("records").expect("cannot get collection records count") as u64
    }

    fn options(&self) -> &Document {
        self.0.get_document("options").expect("cannot get collection options")
    }

    pub fn buckets(&self) -> u64 {
        self.options().get_i64("buckets").expect("cannot get collection buckets count") as u64
    }

    pub fn cached_records(&self) -> u64 {
        self.options().get_i64("cachedrecords").expect("cannot get collection cached records count") as u64
    }

    pub fn large(&self) -> bool {
        self.options().get_bool("large").expect("cannot get collection large flag")
    }

    pub fn compressed(&self) -> bool {
        self.options().get_bool("compressed").expect("cannot get collection compressed flag")
    }

    pub fn indices(&self) -> CollectionIndices {
        self.0.get_array("indexes").expect("cannot get collection indices array")
            .iter().map(parse_index_metadata)
    }
}

pub type CollectionIndices<'a> = iter::Map<
    slice::Iter<'a, Bson>,
    for<'d> fn(&'d Bson) -> IndexMetadata<'d>
>;

impl<'a> Deref for CollectionMetadata<'a> {
    type Target = Document;

    #[inline]
    fn deref(&self) -> &Document { &*self.0 }
}

fn parse_index_metadata(bson: &Bson) -> IndexMetadata {
    match *bson {
        Bson::Document(ref doc) => IndexMetadata(doc),
        ref something_else => panic!("invalid index metadata: {}", something_else.to_json())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct IndexMetadata<'a>(&'a Document);

impl<'a> IndexMetadata<'a> {
    pub fn field(&self) -> &str {
        self.0.get_str("field").expect("cannot get index field")
    }

    pub fn name(&self) -> &str {
        self.0.get_str("iname").expect("cannot get index name")
    }

    pub fn index_type(&self) -> IndexType {
        self.0.get_str("type").expect("cannot get index type")
            .parse().expect("invalid index type")
    }

    pub fn records(&self) -> Option<u64> {
        match self.0.get_i64("records") {
            Ok(n) => Some(n as u64),
            Err(ValueAccessError::NotPresent) => None,
            Err(_) => panic!("cannot get index records count")
        }
    }

    pub fn file(&self) -> Option<&str> {
        match self.0.get_str("file") {
            Ok(f) => Some(f),
            Err(ValueAccessError::NotPresent) => None,
            Err(_) => panic!("cannot get index file")
        }
    }
}

impl<'a> Deref for IndexMetadata<'a> {
    type Target = Document;

    #[inline]
    fn deref(&self) -> &Document { &*self.0 }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum IndexType {
    Lexical,
    Decimal,
    Token
}

impl FromStr for IndexType {
    type Err = String;

    fn from_str(s: &str) -> result::Result<IndexType, String> {
        match s {
            "lexical" => Ok(IndexType::Lexical),
            "decimal" => Ok(IndexType::Decimal),
            "token"   => Ok(IndexType::Token),
            s => Err(s.into())
        }
    }
}

#[test]
#[ignore]
fn test_metadata() {
    let db = Database::open("db/test", ::OpenMode::default()).unwrap();
    let meta = db.get_metadata().unwrap();
    println!("{}", Bson::Document(meta.into_inner()).to_json());
}
