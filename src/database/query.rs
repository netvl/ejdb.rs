use std::borrow::Cow;

use bson::{Bson, Document};

use utils::bson::BsonNumber;

pub struct QueryHints {
    max: Option<i64>,
    skip: Option<i64>,
    order_by: Vec<(String, i32)>,
    fields: Vec<(String, i32)>
}

impl QueryHints {
    #[inline]
    pub fn new() -> QueryHints {
        QueryHints {
            max: None,
            skip: None,
            order_by: Vec::new(),
            fields: Vec::new()
        }
    }

    #[inline]
    pub fn max(mut self, n: i64) -> QueryHints {
        self.max = Some(n);
        self
    }

    #[inline]
    pub fn skip(mut self, n: i64) -> QueryHints {
        self.skip = Some(n);
        self
    }

    #[inline]
    pub fn order_by<S: Into<String>>(self, field: S) -> QueryHintsOrderBy {
        QueryHintsOrderBy(self, field.into())
    }

    #[inline]
    pub fn field<S: Into<String>>(self, field: S) -> QueryHintsField {
        QueryHintsField(self, field.into())
    }
}

pub struct QueryHintsOrderBy(QueryHints, String);

impl QueryHintsOrderBy {
    #[inline]
    pub fn desc(mut self) -> QueryHints {
        self.0.order_by.push((self.1, -1));
        self.0
    }

    #[inline]
    pub fn asc(mut self) -> QueryHints {
        self.0.order_by.push((self.1, 1));
        self.0
    }
}

pub struct QueryHintsField(QueryHints, String);

impl QueryHintsField {
    #[inline]
    pub fn exclude(mut self) -> QueryHints {
        self.0.fields.push((self.1, -1));
        self.0
    }

    #[inline]
    pub fn include(mut self) -> QueryHints {
        self.0.fields.push((self.1, 1));
        self.0
    }
}

impl Into<Document> for QueryHints {
    fn into(self) -> Document {
        let mut doc = Document::new();

        if let Some(n) = self.max {
            doc.insert("$max", n);
        }

        if let Some(n) = self.skip {
            doc.insert("$skip", n);
        }

        if !self.order_by.is_empty() {
            let mut order_by = Document::new();
            for (k, v) in self.order_by {
                order_by.insert(k, v);
            }
            doc.insert("$orderBy", order_by);
        }

        if !self.fields.is_empty() {
            let mut fields = Document::new();
            for (k, v) in self.fields {
                fields.insert(k, v);
            }
            doc.insert("$fields", fields);
        }

        doc
    }
}

pub struct Query {
    hints: QueryHints,
    query: Document
}

impl Query {
    #[inline]
    pub fn new() -> Query {
        Query {
            hints: QueryHints::new(),
            query: Document::new()
        }
    }

    pub fn and<I, V>(mut self, queries: I) -> Query
            where V: Into<Document>, I: IntoIterator<Item=V>
    {
        self.query.insert(
            "$and",
            queries.into_iter().map(|v| v.into().into())
                .collect::<Vec<Bson>>()
        );
        self
    }

    pub fn or<I, V>(mut self, queries: I) -> Query
            where V: Into<Document>, I: IntoIterator<Item=V>
    {
        self.query.insert(
            "$or",
            queries.into_iter().map(|v| v.into().into())
                .collect::<Vec<Bson>>()
        );
        self
    }

    #[inline]
    pub fn id<V: Into<Bson>>(self, value: V) -> Query {
        self.field("_id").eq(value)
    }

    #[inline]
    pub fn field<S: Into<String>>(self, name: S) -> FieldConstraint {
        FieldConstraint(name.into().into(), FieldConstraintData::Root(self))
    }

    pub fn join<S: Into<String>, C: Into<String>>(self, key: S, coll: C) -> Query {
        self.add_subkey_at_key("$do", key, bson!("$join" => (coll.into())))
    }

    fn modify_document_at_key<K, F1, F2, V>(mut self, key: K, value: V,
                                            on_document: F1, on_something_else: F2) -> Query
        where K: Into<String> + AsRef<str>,
              F1: FnOnce(&mut Document, V),     // Document is value at key K
              F2: FnOnce(&mut Document, K, V),  // Document is the query itself
    {
        // unsafe is to overcome non-lexical borrow issues (and entry API is not available)
        let r = self.query.get_mut(key.as_ref()).map(|p| unsafe { &mut *(p as *mut _) });
        if let Some(&mut Bson::Document(ref mut d)) = r {
            on_document(d, value);
        } else {
            on_something_else(&mut self.query, key, value);
        }

        self
    }

    fn add_subkey_at_key<K, S, V>(self, key: K, subkey: S, value: V) -> Query
        where K: Into<String> + AsRef<str>,
              S: Into<String>,
              V: Into<Bson>
    {
        self.modify_document_at_key(
            key, (subkey, value),
            |d, (s, v)| { d.insert(s, v); },
            |q, k, (s, v)| { q.insert(k.into(), bson! { s.into() => (v.into()) }); }
        )
    }

    fn merge_documents_at_key<K, D>(self, key: K, document: D) -> Query
        where K: Into<String> + AsRef<str>,
              D: Into<Document>
    {
        self.modify_document_at_key(
            key, document,
            |d, v| { for (k, v) in v.into() { d.insert(k, v); } },
            |q, k, v| { q.insert(k.into(), v.into()); }
        )
    }

    pub fn add_to_set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$addToSet", key, value)
    }

    pub fn add_to_set_all<S, I, V>(self, key: S, values: I) -> Query
            where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V>
    {
        let values: Vec<_> = values.into_iter().map(V::into).collect();
        self.add_subkey_at_key("$addToSet", key, values)
    }

    pub fn unset<S: Into<String>>(self, key: S) -> Query {
        self.add_subkey_at_key("$unset", key, "")
    }

    pub fn inc<S: Into<String>, D: BsonNumber>(self, key: S, delta: D) -> Query {
        self.add_subkey_at_key("$inc", key, delta.to_bson())
    }

    pub fn drop_all(mut self) -> Query {
        self.query.insert("$dropall", true);
        self
    }

    pub fn upsert<D: Into<Document>>(mut self, document: D) -> Query {
        self.query.insert("$upsert", document.into());
        self
    }

    pub fn upsert_field<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$upsert", key, value)
    }

    pub fn set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$set", key, value)
    }

    pub fn set_many<D: Into<Document>>(mut self, document: D) -> Query {
        self.query.insert("$set", document.into());
        self
    }

    pub fn pull<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$pull", key, value)
    }

    pub fn pull_all<S, I, V>(self, key: S, values: I) -> Query
            where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V>
    {
        let values: Vec<_> = values.into_iter().map(V::into).collect();
        self.add_subkey_at_key("$pullAll", key, values)
    }

    pub fn push<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$push", key, value)
    }

    pub fn push_all<S, I, V>(self, key: S, values: I) -> Query
            where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V>
    {
        let values: Vec<_> = values.into_iter().map(V::into).collect();
        self.add_subkey_at_key("$pushAll", key, values)
    }

    pub fn rename<S1: Into<String>, S2: Into<String>>(self, key: S1, new_key: S2) -> Query {
        self.add_subkey_at_key("$rename", key, new_key.into())
    }

    pub fn slice<S: Into<String>>(self, key: S, limit: i64) -> Query {
        self.add_subkey_at_key("$do", key, bson!("$slice" => limit))
    }

    pub fn slice_with_offset<S: Into<String>>(self, key: S, offset: i64, limit: i64) -> Query {
        self.add_subkey_at_key(
            "$do", key, bson!("$slice" => [ (offset.to_bson()), (limit.to_bson()) ])
        )
    }

    #[inline]
    pub fn build(self) -> (Document, Document) {
        (self.hints.into(), self.query)
    }
}

impl Into<Document> for Query {
    #[inline]
    fn into(self) -> Document { self.query }
}

enum FieldConstraintData {
    Root(Query),
    Child(Box<FieldConstraint>)
}

pub struct FieldConstraint(Cow<'static, str>, FieldConstraintData);

impl FieldConstraint {
    fn process<T: Into<Bson>>(self, value: T) -> Query {
        match self.1 {
            FieldConstraintData::Root(mut q) => {
                match value.into() {
                    Bson::Document(doc) => q.merge_documents_at_key(self.0.into_owned(), doc),
                    value => {
                        q.query.insert(self.0.into_owned(), value);
                        q
                    }
                }
            }
            FieldConstraintData::Child(fc) => {
                fc.process(bson!((self.0.into_owned()) => (value.into())))
            }
        }
    }

    pub fn field<S: Into<String>>(self, name: S) -> FieldConstraint {
        FieldConstraint(name.into().into(), FieldConstraintData::Child(Box::new(self)))
    }

    pub fn eq<V: Into<Bson>>(self, value: V) -> Query {
        self.process(value)
    }

    pub fn begin<S: Into<String>>(self, value: S) -> Query {
        self.process(bson!("$begin" => (value.into())))
    }

    // TODO: add between, gt, gte, lt, lte operators for numbers
    pub fn between<N1: BsonNumber, N2: BsonNumber>(self, left: N1, right: N2) -> Query {
        self.process(bson!("$bt" => [ (left.to_bson()), (right.to_bson()) ]))
    }

    pub fn gt<N: BsonNumber>(self, value: N) -> Query {
        self.process(bson!("$gt" => (value.to_bson())))
    }

    pub fn gte<N: BsonNumber>(self, value: N) -> Query {
        self.process(bson!("$gte" => (value.to_bson())))
    }

    pub fn lt<N: BsonNumber>(self, value: N) -> Query {
        self.process(bson!("$lt" => (value.to_bson())))
    }

    pub fn lte<N: BsonNumber>(self, value: N) -> Query {
        self.process(bson!("$lte" => (value.to_bson())))
    }

    pub fn exists(self, exists: bool) -> Query {
        self.process(bson!("$exists" => exists))
    }

    pub fn elem_match<Q: Into<Document>>(self, query: Q) -> Query {
        self.process(bson!("$elemMatch" => (query.into())))
    }

    pub fn contained_in<V: Into<Bson>, I: IntoIterator<Item=V>>(self, values: I) -> Query {
        self.process(bson!("$in" => (values.into_iter().map(V::into).collect::<Vec<_>>())))
    }

    pub fn not_contained_in<V: Into<Bson>, I: IntoIterator<Item=V>>(self, values: I) -> Query {
        self.process(bson!("$nin" => (values.into_iter().map(V::into).collect::<Vec<_>>())))
    }

    pub fn case_insensitive(self) -> FieldConstraint {
        FieldConstraint("$icase".into(), FieldConstraintData::Child(Box::new(self)))
    }

    pub fn not(self) -> FieldConstraint {
        FieldConstraint("$not".into(), FieldConstraintData::Child(Box::new(self)))
    }

    pub fn str_and<S: Into<String>, V: IntoIterator<Item=S>>(self, values: V) -> Query {
        self.process(bson! {
            "$strand" => (
                values.into_iter().map(|v| v.into().into())  // S -> String -> Bson
                    .collect::<Vec<Bson>>()
            )
        })
    }

    pub fn str_or<S: Into<String>, V: IntoIterator<Item=S>>(self, values: V) -> Query {
        self.process(bson! {
            "$stror" => (
                values.into_iter().map(|v| v.into().into())  // S -> String -> Bson
                    .collect::<Vec<Bson>>()
            )
        })
    }
}

#[inline(always)]
pub fn query() -> Query { Query::new() }

#[cfg(test)]
mod tests {
    use bson::oid::ObjectId;

    use super::*;

    #[test]
    fn test_and() {
        let q = query().and(vec![
            query().field("a").eq(1),
            query().field("b").eq("c")
        ]);
        assert_eq!(q.query, bson! {
            "$and" => [
                { "a" => 1 },
                { "b" => "c" }
            ]
        });
    }

    #[test]
    fn test_or() {
        let q = query().or(vec![
            query().field("a").eq(1),
            query().field("b").contained_in(vec!["d", "e", "f"])
        ]);
        assert_eq!(q.query, bson! {
            "$or" => [
                { "a" => 1 },
                { "b" => { "$in" => ["d", "e", "f"] } }
            ]
        });
    }

    #[test]
    fn test_join() {
        let q = query()
            .field("_id").eq("12345")
            .join("user", "users")
            .join("tag", "tags");
        assert_eq!(q.query, bson! {
            "_id" => "12345",
            "$do" => {
                "user" => { "$join" => "users" },
                "tag" => { "$join" => "tags" }
            }
        });
    }

    #[test]
    fn test_add_to_set() {
        let q = query().field("_id").eq(12345)
            .add_to_set("tag", "new tag");
        assert_eq!(q.query, bson! {
            "_id"=> 12345,
            "$addToSet" => {
                "tag" => "new tag"
            }
        });
    }

    #[test]
    fn test_add_to_set_all() {
        let q = query().add_to_set_all("tag", vec!["tag 1", "tag 2", "tag 3"]);
        assert_eq!(q.query, bson! {
            "$addToSet" => {
                "tag" => [ "tag 1", "tag 2", "tag 3" ]
            }
        })
    }

    #[test]
    fn test_unset() {
        let q = query().id(12345)
            .unset("some_field")
            .unset("another_field");
        assert_eq!(q.query, bson! {
            "_id" => 12345,
            "$unset" => {
                "some_field" => "",
                "another_field" => ""
            }
        });
    }

    #[test]
    fn test_inc() {
        let q = query().id(12345).inc("x", 12).inc("y", -13i64).inc("z", 14.5);
        assert_eq!(q.query, bson! {
            "_id" => 12345,
            "$inc" => {
                "x" => 12,
                "y" => (-13i64),
                "z" => 14.5
            }
        });
    }

    #[test]
    fn test_drop_all() {
        let q = query().field("x").between(-42, 42.5).drop_all();
        assert_eq!(q.query, bson! {
            "x" => { "$bt" => [ (-42), 42.5 ] },
            "$dropall" => true
        });
    }

    #[test]
    fn test_upsert() {
        let q = query().field("isbn").eq("0123456789")
            .upsert_field("missing", "value")
            .upsert(bson! {   // overwrites
                "isbn" => "0123456789",
                "name" => "my book"
            })
            .upsert_field("another_field", "another_value");
        assert_eq!(q.query, bson! {
            "isbn" => "0123456789",
            "$upsert" => {
                "isbn" => "0123456789",
                "name" => "my book",
                "another_field" => "another_value"
            }
        });
    }

    #[test]
    fn test_set() {
        let q = query().id(12345)
            .set("x", 12)
            .set_many(bson! { "a" => "x", "b" => "y" })  // overwrites
            .set("y", 34);
        assert_eq!(q.query, bson! {
            "_id" => 12345,
            "$set" => {
                "a" => "x",
                "b" => "y",
                "y" => 34
            }
        });
    }

    #[test]
    fn test_pull() {
        let q = query().id(12345)
            .pull("xs", 12)
            .pull_all("ys", bson![34, 56.7]);
        assert_eq!(q.query, bson! {
            "_id" => 12345,
            "$pull" => {
                "xs" => 12
            },
            "$pullAll" => {
                "ys" => [ 34, 56.7 ]
            }
        });
    }

    #[test]
    fn test_push() {
        let q = query().id(12345)
            .push("xs", "a")
            .push_all("ys", bson!["w", "v"]);
        assert_eq!(q.query, bson! {
            "_id" => 12345,
            "$push" => {
                "xs" => "a"
            },
            "$pushAll" => {
                "ys" => [ "w", "v" ]
            }
        });
    }

    #[test]
    fn test_rename() {
        let q = query().id("12345").rename("input", "output").rename("alpha", "omega");
        assert_eq!(q.query, bson! {
            "_id" => "12345",
            "$rename" => {
                "input" => "output",
                "alpha" => "omega"
            }
        });
    }

    #[test]
    fn test_slice() {
        let q = query().id(12345)
            .slice("array", 123)
            .slice_with_offset("array_2", 456, 789);
        assert_eq!(q.query, bson! {
            "_id" => 12345,
            "$do" => {
                "array" => { "$slice" => 123i64 },
                "array_2" => { "$slice" => [ 456i64, 789i64 ] }
            }
        });
    }

    #[test]
    fn test_field_field() {
        let q = query().field("a").field("b").field("c").eq(12345);
        assert_eq!(q.query, bson! {
            "a" => { "b" => { "c" => 12345 } }
        });
    }

    #[test]
    fn test_field_eq() {
        let q = query().field("_id").eq(ObjectId::with_timestamp(128));
        assert_eq!(q.query, bson! {
            "_id" => (ObjectId::with_timestamp(128))
        });
    }

    #[test]
    fn test_field_begin() {
        let q = query().field("name").begin("something");
        assert_eq!(q.query, bson! {
            "name" => {
                "$begin" => "something"
            }
        });
    }

    #[test]
    fn test_field_between() {
        let q = query().field("x").between(0.1, 123i64);
        assert_eq!(q.query, bson! {
            "x" => {
                "$bt" => [ 0.1, 123i64 ]
            }
        });
    }

    #[test]
    fn test_field_gt_lt() {
        let q = query()
            .field("x").gt(0.1)
            .field("x").lt(9.9);
        assert_eq!(q.query, bson! {
            "x" => {
                "$gt" => 0.1,
                "$lt" => 9.9
            }
        });
    }

    #[test]
    fn test_field_gte_lte() {
        let q = query()
            .field("y").gte(1)
            .field("y").lte(99);
        assert_eq!(q.query, bson! {
            "y" => {
                "$gte" => 1,
                "$lte" => 99
            }
        });
    }

    #[test]
    fn test_field_exists() {
        let q = query()
            .field("name").exists(true)
            .field("wat").exists(false);
        assert_eq!(q.query, bson! {
            "name" => { "$exists" => true },
            "wat" => { "$exists" => false }
        });
    }

    #[test]
    fn test_field_elem_match() {
        let q = query()
            .field("props").elem_match(bson! { "a" => 1, "b" => "c" });
        assert_eq!(q.query, bson! {
            "props" => {
                "$elemMatch" => {
                    "a" => 1,
                    "b" => "c"
                }
            }
        });
    }

    #[test]
    fn test_contained_in() {
        let q = query()
            .field("x").contained_in(vec![1, 2, 3])
            .field("y").not_contained_in(vec![7, 8, 9]);
        assert_eq!(q.query, bson! {
            "x" => {
                "$in" => [1, 2, 3]
            },
            "y" => {
                "$nin" => [7, 8, 9]
            }
        });
    }

    #[test]
    fn test_case_insensitive() {
        let q = query()
            .field("msg").case_insensitive().eq("hello world")
            .field("err").case_insensitive().contained_in(vec!["whatever", "pfff"]);
        assert_eq!(q.query, bson! {
            "msg" => {
                "$icase" => "hello world"
            },
            "err" => {
                "$icase" => {
                    "$in" => [ "whatever", "pfff" ]
                }
            }
        });
    }

    #[test]
    fn test_not() {
        let q = query()
            .field("x").not().eq(42)
            .field("y").not().between(10.0, 20.32);
        assert_eq!(q.query, bson! {
            "x" => {
                "$not" => 42
            },
            "y" => {
                "$not" => {
                    "$bt" => [ 10.0, 20.32 ]
                }
            }
        });
    }

    #[test]
    fn test_strand_stror() {
        let q = query()
            .field("name").str_and(["me", "xyzzy", "wab"].iter().cloned())
            .field("title").str_or(["foo", "bar", "baz"].iter().cloned());
        assert_eq!(q.query, bson! {
            "name" => {
                "$strand" => [ "me", "xyzzy", "wab" ]
            },
            "title" => {
                "$stror" => [ "foo", "bar", "baz" ]
            }
        });
    }
}
