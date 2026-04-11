use winnow::combinator::{alt, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::take_till;

use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, ReturnControllerAst, SubjectAst, TagKey, TargetAst,
    TextSpan,
};
use crate::effect::SearchSelectionMode;
use crate::target::PlayerFilter;
use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

use super::super::activation_and_restrictions::{
    parse_cant_restriction_clause, parse_cant_restrictions,
};
use super::super::grammar::structure::{IfClausePredicateSpec, split_if_clause_lexed};
use super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, parser_token_word_positions, parser_token_word_refs,
    split_lexed_sentences, token_word_refs,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::search_library_support::{
    apply_search_library_mana_constraint, extract_search_library_mana_constraint,
    is_same_name_that_reference_words, normalize_search_library_filter,
    parse_restriction_duration_lexed, parse_search_library_disjunction_filter,
    split_search_same_name_reference_filter, word_slice_mentions_nth_from_top,
    word_slice_starts_with_any, zone_slice_contains,
};
use super::super::token_primitives::{
    contains_window as word_slice_contains_sequence, find_any_str_index as word_slice_find_any,
    find_index as find_token_index, find_str_index as word_slice_find,
    find_window_index as word_slice_find_sequence, rfind_index as rfind_token_index,
    slice_contains_all as word_slice_has_all, slice_contains_any as word_slice_contains_any,
    slice_contains_str as word_slice_contains, slice_ends_with as word_slice_ends_with,
    slice_starts_with as word_slice_starts_with,
};
use super::super::util::{
    is_article, parse_number, parse_subject, parse_target_phrase, span_from_tokens, trim_commas,
};
use super::primitives;

#[path = "effects/search_library.rs"]
mod search_library;
pub(crate) use search_library::*;

pub(crate) fn cant_sentence_clause_tokens_for_restriction_scan_lexed(
    clause_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    split_lexed_sentences(clause_tokens)
        .into_iter()
        .next()
        .unwrap_or(clause_tokens)
        .to_vec()
}

pub(crate) fn cant_sentence_has_supported_negation_gate_lexed(
    clause_tokens: &[OwnedLexToken],
) -> bool {
    let Some((neg_start, _)) = find_cant_sentence_negation_span_lexed(clause_tokens) else {
        return false;
    };

    !clause_tokens[..neg_start]
        .iter()
        .any(|token| token.is_word("and"))
}

pub(crate) fn find_cant_sentence_negation_span_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(usize, usize)> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.is_word("can't") || token.is_word("cant") || token.is_word("cannot") {
            return Some((cursor, cursor + 1));
        }
        if token.is_word("doesn't")
            || token.is_word("doesnt")
            || token.is_word("don't")
            || token.is_word("dont")
        {
            if matches!(
                tokens.get(cursor + 1).map(|next| next.parser_text.as_str()),
                Some("control" | "controls" | "own" | "owns")
            ) {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 1));
        }
        if (token.is_word("does") || token.is_word("do") || token.is_word("can"))
            && tokens
                .get(cursor + 1)
                .is_some_and(|next| next.is_word("not"))
        {
            if (token.is_word("does") || token.is_word("do"))
                && matches!(
                    tokens.get(cursor + 2).map(|next| next.parser_text.as_str()),
                    Some("control" | "controls" | "own" | "owns")
                )
            {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 2));
        }
        cursor += 1;
    }

    None
}

fn cant_sentence_next_turn_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["during", "that", "players", "next", "turn"]),
        primitives::phrase(&["during", "that", "player's", "next", "turn"]),
        primitives::phrase(&["during", "that", "player", "s", "next", "turn"]),
    ))
    .void()
    .parse_next(input)
}

fn cant_sentence_for_as_long_as_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["for", "as", "long", "as"])
        .void()
        .parse_next(input)
}

pub(crate) fn split_cant_sentence_next_turn_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let Some((_, rest)) =
            primitives::parse_prefix(&tokens[cursor..], cant_sentence_next_turn_suffix)
        else {
            cursor += 1;
            continue;
        };
        if rest.iter().all(|token| token.is_period()) {
            return Some(tokens[..cursor].to_vec());
        }
        cursor += 1;
    }

    None
}

#[derive(Debug, Clone)]
pub(crate) struct CantSentencePreparedClause {
    pub(crate) duration: crate::effect::Until,
    pub(crate) clause_tokens: Vec<OwnedLexToken>,
}

pub(crate) fn prepare_cant_sentence_restriction_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<CantSentencePreparedClause>, CardTextError> {
    let Some((duration, clause_tokens)) = parse_restriction_duration_lexed(tokens)? else {
        return Ok(None);
    };
    if clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "restriction clause missing body".to_string(),
        ));
    }
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        return Ok(None);
    }

    let clause_tokens = cant_sentence_clause_tokens_for_restriction_scan_lexed(&clause_tokens);
    if !cant_sentence_has_supported_negation_gate_lexed(&clause_tokens) {
        return Ok(None);
    }

    Ok(Some(CantSentencePreparedClause {
        duration,
        clause_tokens,
    }))
}

fn conditional_label_delimiter<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::token_kind(TokenKind::Dash).void(),
        primitives::token_kind(TokenKind::EmDash).void(),
    ))
    .parse_next(input)
}

fn conditional_sentence_family_head<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["then", "if"]),
        (
            conditional_label_phrase,
            opt(conditional_label_delimiter),
            primitives::kw("if"),
        )
            .void(),
        primitives::kw("if").void(),
    ))
    .parse_next(input)
}

pub(crate) fn split_conditional_sentence_family_head_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, conditional_sentence_family_head)?;
    let consumed = tokens.len().checked_sub(rest.len())?;
    consumed.checked_sub(1).map(|if_idx| &tokens[if_idx..])
}

pub(crate) fn parse_conditional_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let split = split_if_clause_lexed(tokens, parse_effect_chain_lexed)?;

    Ok(vec![match split.predicate {
        IfClausePredicateSpec::Conditional(predicate) => EffectAst::Conditional {
            predicate,
            if_true: split.effects,
            if_false: Vec::new(),
        },
        IfClausePredicateSpec::Result(predicate) => EffectAst::IfResult {
            predicate,
            effects: split.effects,
        },
    }])
}

pub(crate) fn parse_conditional_sentence_family_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(conditional_tokens) = split_conditional_sentence_family_head_lexed(tokens) else {
        return Ok(None);
    };

    parse_conditional_sentence_with_grammar_entrypoint_lexed(
        conditional_tokens,
        parse_effect_chain_lexed,
    )
    .map(Some)
}

pub(crate) fn parse_cant_effect_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(prefix_tokens) = split_cant_sentence_next_turn_prefix_lexed(tokens) {
        let prefix_tokens = prefix_tokens.as_slice();
        if let Some(parsed) = parse_cant_restriction_clause(prefix_tokens)? {
            let next_turn_effects = match parsed.restriction {
                crate::effect::Restriction::CastSpellsMatching(player, spell_filter) => {
                    let nested = crate::effect::Restriction::cast_spells_matching(
                        PlayerFilter::Active,
                        spell_filter,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: crate::effect::Until::EndOfTurn,
                                    condition: None,
                                }],
                            }],
                        }]),
                        PlayerFilter::IteratedPlayer => {
                            Some(vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: crate::effect::Until::EndOfTurn,
                                    condition: None,
                                }],
                            }])
                        }
                        _ => None,
                    }
                }
                crate::effect::Restriction::CastMoreThanOneSpellEachTurn(player, spell_filter) => {
                    let nested = crate::effect::Restriction::CastMoreThanOneSpellEachTurn(
                        PlayerFilter::Active,
                        spell_filter,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: crate::effect::Until::EndOfTurn,
                                    condition: None,
                                }],
                            }],
                        }]),
                        PlayerFilter::IteratedPlayer => {
                            Some(vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: crate::effect::Until::EndOfTurn,
                                    condition: None,
                                }],
                            }])
                        }
                        _ => None,
                    }
                }
                _ => None,
            };

            if let Some(next_turn_effects) = next_turn_effects {
                return Ok(Some(next_turn_effects));
            }
        }
    }

    let source_tapped_duration = cant_sentence_has_source_remains_tapped_duration(tokens);
    let Some(prepared_clause) = prepare_cant_sentence_restriction_clause_lexed(tokens)? else {
        return Ok(None);
    };
    let duration = prepared_clause.duration;
    let clause_tokens = prepared_clause.clause_tokens;

    let Some(restrictions) = parse_cant_restrictions(&clause_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported restriction clause body (clause: '{}')",
            token_word_refs(&clause_tokens).join(" ")
        )));
    };

    let mut target: Option<crate::cards::builders::TargetAst> = None;
    let mut effects = Vec::new();
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target {
                if *existing != parsed_target {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported mixed restriction targets (clause: '{}')",
                        token_word_refs(&clause_tokens).join(" ")
                    )));
                }
            } else {
                target = Some(parsed_target);
            }
        }
        effects.push(EffectAst::Cant {
            restriction: parsed.restriction,
            duration: duration.clone(),
            condition: source_tapped_duration.then_some(crate::ConditionExpr::SourceIsTapped),
        });
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::TargetOnly { target });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_cant_effect_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence_with_grammar_entrypoint_lexed(tokens)
}

pub(crate) fn parse_search_library_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    subject_starts_effect_lexed: fn(&[OwnedLexToken]) -> bool,
    parse_leading_effects_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
    parse_effect_clause_lexed: fn(&[OwnedLexToken]) -> Result<EffectAst, CardTextError>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn has_trailing_that_player_shuffle(tokens: &[OwnedLexToken]) -> bool {
        super::primitives::words_find_phrase(tokens, &["then", "that", "player", "shuffle"])
            .is_some()
            || super::primitives::words_find_phrase(tokens, &["then", "that", "player", "shuffles"])
                .is_some()
            || super::primitives::words_find_phrase(tokens, &["that", "player", "shuffle"])
                .is_some()
            || super::primitives::words_find_phrase(tokens, &["that", "player", "shuffles"])
                .is_some()
    }

    let words_all = parser_text_word_refs(tokens);
    let Some(head_split) = split_search_library_sentence_head_lexed(tokens) else {
        return Ok(None);
    };

    let subject_prelude = parse_search_library_leading_effect_prelude_lexed(
        head_split.subject_tokens,
        subject_starts_effect_lexed,
        parse_leading_effects_lexed,
    )?;
    let subject_tokens = subject_prelude.subject_tokens;
    let sentence_has_direct_may = head_split.sentence_has_direct_may;
    let mut leading_effects = subject_prelude.leading_effects;
    let wrap_each_target_player =
        search_library_subject_wraps_each_target_player_lexed(subject_tokens);
    let chooser = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let search_tokens = head_split.search_tokens;
    if !search_library_starts_with_search_verb_lexed(search_tokens) {
        return Ok(None);
    }
    let search_words = parser_text_word_refs(search_tokens);
    if search_words.is_empty() {
        return Ok(None);
    }
    let Some(subject_routing) = derive_search_library_subject_routing_lexed(search_tokens, chooser)
    else {
        return Ok(None);
    };
    let player = subject_routing.player;
    let search_player_target = subject_routing.search_player_target;
    let forced_library_owner = subject_routing.forced_library_owner;
    let search_zones_override = subject_routing.search_zones_override;
    if search_library_has_unsupported_top_position_probe(&search_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported search-library top-position clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let clause_markers = scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned search-library clause marker scan should produce defaults");
    let for_idx = clause_markers.for_idx;
    let put_idx = clause_markers.put_idx;
    let has_explicit_destination = clause_markers.has_explicit_destination;
    let filter_boundary = clause_markers.filter_boundary;

    let filter_end =
        find_search_library_filter_boundary_lexed(search_tokens, for_idx, filter_boundary)
            .filter_end;

    if filter_end <= for_idx + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing search filter in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let count_tokens = &search_tokens[for_idx + 1..filter_end];
    let count_prefix = parse_search_library_count_prefix_lexed(count_tokens);
    let count = count_prefix.count;
    let search_mode = count_prefix.search_mode;
    let count_used = count_prefix.count_used;

    let filter_start = for_idx + 1 + count_used;
    if filter_start >= filter_end {
        return Err(CardTextError::ParseError(format!(
            "missing object selector in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let raw_filter_tokens = trim_commas(&search_tokens[filter_start..filter_end]);
    let (filter_tokens, mana_constraint) = if let Some((base_filter_tokens, mana_constraint)) =
        extract_search_library_mana_constraint(&raw_filter_tokens)
    {
        (base_filter_tokens, Some(mana_constraint))
    } else {
        (raw_filter_tokens.clone(), None)
    };
    let same_name_split = parse_search_library_same_name_reference_lexed(
        &raw_filter_tokens,
        filter_tokens,
        &words_all,
    )?;
    let filter_tokens = same_name_split.filter_tokens;
    let same_name_reference = same_name_split.same_name_reference;

    let named_filters = if count_used == 0 {
        split_search_named_item_filters_lexed(&filter_tokens, &words_all)?
    } else {
        None
    };
    let mut filter = parse_search_library_object_filter_lexed(&filter_tokens, &words_all)?;
    if let Some(same_name_tag) = same_name_reference
        .as_ref()
        .map(|reference| match reference {
            SearchLibrarySameNameReference::Tagged(tag) => tag.clone(),
            SearchLibrarySameNameReference::Target(_) => TagKey::from(IT_TAG),
            SearchLibrarySameNameReference::Choose { tag, .. } => tag.clone(),
        })
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: same_name_tag.clone(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    }
    if filter.owner.is_none()
        && let Some(owner) = forced_library_owner.clone()
    {
        filter.owner = Some(owner);
    }
    normalize_search_library_filter(&mut filter);
    if let Some(mana_constraint) = mana_constraint {
        apply_search_library_mana_constraint(&mut filter, mana_constraint);
    }

    let discard_before_shuffle_followup =
        find_search_library_discard_before_shuffle_followup_lexed(search_tokens, put_idx);
    let trailing_discard_before_shuffle = discard_before_shuffle_followup.is_some();
    let effect_routing = derive_search_library_effect_routing_lexed(
        tokens,
        search_tokens,
        clause_markers,
        trailing_discard_before_shuffle,
    );
    let destination = effect_routing.destination;
    let reveal = effect_routing.reveal;
    let face_down_exile = effect_routing.face_down_exile;
    let shuffle = effect_routing.shuffle;
    let split_battlefield_and_hand = effect_routing.split_battlefield_and_hand;
    let mut effects = if let Some(named_filters) = named_filters {
        let searched_tag: TagKey = "searched_named".into();
        let zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let mut sequence = Vec::new();
        for mut named_filter in named_filters {
            if named_filter.owner.is_none()
                && let Some(owner) = forced_library_owner.clone()
            {
                named_filter.owner = Some(owner);
            }
            normalize_search_library_filter(&mut named_filter);
            sequence.push(EffectAst::ChooseObjectsAcrossZones {
                filter: named_filter,
                count: ChoiceCount::exactly(1),
                player: chooser,
                tag: searched_tag.clone(),
                zones: zones.clone(),
                search_mode: Some(SearchSelectionMode::Exact),
            });
        }
        if reveal {
            sequence.push(EffectAst::RevealTagged {
                tag: searched_tag.clone(),
            });
        }
        sequence.push(EffectAst::MoveToZone {
            target: TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
            zone: destination,
            to_top: matches!(destination, Zone::Library),
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: destination == Zone::Battlefield
                && effect_routing.has_tapped_modifier,
            attached_to: None,
        });
        if shuffle && zones.contains(&Zone::Library) {
            sequence.push(EffectAst::ShuffleLibrary { player });
        }
        sequence
    } else if !has_explicit_destination {
        let chosen_tag: TagKey = "searched".into();
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones_override.unwrap_or_else(|| vec![Zone::Library]),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::RevealTagged {
                tag: chosen_tag.clone(),
            });
        }
        if shuffle {
            sequence.push(EffectAst::ShuffleLibrary { player });
        }
        sequence
    } else if let Some(search_zones) = search_zones_override.clone() {
        let chosen_tag: TagKey = "searched_multi_zone".into();
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        let shuffle_player = PlayerAst::That;
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::RevealTagged {
                tag: chosen_tag.clone(),
            });
        }
        if shuffle
            && destination == Zone::Library
            && zone_slice_contains(&search_zones, Zone::Library)
        {
            sequence.push(EffectAst::ShuffleLibrary {
                player: shuffle_player,
            });
        }
        sequence.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::MoveToZone {
                target: TargetAst::Tagged(chosen_tag, span_from_tokens(tokens)),
                zone: destination,
                to_top: matches!(destination, Zone::Library),
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped,
                attached_to: None,
            }],
        });
        if shuffle
            && !(destination == Zone::Library && zone_slice_contains(&search_zones, Zone::Library))
        {
            sequence.push(EffectAst::ShuffleLibrary {
                player: shuffle_player,
            });
        }
        sequence
    } else if split_battlefield_and_hand {
        let battlefield_tapped = effect_routing.has_tapped_modifier;
        vec![
            EffectAst::SearchLibrary {
                filter: filter.clone(),
                destination: Zone::Battlefield,
                chooser,
                player,
                search_mode,
                reveal,
                shuffle: false,
                count: ChoiceCount::up_to(1),
                count_value: None,
                tapped: battlefield_tapped,
            },
            EffectAst::SearchLibrary {
                filter,
                destination: Zone::Hand,
                chooser,
                player,
                search_mode,
                reveal,
                shuffle,
                count: ChoiceCount::up_to(1),
                count_value: None,
                tapped: false,
            },
        ]
    } else if destination == Zone::Exile && face_down_exile {
        let searched_tag: TagKey = "searched_face_down".into();
        let mut sequence = vec![
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                player: chooser,
                tag: searched_tag.clone(),
                zones: vec![Zone::Library],
                search_mode: Some(search_mode),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
                face_down: true,
            },
        ];
        if shuffle {
            sequence.push(EffectAst::ShuffleLibrary { player });
        }
        sequence
    } else {
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        vec![EffectAst::SearchLibrary {
            filter,
            destination,
            chooser,
            player,
            search_mode,
            reveal,
            shuffle,
            count,
            count_value: None,
            tapped: battlefield_tapped,
        }]
    };

    if let Some(discard_followup) = discard_before_shuffle_followup {
        let discard_tokens =
            trim_commas(&search_tokens[discard_followup.discard_idx..discard_followup.discard_end]);
        if !discard_tokens.is_empty() {
            effects.push(parse_effect_clause_lexed(&discard_tokens)?);
        }
        effects.push(EffectAst::ShuffleLibrary { player });
    }

    if has_trailing_that_player_shuffle(tokens) {
        let mut rewrote_existing_shuffle = false;
        for effect in &mut effects {
            if let EffectAst::ShuffleLibrary { player } = effect
                && matches!(*player, PlayerAst::You | PlayerAst::Implicit)
            {
                *player = PlayerAst::That;
                rewrote_existing_shuffle = true;
            }
        }
        if !rewrote_existing_shuffle {
            effects.push(EffectAst::ShuffleLibrary {
                player: PlayerAst::That,
            });
        }
    }

    if let Some(target) = search_player_target {
        effects.insert(0, EffectAst::TargetOnly { target });
    }

    if let Some(trailing_tokens) = find_search_library_trailing_life_followup_lexed(
        search_tokens,
        put_idx.unwrap_or(filter_boundary),
    ) {
        let trailing_effect = parse_effect_clause_lexed(trailing_tokens)?;
        effects.push(trailing_effect);
    }

    if let Some(reference) = same_name_reference {
        match reference {
            SearchLibrarySameNameReference::Tagged(_) => {}
            SearchLibrarySameNameReference::Target(target) => {
                effects.insert(0, EffectAst::TargetOnly { target });
            }
            SearchLibrarySameNameReference::Choose { filter, tag } => {
                effects.insert(
                    0,
                    EffectAst::ChooseObjects {
                        filter,
                        count: ChoiceCount::exactly(1),
                        count_value: None,
                        player,
                        tag,
                    },
                );
            }
        }
    }

    if sentence_has_direct_may {
        effects = vec![if matches!(chooser, PlayerAst::You | PlayerAst::Implicit) {
            EffectAst::May { effects }
        } else {
            EffectAst::MayByPlayer {
                player: chooser,
                effects,
            }
        }];
    }

    if !leading_effects.is_empty() {
        leading_effects.extend(effects);
        return Ok(Some(leading_effects));
    }

    if wrap_each_target_player {
        effects = vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::target_player(),
            effects,
        }];
    }

    Ok(Some(effects))
}
pub(crate) fn cant_sentence_has_source_remains_tapped_duration(tokens: &[OwnedLexToken]) -> bool {
    let mut has_for_as_long_as = false;
    let mut has_remains = false;
    let mut has_tapped = false;
    let mut has_source_word = false;
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        if !has_for_as_long_as
            && primitives::parse_prefix(&tokens[cursor..], cant_sentence_for_as_long_as_marker)
                .is_some()
        {
            has_for_as_long_as = true;
        }

        let token = &tokens[cursor];
        has_remains |= token.is_word("remains");
        has_tapped |= token.is_word("tapped");
        has_source_word |= matches!(
            token.parser_text.as_str(),
            "this" | "source" | "artifact" | "creature" | "permanent"
        );
        cursor += 1;
    }

    has_for_as_long_as && has_remains && has_tapped && has_source_word
}
