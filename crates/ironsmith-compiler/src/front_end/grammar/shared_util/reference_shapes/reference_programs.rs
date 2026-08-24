use super::*;

pub(super) fn filter_keyword_constraint_for_words(
    words: &[&str],
) -> Option<FilterKeywordConstraint> {
    use FilterKeywordConstraint::{Marker, Static};
    let static_id = if permission_shapes::exact_words(words, &["flying"]) {
        Some(StaticAbilityId::Flying)
    } else if permission_shapes::exact_words(words, &["menace"]) {
        Some(StaticAbilityId::Menace)
    } else if permission_shapes::exact_words(words, &["hexproof"]) {
        Some(StaticAbilityId::Hexproof)
    } else if permission_shapes::exact_words(words, &["haste"]) {
        Some(StaticAbilityId::Haste)
    } else if permission_shapes::exact_words(words, &["first", "strike"]) {
        Some(StaticAbilityId::FirstStrike)
    } else if permission_shapes::exact_words(words, &["double", "strike"]) {
        Some(StaticAbilityId::DoubleStrike)
    } else if permission_shapes::exact_words(words, &["deathtouch"]) {
        Some(StaticAbilityId::Deathtouch)
    } else if permission_shapes::exact_words(words, &["lifelink"]) {
        Some(StaticAbilityId::Lifelink)
    } else if permission_shapes::exact_words(words, &["vigilance"]) {
        Some(StaticAbilityId::Vigilance)
    } else if permission_shapes::exact_words(words, &["trample"]) {
        Some(StaticAbilityId::Trample)
    } else if permission_shapes::exact_words(words, &["reach"]) {
        Some(StaticAbilityId::Reach)
    } else if permission_shapes::exact_words(words, &["defender"]) {
        Some(StaticAbilityId::Defender)
    } else if permission_shapes::exact_words(words, &["flash"]) {
        Some(StaticAbilityId::Flash)
    } else if permission_shapes::exact_words(words, &["phasing"]) {
        Some(StaticAbilityId::Phasing)
    } else if permission_shapes::exact_words(words, &["indestructible"]) {
        Some(StaticAbilityId::Indestructible)
    } else if permission_shapes::exact_words(words, &["shroud"]) {
        Some(StaticAbilityId::Shroud)
    } else if permission_shapes::exact_words(words, &["wither"]) {
        Some(StaticAbilityId::Wither)
    } else if permission_shapes::exact_words(words, &["infect"]) {
        Some(StaticAbilityId::Infect)
    } else if permission_shapes::exact_words(words, &["fear"]) {
        Some(StaticAbilityId::Fear)
    } else if permission_shapes::exact_words(words, &["intimidate"]) {
        Some(StaticAbilityId::Intimidate)
    } else if permission_shapes::exact_words(words, &["shadow"]) {
        Some(StaticAbilityId::Shadow)
    } else if permission_shapes::exact_words(words, &["horsemanship"]) {
        Some(StaticAbilityId::Horsemanship)
    } else if permission_shapes::exact_words(words, &["flanking"]) {
        Some(StaticAbilityId::Flanking)
    } else if permission_shapes::exact_words(words, &["skulk"]) {
        Some(StaticAbilityId::Skulk)
    } else if permission_shapes::exact_words(words, &["protection"])
        || permission_shapes::exact_words(words, &["protection", "from", "any", "color"])
    {
        Some(StaticAbilityId::Protection)
    } else if permission_shapes::exact_words(words, &["changeling"]) {
        Some(StaticAbilityId::Changeling)
    } else if permission_shapes::exact_words(words, &["cascade"]) {
        Some(StaticAbilityId::Cascade)
    } else if permission_shapes::exact_words(words, &["convoke"]) {
        Some(StaticAbilityId::Convoke)
    } else if exact_one_of(
        words,
        &[
            &["landwalk"],
            &["nonbasic", "landwalk"],
            &["artifact", "landwalk"],
        ],
    ) {
        Some(StaticAbilityId::Landwalk)
    } else {
        None
    };
    if let Some(id) = static_id {
        return Some(Static(id));
    }
    if permission_shapes::exact_words(words, &["decayed"]) {
        Some(Marker("decayed"))
    } else if permission_shapes::exact_words(words, &["fading"]) {
        Some(Marker("fading"))
    } else if permission_shapes::exact_words(words, &["unearth"]) {
        Some(Marker("unearth"))
    } else if permission_shapes::exact_words(words, &["freerunning"]) {
        Some(Marker("freerunning"))
    } else if permission_shapes::exact_words(words, &["level", "up"]) {
        Some(Marker("level up"))
    } else if permission_shapes::exact_words(words, &["disturb"]) {
        Some(Marker("disturb"))
    } else if permission_shapes::exact_words(words, &["mutate"]) {
        // Costed keyword markers retain their full printed surface (for
        // example, `Mutate {4}{B}`), while ObjectFilter marker matching is
        // deliberately word-aware.  Keep the semantic marker cost-agnostic.
        Some(Marker("mutate"))
    } else if permission_shapes::exact_words(words, &["toxic"]) {
        Some(Marker("toxic"))
    } else if permission_shapes::exact_words(words, &["doctor's", "companion"])
        || permission_shapes::exact_words(words, &["doctors", "companion"])
    {
        Some(Marker("doctor's companion"))
    } else if permission_shapes::exact_words(words, &["islandwalk"]) {
        Some(Marker("islandwalk"))
    } else if permission_shapes::exact_words(words, &["swampwalk"]) {
        Some(Marker("swampwalk"))
    } else if permission_shapes::exact_words(words, &["mountainwalk"]) {
        Some(Marker("mountainwalk"))
    } else if permission_shapes::exact_words(words, &["forestwalk"]) {
        Some(Marker("forestwalk"))
    } else if permission_shapes::exact_words(words, &["plainswalk"]) {
        Some(Marker("plainswalk"))
    } else {
        None
    }
}

pub(super) fn player_base(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    if prefix_one_of(words, &[&["opponent"], &["opponents"]]) {
        Some((PlayerFilter::Opponent, 1))
    } else if prefix_one_of(words, &[&["player"], &["players"]]) {
        Some((PlayerFilter::Any, 1))
    } else {
        None
    }
}

pub(super) fn is_that_player_or_object_controller(words: &[&str]) -> bool {
    words.len() >= 6
        && permission_shapes::prefix_words(words, &["that", "player", "or", "that"])
        && is_controlled_object_plural(words[4])
        && permission_shapes::exact_words(&words[5..6], &["controller"])
}

pub(super) fn is_controlled_object_plural(word: &str) -> bool {
    starts_with_one_of_words(
        &[word],
        0,
        &[
            "artifacts",
            "creatures",
            "enchantments",
            "lands",
            "permanents",
            "planeswalkers",
            "sources",
            "spells",
            "tokens",
        ],
    )
}
