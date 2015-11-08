#[macro_use(bson)]
extern crate ejdb;
extern crate tempdir;
extern crate bson;

use tempdir::TempDir;

use ejdb::{Database, CollectionOptions};
use ejdb::meta::IndexType;

#[test]
fn test_meta() {
    let (db, dir) = make_db();

    db.collection("test_1").unwrap().save_all(vec![
        bson!{ "name" => "Foo", "count" => 123 },
        bson!{ "name" => "Bar", "whatever" => ["a", 1, 42.3] }
    ]).unwrap();

    let options = CollectionOptions::default().compressed(true).cached_records(512);
    let coll_2 = options.get_or_create(&db, "test_2").unwrap();
    coll_2.index("name").string(true).set().unwrap();
    coll_2.index("count").number().set().unwrap();

    let meta = db.get_metadata().unwrap();

    assert_eq!(meta.file(), format!("{}/db", dir.path().display()));

    let collections: Vec<_> = meta.collections().collect();
    assert_eq!(collections.len(), 2);

    for coll in collections {
        match coll.name() {
            "test_1" => {
                assert_eq!(coll.file(), format!("{}/db_test_1", dir.path().display()));
                assert_eq!(coll.records(), 2);
                assert_eq!(coll.buckets(), 262139);
                assert_eq!(coll.cached_records(), 0);
                assert_eq!(coll.compressed(), false);
                assert_eq!(coll.large(), false);
                assert_eq!(coll.indices().len(), 0);
            }
            "test_2" => {
                assert_eq!(coll.file(), format!("{}/db_test_2", dir.path().display()));
                assert_eq!(coll.records(), 0);
                assert_eq!(coll.buckets(), 262139);
                assert_eq!(coll.cached_records(), 512);
                assert_eq!(coll.compressed(), true);
                assert_eq!(coll.large(), false);

                let indices: Vec<_> = coll.indices().collect();
                assert_eq!(indices.len(), 2);

                for index in indices {
                    match index.field() {
                        "name" => {
                            assert_eq!(index.name(), "sname");
                            assert_eq!(index.file(), Some(&*format!("{}/db_test_2.idx.sname.lex", dir.path().display())));
                            assert_eq!(index.records(), Some(0));
                            assert_eq!(index.index_type(), IndexType::Lexical);
                        }
                        "count" => {
                            assert_eq!(index.name(), "ncount");
                            assert_eq!(index.file(), Some(&*format!("{}/db_test_2.idx.ncount.dec", dir.path().display())));
                            assert_eq!(index.records(), Some(0));
                            assert_eq!(index.index_type(), IndexType::Decimal);
                        }
                        _ => panic!("unknown index: {:?}", index)
                    }
                }
            }
            _ => panic!("unknown collection: {:?}", coll)
        }
    }
}

#[test]
fn test_save_load() {
    let (db, _dir) = make_db();

    let coll = db.collection("test").unwrap();
    let ids = coll.save_all(vec![
        bson!{ "name" => "Foo", "count" => 123 },
        bson!{ "name" => "Bar", "items" => [1, "hello", 42.3] },
        bson!{ "title" => "Baz", "subobj" => { "key" => "a", "xyz" => 632 } }
    ]).unwrap();
    assert_eq!(ids.len(), 3);

    let item_1 = coll.load(ids[0].clone()).unwrap().unwrap();
    assert_eq!(item_1, bson! {
        "_id" => (ids[0].clone()),
        "name" => "Foo",
        "count" => 123
    });

    let item_2 = coll.load(ids[1].clone()).unwrap().unwrap();
    assert_eq!(item_2, bson! {
        "_id" => (ids[1].clone()),
        "name" => "Bar",
        "items" => [1, "hello", 42.3]
    });

    let item_3 = coll.load(ids[2].clone()).unwrap().unwrap();
    assert_eq!(item_3, bson! {
        "_id" => (ids[2].clone()),
        "title" => "Baz",
        "subobj" => {
            "key" => "a",
            "xyz" => 632
        }
    });
}

fn make_db() -> (Database, TempDir) {
    let dir = TempDir::new("ejdb").expect("cannot create temporary directory");
    let db = Database::open(dir.path().join("db").to_str().unwrap()).expect("cannot create database");
    (db, dir)
}
