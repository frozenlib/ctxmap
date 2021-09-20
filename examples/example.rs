fn main() {
    ctxmap::schema!(Schema);
    ctxmap::key!(Schema { KEY_A: u32 = 10 });

    let mut m = ctxmap::CtxMap::new();
    assert_eq!(m[&KEY_A], 10);

    m.with(&KEY_A, &20, |m| {
        assert_eq!(m[&KEY_A], 20);
    });

    assert_eq!(m[&KEY_A], 10);
}
