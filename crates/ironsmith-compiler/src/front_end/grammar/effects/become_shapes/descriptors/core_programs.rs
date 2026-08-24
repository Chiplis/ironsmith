use super::*;

pub(super) fn creature_subtypes_only(subtypes: &[Subtype]) -> bool {
    let creature_types = Subtype::all_creature_types();
    subtypes
        .iter()
        .all(|subtype| creature_types.iter().any(|candidate| candidate == subtype))
}
