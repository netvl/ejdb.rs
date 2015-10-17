use std::borrow::Cow;

use bson::{Bson, Document};

use utils::bson::DocumentBuilder;

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

    #[inline]
    pub fn field<S: Into<String>>(self, name: S) -> FieldConstraint {
        FieldConstraint(name.into().into(), FieldConstraintData::Root(self))
    }

    #[inline]
    pub fn build(self) -> (Document, Document) {
        (self.hints.into(), self.query)
    }
}

pub enum FieldConstraintData {
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

    pub fn exists(self, exists: bool) -> Query {
        self.process(ndb().set("$exists", exists))
    }

    pub fn contained_in<V: Into<Vec<Bson>>>(self, input: V) -> Query {
        self.process(ndb().set("$in", input.into()))
    }

    pub fn not_in<V: Into<Vec<Bson>>>(self, input: V) -> Query {
        self.process(ndb().set("$nin", input.into()))
    }

    pub fn icase(self) -> FieldConstraint {
        FieldConstraint("$icase".into(), FieldConstraintData::Child(Box::new(self)))
    }

    pub fn not(self) -> FieldConstraint {
        FieldConstraint("$not".into(), FieldConstraintData::Child(Box::new(self)))
    }

    pub fn str_and<S: Into<String>, V: IntoIterator<Item=S>>(self, values: V) -> Query {
        self.process(ndb()
            .set("$strand", values.into_iter()
                .map(|v| v.into().into())  // S -> String -> Bson
                .collect::<Vec<Bson>>()))
    }

    pub fn str_or<S: Into<String>, V: IntoIterator<Item=S>>(self, values: V) -> Query {
        self.process(ndb()
            .set("$stror", values.into_iter()
                .map(|v| v.into().into())  // S -> String -> Bson
                .collect::<Vec<Bson>>()))
    }
}

#[inline(always)]
fn ndb() -> DocumentBuilder { DocumentBuilder::new() }
