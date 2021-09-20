fn main() {
    ctxmap::schema!(Schema);
    ctxmap::key!(Schema { KEY_A: u32 = 10 });
    ctxmap::key!(Schema { ref KEY_B: str = "abc" });

    let mut m = ctxmap::CtxMap::new();
    assert_eq!(m[&KEY_A], 10);
    assert_eq!(&m[&KEY_B], "abc");

    m.with(&KEY_A, &20, |m| {
        assert_eq!(m[&KEY_A], 20);
    });

    assert_eq!(m[&KEY_A], 10);
}
