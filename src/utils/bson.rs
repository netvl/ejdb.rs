use bson::Bson;

pub trait BsonNumber {
    fn to_bson(self) -> Bson;
}

impl BsonNumber for f32 {
    #[inline]
    fn to_bson(self) -> Bson {
        Bson::FloatingPoint(self as f64)
    }
}

impl BsonNumber for f64 {
    #[inline]
    fn to_bson(self) -> Bson {
        Bson::FloatingPoint(self)
    }
}

impl BsonNumber for i32 {
    #[inline]
    fn to_bson(self) -> Bson {
        Bson::I32(self)
    }
}

impl BsonNumber for i64 {
    #[inline]
    fn to_bson(self) -> Bson {
        Bson::I64(self)
    }
}
