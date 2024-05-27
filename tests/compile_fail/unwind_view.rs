use ctxmap::CtxMap;
use std::panic::catch_unwind;

fn main() {
    ctxmap::schema!(Schema);
    ctxmap::key!(Schema { KEY_A: u8 = 1 });
    let mut m = CtxMap::new();
    let mut v = m.view();
    let _ = catch_unwind(|| {
        v.with(&KEY_A, &2, |v| {
            panic!("");
        });
    });
}
