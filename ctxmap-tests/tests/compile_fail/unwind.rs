use ctxmap::CtxMap;
use std::panic::catch_unwind;

fn main() {
    ctxmap::schema!(Schema);
    ctxmap::key!(Schema { KEY_A: u8 = 1 });
    let mut m = CtxMap::new();
    let _ = catch_unwind(|| {
        m.with(&KEY_A, &2, |m| {
            panic!("");
        });
    });
}
