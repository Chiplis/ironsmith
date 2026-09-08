use std::collections::HashMap;
use std::hash::Hash;

/// Two bounded generations avoid a cold-cache cliff at capacity. Hits promote
/// the previous generation; storage is bounded by twice the generation size.
#[derive(Debug)]
pub(super) struct BoundedCache<K, V, const CAPACITY: usize> {
    current: HashMap<K, V>,
    previous: HashMap<K, V>,
}
impl<K, V, const CAPACITY: usize> Default for BoundedCache<K, V, CAPACITY> {
    fn default() -> Self { Self { current: HashMap::new(), previous: HashMap::new() } }
}
impl<K: Eq + Hash + Clone, V, const CAPACITY: usize> BoundedCache<K, V, CAPACITY> {
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if !self.current.contains_key(key) {
            if let Some(value) = self.previous.remove(key) { self.insert(key.clone(), value); }
        }
        self.current.get(key)
    }
    pub fn insert(&mut self, key: K, value: V) {
        if self.current.len() >= CAPACITY && !self.current.contains_key(&key) {
            self.previous = std::mem::take(&mut self.current);
        }
        self.previous.remove(&key);
        self.current.insert(key, value);
    }
    #[cfg(test)]
    pub fn len(&self) -> usize { self.current.len() + self.previous.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_retains_hot_entries_and_bounds_storage() {
        let mut cache = BoundedCache::<u32, u32, 4>::default();
        for key in 0..4 { cache.insert(key, key); }
        cache.insert(4, 4);
        assert_eq!(cache.get(&0), Some(&0));
        for key in 5..40 {
            cache.insert(key, key);
            assert_eq!(cache.get(&0), Some(&0));
            assert!(cache.len() <= 8);
        }
        cache.insert(0, 100);
        assert_eq!(cache.get(&0), Some(&100));
    }
}
