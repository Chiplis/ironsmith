use std::hash::{Hash, Hasher};

/// Hash a `Debug` representation directly into a hasher without allocating.
pub(crate) fn hash_debug(
    hasher: &mut impl Hasher,
    value: &impl std::fmt::Debug,
) -> std::fmt::Result {
    struct DebugHasher<'a, H> {
        hasher: &'a mut H,
    }

    impl<H: Hasher> std::fmt::Write for DebugHasher<'_, H> {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            s.hash(self.hasher);
            Ok(())
        }
    }

    let mut sink = DebugHasher { hasher };
    std::fmt::write(&mut sink, format_args!("{value:?}"))
}
