use ctxmap::CtxMap;
use std::mem::swap;

ctxmap::schema!(Schema);
ctxmap::key!(Schema { KEY_X: u8 = 1 });

fn main() {
    let mut m0 = CtxMap::new();
    let mut m1 = CtxMap::new();
    m0.with(&KEY_X, &2, |m0| {
        m1.with(&KEY_X, &3, |m1| {
            swap(m0, m1);
        });
    });
}
