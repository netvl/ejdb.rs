use std::borrow::Cow;

use bson::{Bson, Document};

use utils::bson::{DocumentBuilder, BsonNumber};

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
        self.merge_or_add_to_document("$do", key, ndb().set("$join", coll.into()))
    }

    fn merge_or_add_to_document<S: Into<String>, V: Into<Bson>>(mut self, key: &str, subkey: S, value: V) -> Query {
        // unsafe is to overcome non-lexical borrow issues (and entry API is not available)
        let r = self.query.get_mut(key).map(|p| unsafe { &mut *(p as *mut _) });
        if let Some(&mut Bson::Document(ref mut d)) = r {
            d.insert(subkey, value);
        } else {
            self.query.insert(key, ndb().set(subkey, value));
        }

        self
    }

    pub fn add_to_set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.merge_or_add_to_document("$addToSet", key, value)
    }

    pub fn add_to_set_all<S, I, V>(self, key: S, values: I) -> Query
            where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V>
    {
        let values: Vec<_> = values.into_iter().map(V::into).collect();
        self.merge_or_add_to_document("$addToSet", key, values)
    }

    pub fn unset<S: Into<String>>(self, key: S) -> Query {
        self.merge_or_add_to_document("$unset", key, "")
    }

    pub fn inc<S: Into<String>, D: BsonNumber>(self, key: S, delta: D) -> Query {
        self.merge_or_add_to_document("$inc", key, delta.to_bson())
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
        self.merge_or_add_to_document("$upsert", key, value)
    }

    pub fn set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.merge_or_add_to_document("$set", key, value)
    }

    pub fn set_many<D: Into<Document>>(mut self, document: D) -> Query {
        self.query.insert("$set", document.into());
        self
    }

    pub fn pull<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.merge_or_add_to_document("$pull", key, value)
    }

    pub fn pull_all<S, I, V>(self, key: S, values: I) -> Query
            where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V>
    {
        let values: Vec<_> = values.into_iter().map(V::into).collect();
        self.merge_or_add_to_document("$pullAll", key, values)
    }

    pub fn push<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.merge_or_add_to_document("$push", key, value)
    }

    pub fn push_all<S, I, V>(self, key: S, values: I) -> Query
            where S: Into<String>, V: Into<Bson>, I: IntoIterator<Item=V>
    {
        let values: Vec<_> = values.into_iter().map(V::into).collect();
        self.merge_or_add_to_document("$pushAll", key, values)
    }

    pub fn rename<S1: Into<String>, S2: Into<String>>(self, key: S1, new_key: S2) -> Query {
        self.merge_or_add_to_document("$rename", key, new_key.into())
    }

    pub fn slice<S: Into<String>>(self, key: S, limit: i64) -> Query {
        self.merge_or_add_to_document("$do", key, ndb().set("$slice", limit))
    }

    pub fn slice_with_offset<S: Into<String>>(self, key: S, offset: i64, limit: i64) -> Query {
        self.merge_or_add_to_document(
            "$do", key, ndb().set("$slice", vec![offset.to_bson(), limit.to_bson()])
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
                q.query.insert(self.0.into_owned(), value);
                q
            }
            FieldConstraintData::Child(fc) => {
                fc.process(ndb().set(self.0.into_owned(), value))
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
        self.process(ndb().set("$begin", value.into()))
    }

    // TODO: add between, gt, gte, lt, lte operators for numbers
    pub fn between<N1: BsonNumber, N2: BsonNumber>(self, left: N1, right: N2) -> Query {
        self.process(ndb().set("$bt", vec![left.to_bson(), right.to_bson()]))
    }

    pub fn gt<N: BsonNumber>(self, value: N) -> Query {
        self.process(ndb().set("$gt", value.to_bson()))
    }

    pub fn gte<N: BsonNumber>(self, value: N) -> Query {
        self.process(ndb().set("$gte", value.to_bson()))
    }

    pub fn lt<N: BsonNumber>(self, value: N) -> Query {
        self.process(ndb().set("$lt", value.to_bson()))
    }

    pub fn lte<N: BsonNumber>(self, value: N) -> Query {
        self.process(ndb().set("$lte", value.to_bson()))
    }

    pub fn exists(self, exists: bool) -> Query {
        self.process(ndb().set("$exists", exists))
    }

    pub fn elem_match<Q: Into<Document>>(self, query: Q) -> Query {
        self.process(ndb().set("$elemMatch", query.into()))
    }

    pub fn contained_in<V: Into<Bson>, I: IntoIterator<Item=V>>(self, values: I) -> Query {
        self.process(ndb().set("$in", values.into_iter().map(V::into).collect::<Vec<_>>()))
    }

    pub fn not_contained_in<V: Into<Bson>, I: IntoIterator<Item=V>>(self, values: I) -> Query {
        self.process(ndb().set("$nin", values.into_iter().map(V::into).collect::<Vec<_>>()))
    }

    pub fn case_insensitive(self) -> FieldConstraint {
        FieldConstraint("$icase".into(), FieldConstraintData::Child(Box::new(self)))
    }

    pub fn not(self) -> FieldConstraint {
        FieldConstraint("$not".into(), FieldConstraintData::Child(Box::new(self)))
    }

    pub fn str_and<S: Into<String>, V: IntoIterator<Item=S>>(self, values: V) -> Query {
        self.process(
            ndb().set("$strand", values.into_iter()
                .map(|v| v.into().into())  // S -> String -> Bson
                .collect::<Vec<Bson>>())
        )
    }

    pub fn str_or<S: Into<String>, V: IntoIterator<Item=S>>(self, values: V) -> Query {
        self.process(
            ndb().set("$stror", values.into_iter()
                .map(|v| v.into().into())  // S -> String -> Bson
                .collect::<Vec<Bson>>())
        )
    }
}

#[inline(always)]
fn ndb() -> DocumentBuilder { DocumentBuilder::new() }

#[inline(always)]
pub fn query() -> Query { Query::new() }

#[cfg(test)]
mod tests {
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
}
