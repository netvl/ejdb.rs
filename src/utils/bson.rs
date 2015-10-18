use bson::{Bson, Document};

pub struct DocumentBuilder(Document);

impl DocumentBuilder {
    #[inline]
    pub fn new() -> DocumentBuilder { DocumentBuilder(Document::new()) }

    #[inline]
    pub fn set<K: Into<String>, V: Into<Bson>>(mut self, k: K, v: V) -> DocumentBuilder {
        self.0.insert(k, v);
        self
    }

    #[inline]
    pub fn into_inner(self) -> Document { self.0 }
}

impl Into<Document> for DocumentBuilder {
    #[inline]
    fn into(self) -> Document { self.0 }
}

impl Into<Bson> for DocumentBuilder {
    #[inline]
    fn into(self) -> Bson { self.0.into() }
}

pub trait BsonNumber {
    fn to_bson(self) -> Bson;
}

impl BsonNumber for f32 {
    #[inline]
    fn to_bson(self) -> Bson { Bson::FloatingPoint(self as f64) }
}

impl BsonNumber for f64 {
    #[inline]
    fn to_bson(self) -> Bson { Bson::FloatingPoint(self) }
}

impl BsonNumber for i32 {
    #[inline]
    fn to_bson(self) -> Bson { Bson::I32(self) }
}

impl BsonNumber for i64 {
    #[inline]
    fn to_bson(self) -> Bson { Bson::I64(self) }
}
