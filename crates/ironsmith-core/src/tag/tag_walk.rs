//! Enumerating and rewriting the reference keys a value carries.
//!
//! `TagKeyWalk` is the traversal the reference resolver uses to find every
//! `TagKey` in a parsed line without a hand-written visitor arm per field:
//! model types derive it (`#[derive(TagKeyWalk)]`, from
//! `ironsmith-tag-walk-derive`), containers delegate to their elements, and
//! leaves that carry no keys do nothing.

use super::TagKey;

/// Visit or rewrite every reference key inside a value.
pub trait TagKeyWalk {
    /// Calls `f` on every reference key inside `self`, in field order.
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey));
    /// Calls `f` on every reference key inside `self`, letting it rewrite the
    /// key in place. Keys used as map keys are visited but not rewritten.
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey));
}

impl TagKeyWalk for TagKey {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        f(self);
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        f(self);
    }
}

/// Types that carry no reference keys.
#[macro_export]
macro_rules! tag_key_leaves {
    ($($ty:ty),* $(,)?) => {
        $(
            impl $crate::tag::TagKeyWalk for $ty {
                fn for_each_tag_key(&self, _f: &mut dyn FnMut(&$crate::tag::TagKey)) {}
                fn map_tag_keys(&mut self, _f: &mut dyn FnMut(&mut $crate::tag::TagKey)) {}
            }
        )*
    };
}

tag_key_leaves!(
    (), bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64,
    str, String, std::time::Duration
);

impl<T: TagKeyWalk> TagKeyWalk for Option<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        if let Some(value) = self {
            value.for_each_tag_key(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        if let Some(value) = self {
            value.map_tag_keys(f);
        }
    }
}

impl<T: TagKeyWalk> TagKeyWalk for Vec<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self {
            value.map_tag_keys(f);
        }
    }
}

impl<T: TagKeyWalk> TagKeyWalk for [T] {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self {
            value.map_tag_keys(f);
        }
    }
}

impl<T: TagKeyWalk, const N: usize> TagKeyWalk for [T; N] {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self {
            value.map_tag_keys(f);
        }
    }
}

impl<T: TagKeyWalk + ?Sized> TagKeyWalk for &T {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        (**self).for_each_tag_key(f);
    }
    /// A shared reference cannot be rewritten through; its keys stay as they are.
    fn map_tag_keys(&mut self, _f: &mut dyn FnMut(&mut TagKey)) {}
}

impl<T: TagKeyWalk + ?Sized> TagKeyWalk for Box<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        (**self).for_each_tag_key(f);
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        (**self).map_tag_keys(f);
    }
}

impl<T: TagKeyWalk + Clone> TagKeyWalk for std::rc::Rc<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        (**self).for_each_tag_key(f);
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        std::rc::Rc::make_mut(self).map_tag_keys(f);
    }
}

impl<T: TagKeyWalk + Clone> TagKeyWalk for std::sync::Arc<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        (**self).for_each_tag_key(f);
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        std::sync::Arc::make_mut(self).map_tag_keys(f);
    }
}

macro_rules! tag_key_tuples {
    ($(($($name:ident $index:tt),+))+) => {
        $(
            impl<$($name: TagKeyWalk),+> TagKeyWalk for ($($name,)+) {
                fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
                    $(self.$index.for_each_tag_key(f);)+
                }
                fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
                    $(self.$index.map_tag_keys(f);)+
                }
            }
        )+
    };
}

tag_key_tuples!((A 0, B 1) (A 0, B 1, C 2) (A 0, B 1, C 2, D 3));

impl<K: TagKeyWalk, V: TagKeyWalk, S> TagKeyWalk for std::collections::HashMap<K, V, S> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for (key, value) in self {
            key.for_each_tag_key(f);
            value.for_each_tag_key(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self.values_mut() {
            value.map_tag_keys(f);
        }
    }
}

impl<K: TagKeyWalk, V: TagKeyWalk> TagKeyWalk for std::collections::BTreeMap<K, V> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for (key, value) in self {
            key.for_each_tag_key(f);
            value.for_each_tag_key(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self.values_mut() {
            value.map_tag_keys(f);
        }
    }
}

impl<T: TagKeyWalk, S> TagKeyWalk for std::collections::HashSet<T, S> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn map_tag_keys(&mut self, _f: &mut dyn FnMut(&mut TagKey)) {}
}

impl<T: TagKeyWalk> TagKeyWalk for std::collections::BTreeSet<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn map_tag_keys(&mut self, _f: &mut dyn FnMut(&mut TagKey)) {}
}

/// The keys a value carries, in field order, duplicates kept.
pub fn tag_keys_of<T: TagKeyWalk + ?Sized>(value: &T) -> Vec<TagKey> {
    let mut keys = Vec::new();
    value.for_each_tag_key(&mut |key| keys.push(key.clone()));
    keys
}
