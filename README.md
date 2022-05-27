# ctxmap

[![Crates.io](https://img.shields.io/crates/v/ctxmap.svg)](https://crates.io/crates/ctxmap)
[![Docs.rs](https://docs.rs/ctxmap/badge.svg)](https://docs.rs/ctxmap/)
[![Actions Status](https://github.com/frozenlib/ctxmap/workflows/CI/badge.svg)](https://github.com/frozenlib/ctxmap/actions)

A collection that can store references of different types and lifetimes.

## Install

Add this to your Cargo.toml:

```toml
[dependencies]
ctxmap = "0.3.0"
```

## Example

```rust
ctxmap::schema!(Schema);
ctxmap::key!(Schema {
    KEY_NO_DEFAULT: u32,
    KEY_INT: u32 = 10,
    KEY_DYN: dyn std::fmt::Display = 10,
    KEY_STR: str = "abc",
    KEY_STRING: str = format!("abc-{}", 10),
});

let mut m = ctxmap::CtxMap::new();
assert_eq!(m.get(&KEY_NO_DEFAULT), None);
assert_eq!(m.get(&KEY_INT), Some(&10));
assert_eq!(m[&KEY_INT], 10);
assert_eq!(&m[&KEY_STR], "abc");

m.with(&KEY_INT, &20, |m| {
    assert_eq!(m[&KEY_INT], 20);
});

assert_eq!(m[&KEY_INT], 10);
```

## License

This project is dual licensed under Apache-2.0/MIT. See the two LICENSE-\* files for details.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
