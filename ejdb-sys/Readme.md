# Low-level generated bindings to libejdb

This crate provides low-level bindings to libejdb. It is used as a basis for ejdb.rs, high-level
bindings for EJDB.

## Usage

Add a dependency in your `Cargo.toml`:

```toml
[dependencies]
ejdb-sys = "0.1"
```

You need to have `libejdb` installed and available through pkg-config in order for this package
to build correctly.

Note, however, that you usually don't need to depend on this crate directly; use `ejdb`
library instead. Therefore, no compatibility guarantees are given.

## License

This library is provided under MIT license.
