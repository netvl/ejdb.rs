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
        FieldConstraint(name.into(), self)
    }

    #[inline]
    pub fn build(self) -> (Document, Document) {
        (self.hints.into(), self.query)
    }
}

pub struct FieldConstraint(String, Query);

impl FieldConstraint {
    pub fn eq<V: Into<Bson>>(mut self, value: V) -> Query {
        self.1.query.insert(self.0, value);
        self.1
    }

    pub fn begin<S: Into<String>>(mut self, value: S) -> Query {
        self.1.query.insert(self.0, DocumentBuilder::new().set("$begin", value.into()));
        self.1
    }

    // TODO: add between, gt, gte, lt, lte operators for numbers

    pub fn exists(mut self, exists: bool) -> Query {
        self.1.query.insert(self.0, ndb().set("$exists", exists));
        self.1
    }
}

#[inline(always)]
fn ndb() -> DocumentBuilder { DocumentBuilder::new() }
