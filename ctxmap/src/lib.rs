// #![include_doc("../../README.md", start)]
//! # ctxmap
//!
//! [![Crates.io](https://img.shields.io/crates/v/ctxmap.svg)](https://crates.io/crates/ctxmap)
//! [![Docs.rs](https://docs.rs/ctxmap/badge.svg)](https://docs.rs/ctxmap/)
//! [![Actions Status](https://github.com/frozenlib/ctxmap/workflows/CI/badge.svg)](https://github.com/frozenlib/ctxmap/actions)
//!
//! A collection that can store references of different types and lifetimes.
//!
//! ## Install
//!
//! Add this to your Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! ctxmap = "0.1.0"
//! ```
//!
//! ## Example
//!
//! ```rust
//! ctxmap::schema!(Schema);
//! ctxmap::key!(Schema { KEY_A: u32 = 10 });
//! ctxmap::key!(Schema { KEY_B: str = "abc" });
//!
//! let mut m = ctxmap::CtxMap::new();
//! assert_eq!(m[&KEY_A], 10);
//! assert_eq!(&m[&KEY_B], "abc");
//!
//! m.with(&KEY_A, &20, |m| {
//!     assert_eq!(m[&KEY_A], 20);
//! });
//!
//! assert_eq!(m[&KEY_A], 10);
//! ```
// #![include_doc("../../README.md", end("## License"))]

use helpers::*;
use once_cell::sync::Lazy;
use std::{any::Any, cell::RefCell, marker::PhantomData, ops::Index};

/// A collection that can store references of different types and lifetimes.
pub struct CtxMap<S: Schema> {
    schema: PhantomData<S>,
    ptrs: Vec<Option<*const dyn Any>>,
    values: RefCell<Vec<Option<Box<dyn Any>>>>,
}

impl<S: Schema> CtxMap<S> {
    /// Create a new `CtxMap` with initial values.
    ///
    /// # Example
    ///
    /// ```
    /// ctxmap::schema!(S);
    /// ctxmap::key!(S { KEY_A: u16 = 20 });
    /// ctxmap::key!(S { KEY_B: u8 });
    ///
    /// let m = ctxmap::CtxMap::new();
    /// assert_eq!(m[&KEY_A], 20);
    /// assert_eq!(m.get(&KEY_A), Some(&20));
    /// assert_eq!(m.get(&KEY_B), None);
    /// ```
    pub fn new() -> Self {
        Self {
            schema: PhantomData,
            values: RefCell::new(Vec::new()),
            ptrs: Vec::new(),
        }
    }

    /// Sets a value to `CtxMap` only while `f` is being called.
    ///
    /// # Example
    ///
    /// ```
    /// ctxmap::schema!(S);
    /// ctxmap::key!(S { KEY_A: u16 = 20 });
    ///
    /// let mut m = ctxmap::CtxMap::new();
    /// assert_eq!(m[&KEY_A], 20);
    /// m.with(&KEY_A, &30, |m| {
    ///     assert_eq!(m[&KEY_A], 30);
    /// });
    /// assert_eq!(m[&KEY_A], 20);
    /// ```
    pub fn with<T: ?Sized + 'static, U>(
        &mut self,
        key: &'static Key<S, T>,
        value: &T,
        f: impl Fn(&mut Self) -> U,
    ) -> U {
        let index = *key.index;
        if self.ptrs.len() <= index {
            self.ptrs.resize_with(index + 1, || None);
        }
        let old = self.ptrs[index];
        let ptr: *const T = value;
        self.ptrs[index] = Some(&ptr);
        let retval = f(self);
        self.ptrs[index] = old;
        retval
    }

    pub fn get<T: ?Sized>(&self, key: &'static Key<S, T>) -> Option<&T> {
        let index = *key.index;
        if let Some(Some(p)) = self.ptrs.get(index) {
            unsafe { Some(&**<dyn Any>::downcast_ref::<*const T>(&**p).unwrap()) }
        } else {
            let data = key.data.as_ref()?;
            loop {
                if let Some(Some(value)) = self.values.borrow().get(index) {
                    let p: *const dyn Any = value.as_ref();
                    return Some(data.get(unsafe { &*p }));
                }
                let init = data.init();
                let mut values = self.values.borrow_mut();
                if values.len() <= index {
                    values.resize_with(index + 1, || None);
                }
                values[index] = Some(init);
            }
        }
    }
}

impl<S: Schema> Default for CtxMap<S> {
    fn default() -> Self {
        Self::new()
    }
}
impl<S, T> Index<&'static Key<S, T>> for CtxMap<S>
where
    S: Schema,
    T: ?Sized + 'static,
{
    type Output = T;

    fn index(&self, index: &'static Key<S, T>) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}

/// A key for [`CtxMap`].
///
/// Use [`key`] macro to create `Key`.
pub struct Key<S: Schema, T: ?Sized + 'static> {
    schema: PhantomData<S>,
    index: Lazy<usize>,
    data: Option<Box<dyn KeyData<T>>>,
}

trait KeyData<T: ?Sized>: Send + Sync {
    fn get<'a>(&self, value: &'a dyn Any) -> &'a T;
    fn init(&self) -> Box<dyn Any>;
}

/// Available key collection for [`CtxMap`].
///
/// Use [`schema`] macro to define a type that implement `Schema`.
pub trait Schema: 'static + Sized {
    fn data() -> &'static SchemaData;

    fn key<T: ?Sized>() -> Key<Self, T> {
        Key {
            schema: PhantomData,
            index: Lazy::new(|| Self::data().push_key()),
            data: None,
        }
    }

    fn key_with_default<Init, ToRef, V, T>(init: Init, to_ref: ToRef) -> Key<Self, T>
    where
        Init: Send + Sync + Fn() -> V + 'static,
        ToRef: Send + Sync + Fn(&V) -> &T + 'static,
        V: 'static,
        T: ?Sized,
    {
        Key {
            schema: PhantomData,
            index: Lazy::new(|| Self::data().push_key()),
            data: Some(Box::new(KeyDataValue { init, to_ref })),
        }
    }
}

struct KeyDataValue<Init, ToRef> {
    init: Init,
    to_ref: ToRef,
}

impl<Init, ToRef, V, T> KeyData<T> for KeyDataValue<Init, ToRef>
where
    Init: Send + Sync + Fn() -> V,
    ToRef: Send + Sync + Fn(&V) -> &T,
    V: 'static,
    T: ?Sized,
{
    fn get<'a>(&self, value: &'a dyn Any) -> &'a T {
        (self.to_ref)(<dyn Any>::downcast_ref::<V>(value).unwrap())
    }
    fn init(&self) -> Box<dyn Any> {
        Box::new((self.init)())
    }
}

pub mod helpers {
    pub use once_cell::sync::Lazy;
    use std::sync::Mutex;

    pub struct SchemaData(Mutex<Keys>);

    impl SchemaData {
        pub fn new() -> Self {
            SchemaData(Mutex::new(Keys { len: 0 }))
        }
        pub(crate) fn push_key(&self) -> usize {
            let mut d = self.0.lock().unwrap();
            let index = d.len;
            d.len += 1;
            index
        }
    }
    impl Default for SchemaData {
        fn default() -> Self {
            Self::new()
        }
    }

    struct Keys {
        len: usize,
    }
}

/// Define a type that implements [`Schema`].
///
/// # Example
///
/// ```
/// ctxmap::schema!(S1);
/// ctxmap::schema!(pub S2);
/// ```
#[macro_export]
macro_rules! schema {
    ($vis:vis $id:ident) => {
        $vis struct $id;
        impl $crate::Schema for $id {
            fn data() -> &'static $crate::helpers::SchemaData {
                static DATA: $crate::helpers::Lazy<$crate::helpers::SchemaData> =
                    $crate::helpers::Lazy::new($crate::helpers::SchemaData::new);
                &DATA
            }
        }

    };
}

/// Define a key for [`CtxMap`].
///
/// # Example
///
/// ```
/// ctxmap::schema!(Schema1);
/// ctxmap::key!(Schema1 { KEY_A: u8 = 10 });
/// ctxmap::key!(Schema1 {
///     KEY_1: u8,
///     KEY_2: u16 = 10,
///     pub KEY_3: u32,
///     pub KEY_4: str = "abc",
/// });
///
/// ctxmap::schema!(Schema2);
/// ctxmap::key!(Schema2 { KEY_X: u8 = 10 });
/// ```
///
/// ```
/// ctxmap::schema!(S);
/// ctxmap::key!(S { KEY_A: u8 });
/// ctxmap::key!(S { KEY_B: u8 = 50});
///
/// let m = ctxmap::CtxMap::new();
/// assert_eq!(m.get(&KEY_A), None);
/// assert_eq!(m.get(&KEY_B), Some(&50));
/// assert_eq!(m[&KEY_B], 50);
/// ```
///
/// You can specify visibility.
///
/// ```
/// ctxmap::schema!(pub S);
/// ctxmap::key!(S { KEY_A: u8 });
/// ctxmap::key!(S { pub KEY_B: u8 });
/// ctxmap::key!(S { pub(crate) KEY_C: u8 });
/// ```
#[macro_export]
macro_rules! key {
    ($schema:ty { }) => { };
    ($schema:ty { $vis:vis $id:ident: $type:ty }) => {
        $vis static $id: $crate::helpers::Lazy<$crate::Key<$schema, $type>> =
            $crate::helpers::Lazy::new(|| <$schema as $crate::Schema>::key());
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty = $init:expr }) => {
        $vis static $id: $crate::helpers::Lazy<$crate::Key<$schema, $type>> =
            $crate::helpers::Lazy::new(|| <$schema as $crate::Schema>::key_with_default::<_, _, _, $type>(
                || $init,
                |x| x));
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty, $($tt:tt)* }) => {
        $crate::key!($schema { $vis $id: $type });
        $crate::key!($schema { $($tt)* });
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty = $init:expr, $($tt:tt)* }) => {
        $crate::key!($schema { $vis $id: $type = $init });
        $crate::key!($schema { $($tt)* });
    };
}
