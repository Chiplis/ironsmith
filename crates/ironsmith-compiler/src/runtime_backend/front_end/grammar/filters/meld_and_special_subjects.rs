use super::super::super::lexer::{LexStream, LexedClause, OwnedLexToken};
use super::*;
use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraveyardThresholdPrefix {
    Existential,
    YouHave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraveyardThresholdOwner {
    You,
    ThatPlayer,
    TargetPlayer,
    TargetOpponent,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GraveyardThresholdShape<'a> {
    prefix: GraveyardThresholdPrefix,
    body_tokens: &'a [OwnedLexToken],
    owner: GraveyardThresholdOwner,
}

fn parse_graveyard_threshold_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<GraveyardThresholdPrefix> {
    alt((
        primitives::phrase(&["there", "are"]).value(GraveyardThresholdPrefix::Existential),
        primitives::phrase(&["you", "have"]).value(GraveyardThresholdPrefix::YouHave),
    ))
    .parse_next(input)
}

fn parse_graveyard_threshold_owner_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<GraveyardThresholdOwner> {
    alt((
        primitives::phrase(&["your", "graveyard"]).value(GraveyardThresholdOwner::You),
        primitives::any_phrase(&[
            &["that", "player", "graveyard"],
            &["that", "players", "graveyard"],
        ])
        .value(GraveyardThresholdOwner::ThatPlayer),
        primitives::any_phrase(&[
            &["target", "player", "graveyard"],
            &["target", "players", "graveyard"],
        ])
        .value(GraveyardThresholdOwner::TargetPlayer),
        primitives::any_phrase(&[
            &["target", "opponent", "graveyard"],
            &["target", "opponents", "graveyard"],
        ])
        .value(GraveyardThresholdOwner::TargetOpponent),
        primitives::any_phrase(&[&["opponent", "graveyard"], &["opponents", "graveyard"]])
            .value(GraveyardThresholdOwner::Opponent),
    ))
    .parse_next(input)
}

fn parse_graveyard_threshold_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<GraveyardThresholdShape<'a>> {
    let prefix = parse_graveyard_threshold_prefix_lexed.parse_next(input)?;
    let body_tokens = repeat_till(
        1..,
        any.void(),
        peek((
            primitives::kw("in"),
            parse_graveyard_threshold_owner_lexed,
            opt(primitives::period()),
            eof,
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::kw("in").parse_next(input)?;
    let owner = parse_graveyard_threshold_owner_lexed.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(GraveyardThresholdShape {
        prefix,
        body_tokens,
        owner,
    })
}

fn parse_graveyard_threshold_shape(
    tokens: &[OwnedLexToken],
) -> Option<GraveyardThresholdShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_graveyard_threshold_shape_lexed,
        "graveyard-threshold",
    )
    .ok()
}

fn graveyard_threshold_owner_player(owner: GraveyardThresholdOwner) -> PlayerAst {
    match owner {
        GraveyardThresholdOwner::You => PlayerAst::You,
        GraveyardThresholdOwner::ThatPlayer => PlayerAst::That,
        GraveyardThresholdOwner::TargetPlayer => PlayerAst::Target,
        GraveyardThresholdOwner::TargetOpponent => PlayerAst::TargetOpponent,
        GraveyardThresholdOwner::Opponent => PlayerAst::Opponent,
    }
}

fn tokens_contain_type_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .any(|token| token.is_any_word(&["type", "types"]))
}

fn non_article_tokens_are_card_or_cards(tokens: &[OwnedLexToken]) -> bool {
    let mut words = tokens
        .iter()
        .filter(|token| {
            token
                .as_word()
                .is_some_and(|_| !is_article(token.parser_text()))
        })
        .map(OwnedLexToken::parser_text);

    let Some(first) = words.next() else {
        return false;
    };
    words.next().is_none() && matches!(first, "card" | "cards")
}

pub(super) fn parse_graveyard_threshold_predicate(
    tokens: &[OwnedLexToken],
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

    fn parse_at_least_quantity_prefix(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
        let (comparison, used) =
            parse_quantity_comparison_prefix(tokens, false, false, "graveyard threshold").ok()?;
        let count = comparison_to_strict_at_least_threshold(&comparison)?;
        Some((count, used))
    }

    let Some(shape) = parse_graveyard_threshold_shape(tokens) else {
        return Ok(None);
    };
    let constrained_player = match shape.prefix {
        GraveyardThresholdPrefix::Existential => None,
        GraveyardThresholdPrefix::YouHave => Some(PlayerAst::You),
    };

    let Some((count, used)) = parse_at_least_quantity_prefix(shape.body_tokens) else {
        return Ok(None);
    };
    let raw_filter_tokens = shape.body_tokens.get(used..).unwrap_or_default();
    if raw_filter_tokens.is_empty() || tokens_contain_type_marker(raw_filter_tokens) {
        return Ok(None);
    }
    let used_and_or_connective = raw_filter_tokens
        .windows(2)
        .any(|window| window[0].is_word("and") && window[1].is_word("or"))
        || raw_filter_tokens
            .iter()
            .any(|token| token.is_word("and/or"));

    let player = graveyard_threshold_owner_player(shape.owner);
    if constrained_player.is_some_and(|expected| expected != player) {
        return Ok(None);
    }

    let mut normalized_filter_tokens = Vec::with_capacity(raw_filter_tokens.len());
    for (idx, token) in raw_filter_tokens.iter().enumerate() {
        if token.is_word("and")
            && raw_filter_tokens
                .get(idx + 1)
                .is_some_and(|next| next.is_word("or"))
        {
            continue;
        }
        normalized_filter_tokens.push(token.clone());
    }
    if normalized_filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = if non_article_tokens_are_card_or_cards(&normalized_filter_tokens) {
        ObjectFilter::default()
    } else {
        let Ok(filter) = parse_object_filter(&normalized_filter_tokens, false) else {
            return Ok(None);
        };
        filter
    };
    filter.zone = Some(Zone::Graveyard);
    if used_and_or_connective {
        filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    }

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

    Ok(Some(PredicateAst::PlayerHasAtLeast {
        player,
        filter,
        count,
    }))
}

pub(super) fn parse_mana_spent_to_cast_predicate(
    tokens: &[OwnedLexToken],
) -> Option<(u32, Option<ManaSymbol>)> {
    if tokens.len() < 8 {
        return None;
    }

    let (amount, used) =
        parse_greater_than_or_equal_quantity_prefix(tokens, false, false, "mana spent predicate")
            .ok()
            .flatten()?;

    let mut idx = used;
    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }

    let symbol = if let Some(token) = tokens.get(idx) {
        if let Some(parsed) = parse_mana_symbol_word(token.parser_text()) {
            idx += 1;
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    if matches_mana_spent_tail(&tokens[idx..]) {
        return Some((amount, symbol));
    }

    None
}

pub(crate) fn parse_same_color_mana_spent_to_cast_predicate(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    if tokens.len() < 12 {
        return None;
    }

    let (amount, used) = parse_greater_than_or_equal_quantity_prefix(
        tokens,
        false,
        false,
        "same-color mana spent predicate",
    )
    .ok()
    .flatten()?;

    let mut idx = used;
    let (_, after_mana_of) =
        primitives::parse_prefix(&tokens[idx..], primitives::phrase(&["mana", "of"]))?;
    idx = tokens.len().checked_sub(after_mana_of.len())?;
    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    let (_, after_same_color) =
        primitives::parse_prefix(&tokens[idx..], primitives::phrase(&["same", "color"]))?;
    idx = tokens.len().checked_sub(after_same_color.len())?;

    if matches_same_color_mana_spent_tail(&tokens[idx..]) {
        return Some(amount);
    }

    None
}

fn parse_sentence_punctuation_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat(0.., alt((primitives::comma(), primitives::period())).void()).parse_next(input)
}

fn parse_mana_spent_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::any_phrase(&[
        &["mana", "was", "spent", "to", "cast", "this", "spell"],
        &["mana", "were", "spent", "to", "cast", "this", "spell"],
        &["mana", "was", "spent", "to", "cast", "it"],
        &["mana", "were", "spent", "to", "cast", "it"],
        &["mana", "was", "spent", "to", "cast", "that", "spell"],
        &["mana", "were", "spent", "to", "cast", "that", "spell"],
    ])
    .parse_next(input)?;
    parse_sentence_punctuation_lexed.parse_next(input)?;
    eof.void().parse_next(input)
}

fn matches_mana_spent_tail(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(tokens, parse_mana_spent_tail_lexed, "mana-spent-tail").is_ok()
}

fn parse_same_color_mana_spent_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::any_phrase(&[
        &["was", "spent", "to", "cast", "it"],
        &["was", "spent", "to", "cast", "this", "spell"],
    ])
    .parse_next(input)?;
    parse_sentence_punctuation_lexed.parse_next(input)?;
    eof.void().parse_next(input)
}

fn matches_same_color_mana_spent_tail(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_same_color_mana_spent_tail_lexed,
        "same-color-mana-spent-tail",
    )
    .is_ok()
}

pub(super) fn parse_mana_symbol_word(word: &str) -> Option<ManaSymbol> {
    parse_mana_symbol_word_flexible(word)
}

/// Token-backed view of a captured mana-symbol clause.
///
/// Hosts the `word_refs` derivation for mana-spent symbol parsing so the
/// predicate-side parser can validate symbol words and parse each symbol from
/// its token text without rebuilding the word slice locally.
pub(super) fn mana_spent_symbol_clause_words<'a>(symbol_clause: LexedClause<'a>) -> Vec<&'a str> {
    let symbol_words = symbol_clause.word_refs();
    symbol_words
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
    fn parse_object_filter_keeps_basic_land_exception_as_exclusions() {
        let tokens = lex_line("card in a graveyard other than a basic land card", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.any_of.len(), 2, "{filter:?}");
        assert!(filter.any_of.iter().all(|branch| {
            branch.zone == Some(Zone::Graveyard)
                && branch.card_types.is_empty()
                && branch.all_card_types.is_empty()
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.excluded_card_types == vec![CardType::Land]
                && branch.excluded_supertypes.is_empty()
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.excluded_card_types.is_empty()
                && branch.excluded_supertypes == vec![crate::types::Supertype::Basic]
        }));
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
    fn parse_object_filter_lexed_handles_mana_value_lte_counters_on_source_clause() {
        let tokens = lex_line(
            "creature card with mana value less than or equal to the number of void counters on it",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        let Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) = filter.mana_value else {
            panic!(
                "expected mana value <= source-counter expression, got {:?}",
                filter.mana_value
            );
        };
        assert_eq!(
            *value,
            crate::effect::Value::CountersOnSource(crate::object::CounterType::Void)
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
        assert_eq!(
            filter.union_connective(),
            crate::filter::ObjectFilterUnionConnective::AndOr
        );
        assert!(filter.description().contains("and/or"));
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
    fn parse_object_filter_keeps_intrinsic_attachment_to_source_relation() {
        let tokens = lex_line("Aura attached to this creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.subtypes, vec![Subtype::Aura]);
        let attached_to = filter
            .attached_to_object
            .as_deref()
            .expect("typed attachment target filter");
        assert!(attached_to.source);
        assert_eq!(attached_to.card_types, vec![CardType::Creature]);
        assert_eq!(
            attached_to.source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this creature".to_string()
            ))
        );
    }

    #[test]
    fn parse_object_filter_keeps_intrinsic_attachment_to_controlled_creature_relation() {
        let tokens = lex_line(
            "Aura you control that's attached to a creature you control",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.subtypes, vec![Subtype::Aura]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        let attached_to = filter
            .attached_to_object
            .as_deref()
            .expect("typed attachment target filter");
        assert_eq!(attached_to.card_types, vec![CardType::Creature]);
        assert_eq!(attached_to.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn parse_object_filter_keeps_attachment_subject_distinct_from_permanent_host() {
        let tokens = lex_line("Auras attached to permanents you control", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.subtypes, vec![Subtype::Aura]);
        let attached_to = filter
            .attached_to_object
            .as_deref()
            .expect("typed permanent attachment host");
        assert_eq!(attached_to.zone, Some(Zone::Battlefield));
        assert_eq!(attached_to.controller, Some(PlayerFilter::You));
        assert_eq!(
            filter.description(),
            "Aura attached to a permanent you control"
        );
    }

    #[test]
    fn parse_object_filter_routes_that_player_attachment_to_player_relation() {
        let tokens = lex_line("curses attached to that player", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.subtypes, vec![Subtype::Curse]);
        assert!(filter.attached_to_object.is_none());
        assert_eq!(
            filter.attached_to_player,
            Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)))
        );
        assert_eq!(filter.description(), "Curse attached to that player");
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
    fn parse_same_name_as_source_exiled_card_uses_exiled_card_as_antecedent() {
        let tokens = lex_line(
            "Vampire spell with the same name as a card exiled with this",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                }
        }));
        assert!(!filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
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
        assert_eq!(
            filter.same_name_antecedent_surface(),
            Some(ironsmith_core::SameNameAntecedentSurface::Creature)
        );
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
    fn parse_object_filter_lexed_handles_no_shared_creature_type_clause() {
        let tokens = lex_line(
            "creature spell that doesn't share a creature type with a creature you control or a creature card in your graveyard",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.no_shared_creature_types_with.len(), 2);

        let battlefield_filter = &filter.no_shared_creature_types_with[0];
        assert_eq!(battlefield_filter.zone, Some(Zone::Battlefield));
        assert_eq!(battlefield_filter.controller, Some(PlayerFilter::You));
        assert_eq!(battlefield_filter.card_types, vec![CardType::Creature]);

        let graveyard_filter = &filter.no_shared_creature_types_with[1];
        assert_eq!(graveyard_filter.zone, Some(Zone::Graveyard));
        assert_eq!(graveyard_filter.owner, Some(PlayerFilter::You));
        assert_eq!(graveyard_filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_object_filter_lexed_handles_shared_creature_type_with_source_clause() {
        let tokens = lex_line(
            "creature spell that shares a creature type with this creature",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.shares_creature_type_with_source);
    }

    #[test]
    fn parse_object_filter_preserves_battlefield_identity_link_to_source() {
        let tokens =
            lex_line("creature put onto the battlefield with this enchantment", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.put_onto_battlefield_with_source);
        assert_eq!(
            filter.put_onto_battlefield_with_source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string()
            ))
        );
        assert_eq!(
            filter.description(),
            "a creature put onto the battlefield with this enchantment"
        );
    }

    #[test]
    fn parse_object_filter_preserves_token_creation_source_provenance() {
        let tokens = lex_line("tokens created with this enchantment", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert!(filter.token);
        assert!(filter.card_types.is_empty());
        assert!(filter.created_with_source);
        assert_eq!(
            filter.created_with_source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string()
            ))
        );
        assert_eq!(filter.description(), "a token created with this enchantment");
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
