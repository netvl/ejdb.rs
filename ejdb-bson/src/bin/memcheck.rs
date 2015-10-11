extern crate ejdb_bson;

use ejdb_bson::Bson;

fn main() {
    let bson = Bson::new()
        .append_string(b"hello" as &[u8], b"world" as &[u8])
        .append_int(b"id" as &[u8], 123)
        .append_bool(b"awesome" as &[u8], true);
    println!("{}", bson.to_json().unwrap());
}
