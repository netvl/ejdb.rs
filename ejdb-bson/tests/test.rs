extern crate ejdb_bson;

use ejdb_bson::{Bson, BsonIteratorItem};

#[test]
fn test_simple_build_and_iterate() {
    let bson = Bson::new()
        .append_string(b"hello" as &[u8], b"world" as &[u8])
        .append_int(b"id" as &[u8], 123)
        .append_bool(b"awesome" as &[u8], true);

    for (k, v) in bson.iter() {
        match k {
            b"hello" => match v {
                BsonIteratorItem::String(s) => assert_eq!(s, b"world"),
                item => panic!("Invalid item for key hello: {:?}", item)
            },
            b"id" => match v {
                BsonIteratorItem::Int(n) => assert_eq!(n, 123),
                item => panic!("Invalid item for key id: {:?}", item)
            },
            b"awesome" => match v {
                BsonIteratorItem::Bool(b) => assert_eq!(b, true),
                item => panic!("Invalid item for key awesome: {:?}", item)
            },
            key => panic!("Unexpected key: {:?}", key)
        }
    }
}
