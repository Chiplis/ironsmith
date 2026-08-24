use super::*;

pub(super) fn try_apply_no_shared_creature_type_with_your_creatures_or_graveyard_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in NO_SHARED_CREATURE_TYPE_WITH_YOUR_CREATURES_OR_GRAVEYARD_CLAUSES {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;

        filter
            .no_shared_creature_types_with
            .push(ObjectFilter::creature().you_control());
        filter.no_shared_creature_types_with.push(
            ObjectFilter::default()
                .with_type(CardType::Creature)
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}
