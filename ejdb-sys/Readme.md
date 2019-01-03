# Native bindings for libejdb

This crate provides low-level bindings to libejdb. It is used as a basis for ejdb.rs, high-level
bindings for EJDB.

## Usage

Add a dependency in your `Cargo.toml`:

```toml
[dependencies]
ejdb-sys = "0.3"
```
To compile you need to have `cmake` installaled along with `gcc` and `clang`. 
In runtime you need to have `gzlib` installed and available through pkg-config (almost all distros have it preinstalled).


Note, however, that you usually don't need to depend on this crate directly; use `ejdb`
library instead. Therefore, no compatibility guarantees are given.

## License

This library is provided under MIT license.
