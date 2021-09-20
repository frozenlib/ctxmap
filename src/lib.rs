/*!
A collection that can store references of different types and lifetimes.

# Example

```
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
```
*/
use crate::schema::*;
use std::{any::Any, borrow::Borrow, marker::PhantomData, mem::replace, ops::Index};

/// A collection that can store references of different types and lifetimes.
pub struct CtxMap<S> {
    items: Vec<CtxMapItem>,
    _schema: PhantomData<S>,
}
struct CtxMapItem {
    init: Box<dyn Any>,
    value: Option<*const dyn Any>,
}

/// A key for [`CtxMap`].
///
/// Use [`key`] macro to create `CtxMapKey`.
#[derive(Eq, PartialEq, Hash)]
pub struct CtxMapKey<S, T: ?Sized> {
    index: usize,
    _value: PhantomData<fn(&T) -> &T>,
    _schema: PhantomData<S>,
}

/// Available key collection for [`CtxMap`].
///
/// Use [`macro@schema`] macro to define a type that implement `CtxMapSchema`.
pub trait CtxMapSchema: Schema {}
impl<T: Schema> CtxMapSchema for T {}

impl<S: CtxMapSchema> CtxMap<S> {
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
    /// assert_eq!(m[&KEY_B], 0);
    /// ```
    pub fn new() -> Self {
        S::load_all();
        Self {
            items: S::data().new_items(),
            _schema: PhantomData,
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
        key: &CtxMapKey<S, T>,
        value: &T,
        f: impl FnOnce(&mut Self) -> U,
    ) -> U {
        struct Guard<'a, S> {
            m: &'a mut CtxMap<S>,
            index: usize,
            value_old: Option<*const dyn Any>,
        }
        impl<'a, S> Drop for Guard<'a, S> {
            fn drop(&mut self) {
                self.m.items[self.index].value = self.value_old;
            }
        }

        let index = key.index;
        let value_new = &(value as *const T) as &dyn Any;
        let value_old = replace(&mut self.items[index].value, Some(value_new));
        let g = Guard {
            m: self,
            value_old,
            index,
        };
        f(g.m)
    }
}
impl<S: CtxMapSchema, T: ?Sized + 'static> Index<&CtxMapKey<S, T>> for CtxMap<S> {
    type Output = T;

    fn index(&self, index: &CtxMapKey<S, T>) -> &Self::Output {
        let item = &self.items[index.index];
        if let Some(value) = &item.value {
            let value: &dyn Any = unsafe { &**value };
            let p = <dyn Any>::downcast_ref::<*const T>(value).expect("type mismatch.");
            return unsafe { &**p };
        }
        if let Some(p) = <dyn Any>::downcast_ref::<Box<T>>(&*item.init) {
            p
        } else if let Some(p) = <dyn Any>::downcast_ref::<Box<dyn Borrow<T>>>(&*item.init) {
            (**p).borrow()
        } else {
            unreachable!("type mismatch.")
        }
    }
}

impl<S: CtxMapSchema> Default for CtxMap<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Define a type that implements [`CtxMapSchema`].
///
/// The `schema!` macro defines a global item and does not run anything.
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
        $vis struct $id(pub fn());
        impl $crate::schema::Schema for $id {
            fn data() -> &'static $crate::schema::SchemaData {
                static KEYS: $crate::schema::SchemaData = $crate::schema::SchemaData {
                    keys: $crate::schema::exports::once_cell::sync::Lazy::new(
                        std::default::Default::default,
                    ),
                    load: std::sync::Once::new(),
                };
                &KEYS
            }
            fn load(&self) {
                (self.0)();
            }
        }
        $crate::schema::exports::inventory::collect!($id);
    };
}

/// Define a key for [`CtxMap`].
///
/// The `key!` macro defines a global item and does not run anything.
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
///     ref KEY_4: str = "abc",
/// });
///
/// ctxmap::schema!(Schema2);
/// ctxmap::key!(Schema2 { KEY_X: u8 = 10 });
/// ```
///
/// If the initial value is omitted, the initial value will be [`Default::default()`].
///
/// ```
/// ctxmap::schema!(S);
/// ctxmap::key!(S { KEY_A: u8 });
/// ctxmap::key!(S { KEY_B: u8 = 50});
///
/// let m = ctxmap::CtxMap::new();
/// assert_eq!(m[&KEY_A], Default::default());
/// assert_eq!(m[&KEY_B], 50);
/// ```
///
/// If you write `ref` in front of a variable name, you can set the value that implements [`std::borrow::Borrow<T>`] as the initial value.
///
/// ```
/// ctxmap::schema!(S);
/// ctxmap::key!(S { ref KEY_A: str = "abc" });
/// ctxmap::key!(S { ref KEY_B: str = format!("abc-{}", 1) });
///
/// let m = ctxmap::CtxMap::new();
/// assert_eq!(&m[&KEY_A], "abc");
/// assert_eq!(&m[&KEY_B], "abc-1");
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
        $crate::key!($schema { $vis $id: $type = std::default::Default::default() });
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty = $init:expr }) => {
        $vis static $id: $crate::schema::exports::once_cell::sync::Lazy<$crate::CtxMapKey<$schema, $type>> =
            $crate::schema::exports::once_cell::sync::Lazy::new(|| {
                $crate::schema::Schema::register(|| Box::<Box<$type>>::new(Box::new($init)))
            });
        #[allow(non_camel_case_types)]
        struct $id {
            dummy: ()
        }
        impl $id {
            fn _dummy() {
                use $crate::schema::exports::inventory;
                $crate::schema::exports::inventory::submit! { $schema(|| { $crate::schema::exports::once_cell::sync::Lazy::force(&$id); })}
            }
        }
        };
    ($schema:ty { $vis:vis ref $id:ident: $type:ty = $init:expr }) => {
        $vis static $id: $crate::schema::exports::once_cell::sync::Lazy<$crate::CtxMapKey<$schema, $type>> =
            $crate::schema::exports::once_cell::sync::Lazy::new(|| {
                $crate::schema::Schema::register(|| Box::<Box<std::borrow::Borrow<$type>>>::new(Box::new($init)))
            });
        #[allow(non_camel_case_types)]
        struct $id {
            dummy: ()
        }
        impl $id {
            fn _dummy() {
                use $crate::schema::exports::inventory;
                $crate::schema::exports::inventory::submit! { $schema(|| { $crate::schema::exports::once_cell::sync::Lazy::force(&$id); })}
            }
        }
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty, $($tt:tt)* }) => {
        $crate::key!($schema { $vis $id: $type });
        $crate::key!($schema { $($tt)* });
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty = $init:expr, $($tt:tt)* }) => {
        $crate::key!($schema { $vis $id: $type = $init });
        $crate::key!($schema { $($tt)* });
    };
    ($schema:ty { $vis:vis ref $id:ident: $type:ty = $init:expr, $($tt:tt)* }) => {
        $crate::key!($schema { $vis ref $id: $type = $init });
        $crate::key!($schema { $($tt)* });
    };
}

#[doc(hidden)]
pub mod schema {
    use crate::{CtxMapItem, CtxMapKey};
    use once_cell::sync::Lazy;
    use std::{
        any::Any,
        marker::PhantomData,
        sync::{Once, RwLock},
    };
    pub mod exports {
        pub use inventory;
        pub use once_cell;
    }

    pub trait Schema: inventory::Collect + Sized {
        fn load(&self);
        fn data() -> &'static SchemaData;

        fn register<T: ?Sized>(new_default: fn() -> Box<dyn Any>) -> CtxMapKey<Self, T> {
            CtxMapKey {
                index: Self::data().register(new_default),
                _value: PhantomData,
                _schema: PhantomData,
            }
        }
        fn load_all() {
            Self::data().load.call_once(|| {
                for s in inventory::iter::<Self> {
                    s.load();
                }
            });
        }
    }
    pub struct SchemaData {
        pub keys: Lazy<Keys>,
        pub load: Once,
    }

    #[derive(Default)]
    pub struct Keys(RwLock<Vec<Key>>);
    pub(crate) struct Key {
        pub init: fn() -> Box<dyn Any>,
    }

    impl SchemaData {
        pub(crate) fn new_items(&self) -> Vec<CtxMapItem> {
            let keys = self.keys.0.read().unwrap();
            let mut items = Vec::with_capacity(keys.len());
            for key in keys.iter() {
                items.push(CtxMapItem {
                    init: (key.init)(),
                    value: None,
                });
            }
            items
        }
        fn register(&self, init: fn() -> Box<dyn Any>) -> usize {
            let mut keys = self.keys.0.write().unwrap();
            let index = keys.len();
            keys.push(Key { init });
            index
        }
    }
}
