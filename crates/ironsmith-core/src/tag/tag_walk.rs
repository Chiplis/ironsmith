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
    /// Native typed execution references. The callback may handle a complete
    /// node (true) or let a derived model traverse its typed fields (false).
    /// Strings, labels, retained encodings and numeric values are never nodes.
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        self.map_tag_keys(&mut |tag| { f(tag); });
    }

    /// Visits typed object references, including references without a tag key.
    /// Leaves without object references use the default empty traversal.
    fn for_each_object_ref(&self, _f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {}
    fn for_each_subtype(&self, _f: &mut dyn FnMut(&crate::Subtype)) {}

    /// Calls `f` on every reference key inside `self`, in field order.
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey));
    /// Calls `f` on every reference key inside `self`, letting it rewrite the
    /// key in place. Keys used as map keys are visited but not rewritten.
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey));

    /// Calls `f` on every player reference inside `self` (each value of a
    /// type marked `#[tag_walk(player_ref)]`, such as `PlayerFilter` or the
    /// compiler's `PlayerAst`), passed as `&mut dyn Any` so callers downcast
    /// the kinds they rewrite. When `f` returns `true` the reference was
    /// handled and its own fields are not visited. Leaves carry none.
    fn map_player_refs(&mut self, _f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {}
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
    (),
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    str,
    String,
    std::time::Duration
);

impl<T: TagKeyWalk> TagKeyWalk for Option<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        if let Some(value) = self {
            value.for_each_tag_key(f);
        }
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        if let Some(value) = self {
            value.for_each_object_ref(f);
        }
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        if let Some(value) = self {
            value.for_each_subtype(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        if let Some(value) = self {
            value.map_tag_keys(f);
        }
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        if let Some(value) = self {
            value.map_player_refs(f);
        }
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        if let Some(value) = self {
            value.map_execution_nodes(f);
        }
    }
}

impl<T: TagKeyWalk> TagKeyWalk for Vec<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        for value in self {
            value.for_each_object_ref(f);
        }
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        for value in self {
            value.for_each_subtype(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self {
            value.map_tag_keys(f);
        }
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for value in self {
            value.map_player_refs(f);
        }
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for value in self {
            value.map_execution_nodes(f);
        }
    }
}

impl<T: TagKeyWalk> TagKeyWalk for [T] {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        for value in self {
            value.for_each_object_ref(f);
        }
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        for value in self {
            value.for_each_subtype(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self {
            value.map_tag_keys(f);
        }
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for value in self {
            value.map_player_refs(f);
        }
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for value in self {
            value.map_execution_nodes(f);
        }
    }
}

impl<T: TagKeyWalk, const N: usize> TagKeyWalk for [T; N] {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        for value in self {
            value.for_each_object_ref(f);
        }
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        for value in self {
            value.for_each_subtype(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self {
            value.map_tag_keys(f);
        }
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for value in self {
            value.map_player_refs(f);
        }
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for value in self {
            value.map_execution_nodes(f);
        }
    }
}

impl<T: TagKeyWalk + ?Sized> TagKeyWalk for &T {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        (**self).for_each_tag_key(f);
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        (**self).for_each_object_ref(f);
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        (**self).for_each_subtype(f);
    }
    /// A shared reference cannot be rewritten through; its keys stay as they are.
    fn map_tag_keys(&mut self, _f: &mut dyn FnMut(&mut TagKey)) {}
}

impl<T: TagKeyWalk + ?Sized> TagKeyWalk for Box<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        (**self).for_each_tag_key(f);
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        (**self).for_each_object_ref(f);
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        (**self).for_each_subtype(f);
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        (**self).map_tag_keys(f);
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        (**self).map_player_refs(f);
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        (**self).map_execution_nodes(f);
    }
}

impl<T: TagKeyWalk + Clone> TagKeyWalk for std::rc::Rc<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        (**self).for_each_tag_key(f);
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        (**self).for_each_object_ref(f);
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        (**self).for_each_subtype(f);
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        std::rc::Rc::make_mut(self).map_tag_keys(f);
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        std::rc::Rc::make_mut(self).map_player_refs(f);
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        std::rc::Rc::make_mut(self).map_execution_nodes(f);
    }
}

impl<T: TagKeyWalk + Clone> TagKeyWalk for std::sync::Arc<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        (**self).for_each_tag_key(f);
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        (**self).for_each_object_ref(f);
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        (**self).for_each_subtype(f);
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        std::sync::Arc::make_mut(self).map_tag_keys(f);
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        std::sync::Arc::make_mut(self).map_player_refs(f);
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        std::sync::Arc::make_mut(self).map_execution_nodes(f);
    }
}

macro_rules! tag_key_tuples {
    ($(($($name:ident $index:tt),+))+) => {
        $(
            impl<$($name: TagKeyWalk),+> TagKeyWalk for ($($name,)+) {
                fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
                    $(self.$index.for_each_tag_key(f);)+
                }
                fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
                    $(self.$index.for_each_object_ref(f);)+
                }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
                    $(self.$index.for_each_subtype(f);)+
                }
                fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
                    $(self.$index.map_tag_keys(f);)+
                }
                fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
                    $(self.$index.map_player_refs(f);)+
                }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
                    $(self.$index.map_execution_nodes(f);)+
                }
            }
        )+
    };
}

tag_key_tuples!((A 0, B 1) (A 0, B 1, C 2) (A 0, B 1, C 2, D 3));

impl<K: TagKeyWalk + Eq + std::hash::Hash, V: TagKeyWalk, S: std::hash::BuildHasher> TagKeyWalk for std::collections::HashMap<K, V, S> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for (key, value) in self {
            key.for_each_tag_key(f);
            value.for_each_tag_key(f);
        }
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        for (key, value) in self {
            key.for_each_object_ref(f);
            value.for_each_object_ref(f);
        }
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        for (key, value) in self {
            key.for_each_subtype(f);
            value.for_each_subtype(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self.values_mut() {
            value.map_tag_keys(f);
        }
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for value in self.values_mut() {
            value.map_player_refs(f);
        }
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        let entries = self.drain().collect::<Vec<_>>();
        for (mut key, mut value) in entries {
            key.map_execution_nodes(f); value.map_execution_nodes(f); self.insert(key,value);
        }
    }
}

impl<K: TagKeyWalk + Ord, V: TagKeyWalk> TagKeyWalk for std::collections::BTreeMap<K, V> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for (key, value) in self {
            key.for_each_tag_key(f);
            value.for_each_tag_key(f);
        }
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        for (key, value) in self {
            key.for_each_object_ref(f);
            value.for_each_object_ref(f);
        }
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        for (key, value) in self {
            key.for_each_subtype(f);
            value.for_each_subtype(f);
        }
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        for value in self.values_mut() {
            value.map_tag_keys(f);
        }
    }
    fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for value in self.values_mut() {
            value.map_player_refs(f);
        }
    }
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for (mut key, mut value) in std::mem::take(self) {
            key.map_execution_nodes(f); value.map_execution_nodes(f); self.insert(key,value);
        }
    }
}

impl<T: TagKeyWalk + Eq + std::hash::Hash, S: std::hash::BuildHasher> TagKeyWalk for std::collections::HashSet<T, S> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        for value in self {
            value.for_each_object_ref(f);
        }
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        for value in self {
            value.for_each_subtype(f);
        }
    }
    fn map_tag_keys(&mut self, _f: &mut dyn FnMut(&mut TagKey)) {}
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        let values=self.drain().collect::<Vec<_>>();
        for mut value in values {value.map_execution_nodes(f);self.insert(value);}
    }

}

impl<T: TagKeyWalk + Ord> TagKeyWalk for std::collections::BTreeSet<T> {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        for value in self {
            value.for_each_tag_key(f);
        }
    }
    fn for_each_object_ref(&self, f: &mut dyn FnMut(&crate::filter_model::ObjectRef)) {
        for value in self {
            value.for_each_object_ref(f);
        }
    }
    fn for_each_subtype(&self, f: &mut dyn FnMut(&crate::Subtype)) {
        for value in self {
            value.for_each_subtype(f);
        }
    }
    fn map_tag_keys(&mut self, _f: &mut dyn FnMut(&mut TagKey)) {}
    fn map_execution_nodes(&mut self, f: &mut dyn FnMut(&mut dyn std::any::Any) -> bool) {
        for mut value in std::mem::take(self) {value.map_execution_nodes(f);self.insert(value);}
    }

}

/// The keys a value carries, in field order, duplicates kept.
pub fn tag_keys_of<T: TagKeyWalk + ?Sized>(value: &T) -> Vec<TagKey> {
    let mut keys = Vec::new();
    value.for_each_tag_key(&mut |key| keys.push(key.clone()));
    keys
}

#[cfg(test)]
mod typed_object_reference_walk_tests {
    use super::*;
    use crate::ObjectId;
    use crate::filter_model::{ObjectFilter, ObjectRef, PlayerFilter};

    #[test]
    fn nested_player_predicate_keeps_specific_reference_without_a_tag() {
        let reference = ObjectRef::Specific(ObjectId(73));
        let mut child = ObjectFilter::default();
        child.controller = Some(PlayerFilter::OwnerOf(reference.clone()));
        let mut filter = ObjectFilter::default();
        filter.any_of.push(child);
        assert!(tag_keys_of(&filter).is_empty());
        let mut refs = Vec::new();
        filter.for_each_object_ref(&mut |value| refs.push(value.clone()));
        assert_eq!(refs, vec![reference]);
    }

    #[test]
    fn containers_visit_keys_values_and_independent_equal_references() {
        let specific = ObjectRef::Specific(ObjectId(91));
        let values = std::collections::HashMap::from([(
            specific.clone(),
            Some(Box::new(vec![specific.clone(), ObjectRef::Target])),
        )]);
        let wrapped = (
            std::rc::Rc::new(values),
            std::sync::Arc::new([ObjectRef::FilterCandidate]),
        );
        let mut refs = Vec::new();
        wrapped.for_each_object_ref(&mut |value| refs.push(value.clone()));
        assert_eq!(
            refs,
            vec![
                specific.clone(),
                specific,
                ObjectRef::Target,
                ObjectRef::FilterCandidate
            ]
        );
    }
}

#[cfg(test)]
mod typed_execution_node_tests {
    use super::*;
    use crate::EffectId;
    use std::collections::{HashMap,HashSet,BTreeMap,BTreeSet};
    #[derive(crate::tag::TagKeyWalk)]
    struct References {
        ids:HashSet<EffectId>, tags:BTreeSet<TagKey>,
        by_id:HashMap<EffectId,TagKey>, by_tag:BTreeMap<TagKey,EffectId>,
        number:u32, text:String,
    }
    #[test]
    fn typed_execution_nodes_rekey_all_containers_without_rewriting_strings_or_numbers() {
        let mut model=References {
            ids:[EffectId(7)].into_iter().collect(),tags:[TagKey::from("local")].into_iter().collect(),
            by_id:[(EffectId(7),TagKey::from("local"))].into_iter().collect(),
            by_tag:[(TagKey::from("local"),EffectId(7))].into_iter().collect(),
            number:7,text:"local EffectId(7)".into(),
        };
        model.map_execution_nodes(&mut |node| {
            if let Some(id)=node.downcast_mut::<EffectId>() {id.0+=10;return true;}
            if let Some(tag)=node.downcast_mut::<TagKey>() {*tag="scoped".into();return true;}
            false
        });
        assert!(model.ids.contains(&EffectId(17)));assert!(model.tags.contains(&TagKey::from("scoped")));
        assert_eq!(model.by_id.get(&EffectId(17)),Some(&TagKey::from("scoped")));
        assert_eq!(model.by_tag.get(&TagKey::from("scoped")),Some(&EffectId(17)));
        assert_eq!(model.number,7);assert_eq!(model.text,"local EffectId(7)");
    }
}
