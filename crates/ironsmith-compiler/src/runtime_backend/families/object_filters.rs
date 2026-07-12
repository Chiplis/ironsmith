use crate::cards::builders::{CardTextError, IT_TAG};
#[cfg(test)]
use crate::{CardType, PlayerFilter, Subtype, Zone};
use crate::{ColorSet, ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

pub(crate) use super::grammar::filters::parse_simple_object_filter_words;
use super::grammar::filters::{
    apply_filter_tail_decoration, parse_filter_distinct_names_tokens, parse_filter_lexed_envelope,
    parse_filter_tail_decoration_split_words, parse_filter_tail_decoration_tokens,
    parse_filter_word_envelope, parse_simple_object_filter_lexed,
};
use super::grammar::primitives::split_lexed_slices_on_or;
use super::keyword_static::parse_pt_modifier;
use super::lexer::{OwnedLexToken, TokenWordView, parser_token_word_refs, token_slice_at_is};
use super::util::{is_demonstrative_object_head, is_source_reference_words, non_article_word_refs};
use crate::runtime_backend::grammar::shared_util::value_semantics::parse_filter_comparison_tokens;

#[cfg(test)]
const OBJECT_FILTER_ENCHANTED_TAG: &str = "enchanted";

fn object_filter_word_is_any(word: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| word == *candidate)
}

fn object_filter_word_is_other_or_another(word: &str) -> bool {
    object_filter_word_is_any(word, &["other", "another"])
}

pub(super) fn slice_has<T: PartialEq>(items: &[T], expected: &T) -> bool {
    crate::slice_primitives::contains(items, expected)
}

pub(super) fn set_has<T: Eq + std::hash::Hash>(
    items: &std::collections::HashSet<T>,
    expected: &T,
) -> bool {
    items.iter().any(|item| item == expected)
}

pub(super) fn push_unique<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    crate::slice_primitives::push_unique(items, value);
}

pub(super) fn parse_attached_reference_or_another_disjunction(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let segments = split_lexed_slices_on_or(tokens);
    if segments.len() != 2 {
        return Ok(None);
    }

    let first_word_view = TokenWordView::new(segments[0]);
    let first_words = non_article_word_refs(&first_word_view.to_word_refs());
    let second_word_view = TokenWordView::new(segments[1]);
    let second_words = non_article_word_refs(&second_word_view.to_word_refs());

    let first_is_attached_reference = first_words.first().is_some_and(|word| {
        object_filter_word_is_any(word, &["attached", "equipped", "enchanted"])
    });
    let second_starts_with_other = second_words
        .first()
        .is_some_and(|word| object_filter_word_is_other_or_another(word));
    if !first_is_attached_reference || !second_starts_with_other {
        return Ok(None);
    }

    let first_other = first_words
        .first()
        .is_some_and(|word| object_filter_word_is_other_or_another(word));
    let second_other = second_words
        .first()
        .is_some_and(|word| object_filter_word_is_other_or_another(word));

    let first_filter = parse_object_filter(segments[0], first_other)?;
    let second_filter = parse_object_filter(segments[1], second_other)?;

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = vec![first_filter, second_filter];
    Ok(Some(disjunction))
}

pub(crate) fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let envelope = parse_filter_distinct_names_tokens(tokens);
    let tokens = envelope.core_tokens.as_slice();
    let mut filter = if let Some(split) = parse_filter_tail_decoration_tokens(tokens) {
        let mut filter = super::grammar::filters::parse_object_filter_with_grammar_entrypoint(
            &split.base_tokens,
            other,
        )?;
        apply_filter_tail_decoration(&mut filter, split.decoration);
        filter
    } else {
        super::grammar::filters::parse_object_filter_with_grammar_entrypoint(tokens, other)?
    };
    filter = envelope.decorations.apply_distinct_names_only(filter);
    Ok(filter)
}

pub(crate) fn parse_object_filter_words(
    word_refs: &[&str],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let envelope = parse_filter_word_envelope(word_refs);
    if let Some(filter) = parse_simple_object_filter_words(&envelope.core_words, other) {
        return Ok(envelope.decorations.apply(filter));
    }
    if let Some(split) = parse_filter_tail_decoration_split_words(&envelope.core_words)
        && let Some(mut filter) = parse_simple_object_filter_words(split.base_words, other)
    {
        apply_filter_tail_decoration(&mut filter, split.decoration);
        return Ok(envelope.decorations.apply(filter));
    }

    // Preserve the legacy fallback contract: the lexed parser receives the
    // original words and owns envelope normalization for the complex path.
    let tokens = super::lexer::synthetic_word_tokens(word_refs.iter().copied());
    parse_object_filter_lexed(&tokens, other)
}

pub(crate) fn parse_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let envelope = parse_filter_lexed_envelope(tokens);
    if tokens_contain_permanent_or_suspended_card_disjunction(&envelope.core_tokens) {
        let filter = super::grammar::filters::parse_object_filter_with_grammar_entrypoint(
            &envelope.core_tokens,
            other,
        )?;
        return Ok(envelope.decorations.apply(filter));
    }
    if let Some(filter) = parse_simple_object_filter_lexed(&envelope.core_tokens, other) {
        return Ok(envelope.decorations.apply(filter));
    }
    let filter = parse_object_filter(&envelope.core_tokens, other)?;
    // Historical behavior intentionally drops the vote-winner tag on this
    // complex fallback while retaining the different-names fact.
    Ok(envelope.decorations.apply_distinct_names_only(filter))
}

fn tokens_contain_permanent_or_suspended_card_disjunction(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    words.iter().enumerate().any(|(idx, word)| {
        *word == "or"
            && words[..idx]
                .iter()
                .any(|word| matches!(*word, "permanent" | "permanents"))
            && matches!(
                words.get(idx + 1..idx + 3),
                Some(["suspended", "card"] | ["suspended", "cards"])
            )
    })
}

pub(crate) fn spell_filter_has_identity(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.colors.is_some()
        || filter.required_colors.is_some()
        || filter.sticker.is_some()
        || filter.color_count.is_some()
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.total_counters_parity.is_some()
        || filter.cast_by.is_some()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || filter.targets_only_player.is_some()
        || filter.targets_only_object.is_some()
        || filter.target_count.is_some()
        || filter.could_be_targeted_by.is_some()
        || filter.alternative_cast.is_some()
        || filter.shares_creature_type_with_source
        || !filter.tagged_constraints.is_empty()
        || !filter.any_of.is_empty()
}

pub(crate) fn merge_spell_filters(base: &mut ObjectFilter, extra: ObjectFilter) {
    for card_type in extra.card_types {
        push_unique(&mut base.card_types, card_type);
    }
    for card_type in extra.excluded_card_types {
        push_unique(&mut base.excluded_card_types, card_type);
    }
    for subtype in extra.subtypes {
        push_unique(&mut base.subtypes, subtype);
    }
    if let Some(colors) = extra.colors {
        let existing = base.colors.unwrap_or(ColorSet::new());
        base.colors = Some(existing.union(colors));
    }
    base.chosen_color |= extra.chosen_color;
    base.chosen_creature_type |= extra.chosen_creature_type;
    base.excluded_chosen_creature_type |= extra.excluded_chosen_creature_type;
    if base.color_count.is_none() {
        base.color_count = extra.color_count;
    }
    if base.alternative_cast.is_none() {
        base.alternative_cast = extra.alternative_cast;
    }
    if base.power.is_none() {
        base.power = extra.power;
    }
    if base.power_parity.is_none() {
        base.power_parity = extra.power_parity;
    }
    if base.toughness.is_none() {
        base.toughness = extra.toughness;
    }
    if base.mana_value.is_none() {
        base.mana_value = extra.mana_value;
    }
    if base.mana_value_parity.is_none() {
        base.mana_value_parity = extra.mana_value_parity;
    }
    if base.total_counters_parity.is_none() {
        base.total_counters_parity = extra.total_counters_parity;
    }
    if base.cast_by.is_none() {
        base.cast_by = extra.cast_by;
    }
    if base.targets_player.is_none() {
        base.targets_player = extra.targets_player;
    }
    if base.targets_object.is_none() {
        base.targets_object = extra.targets_object;
    }
    if base.targets_only_player.is_none() {
        base.targets_only_player = extra.targets_only_player;
    }
    if base.targets_only_object.is_none() {
        base.targets_only_object = extra.targets_only_object;
    }
    base.targets_only_any_of |= extra.targets_only_any_of;
    if base.target_count.is_none() {
        base.target_count = extra.target_count;
    }
    if base.could_be_targeted_by.is_none() {
        base.could_be_targeted_by = extra.could_be_targeted_by;
    }
    base.shares_creature_type_with_source |= extra.shares_creature_type_with_source;
    for constraint in extra.tagged_constraints {
        crate::slice_primitives::push_unique(&mut base.tagged_constraints, constraint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::util::tokenize_line;

    #[test]
    fn parse_attached_reference_or_another_disjunction_handles_articles_without_word_view() {
        let tokens = tokenize_line("enchanted creature or another creature", 0);

        let filter = parse_attached_reference_or_another_disjunction(&tokens)
            .expect("attached-reference disjunction should parse")
            .expect("attached-reference disjunction should be recognized");

        assert_eq!(filter.any_of.len(), 2);
        assert!(
            filter.any_of[0]
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag.as_str() == OBJECT_FILTER_ENCHANTED_TAG
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                }),
            "{filter:?}"
        );
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[1].other);
    }

    #[test]
    fn parse_object_filter_lexed_parses_suffix_owned_zone() {
        let tokens = tokenize_line("artifact card from your graveyard", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
    }

    #[test]
    fn parse_object_filter_lexed_parses_controller_without_owner_suffix() {
        let tokens = tokenize_line("land you control but don't own", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.owner, Some(PlayerFilter::NotYou));
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Land]);
    }

    #[test]
    fn parse_object_filter_lexed_treats_adjacent_card_types_as_conjunctive() {
        let tokens = tokenize_line("artifact creature", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert!(filter.card_types.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.all_card_types,
            vec![CardType::Artifact, CardType::Creature]
        );
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_keeps_explicit_type_lists_disjunctive() {
        let tokens = tokenize_line("artifact, creature, or land", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(
            filter.card_types,
            vec![CardType::Artifact, CardType::Creature, CardType::Land]
        );
        assert!(filter.all_card_types.is_empty(), "{filter:#?}");
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_keeps_comma_only_type_lists_disjunctive() {
        let tokens = tokenize_line("artifact, creature, enchantment", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(
            filter.card_types,
            vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment
            ]
        );
        assert!(filter.all_card_types.is_empty(), "{filter:#?}");
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_words_parses_target_and_iterated_controller_suffixes() {
        let target_filter =
            parse_simple_object_filter_words(&["artifact", "target", "player", "controls"], false)
                .expect("target-player controller suffix should parse");
        assert_eq!(
            target_filter.controller,
            Some(PlayerFilter::target_player())
        );
        assert_eq!(target_filter.zone, Some(Zone::Battlefield));
        assert_eq!(target_filter.card_types, vec![CardType::Artifact]);

        let iterated_filter =
            parse_simple_object_filter_words(&["creature", "that", "player", "controls"], false)
                .expect("that-player controller suffix should parse");
        assert_eq!(
            iterated_filter.controller,
            Some(PlayerFilter::IteratedPlayer)
        );
        assert_eq!(iterated_filter.zone, Some(Zone::Battlefield));
        assert_eq!(iterated_filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_object_filter_lexed_parses_controlled_spells_on_stack() {
        let tokens = tokenize_line("spells you control", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
        assert!(filter.has_mana_cost);
    }

    #[test]
    fn parse_object_filter_lexed_parses_split_face_state_and_chosen_type_atoms() {
        let tokens = tokenize_line("face down chosen type creatures", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.face_down, Some(true));
        assert!(filter.chosen_creature_type);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_parses_hyphenated_face_state_and_nonchosen_type_atoms() {
        let tokens = tokenize_line("face-up nonchosen type creatures", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.face_down, Some(false));
        assert!(filter.excluded_chosen_creature_type);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_parses_negated_chosen_type_suffix() {
        let tokens = tokenize_line("creatures that aren't of the chosen type", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(!filter.chosen_creature_type);
        assert!(filter.excluded_chosen_creature_type);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_parses_split_hyphenated_non_subtype_and_type() {
        let tokens = tokenize_line("Non-Elf creatures", 0);
        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.excluded_subtypes, vec![Subtype::Elf]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));

        let tokens = tokenize_line("non-artifact creatures", 0);
        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.excluded_card_types, vec![CardType::Artifact]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_preserves_chosen_color_and_type_qualifiers() {
        let color_tokens = tokenize_line("creatures you control of the chosen color", 0);
        let color_filter =
            parse_object_filter_lexed(&color_tokens, false).expect("object filter should parse");

        assert!(color_filter.chosen_color);
        assert_eq!(color_filter.controller, Some(PlayerFilter::You));
        assert_eq!(color_filter.card_types, vec![CardType::Creature]);

        let type_tokens = tokenize_line("other creatures you control of the chosen type", 0);
        let type_filter =
            parse_object_filter_lexed(&type_tokens, false).expect("object filter should parse");

        assert!(type_filter.other);
        assert!(type_filter.chosen_creature_type);
        assert_eq!(type_filter.controller, Some(PlayerFilter::You));
        assert_eq!(type_filter.card_types, vec![CardType::Creature]);

        let that_type_tokens = tokenize_line("cards of that type from their graveyard", 0);
        let that_type_filter = parse_object_filter_lexed(&that_type_tokens, false)
            .expect("that-type graveyard filter should parse");

        assert!(that_type_filter.chosen_creature_type);
        assert_eq!(that_type_filter.zone, Some(Zone::Graveyard));
        assert_eq!(that_type_filter.owner, Some(PlayerFilter::IteratedPlayer));
    }

    #[test]
    fn parse_object_filter_lexed_treats_other_than_types_as_exclusions() {
        let tokens = tokenize_line("creatures other than Werewolves and Wolves", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert!(!filter.other);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.excluded_subtypes,
            vec![Subtype::Werewolf, Subtype::Wolf]
        );
    }

    #[test]
    fn parse_object_filter_words_treats_other_than_types_as_exclusions_without_synthetic_tokens() {
        let filter = parse_simple_object_filter_words(
            &["creatures", "other", "than", "werewolves", "and", "wolves"],
            false,
        )
        .expect("object filter should parse");

        assert!(!filter.other);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.excluded_subtypes,
            vec![Subtype::Werewolf, Subtype::Wolf]
        );
    }

    #[test]
    fn parse_object_filter_lexed_preserves_outer_controller_across_aggregate_scope() {
        let tokens = tokenize_line(
            "a creature an opponent controls with the greatest power among creatures that player controls",
            0,
        );

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_object_filter_lexed_parses_permanent_or_owned_suspended_card_disjunction() {
        let tokens = tokenize_line("a permanent you control or suspended card you own", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.any_of.len(), 2, "{filter:?}");
        assert!(
            filter.any_of.iter().any(|arm| {
                arm.zone == Some(Zone::Battlefield)
                    && arm.controller == Some(PlayerFilter::You)
                    && arm.alternative_cast.is_none()
            }),
            "{filter:?}"
        );
        assert!(
            filter.any_of.iter().any(|arm| {
                arm.zone == Some(Zone::Exile)
                    && arm.owner == Some(PlayerFilter::You)
                    && arm.alternative_cast == Some(crate::filter::AlternativeCastKind::Suspend)
            }),
            "{filter:?}"
        );
    }
}

pub(crate) fn is_comparison_or_delimiter(tokens: &[OwnedLexToken], idx: usize) -> bool {
    if !token_slice_at_is(tokens, idx, "or") {
        return false;
    }
    let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
    let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
    if next_word
        .is_some_and(|word| object_filter_word_is_any(word, &["less", "greater", "more", "fewer"]))
    {
        return true;
    }
    if previous_word.is_some_and(|word| word == "than")
        && next_word.is_some_and(|word| word == "equal")
    {
        return true;
    }
    false
}
