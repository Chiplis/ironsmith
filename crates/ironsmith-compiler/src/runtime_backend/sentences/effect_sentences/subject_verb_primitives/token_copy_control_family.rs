use super::*;
use crate::runtime_backend::front_end::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom, LexPatternMatch,
};
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const RETURN_WORD: &str = "return";
const CHOOSE_WORD: &str = "choose";
const CREATE_WORD: &str = "create";
const SACRIFICE_WORD: &str = "sacrifice";
const TARGET_WORD: &str = "target";
const WITH_WORD: &str = "with";
const EXILE_PREFIX: &[&str] = &["exile"];
const WHERE_X_IS_WORDS: &[&str] = &["where", "x", "is"];
const PUTS_ALL_PERMANENT_CARDS_PREFIX: &[&str] = &["puts", "all", "permanent", "cards"];
const REVEALED_THIS_WAY_PHRASE: &[&str] = &["revealed", "this", "way"];
const ONTO_THE_BATTLEFIELD_PHRASE: &[&str] = &["onto", "the", "battlefield"];
const EACH_PLAYER_REVEALS_TOP_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "each", "player", "reveals", "a", "number", "of", "cards", "from", "the", "top", "of",
            "their", "library", "equal", "to",
        ]
);
const EACH_PLAYER_PUTS_REST_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["puts", "the", "rest", "into", "their", "graveyard"]);
const TOKEN_COPY_OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const TOKEN_COPY_AND_OR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"]]);
const TOKEN_COPY_DEAL_OR_DEALS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["deal"], &["deals"]]);
const SHUFFLE_OR_SHUFFLES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["shuffle"], &["shuffles"]]);
const GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["graveyard"], &["graveyards"]]);
const LIBRARY_OR_LIBRARIES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["library"], &["libraries"]]);
const IT_OR_THEM_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["it"], &["them"]]);
const THAT_PLAYER_TAIL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "player"]);
const RETURN_THIS_OWNER_HAND_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["return", "this"];
    contains_words & ["owner", "hand"]
);
const PUT_THIS_OWNER_TOP_LIBRARY_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["put", "this"];
    suffix & ["library"];
    contains_words & ["top", "owner"]
);
const CHOOSE_CARD_NAME_TAIL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["choose", "any", "card", "name"],
            &["choose", "a", "card", "name"]
        ]
);
const EXILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exile"]);
const YOU_EXILE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "exile"]);
const LOOK_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["look"]);
const DRAW_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["draw"]);
const TAP_OR_UNTAP_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["tap"], &["untap"]]);

fn token_copy_clause_first_is_word(clause: SubjectVerbPrimitiveClause<'_>, expected: &str) -> bool {
    clause.first_is_word(expected)
}

pub(crate) const DESTROY_ALL_ATTACHED_TO_TARGET_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("destroy"),
    LexPattern::any_word(ALL_OR_EACH_WORDS),
    LexPattern::role_capture(
        "filter",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["attached", "to"]),
    ),
    LexPattern::phrase(&["attached", "to"]),
    LexPattern::role_capture(
        "target",
        LexCaptureRole::Tail,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const RETURN_THEN_CREATE_HEAD_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::word("return"),
    LexPattern::capture("return_tail", LexCaptureKind::UntilPhrase(&["then"])),
];
const EXILE_THEN_MAY_PUT_HEAD_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::word("exile"),
    LexPattern::capture("exile_tail", LexCaptureKind::UntilPhrase(&["then"])),
];
const YOU_EXILE_THEN_MAY_PUT_HEAD_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["you", "exile"]),
    LexPattern::capture("exile_tail", LexCaptureKind::UntilPhrase(&["then"])),
];
const EXILE_THEN_MAY_PUT_HEAD_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    EXILE_THEN_MAY_PUT_HEAD_SEQUENCE,
    YOU_EXILE_THEN_MAY_PUT_HEAD_SEQUENCE,
];
pub(crate) const RETURN_THEN_CREATE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_sequence(&[RETURN_THEN_CREATE_HEAD_SEQUENCE]),
    LexPattern::word("then"),
    LexPattern::word("create"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
pub(crate) const EXILE_THEN_MAY_PUT_FROM_EXILE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_sequence(EXILE_THEN_MAY_PUT_HEAD_SEQUENCES),
    LexPattern::word("then"),
    LexPattern::phrase(&["you", "may", "put"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
pub(crate) const THEN_CHAIN_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
const OPTIONAL_OF_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::word("of")];
pub(crate) const SACRIFICE_ANY_NUMBER_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("sacrifice"),
    LexPattern::phrase(&["any", "number"]),
    LexPattern::optional(OPTIONAL_OF_PATTERN_ATOMS),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const SACRIFICE_ONE_OR_MORE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("sacrifice"),
    LexPattern::capture("minimum", LexCaptureKind::WordCount(3)),
    LexPattern::optional(OPTIONAL_OF_PATTERN_ATOMS),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const CHOOSE_THEN_DO_SAME_FOR_FILTER_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["then", "do", "the", "same", "for"]),
    ),
    LexPattern::phrase(&["then", "do", "the", "same", "for"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
pub(crate) const CHOOSE_THEN_CHOOSE_OBJECTS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
pub(crate) const RETURN_THEN_DO_SAME_FOR_SUBTYPES_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("return"),
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["do", "the", "same", "for"]),
    ),
    LexPattern::phrase(&["do", "the", "same", "for"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
pub(crate) const EACH_PLAYER_REVEALS_TOP_PUT_PERMANENTS_PATTERN_ATOMS: &[LexPatternAtom<
    'static,
>] = &[
    LexPattern::phrase(&["each", "player", "reveals"]),
    LexPattern::role_capture(
        "reveal_count",
        LexCaptureRole::Amount,
        LexCaptureKind::UntilPhrase(&["puts", "all", "permanent", "cards"]),
    ),
    LexPattern::phrase(&["puts", "all", "permanent", "cards"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
pub(crate) const EXILE_SOURCE_WITH_COUNTERS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("exile"),
    LexPattern::role_capture(
        "source",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["with"]),
    ),
    LexPattern::word("with"),
    LexPattern::role_capture("counter", LexCaptureRole::Modifier, LexCaptureKind::Rest),
];
pub(crate) const DESTROY_THEN_LAND_GRAVEYARD_DAMAGE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("destroy"),
    LexPattern::role_capture(
        "destroy_clause",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("damage_clause", LexCaptureRole::Tail, LexCaptureKind::Rest),
];

pub(crate) fn parse_sentence_each_player_return_with_additional_counter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_return_with_additional_counter_sentence(clause)
}

pub(crate) fn parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_comma_segments();
    if segments.len() != 3 {
        return Ok(None);
    }

    let reveal_clause = segments[0];
    let reveal_words = reveal_clause.word_refs();
    let Some(count_word_count) =
        EACH_PLAYER_REVEALS_TOP_PREFIX_PATTERN.matched_prefix_len(&reveal_words)
    else {
        return Ok(None);
    };
    let Some(count_clause) = reveal_clause.after_words(count_word_count) else {
        return Ok(None);
    };
    let mut synthetic_where_clause =
        SubjectVerbPrimitiveOwnedClause::synthetic_words(WHERE_X_IS_WORDS);
    synthetic_where_clause.append_clause(count_clause);
    let Some(count) = parse_value_binding_clause(synthetic_where_clause.tokens()) else {
        return Ok(None);
    };

    let put_clause = segments[1];
    if put_clause
        .strip_prefix(PUTS_ALL_PERMANENT_CARDS_PREFIX)
        .is_none()
        || !put_clause.contains_phrase(REVEALED_THIS_WAY_PHRASE)
        || !put_clause.contains_phrase(ONTO_THE_BATTLEFIELD_PHRASE)
    {
        return Ok(None);
    }

    let rest_words = segments[2].without_leading_connectors_clause().word_refs();
    let rest_words = rest_words.as_slice();
    if !EACH_PLAYER_PUTS_REST_GRAVEYARD_PATTERN.matches_words(rest_words) {
        return Ok(None);
    }

    let revealed_tag_key = helper_tag_for_tokens(clause.tokens(), "revealed");
    let iterated_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::That,
                count,
                revealed_tag_key.clone(),
            ),
            EffectAst::subject_verb_reveal_tagged(revealed_tag_key.clone()),
            EffectAst::ForEachTagged {
                tag: revealed_tag_key,
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::ItMatches(ObjectFilter::permanent_card()),
                    if_true: vec![EffectAst::subject_verb_move_to_zone(
                        iterated_target.clone(),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Owner,
                        false,
                        None,
                    )],
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        iterated_target,
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            },
        ],
    }]))
}

pub(crate) fn parse_return_then_do_same_for_subtypes_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(RETURN_THEN_DO_SAME_FOR_SUBTYPES_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_return_then_do_same_for_subtypes_sentence_matched(clause, &matched)
}

pub(crate) fn parse_return_then_do_same_for_subtypes_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !token_copy_clause_first_is_word(clause, RETURN_WORD) {
        return Ok(None);
    }
    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let tail_words = tail_clause.word_refs();
    if tail_clause
        .strip_prefix(&["do", "the", "same", "for"])
        .is_none()
    {
        return Ok(None);
    }
    let subtype_words = &tail_words[4..];
    if subtype_words.is_empty() {
        return Ok(None);
    }

    let mut extra_subtypes = Vec::new();
    for word in subtype_words {
        if TOKEN_COPY_AND_OR_WORD_PATTERN.matches_word(word) {
            continue;
        }
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        extra_subtypes.push(subtype);
    }
    if extra_subtypes.is_empty() {
        return Ok(None);
    }

    let mut effects = parse_effect_chain(head_clause.tokens())?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let base_effect = effects[0].clone();
    for subtype in extra_subtypes {
        let Some(cloned) = clone_return_effect_with_subtype(&base_effect, subtype) else {
            return Ok(None);
        };
        effects.push(cloned);
    }

    Ok(Some(effects))
}

fn split_choose_same_followup_filters(filter: &ObjectFilter) -> Vec<ObjectFilter> {
    match filter.mana_value.clone() {
        Some(crate::filter::Comparison::OneOf(values)) if !values.is_empty() => values
            .into_iter()
            .map(|value| {
                let mut cloned = filter.clone();
                cloned.mana_value = Some(crate::filter::Comparison::Equal(value));
                cloned
            })
            .collect(),
        _ => vec![filter.clone()],
    }
}

pub(crate) fn parse_choose_then_do_same_for_filter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(CHOOSE_THEN_DO_SAME_FOR_FILTER_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_choose_then_do_same_for_filter_sentence_matched(clause, &matched)
}

pub(crate) fn parse_choose_then_do_same_for_filter_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !token_copy_clause_first_is_word(clause, CHOOSE_WORD) {
        return Ok(None);
    }
    let Some((head_clause, tail_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    if tail_clause
        .strip_prefix(&["do", "the", "same", "for"])
        .is_none()
    {
        return Ok(None);
    }

    let followup_filter_clause = tail_clause.from(4);
    if followup_filter_clause.is_empty() {
        return Ok(None);
    }

    let Some((player, base_filter, count)) = parse_you_choose_objects_clause(head_clause.tokens())?
        .or_else(|| {
            parse_target_player_choose_objects_clause(head_clause.tokens())
                .ok()
                .flatten()
        })
    else {
        return Ok(None);
    };
    let tag = TagKey::from(IT_TAG);

    let followup_filter = parse_object_filter(followup_filter_clause.tokens(), false)?;
    if followup_filter.controller.is_some() || followup_filter.owner.is_some() {
        return Ok(None);
    }

    let merged_filter = merge_filters(&base_filter, &followup_filter);
    let followup_filters = split_choose_same_followup_filters(&merged_filter);
    if followup_filters.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::ChooseObjects {
        filter: base_filter.clone(),
        count: count.clone(),
        count_value: None,
        player: player.clone(),
        tag: tag.clone(),
    }];
    for filter in followup_filters {
        effects.push(EffectAst::ChooseObjects {
            filter,
            count: count.clone(),
            count_value: None,
            player: player.clone(),
            tag: tag.clone(),
        });
    }

    Ok(Some(effects))
}

fn parse_choose_objects_clause_for_chain(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    if let Some(parsed) = clause.parse_value_with_lexed(parse_you_choose_objects_clause)? {
        return Ok(Some(parsed));
    }
    clause.parse_value_with_lexed(parse_target_player_choose_objects_clause)
}

fn choose_clause_trails_from_it(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    matches!(
        words.as_slice(),
        [.., "from", "it"] | [.., "from", "them"] | [.., "in", "it"] | [.., "in", "them"]
    )
}

fn preserve_choose_clause_it_reference(
    clause: SubjectVerbPrimitiveClause<'_>,
    filter: &mut ObjectFilter,
) {
    if !choose_clause_trails_from_it(clause) {
        return;
    }
    if filter.zone.is_none() || filter.zone == Some(Zone::Battlefield) {
        filter.zone = Some(Zone::Hand);
    }
    filter.controller = None;
    filter.owner = None;
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

pub(crate) fn parse_choose_then_choose_objects_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(CHOOSE_THEN_CHOOSE_OBJECTS_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_choose_then_choose_objects_sentence_matched(clause, &matched)
}

pub(crate) fn parse_choose_then_choose_objects_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let Some((first_player, mut first_filter, first_count)) =
        parse_choose_objects_clause_for_chain(head_clause)?
    else {
        return Ok(None);
    };
    let Some((mut second_player, mut second_filter, second_count)) =
        parse_choose_objects_clause_for_chain(tail_clause)?
    else {
        return Ok(None);
    };

    preserve_choose_clause_it_reference(head_clause, &mut first_filter);
    preserve_choose_clause_it_reference(tail_clause, &mut second_filter);

    if second_player == PlayerAst::Implicit {
        second_player = first_player.clone();
    }

    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: first_filter,
            count: first_count,
            count_value: None,
            player: first_player,
            tag: tag.clone(),
        },
        EffectAst::ChooseObjects {
            filter: second_filter,
            count: second_count,
            count_value: None,
            player: second_player,
            tag,
        },
    ]))
}

pub(crate) fn parse_sentence_return_then_do_same_for_subtypes(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_then_do_same_for_subtypes_sentence(clause)
}

pub(crate) fn parse_sentence_choose_then_choose_objects(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_choose_objects_sentence(clause)
}

pub(crate) fn parse_sentence_choose_then_do_same_for_filter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_do_same_for_filter_sentence(clause)
}

pub(crate) fn parse_sacrifice_any_number_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(SACRIFICE_ANY_NUMBER_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sacrifice_any_number_sentence_matched(clause, &matched)
}

pub(crate) fn parse_sacrifice_any_number_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        if head.is_empty() {
            return Ok(None);
        }
        (head, Some(tail))
    } else {
        (clause, None)
    };

    if !token_copy_clause_first_is_word(head_clause, SACRIFICE_WORD) {
        return Ok(None);
    }

    let Some((count, used)) = parse_choice_count_token_prefix_consumed(&head_clause.tokens()[1..])
    else {
        return Ok(None);
    };
    if count != ChoiceCount::any_number() {
        return Ok(None);
    }
    let idx = 1 + used;
    if idx >= head_clause.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            clause.text()
        )));
    }

    let filter_clause = head_clause.from(idx).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            clause.text()
        )));
    }

    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    let tag = TagKey::from(IT_TAG);

    let mut effects = vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_sacrifice_all(PlayerAst::Implicit, ObjectFilter::tagged(tag)),
    ];
    if let Some(tail_clause) = tail_clause
        && !tail_clause.is_empty()
    {
        let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
        effects.append(&mut tail_effects);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_sacrifice_any_number(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_any_number_sentence(clause)
}

pub(crate) fn parse_sacrifice_one_or_more_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(SACRIFICE_ONE_OR_MORE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sacrifice_one_or_more_sentence_matched(clause, &matched)
}

pub(crate) fn parse_sacrifice_one_or_more_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !token_copy_clause_first_is_word(clause, SACRIFICE_WORD) {
        return Ok(None);
    }

    let idx = 1usize;
    let Ok(Some((minimum, used))) =
        crate::runtime_backend::util::parse_greater_than_or_equal_quantity_prefix(
            clause.from(idx).tokens(),
            false,
            false,
            "sacrifice count",
        )
    else {
        return Ok(None);
    };
    let mut idx = idx + used;
    if clause
        .token(idx)
        .is_some_and(|token| TOKEN_COPY_OF_WORD_PATTERN.matches_token(token))
    {
        idx += 1;
    }
    if idx >= clause.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            clause.text()
        )));
    }

    let filter_clause = clause.from(idx).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            clause.text()
        )));
    }
    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::at_least(minimum as usize),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_sacrifice_all(PlayerAst::Implicit, ObjectFilter::tagged(tag)),
    ]))
}

pub(crate) fn parse_sentence_sacrifice_one_or_more(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_one_or_more_sentence(clause)
}

pub(crate) fn parse_sentence_keyword_then_chain(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(THEN_CHAIN_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_keyword_then_chain_matched(clause, &matched)
}

pub(crate) fn parse_sentence_keyword_then_chain_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };

    let Some(head_effect) = parse_keyword_mechanic_clause(head_clause.tokens())? else {
        return Ok(None);
    };

    if tail_clause.is_empty() {
        return Ok(Some(vec![head_effect]));
    }

    let mut effects = vec![head_effect];
    if let Some(mut counter_effects) = parse_sentence_put_counter_sequence(tail_clause)? {
        effects.append(&mut counter_effects);
        return Ok(Some(effects));
    }

    let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
    effects.append(&mut tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_chain_then_keyword(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(THEN_CHAIN_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_chain_then_keyword_matched(clause, &matched)
}

pub(crate) fn parse_sentence_chain_then_keyword_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let Some(keyword_effect) = parse_keyword_mechanic_clause(tail_clause.tokens())? else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    head_effects.push(keyword_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_return_then_create(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(RETURN_THEN_CREATE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_return_then_create_matched(clause, &matched)
}

pub(crate) fn parse_sentence_return_then_create_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = clause.split_once_on_then_trimmed();
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    if head_slice.is_empty() || tail_slice.is_empty() {
        return Ok(None);
    }

    if !token_copy_clause_first_is_word(head_slice, RETURN_WORD)
        || !token_copy_clause_first_is_word(tail_slice, CREATE_WORD)
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_slice.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(tail_slice.tokens())?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_exile_then_may_put_from_exile(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(EXILE_THEN_MAY_PUT_FROM_EXILE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_exile_then_may_put_from_exile_matched(clause, &matched)
}

pub(crate) fn parse_sentence_exile_then_may_put_from_exile_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = clause.split_once_on_then_trimmed();
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    if head_slice.is_empty() || tail_slice.is_empty() {
        return Ok(None);
    }

    let Some(put_tail) = tail_slice.strip_prefix(&["you", "may", "put"]) else {
        return Ok(None);
    };
    if parse_choice_count_token_prefix_consumed(put_tail).is_none()
        || !tail_slice.contains_word("from")
        || !tail_slice.contains_word("exile")
        || !tail_slice.contains_word("battlefield")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_slice.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    let mut tail_effects = parse_effect_chain(tail_slice.tokens())?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_then_shuffle_graveyard_into_library_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = clause.split_once_on_then_trimmed();
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    if head_slice.is_empty() || tail_slice.is_empty() {
        return Ok(None);
    }

    let head_words = head_slice.word_refs();
    if !head_words
        .first()
        .is_some_and(|word| EXILE_WORD_PATTERN.matches_word(word))
        && !YOU_EXILE_PREFIX_PATTERN.matches_words(&head_words)
    {
        return Ok(None);
    }

    let tail_words = tail_slice.word_refs();
    if !tail_words
        .first()
        .is_some_and(|word| SHUFFLE_OR_SHUFFLES_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if !tail_words
        .iter()
        .any(|word| GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN.matches_word(word))
        || !tail_words
            .iter()
            .any(|word| LIBRARY_OR_LIBRARIES_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_slice.tokens())?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Exile { .. },
                ..
            }) | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileAll { .. },
                ..
            }) | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileUntilSourceLeaves { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(tail_slice.tokens())?;
    if !tail_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ShuffleGraveyardIntoLibrary,
                ..
            })
        )
    }) {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_source_with_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "exile <source> with <counter descriptor> on it/them"
    let Some(after_exile) = clause.strip_prefix_clause(EXILE_PREFIX) else {
        return Ok(None);
    };
    let Some((source_name_clause, counter_clause)) = after_exile.split_once_on_word(WITH_WORD)
    else {
        return Ok(None);
    };

    let source_name_clause = source_name_clause.trimmed();
    if source_name_clause.is_empty() {
        return Ok(None);
    }
    let source_name_words = source_name_clause.word_refs();
    if !is_likely_named_or_source_reference_words(&source_name_words) {
        return Ok(None);
    }

    let counter_clause = counter_clause.trimmed();
    let Some(on_idx) = counter_clause.rfind_token_word("on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause.len() {
        return Ok(None);
    }

    let on_target_words = counter_clause.from(on_idx + 1).word_refs();
    if !IT_OR_THEM_PATTERN.matches_words(&on_target_words) {
        return Ok(None);
    }

    let descriptor_clause = counter_clause.before(on_idx).trimmed();
    if descriptor_clause.is_empty() {
        return Ok(None);
    }
    let (count, counter_type) = parse_counter_descriptor(descriptor_clause.tokens())?;

    let source_target = TargetAst::Source(clause.span());
    Ok(Some(vec![
        EffectAst::subject_verb_exile(source_target.clone(), false),
        EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            source_target,
            None,
            false,
        ),
    ]))
}

pub(crate) fn parse_sentence_exile_source_with_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_source_with_counters_sentence(clause)
}

pub(crate) fn parse_sentence_comma_then_chain_special(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        crate::runtime_backend::util::possessive_normalized_word_refs(words)
    }

    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let head_word_storage = head_clause.word_refs();
    let tail_word_storage = tail_clause.word_refs();
    let head_words = normalize_words(&head_word_storage);
    let tail_words = normalize_words(&tail_word_storage);
    let is_that_player_tail = THAT_PLAYER_TAIL_PREFIX_PATTERN.matches_words(&tail_words);
    let is_return_source_tail = RETURN_THIS_OWNER_HAND_TAIL_PATTERN.matches_words(&tail_words);
    let is_put_source_on_top_of_library_tail =
        PUT_THIS_OWNER_TOP_LIBRARY_TAIL_PATTERN.matches_words(&tail_words);
    let is_choose_card_name_tail = CHOOSE_CARD_NAME_TAIL_PREFIX_PATTERN.matches_words(&tail_words)
        && head_words
            .first()
            .is_some_and(|word| LOOK_WORD_PATTERN.matches_word(word));
    if !is_that_player_tail
        && !is_return_source_tail
        && !is_put_source_on_top_of_library_tail
        && !is_choose_card_name_tail
    {
        return Ok(None);
    }
    if is_return_source_tail
        && !head_words
            .first()
            .is_some_and(|word| TAP_OR_UNTAP_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if is_put_source_on_top_of_library_tail
        && !head_words
            .first()
            .is_some_and(|word| DRAW_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_destroy_then_land_controller_graveyard_count_damage_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let tail_words = tail_clause.word_refs();
    let suffix = [
        "damage",
        "to",
        "that",
        "lands",
        "controller",
        "equal",
        "to",
        "the",
        "number",
        "of",
        "land",
        "cards",
        "in",
        "that",
        "players",
        "graveyard",
    ];
    let Some(suffix_start) = tail_clause.find_phrase_start(&suffix) else {
        return Ok(None);
    };
    if suffix_start == 0
        || !TOKEN_COPY_DEAL_OR_DEALS_WORD_PATTERN.matches_word(tail_words[suffix_start - 1])
    {
        return Ok(None);
    }
    if suffix_start + suffix.len() != tail_words.len() {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Destroy { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    let mut count_filter = ObjectFilter::default();
    count_filter.zone = Some(Zone::Graveyard);
    let tagged_ref = crate::target::ObjectRef::tagged(IT_TAG);
    count_filter.owner = Some(PlayerFilter::ControllerOf(tagged_ref.clone()));
    count_filter.card_types.push(CardType::Land);
    head_effects.push(EffectAst::subject_verb_damage(
        Value::Count(count_filter),
        TargetAst::Player(PlayerFilter::ControllerOf(tagged_ref), tail_clause.span()),
    ));
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_destroy_all_attached_to_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "destroy all/each <filter> attached to <target>"
    let pattern = LexPattern::new(DESTROY_ALL_ATTACHED_TO_TARGET_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_destroy_all_attached_to_target_matched(clause, &matched)
}

pub(crate) fn parse_sentence_destroy_all_attached_to_target_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(filter_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Object)
        .map(|clause| clause.without_trailing_words_clause(&["that", "were", "was", "is", "are"]))
    else {
        return Ok(None);
    };
    let Some(target_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Tail)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let has_timing_tail = target_clause.contains_any_word(&[
        "at",
        "beginning",
        "end",
        "combat",
        "turn",
        "step",
        "until",
    ]);
    const SUPPORTED_THAT_TARGET_PREFIXES: &[&[&str]] = &[
        &["that", "creature"],
        &["that", "permanent"],
        &["that", "land"],
        &["that", "artifact"],
        &["that", "enchantment"],
    ];

    let supported_target = token_copy_clause_first_is_word(target_clause, TARGET_WORD)
        || target_clause.contains_word("you") && target_clause.len() == 1
        || target_clause.contains_word("it") && target_clause.len() == 1
        || target_clause
            .strip_any_prefix(SUPPORTED_THAT_TARGET_PREFIXES)
            .is_some();
    if filter_clause.is_empty() || target_clause.is_empty() || !supported_target || has_timing_tail
    {
        return Ok(None);
    }

    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    let target = parse_target_phrase(target_clause.tokens())?;
    Ok(Some(vec![EffectAst::subject_verb_destroy_all_attached_to(
        filter, target,
    )]))
}

pub(crate) fn parse_sentence_destroy_then_land_controller_graveyard_count_damage(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_then_land_controller_graveyard_count_damage_sentence(clause)
}

pub(crate) fn find_creature_type_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const CREATURE_TYPE_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "creature", "type", "of", "your", "choice"],
        &["of", "creature", "type", "of", "your", "choice"],
    ];
    clause.find_any_phrase_span(CREATURE_TYPE_CHOICE_PHRASES)
}

pub(super) fn find_type_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const TYPE_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "chosen", "type"],
        &["of", "chosen", "type"],
        &["of", "that", "type"],
        &["that", "type"],
    ];
    find_creature_type_choice_phrase(clause)
        .or_else(|| clause.find_any_phrase_span(TYPE_CHOICE_PHRASES))
}

pub(crate) fn find_color_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const COLOR_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "color", "of", "your", "choice"],
        &["of", "the", "color", "of", "their", "choice"],
        &["of", "color", "of", "your", "choice"],
        &["of", "color", "of", "their", "choice"],
    ];
    clause.find_any_phrase_span(COLOR_CHOICE_PHRASES)
}
