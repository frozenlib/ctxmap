use ctxmap::CtxMap;

fn main() {
    ctxmap::schema!(Schema);
    ctxmap::key!(Schema { mut KEY_A: u8 = 1 });
    let mut m = CtxMap::new();
    m.with(&KEY_A, &10, |_| {});
}
