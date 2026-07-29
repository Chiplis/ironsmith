use super::super::grammar::effects as search_grammar;
use super::super::grammar::primitives as grammar;
use super::super::lexer::{OwnedLexToken, split_lexed_sentences, token_word_refs};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::util::{
    helper_tag_for_tokens, parse_number_word_u32, parse_subject, parse_target_phrase,
    span_from_tokens, strip_leading_token_words_any, trim_commas, trim_edge_punctuation,
};
use super::parse_effect_chain;
use super::sentence_helpers::*;
use crate::cards::builders::{
    CardTextError, CarryContext, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

const SEARCH_ENCHANT_WORD: &str = "enchant";
const SEARCH_EARTHBEND_WORD: &str = "earthbend";

#[derive(Clone)]
struct SearchZonePairClause {
    owner: PlayerFilter,
    first_zone: Zone,
    second_zone: Zone,
}

fn segment_starts_effect_lexed(tokens: &[OwnedLexToken]) -> bool {
    super::lex_chain_helpers::segment_has_effect_head_lexed(tokens)
}

pub(crate) fn parse_search_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn carry_conjugated_search_player(leading: &[EffectAst], search: &mut [EffectAst]) {
        let Some(CarryContext::Player(player)) = leading
            .iter()
            .rev()
            .find_map(super::chain_carry::explicit_player_for_carry)
        else {
            return;
        };
        let player = match player {
            PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
            player => player,
        };
        for effect in search {
            super::chain_carry::bind_implicit_player_context(effect, player);
        }
    }

    super::super::grammar::effects::parse_search_library_sentence_with_grammar_entrypoint_lexed(
        tokens,
        segment_starts_effect_lexed,
        super::chain_carry::parse_effect_chain_with_subject_verb_primitives_lexed,
        super::clause_dispatch::parse_effect_clause_lexed,
        carry_conjugated_search_player,
    )
}

pub(crate) fn parse_restriction_duration_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    Ok(
        search_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
            .map(|shape| (shape.duration, shape.remainder)),
    )
}

pub(crate) fn normalize_search_library_filter(filter: &mut ObjectFilter) {
    filter.zone = None;
    if filter.subtypes.iter().any(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
                | Subtype::Desert
        )
    }) && !filter
        .card_types
        .iter()
        .any(|card_type| *card_type == CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    for nested in &mut filter.any_of {
        normalize_search_library_filter(nested);
    }
}

pub(crate) fn parse_shuffle_graveyard_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_shuffle_graveyard_shape_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = shape.subject_tokens;
    let optional_shuffle = shape.optional_shuffle;
    let each_player_subject = shape.each_player_subject;
    let subject = if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else if each_player_subject {
        SubjectAst::Player(PlayerAst::Implicit)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This | SubjectAst::TriggeringSourceController => return Ok(None),
    };
    let owner_library_destination = shape.owner_library_destination;
    let trailing_tokens = trim_edge_punctuation(shape.trailing_tokens);
    let append_trailing =
        |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
            if trailing_tokens.is_empty() {
                return Ok(Some(effects));
            }
            // The shuffle shape deliberately captures the same-sentence tail
            // after "library" so a coordinated action such as "then draws
            // seven cards" can inherit the each-player subject. Do not carry
            // that grammatical subject through a real sentence boundary:
            // doing so turns a following "If it's your turn, end the turn"
            // into a trailing condition on the draw and makes each player end
            // the turn.
            let trailing_sentences = split_lexed_sentences(&trailing_tokens)
                .into_iter()
                .filter(|sentence| !sentence.is_empty())
                .collect::<Vec<_>>();
            let Some((same_sentence_tail, later_sentences)) = trailing_sentences.split_first()
            else {
                return Ok(Some(effects));
            };
            let mut trailing_effects = parse_effect_chain(same_sentence_tail)?;
            if each_player_subject {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player(effect, CarryContext::ForEachPlayer);
                }
                // These effects are still part of the same authored
                // each-player sentence. Keep them inside one player loop so
                // the renderer can retain the shared grammatical subject
                // ("..., then draws seven cards") without weakening the
                // executable per-player scope.
                for trailing_effect in trailing_effects {
                    match trailing_effect {
                        EffectAst::ForEachPlayer {
                            effects: mut trailing_player_effects,
                        } => {
                            if let Some(EffectAst::ForEachPlayer {
                                effects: player_effects,
                            }) = effects.last_mut()
                            {
                                player_effects.append(&mut trailing_player_effects);
                            } else {
                                effects.push(EffectAst::ForEachPlayer {
                                    effects: trailing_player_effects,
                                });
                            }
                        }
                        effect => effects.push(effect),
                    }
                }
            } else {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player_with_clause(
                        effect,
                        CarryContext::Player(player),
                        same_sentence_tail,
                    );
                }
                // The shuffle parser owns the leading action while the
                // generic chain parser owns the captured same-sentence tail.
                // When that tail is explicitly coordinated, keep the whole
                // authored clause behind one typed coordination boundary.
                // Otherwise lowering leaves the shuffle outside the
                // `SequenceEffect` and the renderer prints a false sentence
                // break before the remaining actions.
                if let (
                    [leading],
                    [
                        EffectAst::Coordinated {
                            effects: coordinated,
                            leading_duration,
                            result_conjunction,
                        },
                    ],
                ) = (effects.as_slice(), trailing_effects.as_slice())
                {
                    let mut combined = Vec::with_capacity(coordinated.len() + 1);
                    combined.push(leading.clone());
                    combined.extend(coordinated.iter().cloned());
                    effects = vec![EffectAst::Coordinated {
                        effects: combined,
                        leading_duration: *leading_duration,
                        result_conjunction: *result_conjunction,
                    }];
                } else {
                    effects.extend(trailing_effects);
                }
            }
            for sentence in later_sentences {
                effects.extend(super::parse_effect_sentences_lexed(sentence)?);
            }
            Ok(Some(effects))
        };
    let wrap_optional = |effects: Vec<EffectAst>| -> Vec<EffectAst> {
        if optional_shuffle {
            vec![EffectAst::MayByPlayer { player, effects }]
        } else {
            effects
        }
    };

    let target_tokens = shape.target_tokens;
    let has_target_selector = shape.has_target_selector;
    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    let explicit_all_cards_from = target_words
        .get(..3)
        .is_some_and(|prefix| prefix == ["all", "cards", "from"]);
    // "Shuffle all creature cards of that type from your graveyard ..." moves
    // a filtered subset, not the whole graveyard; only a bare possessive
    // graveyard phrase (optionally "all cards from ...") is the whole-zone
    // shuffle. A filtered phrase that fails to parse still falls back to the
    // whole-zone reading rather than failing the card.
    let whole_graveyard_target = {
        let rest: &[&str] = if explicit_all_cards_from {
            &target_words[3..]
        } else {
            &target_words[..]
        };
        matches!(
            rest,
            ["graveyard"]
                | ["your", "graveyard"]
                | ["their", "graveyard"]
                | ["their", "graveyards"]
                | ["his", "or", "her", "graveyard"]
                | ["that", "player's", "graveyard"]
                | ["each", "player's", "graveyard"]
        )
    };
    let filtered_graveyard_target = !has_target_selector
        && !whole_graveyard_target
        && !shape.has_source_and_graveyard_clause
        && !shape.has_hand_clause
        && parse_target_phrase(&target_tokens).is_ok();
    if !has_target_selector && !filtered_graveyard_target {
        let mut effects = Vec::new();
        let has_source_and_graveyard_clause = shape.has_source_and_graveyard_clause;
        let has_hand_clause = shape.has_hand_clause;
        if has_source_and_graveyard_clause {
            effects.push(EffectAst::subject_verb_move_to_zone(
                TargetAst::Source(None),
                Zone::Library,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
            if owner_library_destination {
                effects.push(EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::ShuffleLibrary,
                ));
            }
            effects.push(EffectAst::subject_verb_shuffle_graveyard_into_library(
                player,
            ));
        } else if has_hand_clause {
            let words = crate::runtime_backend::token_word_refs(tokens);
            let includes_owned_permanents = words
                .windows(4)
                .any(|window| matches!(window, ["all", "permanents", "you" | "they", "own"]));
            effects.push(if includes_owned_permanents {
                EffectAst::subject_verb_shuffle_hand_graveyard_and_owned_permanents_into_library(
                    player,
                )
            } else {
                EffectAst::subject_verb_shuffle_hand_and_graveyard_into_library(player)
            });
        } else {
            effects.push(
                EffectAst::subject_verb_shuffle_graveyard_into_library_with_surface(
                    player,
                    explicit_all_cards_from,
                ),
            );
        }
        if each_player_subject {
            return append_trailing(vec![EffectAst::ForEachPlayer {
                effects: wrap_optional(effects),
            }]);
        }
        return append_trailing(wrap_optional(effects));
    }

    let mut target = parse_target_phrase(&target_tokens)?;
    apply_shuffle_subject_graveyard_owner_context(&mut target, subject);
    let shuffle_player = if owner_library_destination {
        match super::zone_counter_helpers::target_object_filter_mut(&mut target)
            .and_then(|filter| filter.owner.as_ref())
        {
            Some(PlayerFilter::Target(target)) if **target == PlayerFilter::Opponent => {
                PlayerAst::TargetOpponent
            }
            Some(PlayerFilter::Target(_)) => PlayerAst::Target,
            Some(PlayerFilter::You) => PlayerAst::You,
            _ => player,
        }
    } else {
        player
    };

    let shuffle = if shuffle_target_moves_all(&target_tokens) {
        EffectAst::subject_verb_shuffle_all_objects_into_library(shuffle_player, target)
    } else {
        EffectAst::subject_verb_shuffle_objects_into_library(shuffle_player, target)
    };
    append_trailing(vec![shuffle])
}

pub(crate) fn parse_shuffle_object_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_shuffle_object_shape_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = shape.subject_tokens;
    let owner_of_subject_target = shape
        .owner_subject_target_tokens
        .as_deref()
        .map(parse_target_phrase)
        .transpose()?;

    let subject = if owner_of_subject_target.is_some() {
        SubjectAst::Player(PlayerAst::ItsOwner)
    } else if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This | SubjectAst::TriggeringSourceController => return Ok(None),
    };
    let owner_library_destination = shape.owner_library_destination;

    let trailing_tokens = shape.trailing_tokens.to_vec();
    let append_trailing =
        |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
            if trailing_tokens.is_empty() {
                return Ok(Some(effects));
            }
            let mut trailing_effects = parse_effect_chain(&trailing_tokens)?;
            for effect in &mut trailing_effects {
                maybe_apply_carried_player_with_clause(
                    effect,
                    CarryContext::Player(player),
                    &trailing_tokens,
                );
            }
            effects.extend(trailing_effects);
            Ok(Some(effects))
        };

    let target_tokens = shape.target_tokens;
    if let Some(target) = owner_of_subject_target {
        if shape.reference == search_grammar::SearchShuffleObjectReference::SingularBackReference {
            if !trailing_tokens.is_empty() {
                return append_trailing(vec![
                    EffectAst::subject_verb_move_to_zone(
                        target,
                        Zone::Library,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::LibraryOwner,
                        PlayerAst::ItsOwner,
                        SubjectVerbActionAst::ShuffleLibrary,
                    ),
                ]);
            }
            let shuffle = if owner_library_destination {
                EffectAst::subject_verb_shuffle_objects_into_owner_library(target)
            } else if shape.possessive_owner_subject {
                EffectAst::subject_verb_shuffle_objects_into_library_possessive_owner(target)
            } else {
                EffectAst::subject_verb_shuffle_objects_into_library(PlayerAst::ItsOwner, target)
            };
            return append_trailing(vec![shuffle]);
        }
        return Ok(None);
    }
    if matches!(subject, SubjectAst::Player(PlayerAst::ItsOwner))
        && shape.reference == search_grammar::SearchShuffleObjectReference::PluralTaggedReference
    {
        return append_trailing(vec![EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
                    Zone::Library,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::ShuffleLibrary,
                ),
            ],
        }]);
    }
    let target = parse_target_phrase(&target_tokens)?;
    let moves_all = shuffle_target_moves_all(&target_tokens);
    let shuffle = if owner_library_destination && moves_all {
        EffectAst::subject_verb_shuffle_all_objects_into_owner_library(target)
    } else if owner_library_destination {
        EffectAst::subject_verb_shuffle_objects_into_owner_library(target)
    } else if moves_all {
        EffectAst::subject_verb_shuffle_all_objects_into_library(player, target)
    } else {
        EffectAst::subject_verb_shuffle_objects_into_library(player, target)
    };

    append_trailing(vec![shuffle])
}

fn shuffle_target_moves_all(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    matches!(words.as_slice(), ["all" | "each", ..])
        || matches!(
            words.as_slice(),
            [
                "the",
                "cards" | "creatures" | "permanents" | "tokens" | "objects",
                ..
            ]
        )
}

pub(crate) fn parse_exile_hand_and_graveyard_bundle_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let trimmed_tokens = trim_commas(tokens);
    let clause_tokens = strip_leading_token_words_any(&trimmed_tokens, &["then", "and"]).to_vec();
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    if grammar::match_word_prefix(&clause_tokens, &["exile", "all", "cards", "from"]).is_none() {
        return Ok(None);
    }
    if !grammar::contains_word(&clause_tokens, "hand")
        && !grammar::contains_word(&clause_tokens, "hands")
    {
        return Ok(None);
    }
    if !grammar::contains_word(&clause_tokens, "graveyard")
        && !grammar::contains_word(&clause_tokens, "graveyards")
    {
        return Ok(None);
    }
    let Some(zone_pair) = parse_search_exile_all_cards_zone_pair(&clause_tokens) else {
        return Ok(None);
    };

    let mut first_filter = ObjectFilter::default().in_zone(zone_pair.first_zone);
    first_filter.owner = Some(zone_pair.owner.clone());
    let mut second_filter = ObjectFilter::default().in_zone(zone_pair.second_zone);
    second_filter.owner = Some(zone_pair.owner);

    Ok(Some(vec![
        EffectAst::subject_verb_exile_all(first_filter, false),
        EffectAst::subject_verb_exile_all(second_filter, false),
    ]))
}

fn parse_search_exile_all_cards_zone_pair(
    tokens: &[OwnedLexToken],
) -> Option<SearchZonePairClause> {
    search_grammar::parse_search_exile_zone_pair_shape_lexed(tokens).map(|shape| {
        SearchZonePairClause {
            owner: shape.owner,
            first_zone: shape.first_zone,
            second_zone: shape.second_zone,
        }
    })
}

pub(crate) fn parse_target_player_exiles_creature_and_graveyard_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_target_exile_bundle_shape_lexed(tokens) else {
        return Ok(None);
    };
    let subject_player = shape.player;
    let subject_filter = shape.filter;

    let mut creature_filter = ObjectFilter::creature();
    creature_filter.controller = Some(subject_filter.clone());

    let mut graveyard_filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    graveyard_filter.owner = Some(subject_filter);

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: creature_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: subject_player,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(TagKey::from(IT_TAG), None), false),
        EffectAst::subject_verb_exile_all(graveyard_filter, false),
    ]))
}

pub(crate) fn parse_for_each_exiled_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::Exiled {
        return Ok(None);
    }
    let words_all = token_word_refs(tokens);
    if shape.permanent_card_type_consult {
        let filter = ObjectFilter::permanent().shares_card_type_with_tagged(IT_TAG);
        let revealed_tag = helper_tag_for_tokens(tokens, "revealed");
        let matched_tag = helper_tag_for_tokens(tokens, "chosen");

        return Ok(Some(vec![EffectAst::ForEachTagged {
            tag: IT_TAG.into(),
            effects: vec![
                EffectAst::subject_verb_consult_top_of_library(
                    PlayerAst::Implicit,
                    LibraryConsultModeAst::Reveal,
                    filter,
                    LibraryConsultStopRuleAst::FirstMatch,
                    revealed_tag.clone(),
                    matched_tag.clone(),
                ),
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(matched_tag.clone(), None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    revealed_tag,
                    Some(matched_tag),
                    LibraryBottomOrderAst::Random,
                    PlayerAst::Implicit,
                ),
            ],
        }]));
    }

    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    if let Some(consult) = search_grammar::parse_search_exiled_consult_shape_lexed(effect_tokens) {
        let filter = parse_object_filter_lexed(consult.filter_tokens, false)?;
        let revealed_tag = helper_tag_for_tokens(tokens, "revealed");
        let matched_tag = helper_tag_for_tokens(tokens, "chosen");
        let finish = match consult.finish {
            search_grammar::SearchExiledConsultFinish::Shuffle => EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::ItsController,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
            search_grammar::SearchExiledConsultFinish::PutRestOnBottom => {
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    revealed_tag.clone(),
                    Some(matched_tag.clone()),
                    LibraryBottomOrderAst::Random,
                    PlayerAst::ItsController,
                )
            }
        };
        return Ok(Some(vec![EffectAst::ForEachTagged {
            tag: crate::tag::SOURCE_EXILED_TAG.into(),
            effects: vec![
                EffectAst::subject_verb_consult_top_of_library(
                    PlayerAst::ItsController,
                    LibraryConsultModeAst::Reveal,
                    filter,
                    LibraryConsultStopRuleAst::FirstMatch,
                    revealed_tag,
                    matched_tag.clone(),
                ),
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(matched_tag, None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                finish,
            ],
        }]));
    }
    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_each_player_put_permanent_cards_exiled_with_source_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !search_grammar::search_each_player_exiled_permanents_shape_lexed(tokens) {
        return Ok(None);
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![EffectAst::subject_verb_put_all_onto_battlefield(
            filter,
            false,
            false,
            ReturnControllerAst::Owner,
        )],
    }]))
}

pub(crate) fn parse_for_each_destroyed_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::DestroyedOrDied {
        return Ok(None);
    }
    let words_all = token_word_refs(tokens);
    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

fn bind_sacrificed_snapshot_controller(effect: &mut EffectAst) {
    match effect {
        EffectAst::MayByPlayer { player, .. } if *player == PlayerAst::ItsController => {
            *player = PlayerAst::That;
        }
        EffectAst::SubjectVerb(subject_verb) => {
            if subject_verb.subject.player == PlayerAst::ItsController {
                subject_verb.subject.player = PlayerAst::That;
            }
            if let SubjectVerbActionAst::SearchLibrary { player, .. } = &mut subject_verb.action
                && *player == PlayerAst::ItsController
            {
                *player = PlayerAst::That;
            }
        }
        _ => {}
    }

    crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
        effect,
        true,
        |nested| {
            for nested_effect in nested {
                bind_sacrificed_snapshot_controller(nested_effect);
            }
        },
    );
}

/// Parse a typed iterator over the actual objects sacrificed by the preceding
/// instruction.  The last-known predicate deliberately stays inside the
/// object loop: it preserves both the exact sacrificed subset and the
/// controller each object had before leaving the battlefield.
pub(crate) fn parse_for_each_sacrificed_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::Sacrificed {
        return Ok(None);
    }
    let filter_tokens = shape.iterated_filter_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing object type in sacrificed-this-way iterator (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty object type in sacrificed-this-way iterator (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
    // Sacrifice result predicates use the permanent's battlefield snapshot,
    // not its current graveyard object.
    filter.zone = None;

    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... sacrificed this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after sacrificed-this-way iterator (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    let mut effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after sacrificed-this-way iterator (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    for effect in &mut effects {
        bind_sacrificed_snapshot_controller(effect);
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::ItMatchedLastKnown(filter),
            if_true: effects,
            if_false: Vec::new(),
        }],
    }]))
}

pub(crate) fn parse_for_each_put_into_graveyard_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::PutIntoGraveyard {
        return Ok(None);
    }
    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_earthbend_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if !words
        .first()
        .is_some_and(|word| *word == SEARCH_EARTHBEND_WORD)
    {
        return Ok(None);
    }

    let count = parse_number(tokens.get(1..).unwrap_or_default())
        .map(|(value, _)| value)
        .or_else(|| words.get(1).and_then(|word| parse_number_word_u32(word)))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing earthbend count (clause: '{}')",
                words.join(" ")
            ))
        })?;

    Ok(Some(EffectAst::subject_verb_earthbend(count)))
}

pub(crate) fn parse_enchant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if words.is_empty() || words[0] != SEARCH_ENCHANT_WORD {
        return Ok(None);
    }

    let remaining = if tokens.len() > 1 { &tokens[1..] } else { &[] };
    let filter = match words.get(1..) {
        Some(["player"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::Any),
        Some(["opponent"]) | Some(["an", "opponent"]) => {
            crate::object::AuraAttachmentFilter::Player(PlayerFilter::Opponent)
        }
        Some(["you"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::You),
        _ => crate::object::AuraAttachmentFilter::Object(parse_object_filter(remaining, false)?),
    };
    Ok(Some(EffectAst::subject_verb_enchant(filter)))
}

pub(crate) fn parse_restriction_duration(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    parse_restriction_duration_lexed(tokens)
}
