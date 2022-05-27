// #![include_doc("../../README.md", start("A collection that can store references of different types and lifetimes."))]
//! A collection that can store references of different types and lifetimes.
//!
//! ## Install
//!
//! Add this to your Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! ctxmap = "0.2.0"
//! ```
//!
//! ## Example
//!
//! ```rust
//! ctxmap::schema!(Schema);
//! ctxmap::key!(Schema {
//!     KEY_NO_DEFAULT: u32,
//!     KEY_INT: u32 = 10,
//!     KEY_DYN: dyn std::fmt::Display = 10,
//!     KEY_STR: str = "abc",
//!     KEY_STRING: str = format!("abc-{}", 10),
//! });
//!
//! let mut m = ctxmap::CtxMap::new();
//! assert_eq!(m.get(&KEY_NO_DEFAULT), None);
//! assert_eq!(m.get(&KEY_INT), Some(&10));
//! assert_eq!(m[&KEY_INT], 10);
//! assert_eq!(&m[&KEY_STR], "abc");
//!
//! m.with(&KEY_INT, &20, |m| {
//!     assert_eq!(m[&KEY_INT], 20);
//! });
//!
//! assert_eq!(m[&KEY_INT], 10);
//! ```
// #![include_doc("../../README.md", end("## License"))]

use helpers::*;
use once_cell::sync::Lazy;
use std::{
    any::Any,
    cell::RefCell,
    marker::PhantomData,
    ops::{Deref, Index},
};

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

    /// Sets a value corresponding to the key only while `f` is being called.
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
        f: impl FnOnce(&mut CtxMapView<S>) -> U,
    ) -> U {
        self.view().with(key, value, f)
    }

    /// Get [`CtxMapView`] that references `self`.
    pub fn view(&mut self) -> CtxMapView<S> {
        CtxMapView(self)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// # Example
    ///
    /// ```
    /// ctxmap::schema!(S);
    /// ctxmap::key!(S { KEY_A: u16 });
    ///
    /// let mut m = ctxmap::CtxMap::new();
    /// assert_eq!(m.get(&KEY_A), None);
    /// m.with(&KEY_A, &10, |m| {
    ///     assert_eq!(m.get(&KEY_A), Some(&10));
    /// });
    /// assert_eq!(m.get(&KEY_A), None);
    /// ```
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

/// Mutable reference to [`CtxMap`] where the value has changed.
///
/// Use `CtxMapViwe` instead of `&mut CtxMap` because `&mut CtxMap`,
/// whose value has been changed, will be broken if [`std::mem::swap`] is used.
pub struct CtxMapView<'a, S: Schema>(&'a mut CtxMap<S>);

impl<'a, S: Schema> CtxMapView<'a, S> {
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
    ///     m.with(&KEY_A, &40, |m| {
    ///        assert_eq!(m[&KEY_A], 40);
    ///    });
    ///    assert_eq!(m[&KEY_A], 30);
    /// });
    /// assert_eq!(m[&KEY_A], 20);
    /// ```
    pub fn with<T: ?Sized + 'static, U>(
        &mut self,
        key: &'static Key<S, T>,
        value: &T,
        f: impl FnOnce(&mut CtxMapView<S>) -> U,
    ) -> U {
        let index = *key.index;
        if self.0.ptrs.len() <= index {
            self.0.ptrs.resize_with(index + 1, || None);
        }
        let old = self.0.ptrs[index];
        let ptr: *const T = value;
        self.0.ptrs[index] = Some(&ptr);
        let retval = f(self);
        self.0.ptrs[index] = old;
        retval
    }

    /// Return `CtxMapView` with modified lifetime.
    pub fn viwe(&mut self) -> CtxMapView<S> {
        CtxMapView(self.0)
    }
}
impl<'a, S: Schema> Deref for CtxMapView<'a, S> {
    type Target = CtxMap<S>;

    fn deref(&self) -> &Self::Target {
        self.0
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

/// Key collection for [`CtxMap`].
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

#[doc(hidden)]
pub mod helpers {
    pub use once_cell::sync::Lazy;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct SchemaData {
        next: AtomicUsize,
    }

    impl SchemaData {
        pub const fn new() -> Self {
            SchemaData {
                next: AtomicUsize::new(0),
            }
        }
        pub(crate) fn push_key(&self) -> usize {
            self.next.fetch_add(1, Ordering::SeqCst)
        }
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
                static DATA: $crate::helpers::SchemaData = $crate::helpers::SchemaData::new();
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
/// ctxmap::schema!(S);
/// ctxmap::key!(S { KEY_1: u8 });
/// ctxmap::key!(S { KEY_2: str });
/// ```
///
/// You can define multiple keys at once.
///
/// ```
/// ctxmap::schema!(S);
/// ctxmap::key!(S {
///     KEY_1: u8,
///     KEY_2: str,
/// });
/// ```
///
/// You can specify a default value.
///
/// The default value can be an expression that, when applied with `&` operator, becomes a reference to the type of the key.
///
/// For example, `&"abc"` and `&String::new()` can be `&str`,
/// so `"abc"` and `String::new()` can be used as default values for keys of type `str`.
///
/// ```
/// use std::fmt::Display;
///
/// ctxmap::schema!(S);
/// ctxmap::key!(S {
///     KEY_1: u8 = 10,
///     KEY_2: str = "abc",
///     KEY_3: str = String::new(),
///     KEY_4: dyn Display = 10,
///     KEY_5: dyn Display = "xyz",
/// });
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
