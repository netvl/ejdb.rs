//! Types returned by metadata query method on `Database` structure.
//!
//! You can use `Database::get_metadata()` method to obtain an instance of `DatabaseMetadata`
//! structure. It provides a number of methods to access some information about the database:
//! file paths, collection names, indices, etc.
//!
//! Database metadata is a BSON object, therefore all significant types in this module
//! have `Deref<Target=bson::Document>` implementations. `DatabaseMetadata` additionally has
//! `into_inner()` method in case you need raw metadata document for some reason.

use std::iter;
use std::slice;
use std::ops::Deref;
use std::str::FromStr;
use std::result;

use bson::{Document, Bson, ValueAccessError};
use ejdb_sys;

use super::Database;
use ejdb_bson::EjdbBsonDocument;
use Result;

impl Database {
    /// Loads and returns information about the database.
    ///
    /// This method always reloads the metadata each time it is called, therefore, for example,
    /// if you called this method, then changed something in the database, and then called it
    /// again, its results will be different.
    ///
    /// # Failures
    ///
    /// Fails when the underlying EJDB operation can't be completed successfully or when
    /// the loaded BSON document can't be deserialized from EJDB representation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// let db = Database::open("/path/to/db").unwrap();
    /// let meta = db.get_metadata().unwrap();
    /// // work with the metadata object.
    /// ```
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

/// Represents metainformation about an EJDB database.
///
/// EJDB returns metadata in form of a BSON document, therefore this struct is just a wrapper
/// around a BSON document value. Due to `Deref` implementation, it is possible to call
/// `bson::Document` methods on this structure directly.
///
/// Note that EJDB metadata has fixed form, therefore every method which provides
/// access to the parts of metadata will panic if it can't obtain this part or if the actual
/// BSON value is of different type. If this happens, then it is a bug in this library.
#[derive(Clone, PartialEq, Debug)]
pub struct DatabaseMetadata(Document);

impl DatabaseMetadata {
    /// Consumes the metadata object, returning the underlying BSON document.
    #[inline]
    pub fn into_inner(self) -> Document { self.0 }

    /// Returns the file name of the main database file.
    pub fn file(&self) -> &str {
        self.0.get_str("file").expect("cannot get database file name")
    }

    /// Returns an iterator of metadata for each collection in the database.
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

/// A type alias for collections metadata iterator.
pub type Collections<'a> = iter::Map<
    slice::Iter<'a, Bson>,
    for<'d> fn(&'d Bson) -> CollectionMetadata<'d>
>;

fn parse_collection_metadata(bson: &Bson) -> CollectionMetadata {
    match *bson {
        Bson::Document(ref doc) => CollectionMetadata(doc),
        ref something_else => {
            panic!("invalid collections metadata: {}", something_else)}
    }
}

/// Represents metainformation about a collection in an EJDB database.
///
/// This structure only provides a view into the original BSON document, therefore it has
/// a lifetime dependency on the database metadata structure. It is also not possible to
/// unwrap the inner document, though it still has a `Deref` implementation with `bson::Document`
/// as a target.
#[derive(Clone, PartialEq, Debug)]
pub struct CollectionMetadata<'a>(&'a Document);

impl<'a> CollectionMetadata<'a> {
    /// Returns the name of this collection.
    pub fn name(&self) -> &str {
        self.0.get_str("name").expect("cannot get collection name")
    }

    /// Returns the file path of this collection.
    pub fn file(&self) -> &str {
        self.0.get_str("file").expect("cannot get collection file name")
    }

    /// Returns the number of records in this collection.
    pub fn records(&self) -> u64 {
        self.0.get_i64("records").expect("cannot get collection records count") as u64
    }

    fn options(&self) -> &Document {
        self.0.get_document("options").expect("cannot get collection options")
    }

    /// Returns the number of buckets in this collection.
    pub fn buckets(&self) -> u64 {
        self.options().get_i64("buckets").expect("cannot get collection buckets count") as u64
    }

    /// Returns the number of cached records for this collection.
    pub fn cached_records(&self) -> u64 {
        self.options().get_i64("cachedrecords").expect("cannot get collection cached records count") as u64
    }

    /// Returns `true` if the collection can hold more than 2GB of data, `false` otherwise.
    pub fn large(&self) -> bool {
        self.options().get_bool("large").expect("cannot get collection large flag")
    }

    /// Returns `true` if DEFLATE compression is applied to this collection's records, `false` otherwise.
    pub fn compressed(&self) -> bool {
        self.options().get_bool("compressed").expect("cannot get collection compressed flag")
    }

    /// Returns an iterator of metadata of indices in this collection.
    pub fn indices(&self) -> CollectionIndices {
        self.0.get_array("indexes").expect("cannot get collection indices array")
            .iter().map(parse_index_metadata)
    }
}

impl<'a> Deref for CollectionMetadata<'a> {
    type Target = Document;

    #[inline]
    fn deref(&self) -> &Document { &*self.0 }
}

/// A type alias for indices metadata iterator.
pub type CollectionIndices<'a> = iter::Map<
    slice::Iter<'a, Bson>,
    for<'d> fn(&'d Bson) -> IndexMetadata<'d>
>;

fn parse_index_metadata(bson: &Bson) -> IndexMetadata {
    match *bson {
        Bson::Document(ref doc) => IndexMetadata(doc),
        ref something_else => panic!("invalid index metadata: {}", something_else)
    }
}

/// Represents metainformation about an index of a collection in an EJDB database.
///
/// Like `CollectionMetadata`, this structure only provides a view into the full database
/// metadata object, so it is not possible to obtain a `bson::Document` directly.
/// `Deref<Target=bson::Document>` implementation is available.
#[derive(Clone, PartialEq, Debug)]
pub struct IndexMetadata<'a>(&'a Document);

impl<'a> IndexMetadata<'a> {
    /// Returns the name of the field on which this index is defined.
    pub fn field(&self) -> &str {
        self.0.get_str("field").expect("cannot get index field")
    }

    /// Returns the name of this index itself (usually it is automatically generated).
    pub fn name(&self) -> &str {
        self.0.get_str("iname").expect("cannot get index name")
    }

    /// Returns the type of this index.
    pub fn index_type(&self) -> IndexType {
        self.0.get_str("type").expect("cannot get index type")
            .parse().expect("invalid index type")
    }

    /// Returns the number of records using this index, if available.
    pub fn records(&self) -> Option<u64> {
        match self.0.get_i64("records") {
            Ok(n) => Some(n as u64),
            Err(ValueAccessError::NotPresent) => None,
            Err(_) => panic!("cannot get index records count")
        }
    }

    /// Returns the path to the file of this index, if available.
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

/// Represents an EJDB index type.
///
/// According to EJDB sources, `Lexical` is used for string indices, `Decimal` is used for
/// numerical indices and `Token` is used for array indices.
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
    let db = Database::open("db/test").unwrap();
    let meta = db.get_metadata().unwrap();
    println!("{}", Bson::Document(meta.into_inner()));
}
