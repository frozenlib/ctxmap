use std::fmt::Display;

ctxmap::schema!(Schema);
ctxmap::key!(Schema { KEY_X: u8 = 10 });
ctxmap::key!(Schema {
    KEY_Y: dyn Display = 5
});
ctxmap::key!(Schema { KEY_Z: Option<u8> });

ctxmap::key!(Schema {
    KEY_MANY_0: u8,
    KEY_MANY_1: u8 = 10,
});

ctxmap::schema!(pub PubSchema);
ctxmap::key!(PubSchema { pub PUB_KEY: u8 });

ctxmap::key!(Schema { ref KEY_STR: str = "abc" });
ctxmap::key!(Schema { ref KEY_STRING: str = format!("abc-{}", 1) });

mod mod_a {
    ctxmap::schema!(pub ModASchema);
}
mod mod_b {
    ctxmap::key!(super::mod_a::ModASchema { KEY: u8 = 10 });
}

use ctxmap::CtxMap;

#[test]
fn new() {
    let m = CtxMap::new();
    assert_eq!(m[&KEY_X], 10);
    assert_eq!(m[&KEY_Y].to_string(), "5");
    assert_eq!(m[&KEY_Z], None);

    assert_eq!(m[&KEY_MANY_0], 0);
    assert_eq!(m[&KEY_MANY_1], 10);
    assert_eq!(&m[&KEY_STR], "abc");
    assert_eq!(&m[&KEY_STRING], "abc-1");
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

#[test]
fn with_dst() {
    let mut m = CtxMap::new();
    m.with(&KEY_Y, &100, |m| {
        assert_eq!(m[&KEY_Y].to_string(), "100");
    });
    m.with(&KEY_Y, &String::from("abc"), |m| {
        assert_eq!(m[&KEY_Y].to_string(), "abc");
    });
    m.with(&KEY_Y, &"def", |m| {
        assert_eq!(m[&KEY_Y].to_string(), "def");
    });
    assert_eq!(m[&KEY_Y].to_string(), "5");
}

#[test]
fn with_str() {
    let mut m = CtxMap::new();
    m.with(&KEY_STR, "zzz", |m| {
        assert_eq!(&m[&KEY_STR], "zzz");
    });
    assert_eq!(&m[&KEY_STR], "abc");
}

#[test]
fn in_func_key() {
    ctxmap::key!(Schema { KEY_A: u8 = 99 });
    let m = CtxMap::new();
    assert_eq!(m[&KEY_A], 99);
}
