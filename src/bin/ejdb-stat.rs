extern crate ejdb;

use std::env;
use std::io::Write;

use ejdb::{Database, OpenMode};

macro_rules! abort {
    ($code:expr, $($args:tt)*) => {{
        let _ = writeln!(&mut ::std::io::stderr(), $($args)*);
        ::std::process::exit($code);
    }}
}

fn main() {
    let db_path = env::args().nth(1).unwrap_or_else(|| abort!(1, "Usage: ejdb-stat <database>"));

    let db = Database::open(db_path, OpenMode::default())
        .unwrap_or_else(|e| abort!(1, "Error opening database: {}", e));
    println!("Collections:");
    for name in db.get_collection_names()
            .unwrap_or_else(|e| abort!(1, "Error getting collection names: {}", e)) {
        println!("* {}", name);
    }
}
