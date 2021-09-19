ctxmap::schema!(Schema);

static KEY_X: once_cell::sync::Lazy<ctxmap::CtxMapKey<Schema, u8>> =
    once_cell::sync::Lazy::new(|| {
        ctxmap::schema::Schema::register(|| Box::new(Box::<u8>::new(10)))
    });
inventory::submit! { Schema(|| { once_cell::sync::Lazy::force(&KEY_X); })}

use ctxmap::CtxMap;

#[test]
fn new() {
    let m = CtxMap::new();
    assert_eq!(m[&KEY_X], 10);
}

#[test]
fn with() {
    let mut m = CtxMap::new();
    m.with(&KEY_X, &20, |m| {
        assert_eq!(m[&KEY_X], 20);
    });
    assert_eq!(m[&KEY_X], 10);
}

#[test]
fn with_nest() {
    let mut m = CtxMap::new();
    m.with(&KEY_X, &20, |m| {
        assert_eq!(m[&KEY_X], 20);
        m.with(&KEY_X, &30, |m| {
            assert_eq!(m[&KEY_X], 30);
        });
        assert_eq!(m[&KEY_X], 20);
    });
    assert_eq!(m[&KEY_X], 10);
}
