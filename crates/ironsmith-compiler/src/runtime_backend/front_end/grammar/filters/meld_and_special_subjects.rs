use super::*;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const THERE_ARE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["there", "are"]);
const YOU_HAVE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "have"]);
const IN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["in"]);
const TYPE_OR_TYPES_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["type", "types"]]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CARD_OR_CARDS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const MANA_OF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["mana", "of"]);
const SAME_COLOR_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["same", "color"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const MANA_SPENT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["mana", "was", "spent", "to", "cast", "this", "spell"],
            &["mana", "were", "spent", "to", "cast", "this", "spell"],
        ]
);
const SAME_COLOR_MANA_SPENT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["was", "spent", "to", "cast", "it"],
            &["was", "spent", "to", "cast", "this", "spell"],
        ]
);

pub(super) fn parse_graveyard_threshold_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    fn player_filter_for_graveyard_threshold(player: &PlayerAst) -> Option<PlayerFilter> {
        match player {
            PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
            PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
            PlayerAst::Target => Some(PlayerFilter::target_player()),
            PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
            PlayerAst::Opponent => Some(PlayerFilter::Opponent),
            _ => None,
        }
    }

    fn parse_at_least_quantity_prefix(words: &[&str]) -> Option<(u32, usize)> {
        let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
        let (comparison, used) =
            parse_quantity_comparison_prefix(&tokens, false, false, "graveyard threshold").ok()?;
        let count = comparison_to_strict_at_least_threshold(&comparison)?;
        Some((count, used))
    }

    let (count, tail_start, constrained_player) =
        if filtered.len() >= 5 && THERE_ARE_PREFIX_PATTERN.matches_words(filtered) {
            let Some((count, used)) = parse_at_least_quantity_prefix(&filtered[2..]) else {
                return Ok(None);
            };
            (count, 2 + used, None)
        } else if filtered.len() >= 5 && YOU_HAVE_PREFIX_PATTERN.matches_words(filtered) {
            let Some((count, used)) = parse_at_least_quantity_prefix(&filtered[2..]) else {
                return Ok(None);
            };
            (count, 2 + used, Some(PlayerAst::You))
        } else {
            return Ok(None);
        };

    let tail = &filtered[tail_start..];
    let Some(in_idx) = rfind_index(tail, |word| IN_WORD_PATTERN.matches_word(word)) else {
        return Ok(None);
    };
    if in_idx == 0 || in_idx + 1 >= tail.len() {
        return Ok(None);
    }

    let graveyard_owner_words = &tail[in_idx + 1..];
    let player = match graveyard_owner_words {
        ["your", "graveyard"] => PlayerAst::You,
        ["that", "player", "graveyard"] | ["that", "players", "graveyard"] => PlayerAst::That,
        ["target", "player", "graveyard"] | ["target", "players", "graveyard"] => PlayerAst::Target,
        ["target", "opponent", "graveyard"] | ["target", "opponents", "graveyard"] => {
            PlayerAst::TargetOpponent
        }
        ["opponent", "graveyard"] | ["opponents", "graveyard"] => PlayerAst::Opponent,
        _ => return Ok(None),
    };
    if constrained_player.is_some_and(|expected| expected != player) {
        return Ok(None);
    }

    let raw_filter_words = &tail[..in_idx];
    if raw_filter_words.is_empty() || TYPE_OR_TYPES_MARKER_PATTERN.matches_words(raw_filter_words) {
        return Ok(None);
    }

    let mut normalized_filter_words = Vec::with_capacity(raw_filter_words.len());
    for (idx, word) in raw_filter_words.iter().enumerate() {
        if AND_WORD_PATTERN.matches_word(word)
            && raw_filter_words
                .get(idx + 1)
                .is_some_and(|next| OR_WORD_PATTERN.matches_word(next))
        {
            continue;
        }
        normalized_filter_words.push(*word);
    }
    if normalized_filter_words.is_empty() {
        return Ok(None);
    }

    let mut filter = if CARD_OR_CARDS_PATTERN.matches_words(&normalized_filter_words) {
        ObjectFilter::default()
    } else {
        let filter_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(normalized_filter_words);
        let Ok(filter) = parse_object_filter(&filter_tokens, false) else {
            return Ok(None);
        };
        filter
    };
    filter.zone = Some(Zone::Graveyard);

    if constrained_player.is_none() {
        if let Some(owner) = player_filter_for_graveyard_threshold(&player) {
            if filter.owner.is_none() {
                filter.owner = Some(owner);
            }
            return Ok(Some(PredicateAst::ValueComparison {
                left: Value::Count(filter),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(count as i32),
            }));
        }
    }

    Ok(Some(PredicateAst::PlayerControlsAtLeast {
        player,
        filter,
        count,
    }))
}

pub(super) fn parse_mana_spent_to_cast_predicate(
    words: &[&str],
) -> Option<(u32, Option<ManaSymbol>)> {
    if words.len() < 10 {
        return None;
    }

    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let (amount, used) =
        parse_greater_than_or_equal_quantity_prefix(&tokens, false, false, "mana spent predicate")
            .ok()
            .flatten()?;

    let mut idx = used;
    if words
        .get(idx)
        .is_some_and(|word| OF_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    let symbol = if let Some(word) = words.get(idx).copied() {
        if let Some(parsed) = parse_mana_symbol_word(word) {
            idx += 1;
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    if MANA_SPENT_TAIL_PATTERN.matches_words(&words[idx..]) {
        return Some((amount, symbol));
    }

    None
}

pub(crate) fn parse_same_color_mana_spent_to_cast_predicate(words: &[&str]) -> Option<u32> {
    if words.len() < 12 {
        return None;
    }

    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let (amount, used) = parse_greater_than_or_equal_quantity_prefix(
        &tokens,
        false,
        false,
        "same-color mana spent predicate",
    )
    .ok()
    .flatten()?;

    let mut idx = used;
    if !MANA_OF_PREFIX_PATTERN.matches_words(&words[idx..]) {
        return None;
    }
    idx += 2;
    if words
        .get(idx)
        .is_some_and(|word| THE_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }
    if !SAME_COLOR_PREFIX_PATTERN.matches_words(&words[idx..]) {
        return None;
    }
    idx += 2;

    if SAME_COLOR_MANA_SPENT_TAIL_PATTERN.matches_words(&words[idx..]) {
        return Some(amount);
    }

    None
}

pub(super) fn parse_mana_symbol_word(word: &str) -> Option<ManaSymbol> {
    parse_mana_symbol_word_flexible(word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn parse_object_filter_lexed_handles_with_keyword_disjunction() {
        let tokens = lex_line("creatures with flying or reach", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        let debug = format!("{filter:?}");
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(
            (filter.any_of.len() == 2 && debug.contains("Flying") && debug.contains("Reach"))
                || (filter.static_abilities.contains(&StaticAbilityId::Flying)
                    && filter.static_abilities.contains(&StaticAbilityId::Reach))
                || debug.contains("Flying"),
            "{debug}"
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_with_decayed_keyword_marker() {
        let tokens = lex_line("creatures with decayed", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.ability_markers, vec!["decayed".to_string()]);
        assert!(filter.static_abilities.is_empty(), "{filter:?}");
    }

    #[test]
    fn parse_object_filter_lexed_handles_without_keyword_clause() {
        let tokens = lex_line("creatures without flying", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.excluded_static_abilities,
            vec![StaticAbilityId::Flying]
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_with_no_abilities_clause() {
        let tokens = lex_line("creatures with no abilities", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.no_abilities);
    }

    #[test]
    fn parse_object_filter_lexed_handles_joint_owner_controller_clause() {
        let tokens = lex_line("permanents you both own and control", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_handles_chosen_player_graveyard_clause() {
        let tokens = lex_line("artifact card in the chosen player's graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.owner, Some(PlayerFilter::ChosenPlayer));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_owner_or_controller_disjunction() {
        let tokens = lex_line("artifacts target opponent owns or controls", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Artifact]);
        assert_eq!(
            filter.any_of[0].owner,
            Some(PlayerFilter::target_opponent())
        );
        assert_eq!(filter.any_of[0].controller, None);
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Artifact]);
        assert_eq!(filter.any_of[1].owner, None);
        assert_eq!(
            filter.any_of[1].controller,
            Some(PlayerFilter::target_opponent())
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_target_player_reference_clause() {
        let tokens = lex_line("spell that targets player", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.targets_player, Some(PlayerFilter::Any));
    }

    #[test]
    fn parse_object_filter_lexed_handles_attacking_target_opponent_clause() {
        let tokens = lex_line("creature attacking target opponent", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.attacking_player_or_planeswalker_controlled_by,
            Some(PlayerFilter::target_opponent())
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_attacking_one_of_your_opponents_clause() {
        let tokens = lex_line("creature attacking one of your opponents", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.attacking_player_or_planeswalker_controlled_by,
            Some(PlayerFilter::Opponent)
        );
    }

    #[test]
    fn temporal_graveyard_from_battlefield_phrase_parser_matches() {
        assert_eq!(
            parse_graveyard_from_battlefield_this_turn_words(&[
                "graveyard",
                "from",
                "battlefield",
                "this",
                "turn",
            ]),
            Some(5)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_put_there_from_anywhere_this_turn_clause() {
        let tokens = lex_line(
            "creature cards in a graveyard that were put there from anywhere this turn",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(filter.entered_graveyard_this_turn);
    }

    #[test]
    fn parse_object_filter_lexed_handles_entered_battlefield_this_turn_clause() {
        let tokens = lex_line(
            "creatures that entered the battlefield under your control this turn",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert!(filter.entered_battlefield_this_turn);
        assert_eq!(
            filter.entered_battlefield_controller,
            Some(PlayerFilter::You)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_drawn_this_turn_clause() {
        let tokens = lex_line("cards in your hand drawn this turn", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert!(filter.drawn_this_turn);
    }

    #[test]
    fn parse_object_filter_lexed_handles_named_clause_with_trailing_zone() {
        let tokens = lex_line("artifact card named Sol Ring from your graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.name.as_deref(), Some("sol ring"));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_named_clause_with_trailing_zone() {
        let tokens = lex_line("artifact card not named Sol Ring from your graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.excluded_name.as_deref(), Some("sol ring"));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_tagged_reference_prefix() {
        let tokens = lex_line("that creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.tagged_constraints,
            vec![TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            }]
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_entered_since_your_last_turn_ended_clause() {
        let tokens = lex_line("creatures that entered since your last turn ended", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.entered_since_your_last_turn_ended);
    }

    #[test]
    fn parse_object_filter_lexed_handles_split_face_state_words() {
        let tokens = lex_line("face up creature cards", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.face_down, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_single_graveyard_phrase() {
        let tokens = lex_line("creature cards in a single graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(filter.single_graveyard);
    }

    #[test]
    fn parse_object_filter_lexed_handles_one_or_more_colors_phrase() {
        let tokens = lex_line("creatures of one or more colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        let any_color: ColorSet = Color::ALL.into_iter().collect();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.colors, Some(any_color));
    }

    #[test]
    fn parse_object_filter_lexed_handles_mana_value_eq_counters_on_source_clause() {
        let tokens = lex_line(
            "creature card with mana value equal to the number of charge counters on this artifact",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.mana_value_eq_counters_on_source,
            Some(crate::object::CounterType::Charge)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_attached_exclusion_phrase() {
        let tokens = lex_line("creatures other than enchanted creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("enchanted"),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_different_one_of_prefix() {
        let tokens = lex_line("different one of those creatures", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_pt_literal_prefix() {
        let tokens = lex_line("2/2 creature token", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.power, Some(crate::filter::Comparison::Equal(2)));
        assert_eq!(filter.toughness, Some(crate::filter::Comparison::Equal(2)));
    }

    #[test]
    fn parse_object_filter_lexed_handles_and_or_subtype_distinct_powers() {
        let tokens = lex_line("creature and/or vehicle cards with different powers", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.subtypes, vec![Subtype::Vehicle]);
        assert!(filter.type_or_subtype_union);
        assert!(filter.distinct_powers);
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_all_colors_clause() {
        let tokens = lex_line("creature that isnt all colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.all_colors, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_exactly_two_colors_clause() {
        let tokens = lex_line("creature that isnt exactly two colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.exactly_two_colors, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_attached_to_tagged_reference() {
        let tokens = lex_line("creature attached to it", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_attached_to_enchanted_player() {
        let tokens = lex_line("Curse attached to enchanted player", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.subtypes, vec![Subtype::Curse]);
        assert_eq!(
            filter.attached_to_player,
            Some(PlayerFilter::TaggedPlayer(TagKey::from("enchanted")))
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_room_subtype_target() {
        let tokens = lex_line("room", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.subtypes, vec![Subtype::Room]);
    }

    #[test]
    fn parse_object_filter_lexed_handles_its_attached_to_reference_alias() {
        let tokens = lex_line("creature its attached to", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_source_linked_exile_reference() {
        let tokens = lex_line("spell exiled with this", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_revealed_cards_reference() {
        let tokens = lex_line("revealed cards", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_same_mana_value_as_sacrificed_reference() {
        let tokens = lex_line(
            "creature with same mana value as the sacrificed creature",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("sacrifice_cost_0"),
                    relation: TaggedOpbjectRelation::SameManaValueAsTagged,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_same_name_as_tagged_reference() {
        let tokens = lex_line("creature with same name as that creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_tap_activated_ability_phrase() {
        let tokens = lex_line(
            "creature with activated abilities with {T} in their costs",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.has_tap_activated_ability);
    }

    #[test]
    fn parse_object_filter_lexed_handles_convoked_it_reference() {
        let tokens = lex_line("creature that convoked it", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("convoked_this_spell"),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_spell_filter_lexed_handles_split_face_state_words() {
        let tokens = lex_line("Face up noncreature spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(filter.face_down, Some(false));
        assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_spell_filter_raw_handles_hyphenated_face_state_words() {
        let tokens = lex_line("face-down noncreature spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint(&tokens);
        assert_eq!(filter.face_down, Some(true));
        assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_spell_filter_lexed_builds_power_or_toughness_disjunction() {
        let tokens = lex_line("creature spells with power or toughness 2 or less", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[0].power.is_some());
        assert!(filter.any_of[0].toughness.is_none());
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[1].power.is_none());
        assert!(filter.any_of[1].toughness.is_some());
    }

    #[test]
    fn parse_spell_filter_lexed_handles_even_mana_value_phrase() {
        let tokens = lex_line("even mana value spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(
            filter.mana_value_parity,
            Some(crate::filter::ParityRequirement::Even)
        );
    }
}
