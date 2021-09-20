# ctxmap

[![Crates.io](https://img.shields.io/crates/v/ctxmap.svg)](https://crates.io/crates/ctxmap)
[![Docs.rs](https://docs.rs/ctxmap/badge.svg)](https://docs.rs/ctxmap/)
[![Actions Status](https://github.com/frozenlib/ctxmap/workflows/CI/badge.svg)](https://github.com/frozenlib/ctxmap/actions)

Safe, `HashMap<&'static _, *const dyn Any>` like collection.

## Install

Add this to your Cargo.toml:

```toml
[dependencies]
ctxmap = "0.1.0"
```

## Example

```rust
ctxmap::schema!(Schema);
ctxmap::key!(Schema { KEY_A: u32 = 10 });
ctxmap::key!(Schema { ref KEY_B: str = "abc" });

let mut m = ctxmap::CtxMap::new();
assert_eq!(m[&KEY_A], 10);
assert_eq!(m[&KEY_B], "abc");

m.with(&KEY_A, &20, |m| {
    assert_eq!(m[&KEY_A], 20);
});

assert_eq!(m[&KEY_A], 10);
```

## License

This project is dual licensed under Apache-2.0/MIT. See the two LICENSE-\* files for details.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
