ejdb.rs, high-level bindings for Embedded JSON Database engine
==============================================================

### Unmaintained

I no longer have a capacity to maintain this project. Feel free to reach out if you want to continue its development and take its name on crates.io.

---

![Maintenance](https://img.shields.io/badge/maintentance-looking--for--maintainers-yellow?style=flat-square)
[![Build Status][actions]](https://github.com/netvl/ejdb.rs/actions?query=workflow%3ACI)
[![crates.io][crates]](https://crates.io/crates/ejdb)
[![ejdb docs][docs-ejdb]](https://docs.rs/ejdb)
[![ejdb-sys docs][docs-ejdb-sys]](https://docs.rs/ejdb-sys)

  [actions]: https://img.shields.io/github/workflow/status/netvl/ejdb.rs/CI/master?style=flat-square
  [crates]: https://img.shields.io/crates/v/ejdb.svg?style=flat-square
  [docs-ejdb]: https://img.shields.io/badge/docs-ejdb-6495ed.svg?style=flat-square
  [docs-ejdb-sys]: https://img.shields.io/badge/docs-ejdb--sys-6495ed.svg?style=flat-square

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
ejdb = "0.4"
```

To build the library, you need to have `cmake` installaled along with `gcc` and `clang`. 

## Changelog

### Version 0.4.0

* Switched to manual compilation and linking of ejdb statically.

### Version 0.3.0

* Bumped the `bson` dependency version.

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
