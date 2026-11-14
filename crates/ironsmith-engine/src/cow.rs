//! Copy-on-write sharing for large game-state parts without interior
//! mutability: a clone shares the part; the first write through `DerefMut`
//! copies it (`Arc::make_mut`).
use std::sync::Arc;

#[derive(Default)]
pub struct CowArc<T: Clone>(Arc<T>);

impl<T: Clone> CowArc<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl<T: Clone> Clone for CowArc<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Clone> std::ops::Deref for CowArc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> std::ops::DerefMut for CowArc<T> {
    fn deref_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }
}

impl<T: Clone + std::fmt::Debug> std::fmt::Debug for CowArc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.as_ref().fmt(f)
    }
}
