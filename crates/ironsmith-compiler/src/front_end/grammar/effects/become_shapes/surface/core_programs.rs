use super::*;

pub(super) fn basic_land_type(words: &[&str]) -> Option<Subtype> {
    let [word] = words else {
        return None;
    };
    let subtype = leaf::parse_leaf_subtype_flexible_complete(word).ok()?;
    matches!(
        subtype,
        Subtype::Plains | Subtype::Island | Subtype::Swamp | Subtype::Mountain | Subtype::Forest
    )
    .then_some(subtype)
}
