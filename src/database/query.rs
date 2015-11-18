//! Query API, a simple builder-like constructor for EJDB queries.

use std::borrow::Cow;

use bson::{Bson, Document};

use utils::bson::BsonNumber;

/// A container of EJDB query options.
///
/// This structure is a wrapper around a BSON document with various options affecting query
/// execution in EJDB. It is a part of any query, but by default it is empty and has no effect.
#[derive(Clone, PartialEq, Debug)]
pub struct QueryHints {
    hints: Document
}

impl QueryHints {
    /// Creates a new, empty query hints set.
    #[inline]
    pub fn new() -> QueryHints {
        QueryHints { hints: Document::new() }
    }

    /// Sets the maximum number of entries which should be returned by the query.
    ///
    /// Corresponds to `$max` hint in EJDB query hints syntax.
    #[inline]
    pub fn max(mut self, n: i64) -> QueryHints {
        self.hints.insert("$max", n);
        self
    }

    /// Sets the number of entries which should be skipped first when query results are inspected.
    ///
    /// Corresponds to `$skip` hint in EJDB query hints syntax.
    #[inline]
    pub fn skip(mut self, n: i64) -> QueryHints {
        self.hints.insert("$skip", n);
        self
    }

    /// Returns a builder for ordering hint for the provided field.
    ///
    /// Corresponds to `$orderBy` hint in EJDB query hints syntax.
    #[inline]
    pub fn order_by<S: Into<String>>(self, field: S) -> QueryHintsOrderBy {
        QueryHintsOrderBy(self, field.into())
    }

    /// Returns a builder for setting inclusion/exclusion flag of the provided field.
    ///
    /// Corresponds to `$fields` hint in EJDB query syntax.
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

/// A builder for ordering hint by a specific field.
pub struct QueryHintsOrderBy(QueryHints, String);

impl QueryHintsOrderBy {
    fn add_hint(mut self, value: i32) -> QueryHints {
        self.0.add_hint("$orderBy", self.1, value);
        self.0
    }

    /// Sets that query results must be sorted by this field in descending order.
    #[inline]
    pub fn desc(self) -> QueryHints {
        self.add_hint(-1)
    }

    /// Sets that query results must be sorted by this field in ascending order.
    #[inline]
    pub fn asc(self) -> QueryHints {
        self.add_hint(1)
    }
}

/// A builder for inclusion/exclusion flag for a specific field.
pub struct QueryHintsField(QueryHints, String);

impl QueryHintsField {
    fn add_hint(mut self, value: i32) -> QueryHints {
        self.0.add_hint("$fields", self.1, value);
        self.0
    }

    /// Sets that query results must not contain a field with this name.
    #[inline]
    pub fn exclude(self) -> QueryHints {
        self.add_hint(-1)
    }

    /// Sets that query results must contain a field with this name, if available.
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

/// An entry point for constructing query hints.
///
/// This is a convenience API. This structure provides the same methods as `QueryHints`
/// structure and inside them a fresh `QueryHints` instance is created and the corresponding
/// method is called on it. This is the main approach for constructing query hints.
///
/// # Example
///
/// ```
/// use ejdb::query::{QueryHints, QH};
///
/// assert_eq!(
///     QueryHints::new().max(128).field("name").include().order_by("date").desc(),
///     QH.max(128).field("name").include().order_by("date").desc()
/// )
/// ```
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

/// An EJDB query.
///
/// This structure represents both a find query and an update query, because both kind of
/// queries are executed in the same way.
///
/// A query internally is a BSON document of a [certain format][queries], very similar to
/// MongoDB queries. This structure provides convenience methods to build such document.
/// Additionally, a query may have hints, i.e. non-data parameters affecting its behavior.
/// These are also encapsulated here.
///
/// Most of the building methods here are as generic as possible; e.g. they take `Into<String>`
/// instead of `&str` or `String`; same with iterable objects. This is done for maximum
/// flexibility - these methods will consume almost anything which is sensible to pass to them.
///
///   [queries]: http://ejdb.org/doc/ql/ql.html
#[derive(Clone, PartialEq, Debug)]
pub struct Query {
    hints: QueryHints,
    query: Document
}

impl Query {
    /// Creates a new empty query with empty hints.
    ///
    /// This method can be a starting point for building a query; see `Q` struct, however.
    #[inline]
    pub fn new() -> Query {
        Query {
            hints: QueryHints::new(),
            query: Document::new()
        }
    }

    /// Sets hints for this query.
    pub fn hints(mut self, hints: QueryHints) -> Query {
        self.hints = hints;
        self
    }

    /// Builds `$and` query.
    ///
    /// Selects all records which satisfy all of the provided queries simultaneously.
    pub fn and<I>(mut self, queries: I) -> Query
        where I: IntoIterator, I::Item: Into<Document>
    {
        self.query.insert(
            "$and",
            queries.into_iter().map(|v| v.into().into()).collect::<Vec<Bson>>()
        );
        self
    }

    /// Builds `$or` query.
    ///
    /// Selects all records which satisfy at least one of the provided queries.
    pub fn or<I>(mut self, queries: I) -> Query
        where I: IntoIterator, I::Item: Into<Document>
    {
        self.query.insert(
            "$or",
            queries.into_iter().map(|v| v.into().into()).collect::<Vec<Bson>>()
        );
        self
    }

    /// Sets equality constraint for `_id` field.
    ///
    /// This is just a shortcut for `query.field("_id").eq(value)`.
    #[inline]
    pub fn id<V: Into<Bson>>(self, value: V) -> Query {
        self.field("_id").eq(value)
    }

    /// Returns a builder object for a field constraint.
    ///
    /// This is the main method to set query constraints. Usually a query contains one or
    /// more such constraints.
    #[inline]
    pub fn field<S: Into<String>>(self, name: S) -> FieldConstraint {
        FieldConstraint(name.into().into(), FieldConstraintData::Root(self))
    }

    /// Constructs a `$join` query.
    ///
    /// Joins this collection with another one by the value of `_id` field.
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
        where K: Into<String> + AsRef<str>, D: Into<Document>
    {
        self.modify_document_at_key(
            key, document,
            |d, v| { for (k, v) in v.into() { d.insert(k, v); } },
            |q, k, v| { q.insert(k.into(), v.into()); }
        )
    }

    /// Constructs an `$addToSet` update query.
    ///
    /// Adds `value` to the set (represented as a BSON array) at the field `key`.
    pub fn add_to_set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$addToSet", key, value)
    }

    /// Constructs a multi-valued `$addToSet` update query.
    ///
    /// Adds all items from `values` to the set (represented as a BSON array) at the field `key`.
    pub fn add_to_set_all<S, I>(self, key: S, values: I) -> Query
        where S: Into<String>, I: IntoIterator, I::Item: Into<Bson>
    {
        let values: Vec<_> = values.into_iter().map(I::Item::into).collect();
        self.add_subkey_at_key("$addToSet", key, values)
    }

    /// Constructs an `$unset` update query.
    ///
    /// Removes the field `key`.
    pub fn unset<S: Into<String>>(self, key: S) -> Query {
        self.add_subkey_at_key("$unset", key, "")
    }

    /// Constructs an `$inc` update query.
    ///
    /// Increments the numerical value in field `key` by `delta`. Use negative `delta` for
    /// decrements.
    pub fn inc<S: Into<String>, D: BsonNumber>(self, key: S, delta: D) -> Query {
        self.add_subkey_at_key("$inc", key, delta.to_bson())
    }

    /// Constructs a `$dropall` update query.
    ///
    /// Removes all records from the collection.
    pub fn drop_all(mut self) -> Query {
        self.query.insert("$dropall", true);
        self
    }

    /// Constructs a `$set` update query for a single field.
    ///
    /// Sets the field `key` to the value `value` in all matched records. Multiple sequential
    /// `set()`s will be merged.
    pub fn set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$set", key, value)
    }

    /// Constructs an entire `$set` update query.
    ///
    /// Sets all fields from the `document` to their respective values in all matched records.
    /// Overwrites all previous `set()` and `set_many()` invocations.
    pub fn set_many<D: Into<Document>>(mut self, document: D) -> Query {
        self.query.insert("$set", document.into());
        self
    }

    /// Constructs an `$upsert` update query for a single field.
    ///
    /// Like `set()`, but will insert new record if none matched. Multiple sequential
    /// `set()`s will be merged.
    pub fn upsert<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$upsert", key, value)
    }

    /// Constructs an entire `$upsert` update query.
    ///
    /// Like `set()`, but will insert new record if none matched. Overwrites all previous
    /// `upsert()` and `upsert_field()` calls.
    pub fn upsert_many<D: Into<Document>>(mut self, document: D) -> Query {
        self.query.insert("$upsert", document.into());
        self
    }

    /// Constructs a `$pull` update query.
    ///
    /// Removes the `value` from an array at the field `key` in all matched records.
    pub fn pull<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$pull", key, value)
    }

    /// Constructs a multiple-valued `$pull` update query.
    ///
    /// Removes all values from `values` from an array at the field `key` in all matched records.
    /// Multiple `push_all()` calls will be merged.
    pub fn pull_all<S, I>(self, key: S, values: I) -> Query
        where S: Into<String>, I: IntoIterator, I::Item: Into<Bson>
    {
        let values: Vec<_> = values.into_iter().map(I::Item::into).collect();
        self.add_subkey_at_key("$pullAll", key, values)
    }

    /// Constructs a `$push` update query.
    ///
    /// Appends the provided `value` to an array field `key` in all matched records. Multiple
    /// `push()` calls will be merged.
    pub fn push<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        self.add_subkey_at_key("$push", key, value)
    }

    /// Constructs a multiple-valued `$push` update query.
    ///
    /// Appends all values from `values` to an array field `key` in all matched records. Multiple
    /// `push_all()` calls will be merged.
    pub fn push_all<S, I>(self, key: S, values: I) -> Query
        where S: Into<String>, I: IntoIterator, I::Item: Into<Bson>
    {
        let values: Vec<_> = values.into_iter().map(I::Item::into).collect();
        self.add_subkey_at_key("$pushAll", key, values)
    }

    /// Constructs a `$rename` update query.
    ///
    /// Renames field `key` to `new_key` in all matched records. Multiple `rename()` calls will
    /// be merged.
    pub fn rename<S1: Into<String>, S2: Into<String>>(self, key: S1, new_key: S2) -> Query {
        self.add_subkey_at_key("$rename", key, new_key.into())
    }

    /// Constructs a limit-only `$slice` query.
    ///
    /// Limits the number of array items of the field `key` in the returned result. `limit` is
    /// the number of elements which will be taken from the beginning of the array.
    pub fn slice<S: Into<String>>(self, key: S, limit: i64) -> Query {
        self.add_subkey_at_key("$do", key, bson!("$slice" => limit))
    }

    /// Constructs a full `$slice` query.
    ///
    /// Limits the number of array items of the field `key` in the returned result. `limit` is
    /// the maximum number of elements to be returned starting from `offset`.
    pub fn slice_with_offset<S: Into<String>>(self, key: S, offset: i64, limit: i64) -> Query {
        self.add_subkey_at_key(
            "$do", key, bson!("$slice" => [ (offset.to_bson()), (limit.to_bson()) ])
        )
    }

    /// Converts this query to a pair of BSON documents, a query and a set of query hints.
    #[inline]
    pub fn build(self) -> (Document, Document) {
        (self.hints.into(), self.query)
    }

    /// Returns references to the current query and the current set of query hints.
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

/// A transient builder for adding field-based query constraints.
///
/// Instances of this structure are returned by `Query::field()` and `FieldConstrait::field()`
/// methods.
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
                fc.process(bson!(self.0.into_owned() => (value.into())))
            }
        }
    }

    /// Returns a constraint builder for a deeper field.
    pub fn field<S: Into<String>>(self, name: S) -> FieldConstraint {
        FieldConstraint(name.into().into(), FieldConstraintData::Child(Box::new(self)))
    }

    /// Adds an equality constraint for this field and `value`.
    pub fn eq<V: Into<Bson>>(self, value: V) -> Query {
        self.process(value)
    }

    /// Adds a `$begin` constraint for this field.
    ///
    /// The value of this field must start with `value`. The field type should be string.
    pub fn begin<S: Into<String>>(self, value: S) -> Query {
        self.process(bson!("$begin" => (value.into())))
    }

    /// Adds a `$between` constraint for this field.
    ///
    /// The value of this field must be greater than or equal to `left` and less than or
    /// equal to `right`. The field type should be numeric.
    pub fn between<N1: BsonNumber, N2: BsonNumber>(self, left: N1, right: N2) -> Query {
        self.process(bson!("$bt" => [ (left.to_bson()), (right.to_bson()) ]))
    }

    /// Adds a `$gt` constraint for this field.
    ///
    /// The value of this field must be strictly greater than `value`. The field type
    /// should be numeric.
    pub fn gt<N: BsonNumber>(self, value: N) -> Query {
        self.process(bson!("$gt" => (value.to_bson())))
    }

    /// Adds a `$gte` constraint for this field.
    ///
    /// The value of this field must be greater than or equal to `value`. The field type
    /// should be numeric.
    pub fn gte<N: BsonNumber>(self, value: N) -> Query {
        self.process(bson!("$gte" => (value.to_bson())))
    }

    /// Adds an `$lt` constraint for this field.
    ///
    /// The value of this field must be strictly less than `value`. The field type
    /// should be numeric.
    pub fn lt<N: BsonNumber>(self, value: N) -> Query {
        self.process(bson!("$lt" => (value.to_bson())))
    }

    /// Adds an `$lte` constraint for this field.
    ///
    /// The value of this field must be less than or equal to `value`. The field type
    /// should be numeric.
    pub fn lte<N: BsonNumber>(self, value: N) -> Query {
        self.process(bson!("$lte" => (value.to_bson())))
    }

    /// Adds an `$exists` constraint for this field.
    ///
    /// The field must exists if `exists` is `true`, the opposite otherwise.
    pub fn exists(self, exists: bool) -> Query {
        self.process(bson!("$exists" => exists))
    }

    /// Adds an `$elemMatch` constraint for this field.
    ///
    /// Any element of the array contained in this field must match `query`. The query argument
    /// is a regular EJDB query; you can pass another `Query` object to it.
    pub fn elem_match<Q: Into<Document>>(self, query: Q) -> Query {
        self.process(bson!("$elemMatch" => (query.into())))
    }

    /// Adds an `$in` constraint for this field.
    ///
    /// The value of this field must be equal to one of the values yielded by `values`.
    pub fn contained_in<I>(self, values: I) -> Query
        where I: IntoIterator, I::Item: Into<Bson>
    {
        self.process(bson!("$in" => (values.into_iter().map(I::Item::into).collect::<Vec<_>>())))
    }

    /// Adds an `$nin` constraint for this field.
    ///
    /// The value of this field must not be equal to all of the values yielded by `values`.
    pub fn not_contained_in<I>(self, values: I) -> Query
        where I: IntoIterator, I::Item: Into<Bson>
    {
        self.process(bson!("$nin" => (values.into_iter().map(I::Item::into).collect::<Vec<_>>())))
    }

    /// Adds a `$icase` constraint for this field.
    ///
    /// Makes all further constraints for this field case insensitive with regard to string
    /// values.
    pub fn case_insensitive(self) -> FieldConstraint {
        FieldConstraint("$icase".into(), FieldConstraintData::Child(Box::new(self)))
    }

    /// Adds a `$not` constraint for this field.
    ///
    /// Inverts the further query, making only those elements which do NOT satisfy the
    /// following constraints to match.
    pub fn not(self) -> FieldConstraint {
        FieldConstraint("$not".into(), FieldConstraintData::Child(Box::new(self)))
    }

    /// Adds an `$strand` constraint for this field.
    ///
    /// 1. If this field holds an array of strings, `$strand` returns those records whose
    ///    array contains all elements from `values`.
    /// 2. If this field holds a string, it is first split into an array by space `' '` or comma
    ///    `','` characters, and the resulting array is queried like in 1.
    pub fn str_and<I>(self, values: I) -> Query
        where I: IntoIterator, I::Item: Into<String>
    {
        self.process(bson! {
            "$strand" => (
                values.into_iter().map(|v| v.into().into())  // S -> String -> Bson
                    .collect::<Vec<Bson>>()
            )
        })
    }

    /// Adds an `$stror` constraint for this field.
    ///
    /// 1. If this field holds an array of strings, `$stror` returns those records whose
    ///    array contains at least one element from `values`.
    /// 2. If this field holds a string, it is first split into an array by space `' '` or comma
    ///    `','` characters, and the resulting array is queried like in 1.
    pub fn str_or<I>(self, values: I) -> Query
        where I: IntoIterator, I::Item: Into<String>
    {
        self.process(bson! {
            "$stror" => (
                values.into_iter().map(|v| v.into().into())  // S -> String -> Bson
                    .collect::<Vec<Bson>>()
            )
        })
    }
}

/// An entry point for constructing queries.
///
/// This is a convenience API. This structure provides the same methods as `Query`
/// structure and inside them a fresh `Query` instance is created and the corresponding
/// method is called on it. This is the main approach for constructing queries.
///
/// # Example
///
/// ```
/// use ejdb::query::{Query, Q};
///
/// assert_eq!(
///     Query::new().field("name").eq("Foo").inc("rating", 1).set("favorite", true),
///     Q.field("name").eq("Foo").inc("rating", 1).set("favorite", true)
/// )
/// ```
pub struct Q;

impl Q {
    #[inline(always)]
    pub fn hints(self, hints: QueryHints) -> Query {
        Query::new().hints(hints)
    }

    #[inline(always)]
    pub fn and<I>(self, queries: I) -> Query where I: IntoIterator, I::Item: Into<Document> {
        Query::new().and(queries)
    }

    #[inline(always)]
    pub fn or<I>(self, queries: I) -> Query where I: IntoIterator, I::Item: Into<Document> {
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
    pub fn add_to_set_all<S, I>(self, key: S, values: I) -> Query
        where S: Into<String>, I: IntoIterator, I::Item: Into<Bson>
    {
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
    pub fn upsert_many<D: Into<Document>>(self, document: D) -> Query {
        Query::new().upsert_many(document)
    }

    #[inline(always)]
    pub fn upsert<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().upsert(key, value)
    }

    #[inline(always)]
    pub fn set<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().set(key, value)
    }

    #[inline(always)]
    pub fn set_many<D: Into<Document>>(self, document: D) -> Query {
        Query::new().set_many(document)
    }

    #[inline(always)]
    pub fn pull<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().pull(key, value)
    }

    #[inline(always)]
    pub fn pull_all<S, I>(self, key: S, values: I) -> Query
        where S: Into<String>, I: IntoIterator, I::Item: Into<Bson>
    {
        Query::new().pull_all(key, values)
    }

    #[inline(always)]
    pub fn push<S: Into<String>, V: Into<Bson>>(self, key: S, value: V) -> Query {
        Query::new().push(key, value)
    }

    #[inline(always)]
    pub fn push_all<S, I>(self, key: S, values: I) -> Query
        where S: Into<String>, I: IntoIterator, I::Item: Into<Bson>
    {
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
            .upsert("missing", "value")
            .upsert_many(bson! {   // overwrites
                "isbn" => "0123456789",
                "name" => "my book"
            })
            .upsert("another_field", "another_value")
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
