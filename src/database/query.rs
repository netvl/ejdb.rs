//! Query API, a simple builder-like constructor for EJDB queries.

use std::borrow::Cow;

use bson::{Bson, Document};

use utils::bson::BsonNumber;

pub struct QueryHints {
    hints: Document
}

impl QueryHints {
    #[inline]
    pub fn new() -> QueryHints {
        QueryHints { hints: Document::new() }
    }

    #[inline]
    pub fn max(mut self, n: i64) -> QueryHints {
        self.hints.insert("$max", n);
        self
    }

    #[inline]
    pub fn skip(mut self, n: i64) -> QueryHints {
        self.hints.insert("$skip", n);
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

    fn add_hint(&mut self, key: &str, subkey: String, value: i32) {
        if !self.hints.contains_key(key) {
            self.hints.insert(key, bson! { subkey => value });
        } else {
            match self.hints.get_mut(key) {
                Some(&mut Bson::Document(ref mut doc)) => {
                    doc.insert(subkey, value);
                }
                _ => unreachable!()
            }
        }
    }
}

pub struct QueryHintsOrderBy(QueryHints, String);

impl QueryHintsOrderBy {
    fn add_hint(mut self, value: i32) -> QueryHints {
        self.0.add_hint("$orderBy", self.1, value);
        self.0
    }

    #[inline]
    pub fn desc(self) -> QueryHints {
        self.add_hint(-1)
    }

    #[inline]
    pub fn asc(self) -> QueryHints {
        self.add_hint(1)
    }
}

pub struct QueryHintsField(QueryHints, String);

impl QueryHintsField {
    pub fn add_hint(mut self, value: i32) -> QueryHints {
        self.0.add_hint("$fields", self.1, value);
        self.0
    }

    #[inline]
    pub fn exclude(self) -> QueryHints {
        self.add_hint(-1)
    }

    #[inline]
    pub fn include(self) -> QueryHints {
        self.add_hint(1)
    }
}

impl Into<Document> for QueryHints {
    #[inline]
    fn into(self) -> Document {
        self.hints
    }
}

pub struct QH;

impl QH {
    #[inline(always)]
    pub fn max(self, n: i64) -> QueryHints {
        QueryHints::new().max(n)
    }

    #[inline(always)]
    pub fn skip(self, n: i64) -> QueryHints {
        QueryHints::new().skip(n)
    }

    #[inline(always)]
    pub fn order_by<S: Into<String>>(self, field: S) -> QueryHintsOrderBy {
        QueryHints::new().order_by(field)
    }

    #[inline(always)]
    pub fn field<S: Into<String>>(self, field: S) -> QueryHintsField {
        QueryHints::new().field(field)
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

    pub fn hints(mut self, hints: QueryHints) -> Query {
        self.hints = hints;
        self
    }

    pub fn and<I, V>(mut self, queries: I) -> Query
        where V: Into<Document>, I: IntoIterator<Item=V>
    {
        self.query.insert(
            "$and",
            queries.into_iter().map(|v| v.into().into()).collect::<Vec<Bson>>()
        );
        self
    }

    pub fn or<I, V>(mut self, queries: I) -> Query
        where V: Into<Document>, I: IntoIterator<Item=V>
    {
        self.query.insert(
            "$or",
            queries.into_iter().map(|v| v.into().into()).collect::<Vec<Bson>>()
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

    #[inline]
    pub fn build_ref(&self) -> (&Document, &Document) {
        (&self.hints.hints, &self.query)
    }
}

impl From<Document> for Query {
    #[inline]
    fn from(document: Document) -> Query {
        Query {
            query: document,
            hints: QueryHints::new()
        }
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

pub struct Q;

impl Q {
    #[inline(always)]
    pub fn hints(self, hints: QueryHints) -> Query {
        Query::new().hints(hints)
    }

    #[inline(always)]
    pub fn and<I, V>(self, queries: I) -> Query where V: Into<Document>, I: IntoIterator<Item=V> {
        Query::new().and(queries)
    }

    #[inline(always)]
    pub fn or<I, V>(self, queries: I) -> Query where V: Into<Document>, I: IntoIterator<Item=V> {
        Query::new().or(queries)
    }

    #[inline(always)]
    pub fn id<V: Into<Bson>>(self, value: V) -> Query {
        Query::new().id(value)
    }

    #[inline(always)]
    pub fn field<S: Into<String>>(self, name: S) -> FieldConstraint {
        Query::new().field(name)
    }

    #[inline(always)]
    pub fn join<S: Into<String>, C: Into<String>>(self, key: S, coll: C) -> Query {
        Query::new().join(key, coll)
    }

    #[inline(always)]
    pub fn add_to_set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().add_to_set(key, value)
    }

    #[inline(always)]
    pub fn add_to_set_all<S, I, V>(self, key: S, values: I) -> Query where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V> {
        Query::new().add_to_set_all(key, values)
    }

    #[inline(always)]
    pub fn unset<S: Into<String>>(self, key: S) -> Query {
        Query::new().unset(key)
    }

    #[inline(always)]
    pub fn inc<S: Into<String>, D: BsonNumber>(self, key: S, delta: D) -> Query {
        Query::new().inc(key, delta)
    }

    #[inline(always)]
    pub fn drop_all(self) -> Query {
        Query::new().drop_all()
    }

    #[inline(always)]
    pub fn upsert<D: Into<Document>>(self, document: D) -> Query {
        Query::new().upsert(document)
    }

    #[inline(always)]
    pub fn upsert_field<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().upsert_field(key, value)
    }

    #[inline(always)]
    pub fn set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().set(key, value)
    }

    #[inline(always)]
    pub fn set_many<D: Into<Document>>(self, document: D) -> Query {
        Query::new().set_many(document)
    }

    pub fn pull<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().pull(key, value)
    }

    #[inline(always)]
    pub fn pull_all<S, I, V>(self, key: S, values: I) -> Query where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V> {
        Query::new().pull_all(key, values)
    }

    #[inline(always)]
    pub fn push<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().push(key, value)
    }

    #[inline(always)]
    pub fn push_all<S, I, V>(self, key: S, values: I) -> Query where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V> {
        Query::new().push_all(key, values)
    }

    #[inline(always)]
    pub fn rename<S1: Into<String>, S2: Into<String>>(self, key: S1, new_key: S2) -> Query {
        Query::new().rename(key, new_key)
    }

    #[inline(always)]
    pub fn slice<S: Into<String>>(self, key: S, limit: i64) -> Query {
        Query::new().slice(key, limit)
    }

    #[inline(always)]
    pub fn slice_with_offset<S: Into<String>>(self, key: S, offset: i64, limit: i64) -> Query {
        Query::new().slice_with_offset(key, offset, limit)
    }

}

#[cfg(test)]
mod tests {
    use bson::oid::ObjectId;

    use super::*;

    #[test]
    fn test_and() {
        let (_, q) = Q.and(vec![
            Q.field("a").eq(1),
            Q.field("b").eq("c")
        ]).build();
        assert_eq!(q, bson! {
            "$and" => [
                { "a" => 1 },
                { "b" => "c" }
            ]
        });
    }

    #[test]
    fn test_or() {
        let (_, q) = Q.or(vec![
            Q.field("a").eq(1),
            Q.field("b").contained_in(vec!["d", "e", "f"])
        ]).build();
        assert_eq!(q, bson! {
            "$or" => [
                { "a" => 1 },
                { "b" => { "$in" => ["d", "e", "f"] } }
            ]
        });
    }

    #[test]
    fn test_join() {
        let (_, q) = Q
            .field("_id").eq("12345")
            .join("user", "users")
            .join("tag", "tags")
            .build();
        assert_eq!(q, bson! {
            "_id" => "12345",
            "$do" => {
                "user" => { "$join" => "users" },
                "tag" => { "$join" => "tags" }
            }
        });
    }

    #[test]
    fn test_add_to_set() {
        let (_, q) = Q.field("_id").eq(12345)
            .add_to_set("tag", "new tag")
            .build();
        assert_eq!(q, bson! {
            "_id"=> 12345,
            "$addToSet" => {
                "tag" => "new tag"
            }
        });
    }

    #[test]
    fn test_add_to_set_all() {
        let (_, q) = Q.add_to_set_all("tag", vec!["tag 1", "tag 2", "tag 3"]).build();
        assert_eq!(q, bson! {
            "$addToSet" => {
                "tag" => [ "tag 1", "tag 2", "tag 3" ]
            }
        })
    }

    #[test]
    fn test_unset() {
        let (_, q) = Q.id(12345)
            .unset("some_field")
            .unset("another_field")
            .build();
        assert_eq!(q, bson! {
            "_id" => 12345,
            "$unset" => {
                "some_field" => "",
                "another_field" => ""
            }
        });
    }

    #[test]
    fn test_inc() {
        let (_, q) = Q.id(12345).inc("x", 12).inc("y", -13i64).inc("z", 14.5).build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q.field("x").between(-42, 42.5).drop_all().build();
        assert_eq!(q, bson! {
            "x" => { "$bt" => [ (-42), 42.5 ] },
            "$dropall" => true
        });
    }

    #[test]
    fn test_upsert() {
        let (_, q) = Q.field("isbn").eq("0123456789")
            .upsert_field("missing", "value")
            .upsert(bson! {   // overwrites
                "isbn" => "0123456789",
                "name" => "my book"
            })
            .upsert_field("another_field", "another_value")
            .build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q.id(12345)
            .set("x", 12)
            .set_many(bson! { "a" => "x", "b" => "y" })  // overwrites
            .set("y", 34)
            .build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q.id(12345)
            .pull("xs", 12)
            .pull_all("ys", bson![34, 56.7])
            .build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q.id(12345)
            .push("xs", "a")
            .push_all("ys", bson!["w", "v"])
            .build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q.id("12345").rename("input", "output").rename("alpha", "omega").build();
        assert_eq!(q, bson! {
            "_id" => "12345",
            "$rename" => {
                "input" => "output",
                "alpha" => "omega"
            }
        });
    }

    #[test]
    fn test_slice() {
        let (_, q) = Q.id(12345)
            .slice("array", 123)
            .slice_with_offset("array_2", 456, 789)
            .build();
        assert_eq!(q, bson! {
            "_id" => 12345,
            "$do" => {
                "array" => { "$slice" => 123i64 },
                "array_2" => { "$slice" => [ 456i64, 789i64 ] }
            }
        });
    }

    #[test]
    fn test_field_field() {
        let (_, q) = Q.field("a").field("b").field("c").eq(12345).build();
        assert_eq!(q, bson! {
            "a" => { "b" => { "c" => 12345 } }
        });
    }

    #[test]
    fn test_field_eq() {
        let (_, q) = Q.field("_id").eq(ObjectId::with_timestamp(128)).build();
        assert_eq!(q, bson! {
            "_id" => (ObjectId::with_timestamp(128))
        });
    }

    #[test]
    fn test_field_begin() {
        let (_, q) = Q.field("name").begin("something").build();
        assert_eq!(q, bson! {
            "name" => {
                "$begin" => "something"
            }
        });
    }

    #[test]
    fn test_field_between() {
        let (_, q) = Q.field("x").between(0.1, 123i64).build();
        assert_eq!(q, bson! {
            "x" => {
                "$bt" => [ 0.1, 123i64 ]
            }
        });
    }

    #[test]
    fn test_field_gt_lt() {
        let (_, q) = Q
            .field("x").gt(0.1)
            .field("x").lt(9.9)
            .build();
        assert_eq!(q, bson! {
            "x" => {
                "$gt" => 0.1,
                "$lt" => 9.9
            }
        });
    }

    #[test]
    fn test_field_gte_lte() {
        let (_, q) = Q
            .field("y").gte(1)
            .field("y").lte(99)
            .build();
        assert_eq!(q, bson! {
            "y" => {
                "$gte" => 1,
                "$lte" => 99
            }
        });
    }

    #[test]
    fn test_field_exists() {
        let (_, q) = Q
            .field("name").exists(true)
            .field("wat").exists(false)
            .build();
        assert_eq!(q, bson! {
            "name" => { "$exists" => true },
            "wat" => { "$exists" => false }
        });
    }

    #[test]
    fn test_field_elem_match() {
        let (_, q) = Q
            .field("props").elem_match(bson! { "a" => 1, "b" => "c" })
            .build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q
            .field("x").contained_in(vec![1, 2, 3])
            .field("y").not_contained_in(vec![7, 8, 9])
            .build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q
            .field("msg").case_insensitive().eq("hello world")
            .field("err").case_insensitive().contained_in(vec!["whatever", "pfff"])
            .build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q
            .field("x").not().eq(42)
            .field("y").not().between(10.0, 20.32)
            .build();
        assert_eq!(q, bson! {
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
        let (_, q) = Q
            .field("name").str_and(["me", "xyzzy", "wab"].iter().cloned())
            .field("title").str_or(["foo", "bar", "baz"].iter().cloned())
            .build();
        assert_eq!(q, bson! {
            "name" => {
                "$strand" => [ "me", "xyzzy", "wab" ]
            },
            "title" => {
                "$stror" => [ "foo", "bar", "baz" ]
            }
        });
    }

    #[test]
    fn test_hints_empty() {
        let (qh, _) = Q.hints(QueryHints::new()).build();
        assert!(qh.is_empty());
    }

    #[test]
    fn test_hints_max() {
        let (qh, _) = Q.hints(QH.max(12)).build();
        assert_eq!(qh, bson! {
            "$max" => 12i64
        });
    }
}
