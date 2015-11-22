ejdb.rs, high-level bindings for Embedded JSON Database engine
==============================================================

[![Build Status][travis]](https://travis-ci.org/netvl/ejdb.rs) [![crates.io][crates]](https://crates.io/crates/ejdb)

  [travis]: https://img.shields.io/travis/netvl/ejdb.svg?style=flat-square
  [crates]: https://img.shields.io/crates/v/ejdb.svg?style=flat-square

[Documentation](https://netvl.github.io/ejdb.rs/)

This library provides high-level bindings to [EJDB], an Embedded JSON Database engine.

EJDB is a document-oriented NoSQL embedded database, very similar to MongoDB. It allows storing,
querying and manipulation of collections of BSON documents. It has MongoDB-like query language,
collection-level transactions and typed indices.

This library attempts to provide idiomatic and safe Rust bindings to EJDB. It exposes all
main features of EJDB: databases, collections, queries, transactions, indices and metadata.

See crate documentation for usage examples.

  [EJDB]: http://ejdb.org/

## Usage

Add a dependency in your `Cargo.toml`:

```toml
[dependencies]
ejdb = "0.1"
```

## Changelog

### Version 0.1.0

* Initial release
