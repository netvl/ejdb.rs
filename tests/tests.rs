#[macro_use(bson)]
extern crate ejdb;
extern crate tempdir;
extern crate bson;

use tempdir::TempDir;

use ejdb::{Database, CollectionOptions};
use ejdb::query::{Q, QH};
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

    let item_1 = coll.load(&ids[0]).unwrap().unwrap();
    assert_eq!(item_1, bson! {
        "_id" => (ids[0].clone()),
        "name" => "Foo",
        "count" => 123
    });

    let item_2 = coll.load(&ids[1]).unwrap().unwrap();
    assert_eq!(item_2, bson! {
        "_id" => (ids[1].clone()),
        "name" => "Bar",
        "items" => [1, "hello", 42.3]
    });

    let item_3 = coll.load(&ids[2]).unwrap().unwrap();
    assert_eq!(item_3, bson! {
        "_id" => (ids[2].clone()),
        "title" => "Baz",
        "subobj" => {
            "key" => "a",
            "xyz" => 632
        }
    });
}

#[test]
fn test_query() {
    let (db, _dir) = make_db();

    let coll = db.collection("test").unwrap();

    let ids = coll.save_all(vec![
        bson!{ "name" => "Foo", "count" => 123 },
        bson!{ "name" => "Foo Foo", "count" => 345 },
        bson!{ "name" => "Foo Bar", "count" => 23 },
        bson!{ "name" => "Bar", "items" => [1, "hello", 42.3] },
        bson!{ "title" => "Baz", "subobj" => { "key" => "a", "xyz" => 632 } }
    ]).unwrap();

    let n_foo = coll.query(Q.field("name").eq(("Foo".to_owned(), "".to_owned())), QH.empty())
        .count().unwrap();
    assert_eq!(n_foo, 3);

    let n_bar = coll.query(Q.field("name").eq(("Bar".to_owned(), "".to_owned())), QH.empty())
        .count().unwrap();
    assert_eq!(n_bar, 2);

    let foos = coll.query(Q.field("count").gt(100), QH.order_by("count").desc()).find().unwrap();
    let foos_vec: ejdb::Result<Vec<_>> = foos.collect();
    assert_eq!(foos_vec.unwrap(), vec![
        bson! { "_id" => (ids[1].clone()), "name" => "Foo Foo", "count" => 345 },
        bson! { "_id" => (ids[0].clone()), "name" => "Foo", "count" => 123 }
    ]);

    let baz = coll.query(Q.field("subobj.xyz").eq(632), QH.empty()).find_one().unwrap();
    assert!(baz.is_some());
    assert_eq!(baz.unwrap(), bson! {
        "_id" => (ids[4].clone()),
        "title" => "Baz",
        "subobj" => {
            "key" => "a",
            "xyz" => 632
        }
    });

    let result = coll.query(Q.field("items").exists(true).push_all("items", vec![1, 2, 3]), QH.empty())
        .update().unwrap();
    assert_eq!(result, 1);
    let bar = coll.load(&ids[3]).unwrap().unwrap();
    assert_eq!(bar, bson! {
        "_id" => (ids[3].clone()),
        "name" => "Bar",
        "items" => [1, "hello", 42.3, 1, 2, 3]
    });
}

fn make_db() -> (Database, TempDir) {
    let dir = TempDir::new("ejdb").expect("cannot create temporary directory");
    let db = Database::open(dir.path().join("db").to_str().unwrap()).expect("cannot create database");
    (db, dir)
}
