use crate::schema::*;
use std::{any::Any, marker::PhantomData, mem::replace, ops::Index};

/// Safe, `HashMap<&CtxMapKey, *const dyn Any>` like collection.
pub struct CtxMap<S> {
    items: Vec<CtxMapItem>,
    _schema: PhantomData<S>,
}
struct CtxMapItem {
    default: Box<dyn Any>,
    value: Option<*const dyn Any>,
}

/// A key for [`CtxMap`].
#[derive(Eq, PartialEq, Hash)]
pub struct CtxMapKey<S, T: ?Sized + 'static> {
    index: usize,
    _value: PhantomData<fn() -> &'static T>,
    _schema: PhantomData<S>,
}

/// Available key collection for [`CtxMap`].
pub trait CtxMapSchema: Schema {}
impl<T: Schema> CtxMapSchema for T {}

impl<S: CtxMapSchema> CtxMap<S> {
    /// Create a new `CtxMap` with default values.
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
    pub fn with<T: ?Sized + 'static, U>(
        &mut self,
        key: &CtxMapKey<S, T>,
        value: &T,
        f: impl FnOnce(&mut Self) -> U,
    ) -> U {
        let value_new = &(value as *const T) as &dyn Any;
        let value_old = replace(&mut self.items[key.index].value, Some(value_new));
        let retval = f(self);
        self.items[key.index].value = value_old;
        retval
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
        <dyn Any>::downcast_ref::<Box<T>>(&*item.default).expect("type mismatch.")
    }
}

impl<S: CtxMapSchema> Default for CtxMap<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Define a type that implements [`CtxMapSchema`].
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
        $vis struct $id(fn());
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
/// # Example
///
/// ```
/// ctxmap::schema!(Schema1);
///
/// ctxmap::key!(Schema1 { KEY_A: u8 }); // default value will be `Default::default()`.
/// ctxmap::key!(Schema1 { KEY_B: u8 = 10 });
/// ctxmap::key!(Schema1 {
///     KEY_1: u8,
///     KEY_2: u16,
/// });
///
/// ctxmap::schema!(pub Schema2);
/// ctxmap::key!(Schema2 { KEY_X: u8 });
/// ctxmap::key!(Schema2 { pub KEY_Y: u8 });
/// ```
#[macro_export]
macro_rules! key {
    ($schema:ty { }) => { };
    ($schema:ty { $vis:vis $id:ident: $type:ty }) => {
        $crate::key!($schema { $vis $id: $type = std::default::Default::default() });
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty = $default:expr }) => {
        $vis static $id: $crate::schema::exports::once_cell::sync::Lazy<$crate::CtxMapKey<$schema, $type>> =
            $crate::schema::exports::once_cell::sync::Lazy::new(|| {
                $crate::schema::Schema::register(|| Box::<Box<$type>>::new(Box::new($default)))
            });
        $crate::schema::exports::inventory::submit! { $schema(|| { $crate::schema::exports::once_cell::sync::Lazy::force(&$id); })}
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty, $($tt:tt)* }) => {
        $crate::key!($schema { $vis $id: $type });
        $crate::key!($schema { $($tt)* });
    };
    ($schema:ty { $vis:vis $id:ident: $type:ty = $default:expr, $($tt:tt)* }) => {
        $crate::key!($schema { $vis $id: $type = $default });
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
        pub new_default: fn() -> Box<dyn Any>,
    }

    impl SchemaData {
        pub(crate) fn new_items(&self) -> Vec<CtxMapItem> {
            let keys = self.keys.0.read().unwrap();
            let mut items = Vec::with_capacity(keys.len());
            for key in keys.iter() {
                items.push(CtxMapItem {
                    default: (key.new_default)(),
                    value: None,
                });
            }
            items
        }
        fn register(&self, new_default: fn() -> Box<dyn Any>) -> usize {
            let mut keys = self.keys.0.write().unwrap();
            let index = keys.len();
            keys.push(Key { new_default });
            index
        }
    }
}
