// #![include_doc("../../README.md", start("A collection that can store references of different types and lifetimes."))]
//! A collection that can store references of different types and lifetimes.
//!
//! ## Install
//!
//! Add this to your Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! ctxmap = "0.5.0"
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
//!     mut KEY_MUT: u32 = 30,
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
//! assert_eq!(m[&KEY_INT], 10);
//!
//! assert_eq!(m[&KEY_MUT], 30);
//! m[&KEY_MUT] = 40;
//! assert_eq!(m[&KEY_MUT], 40);
//!
//! m.with_mut(&KEY_MUT, &mut 50, |m| {
//!     assert_eq!(m[&KEY_MUT], 50);
//!     m[&KEY_MUT] = 60;
//!     assert_eq!(m[&KEY_MUT], 60);
//! });
//! assert_eq!(m[&KEY_MUT], 40);
//! ```
// #![include_doc("../../README.md", end("## License"))]

use helpers::*;
use once_cell::sync::Lazy;
use std::{
    any::Any,
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// A collection that can store references of different types and lifetimes.
pub struct CtxMap<S: Schema> {
    schema: PhantomData<S>,
    ptrs: Vec<Option<*const dyn Any>>,
    values: UnsafeCell<Vec<Option<Box<dyn Any>>>>,
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
            values: UnsafeCell::new(Vec::new()),
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

    /// Sets a mutable value corresponding to the key only while `f` is being called.
    ///
    /// # Example
    ///
    /// ```
    /// ctxmap::schema!(S);
    /// ctxmap::key!(S { mut KEY_A: u16 = 20 });
    ///
    /// let mut m = ctxmap::CtxMap::new();
    /// assert_eq!(m[&KEY_A], 20);
    /// m[&KEY_A] = 25;
    /// assert_eq!(m[&KEY_A], 25);
    /// m.with_mut(&KEY_A, &mut 30, |m| {
    ///     assert_eq!(m[&KEY_A], 30);
    ///     m[&KEY_A] = 35;
    ///     assert_eq!(m[&KEY_A], 35);
    /// });
    /// assert_eq!(m[&KEY_A], 25);
    /// ```
    pub fn with_mut<T: ?Sized + 'static, U, const MUT: bool>(
        &mut self,
        key: &'static Key<S, T, MUT>,
        value: &mut T,
        f: impl FnOnce(&mut CtxMapView<S>) -> U,
    ) -> U {
        self.view().with_mut(key, value, f)
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
    pub fn get<T: ?Sized, const MUT: bool>(&self, key: &'static Key<S, T, MUT>) -> Option<&T> {
        let key = &*key.0;
        let index = key.index;
        unsafe {
            if let Some(Some(p)) = self.ptrs.get(index) {
                if let Some(p) = <dyn Any>::downcast_ref::<*const T>(&**p) {
                    Some(&**p)
                } else if let Some(p) = <dyn Any>::downcast_ref::<*mut T>(&**p) {
                    Some(&**p)
                } else {
                    unreachable!()
                }
            } else {
                let data = key.data.as_ref()?.as_ref();
                loop {
                    if let Some(Some(value)) = (*self.values.get()).get(index) {
                        let p: *const dyn Any = value.as_ref();
                        return Some(data.get(&*p));
                    }
                    self.init_value(index, data);
                }
            }
        }
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// # Example
    ///
    /// ```
    /// ctxmap::schema!(S);
    /// ctxmap::key!(S { mut KEY_A: u16 });
    ///
    /// let mut m = ctxmap::CtxMap::new();
    /// assert_eq!(m.get_mut(&KEY_A), None);
    /// m.with_mut(&KEY_A, &mut 10, |m| {
    ///     assert_eq!(m.get_mut(&KEY_A), Some(&mut 10));
    /// });
    /// assert_eq!(m.get_mut(&KEY_A), None);
    /// ```
    pub fn get_mut<T: ?Sized>(&mut self, key: &'static KeyMut<S, T>) -> Option<&mut T> {
        let key = &*key.0;
        let index = key.index;
        unsafe {
            if let Some(Some(p)) = self.ptrs.get(index) {
                Some(&mut **<dyn Any>::downcast_ref::<*mut T>(&**p).unwrap())
            } else {
                let data = key.data.as_ref()?.as_ref();
                loop {
                    if let Some(Some(value)) = (*self.values.get()).get_mut(index) {
                        let p: *mut dyn Any = value.as_mut();
                        return Some(data.get_mut(&mut *p));
                    }
                    self.init_value(index, data);
                }
            }
        }
    }
    unsafe fn init_value<T: ?Sized>(&self, index: usize, data: &dyn KeyData<T>) {
        let init = data.init();
        let values = &mut *self.values.get();
        if values.len() <= index {
            values.resize_with(index + 1, || None);
        }
        values[index] = Some(init);
    }
}

impl<S: Schema> Default for CtxMap<S> {
    fn default() -> Self {
        Self::new()
    }
}
impl<S, T, const MUT: bool> Index<&'static Key<S, T, MUT>> for CtxMap<S>
where
    S: Schema,
    T: ?Sized + 'static,
{
    type Output = T;

    fn index(&self, index: &'static Key<S, T, MUT>) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}
impl<S, T> IndexMut<&'static KeyMut<S, T>> for CtxMap<S>
where
    S: Schema,
    T: ?Sized + 'static,
{
    fn index_mut(&mut self, index: &'static KeyMut<S, T>) -> &mut Self::Output {
        self.get_mut(index).expect("no entry found for key")
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
    /// See [`CtxMap::with`] for more details.
    pub fn with<T: ?Sized + 'static, U>(
        &mut self,
        key: &'static Key<S, T>,
        value: &T,
        f: impl FnOnce(&mut CtxMapView<S>) -> U,
    ) -> U {
        let ptr: *const T = value;
        self.with_impl(key, ptr, f)
    }

    /// Sets a mutable value to `CtxMap` only while `f` is being called.
    ///
    /// See [`CtxMap::with_mut`] for more details.
    pub fn with_mut<T: ?Sized + 'static, U, const MUT: bool>(
        &mut self,
        key: &'static Key<S, T, MUT>,
        value: &mut T,
        f: impl FnOnce(&mut CtxMapView<S>) -> U,
    ) -> U {
        let ptr: *mut T = value;
        self.with_impl(key, ptr, f)
    }

    fn with_impl<T: ?Sized + 'static, U, P: 'static, const MUT: bool>(
        &mut self,
        key: &'static Key<S, T, MUT>,
        ptr: P,
        f: impl FnOnce(&mut CtxMapView<S>) -> U,
    ) -> U {
        let key = &*key.0;
        let index = key.index;
        if self.0.ptrs.len() <= index {
            self.0.ptrs.resize_with(index + 1, || None);
        }
        let old = self.0.ptrs[index];
        self.0.ptrs[index] = Some(&ptr);
        let retval = f(self);
        self.0.ptrs[index] = old;
        retval
    }

    /// Return `CtxMapView` with modified lifetime.
    pub fn view(&mut self) -> CtxMapView<S> {
        CtxMapView(self.0)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// See [`CtxMap::get`] for more details.
    pub fn get<T: ?Sized, const MUT: bool>(&self, key: &'static Key<S, T, MUT>) -> Option<&T> {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// See [`CtxMap::get_mut`] for more details.
    pub fn get_mut<T: ?Sized>(&mut self, key: &'static KeyMut<S, T>) -> Option<&mut T> {
        self.0.get_mut(key)
    }
}

impl<'a, S, T, const MUT: bool> Index<&'static Key<S, T, MUT>> for CtxMapView<'a, S>
where
    S: Schema,
    T: ?Sized + 'static,
{
    type Output = T;

    fn index(&self, index: &'static Key<S, T, MUT>) -> &Self::Output {
        &self.0[index]
    }
}
impl<'a, S, T> IndexMut<&'static KeyMut<S, T>> for CtxMapView<'a, S>
where
    S: Schema,
    T: ?Sized + 'static,
{
    fn index_mut(&mut self, index: &'static KeyMut<S, T>) -> &mut Self::Output {
        &mut self.0[index]
    }
}

/// A key for [`CtxMap`].
///
/// Use [`key`] macro to create `Key`.
pub struct Key<S: Schema, T: ?Sized + 'static, const MUT: bool = false>(Lazy<RawKey<S, T, MUT>>);
pub type KeyMut<S, T> = Key<S, T, true>;
trait KeyData<T: ?Sized>: Send + Sync {
    fn get<'a>(&self, value: &'a dyn Any) -> &'a T;
    fn get_mut<'a>(&self, value: &'a mut dyn Any) -> &'a mut T;
    fn init(&self) -> Box<dyn Any>;
}

/// Key collection for [`CtxMap`].
///
/// Use [`schema`] macro to define a type that implement `Schema`.
pub trait Schema: 'static + Sized {
    fn data() -> &'static SchemaData;
}

struct KeyDataValue<Init, ToRef, ToMut> {
    init: Init,
    to_ref: ToRef,
    to_mut: ToMut,
}

impl<Init, ToRef, ToMut, V, T> KeyData<T> for KeyDataValue<Init, ToRef, ToMut>
where
    Init: Send + Sync + Fn() -> V,
    ToRef: Send + Sync + Fn(&V) -> &T,
    ToMut: Send + Sync + Fn(&mut V) -> &mut T,
    V: 'static,
    T: ?Sized,
{
    fn get<'a>(&self, value: &'a dyn Any) -> &'a T {
        (self.to_ref)(<dyn Any>::downcast_ref::<V>(value).unwrap())
    }
    fn get_mut<'a>(&self, value: &'a mut dyn Any) -> &'a mut T {
        (self.to_mut)(<dyn Any>::downcast_mut::<V>(value).unwrap())
    }
    fn init(&self) -> Box<dyn Any> {
        Box::new((self.init)())
    }
}

#[doc(hidden)]
pub mod helpers {
    use crate::{Key, KeyData, KeyDataValue, Schema};
    use once_cell::sync::Lazy;
    use std::{
        marker::PhantomData,
        sync::atomic::{AtomicUsize, Ordering},
    };

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

    pub struct RawKey<S: Schema, T: ?Sized + 'static, const MUT: bool = false> {
        pub(crate) schema: PhantomData<S>,
        pub(crate) index: usize,
        pub(crate) data: Option<Box<dyn KeyData<T>>>,
    }
    pub type RawKeyMut<S, T> = RawKey<S, T, true>;

    impl<S: Schema, T: ?Sized + 'static, const MUT: bool> RawKey<S, T, MUT> {
        fn new(data: Option<Box<dyn KeyData<T>>>) -> Self {
            Self {
                schema: PhantomData,
                index: S::data().push_key(),
                data,
            }
        }
    }
    impl<S: Schema, T: ?Sized + 'static> RawKey<S, T, false> {
        pub fn new_with_default<Init, ToRef, V>(init: Init, to_ref: ToRef) -> Self
        where
            Init: Send + Sync + Fn() -> V + 'static,
            ToRef: Send + Sync + Fn(&V) -> &T + 'static,
            V: 'static,
        {
            fn to_mut_unreachable<V, T: ?Sized>(_: &mut V) -> &mut T {
                unreachable!()
            }
            Self::new(Some(Box::new(KeyDataValue {
                init,
                to_ref,
                to_mut: to_mut_unreachable,
            })))
        }
    }
    impl<S: Schema, T: ?Sized + 'static> RawKey<S, T, true> {
        pub fn new_with_default_mut<Init, ToRef, ToMut, V>(
            init: Init,
            to_ref: ToRef,
            to_mut: ToMut,
        ) -> Self
        where
            Init: Send + Sync + Fn() -> V + 'static,
            ToRef: Send + Sync + Fn(&V) -> &T + 'static,
            ToMut: Send + Sync + Fn(&mut V) -> &mut T + 'static,
            V: 'static,
        {
            Self::new(Some(Box::new(KeyDataValue {
                init,
                to_ref,
                to_mut,
            })))
        }
    }

    pub const fn new_key_without_default<S: Schema, T: ?Sized + 'static, const MUT: bool>(
    ) -> Key<S, T, MUT> {
        Key(Lazy::new(|| RawKey::new(None)))
    }
    pub const fn new_key<S: Schema, T: ?Sized + 'static, const MUT: bool>(
        f: fn() -> RawKey<S, T, MUT>,
    ) -> Key<S, T, MUT> {
        Key(Lazy::new(f))
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
/// You can specify mutability.
///
/// Keys with `mut` can be used in [`with_mut`](CtxMap::with_mut), [`get_mut`](CtxMap::get_mut) and [`index_mut`](CtxMap::index_mut).
///
/// ```
/// ctxmap::schema!(S);
/// ctxmap::key!(S {
///     mut KEY_1: u8,
///     mut KEY_2: String,
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
        $vis static $id: $crate::Key<$schema, $type> = $crate::helpers::new_key_without_default();
    };
    ($schema:ty { $vis:vis mut $id:ident: $type:ty }) => {
        $vis static $id: $crate::KeyMut<$schema, $type> = $crate::helpers::new_key_without_default();
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty = $init:expr }) => {
        $vis static $id: $crate::Key<$schema, $type> =
            $crate::helpers::new_key(|| $crate::helpers::RawKey::<_, $type>::new_with_default(
                || $init,
                |x| x));
    };
    ($schema:ty { $vis:vis mut $id:ident: $type:ty = $init:expr }) => {
        $vis static $id: $crate::KeyMut<$schema, $type> =
            $crate::helpers::new_key(|| $crate::helpers::RawKeyMut::<_, $type>::new_with_default_mut(
                || $init,
                |x| x,
                |x| x));
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty $(= $init:expr)?, $($tt:tt)* }) => {
        $crate::key!($schema { $vis $id: $type $(= $init)? });
        $crate::key!($schema { $($tt)* });
    };
    ($schema:ty { $vis:vis mut $id:ident: $type:ty $(= $init:expr)?, $($tt:tt)* }) => {
        $crate::key!($schema { $vis mut $id: $type $(= $init)? });
        $crate::key!($schema { $($tt)* });
    };

}
