ejdb.rs, high-level bindings for Embedded JSON Database engine
==============================================================

[![Build Status][travis]](https://travis-ci.org/netvl/ejdb.rs) [![crates.io][crates]](https://crates.io/crates/ejdb)

  [travis]: https://img.shields.io/travis/netvl/ejdb.rs.svg?style=flat-square
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
ejdb = "0.2"
```

To build the library, you need to have libejdb installed and its pkg-config file available
in the correct place for your operating system. Usually this requires just to install
the respective package using your system's native package manager.

## Changelog

### Version 0.2.0

* Bumped versions of various dependencies.

### Version 0.1.2

* Improved `bson!` macro to support optional values.

### Version 0.1.1

* Made `Database` implement `Send`.

### Version 0.1.0

* Initial release.

## License

This library is provided under MIT license.
