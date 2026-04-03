use super::super::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut, try_for_each_nested_effects_mut,
};
use super::super::grammar::primitives::{self as grammar, CompatWordIndex};
use super::super::keyword_static::parse_where_x_value_clause;
use super::super::lexer::{OwnedLexToken, TokenKind, split_lexed_sentences};
use super::super::object_filters::{is_comparison_or_delimiter, parse_object_filter};
use super::super::token_primitives::{
    TurnDurationPhrase, lexed_head_words, parse_turn_duration_prefix, parse_value_comparison_tokens,
};
use super::super::util::{
    helper_tag_for_tokens, is_article, mana_pips_from_token, parse_number, parse_subject,
    parse_target_phrase, span_from_tokens, token_index_for_word_index, trim_commas, words,
};
use super::super::value_helpers::parse_value_from_lexed;
use super::sentence_helpers::*;
use super::{
    find_verb, parse_effect_sentence_lexed, parse_search_library_disjunction_filter,
    parse_token_copy_modifier_sentence, trim_edge_punctuation,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, CarryContext, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate,
    InsteadSemantics, KeywordAction, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectAst, TagKey,
    TargetAst, TextSpan, TokenCopyFollowup, ZoneReplacementDurationAst,
};
use crate::cards::builders::{
    find_index, find_window_by, find_window_index, slice_contains, slice_ends_with,
    slice_starts_with, str_contains, str_ends_with, str_starts_with,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::filter::Comparison;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::zone::Zone;
use std::cell::OnceCell;

type PairSentenceRule =
    fn(&[OwnedLexToken], &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError>;
type TripleSentenceRule = fn(
    &[OwnedLexToken],
    &[OwnedLexToken],
    &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError>;
type QuadSentenceRule = fn(
    &[OwnedLexToken],
    &[OwnedLexToken],
    &[OwnedLexToken],
    &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError>;

fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
    )
}

fn attach_copy_cost_reduction_to_effect(
    effect: &mut EffectAst,
    reduction: &crate::mana::ManaCost,
) -> bool {
    match effect {
        EffectAst::CastTagged {
            as_copy,
            cost_reduction,
            ..
        } if *as_copy => {
            *cost_reduction = Some(reduction.clone());
            true
        }
        _ => {
            let mut attached = false;
            for_each_nested_effects_mut(effect, true, |nested| {
                if attached {
                    return;
                }
                for nested_effect in nested.iter_mut().rev() {
                    if attach_copy_cost_reduction_to_effect(nested_effect, reduction) {
                        attached = true;
                        break;
                    }
                }
            });
            attached
        }
    }
}

fn attach_copy_cost_reduction_to_effects(
    effects: &mut [EffectAst],
    reduction: &crate::mana::ManaCost,
) -> bool {
    for effect in effects.iter_mut().rev() {
        if attach_copy_cost_reduction_to_effect(effect, reduction) {
            return true;
        }
    }
    false
}

fn parse_same_sentence_copy_and_may_cast_copy(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<(
        Vec<EffectAst>,
        crate::cards::builders::parser::activation_and_restrictions::MayCastTaggedSpec,
    )>,
    CardTextError,
> {
    let Some(split_idx) = find_index(tokens, |token: &OwnedLexToken| {
        token.is_word("and") || token.is_word("then")
    }) else {
        return Ok(None);
    };

    let copy_tokens = trim_commas(&tokens[..split_idx]).to_vec();
    if !is_simple_copy_reference_sentence(&copy_tokens) {
        return Ok(None);
    }

    let tail_tokens = trim_commas(&tokens[split_idx + 1..]).to_vec();
    let Some(spec) = parse_may_cast_it_sentence(&tail_tokens) else {
        return Ok(None);
    };
    if !spec.as_copy {
        return Ok(None);
    }

    let copy_effects = parse_effect_sentence_lexed(&copy_tokens)?;
    Ok(Some((copy_effects, spec)))
}

fn parse_single_effect_sentence(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_effect_sentence_lexed(tokens)?
        .into_iter()
        .next()
        .ok_or_else(|| CardTextError::ParseError("missing effect sentence".to_string()))
}

fn try_parse_divvy_sentence_sequence(
    sentences: &[SentenceInput],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let normalized = sentences
        .iter()
        .map(|sentence| {
            crate::cards::builders::parser::token_word_refs(sentence.lowered()).join(" ")
        })
        .collect::<Vec<_>>();
    let joined = normalized.join(". ");

    if joined
        == "separate all creatures target player controls into two piles. destroy all creatures in the pile of that player's choice. they can't be regenerated"
    {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::target_player()),
                count: ChoiceCount::any_number(),
                player: PlayerAst::Target,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::DestroyNoRegeneration {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            },
        ]));
    }

    if joined
        == "separate all creature cards in your graveyard into two piles. exile the pile of an opponent's choice and return the other to the battlefield"
    {
        let mut graveyard_creatures = ObjectFilter::creature();
        graveyard_creatures.zone = Some(Zone::Graveyard);
        graveyard_creatures.owner = Some(PlayerFilter::You);
        let rest_filter = graveyard_creatures
            .clone()
            .not_tagged(TagKey::from("divvy_chosen"));
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: graveyard_creatures,
                count: ChoiceCount::any_number(),
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                face_down: false,
            },
            EffectAst::ReturnAllToBattlefield {
                filter: rest_filter,
                tapped: false,
            },
        ]));
    }

    if str_starts_with(
        &joined,
        "each opponent separates the creatures they control into two piles",
    ) && str_contains(&joined, "for each opponent")
        && str_contains(
            &joined,
            "each opponent sacrifices the creatures in their chosen pile",
        )
    {
        return Ok(Some(vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::Opponent,
            effects: vec![
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    player: PlayerAst::You,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::SacrificeAll {
                    filter: ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .match_tagged(
                            TagKey::from("divvy_chosen"),
                            TaggedOpbjectRelation::IsTaggedObject,
                        ),
                    player: PlayerAst::Implicit,
                },
            ],
        }]));
    }

    if str_starts_with(
        &joined,
        "separate all permanents target player controls into two piles",
    ) && str_contains(
        &joined,
        "that player sacrifices all permanents in the pile of their choice",
    ) {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::permanent().controlled_by(PlayerFilter::target_player()),
                count: ChoiceCount::any_number(),
                player: PlayerAst::Target,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::SacrificeAll {
                filter: ObjectFilter::tagged(TagKey::from("divvy_chosen")),
                player: PlayerAst::Target,
            },
        ]));
    }

    if joined
        == "for each defending player separate all creatures that player controls into two piles and that player chooses one. only creatures in the chosen piles can block this turn"
    {
        return Ok(Some(vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::Defending,
            effects: vec![
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    player: PlayerAst::That,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::Cant {
                    restriction: crate::effect::Restriction::block(
                        ObjectFilter::creature()
                            .controlled_by(PlayerFilter::IteratedPlayer)
                            .not_tagged(TagKey::from("divvy_chosen")),
                    ),
                    duration: Until::EndOfTurn,
                    condition: None,
                },
            ],
        }]));
    }

    if str_starts_with(
        &joined,
        "separate all creatures that player controls into two piles",
    ) && str_contains(
        &joined,
        "only creatures in the pile of their choice can attack this turn",
    ) {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                player: PlayerAst::That,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::Cant {
                restriction: crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                duration: Until::EndOfTurn,
                condition: None,
            },
        ]));
    }

    if joined
        == "each player separates all nontoken lands they control into two piles. for each player one of their piles is chosen by one of their opponents of their choice. destroy all lands in the chosen piles. tap all lands in the other piles"
    {
        return Ok(Some(vec![EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::ChoosePlayer {
                    chooser: PlayerAst::Implicit,
                    filter: PlayerFilter::Opponent,
                    tag: TagKey::from("divvy_opponent"),
                    random: false,
                    exclude_previous_choices: 0,
                },
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    player: PlayerAst::Chosen,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::Destroy {
                    target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                },
                EffectAst::TapAll {
                    filter: ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                },
            ],
        }]));
    }

    if str_starts_with(
        &joined,
        "exile up to five target permanent cards from your graveyard and separate them into two piles",
    ) && str_contains(&joined, "an opponent chooses one of those piles")
        && str_contains(&joined, "put that pile into your hand")
        && str_contains(&joined, "the other into your graveyard")
    {
        return Ok(Some(vec![
            parse_single_effect_sentence(sentences[0].lowered())?,
            EffectAst::TagMatchingObjects {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                zones: vec![Zone::Exile],
                tag: TagKey::from("divvy_source"),
            },
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Exile],
                search_mode: None,
            },
            EffectAst::ReturnToHand {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                random: false,
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::MoveToZone {
                        target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        zone: Zone::Graveyard,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Preserve,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                }],
            },
        ]));
    }

    if joined
        == "exile up to five target creature cards from graveyards. an opponent separates those cards into two piles. put all cards from the pile of your choice onto the battlefield under your control and the rest into their owners' graveyards"
    {
        return Ok(Some(vec![
            parse_single_effect_sentence(sentences[0].lowered())?,
            EffectAst::TagMatchingObjects {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                zones: vec![Zone::Exile],
                tag: TagKey::from("divvy_source"),
            },
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Exile],
                search_mode: None,
            },
            EffectAst::MoveToZone {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                zone: Zone::Battlefield,
                to_top: false,
                battlefield_controller: ReturnControllerAst::You,
                battlefield_tapped: false,
                attached_to: None,
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::MoveToZone {
                        target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        zone: Zone::Graveyard,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Preserve,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                }],
            },
        ]));
    }

    if str_starts_with(
        &joined,
        "search your library and graveyard for up to four creature cards with different names that each have mana value x or less and reveal them",
    ) && str_contains(&joined, "an opponent chooses two of those cards")
        && str_contains(&joined, "shuffle the chosen cards into your library")
        && str_contains(&joined, "put the rest onto the battlefield")
    {
        return Ok(Some(vec![
            parse_single_effect_sentence(sentences[0].lowered())?,
            EffectAst::TagMatchingObjects {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                zones: vec![Zone::Library, Zone::Graveyard],
                tag: TagKey::from("divvy_source"),
            },
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::exactly(2),
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Library, Zone::Graveyard],
                search_mode: None,
            },
            EffectAst::MoveToZone {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            },
            EffectAst::ShuffleLibrary {
                player: PlayerAst::You,
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::MoveToZone {
                        target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        zone: Zone::Battlefield,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::You,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                }],
            },
            EffectAst::Exile {
                target: TargetAst::Source(None),
                face_down: false,
            },
        ]));
    }

    if str_contains(&joined, "an opponent chooses one of them")
        && str_contains(&joined, "put the chosen card into your hand")
        && str_contains(&joined, "the other into your graveyard")
    {
        let mut prefix = Vec::new();
        prefix.extend(parse_effect_sentence_lexed(sentences[0].lowered())?);
        prefix.extend(parse_effect_sentence_lexed(sentences[1].lowered())?);
        let mut effects = prefix;
        effects.push(EffectAst::TagMatchingObjects {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            zones: vec![Zone::Library],
            tag: TagKey::from("divvy_source"),
        });
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            zone: Zone::Hand,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        });
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::MoveToZone {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    zone: Zone::Graveyard,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                }],
            }],
        });
        effects.push(EffectAst::ShuffleLibrary {
            player: PlayerAst::You,
        });
        return Ok(Some(effects));
    }

    if str_contains(&joined, "target opponent chooses one")
        && str_contains(&joined, "put that card into your hand")
        && str_contains(&joined, "the rest into your graveyard")
    {
        let mut effects = parse_effect_sentence_lexed(sentences[0].lowered())?;
        effects.push(EffectAst::TagMatchingObjects {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            zones: vec![Zone::Library],
            tag: TagKey::from("divvy_source"),
        });
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::TargetOpponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            zone: Zone::Hand,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        });
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::MoveToZone {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    zone: Zone::Graveyard,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                }],
            }],
        });
        effects.push(EffectAst::ShuffleLibrary {
            player: PlayerAst::You,
        });
        return Ok(Some(effects));
    }

    Ok(None)
}

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

fn parse_exact_card_effect_bundle_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    let sentence_words = tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::Word | TokenKind::Number | TokenKind::Tilde => Some(token.parser_text()),
            _ => None,
        })
        .collect::<Vec<_>>();

    if sentence_words.as_slice()
        == [
            "look", "at", "the", "top", "x", "cards", "of", "your", "library", "where", "x", "is",
            "your", "devotion", "to", "blue", "put", "up", "to", "one", "of", "them", "on", "top",
            "of", "your", "library", "and", "the", "rest", "on", "the", "bottom", "of", "your",
            "library", "in", "a", "random", "order", "if", "x", "is", "greater", "than", "or",
            "equal", "to", "the", "number", "of", "cards", "in", "your", "library", "you", "win",
            "the", "game",
        ]
    {
        let looked_tag = TagKey::from("thassas_oracle_looked");
        return Some(vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::You,
                count: Value::Devotion {
                    player: PlayerFilter::You,
                    color: crate::color::Color::Blue,
                },
                tag: looked_tag.clone(),
            },
            EffectAst::RearrangeLookedCardsInLibrary {
                tag: looked_tag,
                player: PlayerAst::You,
                count: ChoiceCount::up_to(1),
            },
            EffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::Devotion {
                        player: PlayerFilter::You,
                        color: crate::color::Color::Blue,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::CardsInLibrary(PlayerFilter::You),
                },
                if_true: vec![EffectAst::WinGame {
                    player: PlayerAst::You,
                }],
                if_false: Vec::new(),
            },
        ]);
    }

    if sentence_words.as_slice()
        == [
            "if",
            "this",
            "spell",
            "was",
            "cast",
            "from",
            "a",
            "graveyard",
            "copy",
            "this",
            "spell",
            "and",
            "you",
            "may",
            "choose",
            "a",
            "new",
            "target",
            "for",
            "the",
            "copy",
        ]
    {
        return Some(vec![EffectAst::Conditional {
            predicate: PredicateAst::ThisSpellWasCastFromZone(Zone::Graveyard),
            if_true: vec![EffectAst::CopySpell {
                target: TargetAst::Source(None),
                count: Value::Fixed(1),
                player: PlayerAst::Implicit,
                may_choose_new_targets: true,
            }],
            if_false: Vec::new(),
        }]);
    }

    if sentence_words.as_slice()
        == [
            "search", "your", "library", "for", "a", "basic", "forest", "card", "and", "a",
            "basic", "plains", "card", "reveal", "those", "cards", "put", "them", "into", "your",
            "hand", "then", "shuffle",
        ]
    {
        return Some(vec![EffectAst::SearchLibrarySlotsToHand {
            slots: vec![
                crate::cards::builders::SearchLibrarySlotAst {
                    filter: ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .owned_by(PlayerFilter::You)
                        .with_type(crate::types::CardType::Land)
                        .with_supertype(crate::types::Supertype::Basic)
                        .with_subtype(crate::types::Subtype::Forest),
                    optional: true,
                },
                crate::cards::builders::SearchLibrarySlotAst {
                    filter: ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .owned_by(PlayerFilter::You)
                        .with_type(crate::types::CardType::Land)
                        .with_supertype(crate::types::Supertype::Basic)
                        .with_subtype(crate::types::Subtype::Plains),
                    optional: true,
                },
            ],
            player: PlayerAst::You,
            reveal: true,
            progress_tag: TagKey::from("yasharn_search_progress"),
        }]);
    }

    None
}

fn normalize_parser_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    for token in &mut normalized {
        match token.kind {
            TokenKind::Word | TokenKind::Number | TokenKind::Tilde => {
                let replacement = token.parser_text().to_string();
                let _ = token.replace_word(replacement);
            }
            _ => {}
        }
    }
    normalized
}

#[derive(Debug, Clone)]
struct ConsultSentenceParts {
    effects: Vec<EffectAst>,
    player: PlayerAst,
    all_tag: TagKey,
    match_tag: TagKey,
}

struct ConsultCastClause {
    caster: PlayerAst,
    allow_land: bool,
    timing: ConsultCastTiming,
    cost: ConsultCastCost,
    mana_value_condition: Option<ConsultCastManaValueCondition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConsultCastTiming {
    Immediate,
    UntilEndOfTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConsultCastCost {
    Normal,
    WithoutPayingManaCost,
    PayLifeEqualToManaValue,
}

#[derive(Clone, Debug, PartialEq)]
struct ConsultCastManaValueCondition {
    operator: crate::effect::ValueComparisonOperator,
    right: Value,
}

fn parse_exile_top_library_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let tokens = trim_commas(tokens);
    let token_words = crate::cards::builders::parser::token_word_refs(&tokens);
    let count_word_idx = if slice_starts_with(&token_words, &["exile", "the", "top"]) {
        3usize
    } else if slice_starts_with(&token_words, &["exile", "top"]) {
        2usize
    } else {
        return None;
    };

    let count_tokens = token_words[count_word_idx..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_number(&count_tokens)?;
    if count_tokens
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return None;
    }
    let tail_words = crate::cards::builders::parser::token_word_refs(&count_tokens[used + 1..]);
    if tail_words != ["of", "your", "library"] {
        return None;
    }

    Some(vec![EffectAst::ExileTopOfLibrary {
        count: Value::Fixed(count as i32),
        player: PlayerAst::You,
        tags: Vec::new(),
        accumulated_tags: Vec::new(),
    }])
}

fn parse_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<ConsultSentenceParts>, CardTextError> {
    let mut sentence_tokens = trim_commas(tokens);
    let sentence_words = crate::cards::builders::parser::token_word_refs(&sentence_tokens);
    let leading_if_you_do = slice_starts_with(&sentence_words, &["if", "you", "do"])
        || slice_starts_with(&sentence_words, &["if", "they", "do"]);
    if leading_if_you_do {
        let start_word_idx = 3usize;
        let Some(start_token_idx) = token_index_for_word_index(&sentence_tokens, start_word_idx)
            .or(Some(sentence_tokens.len()))
        else {
            return Ok(None);
        };
        sentence_tokens = trim_commas(&sentence_tokens[start_token_idx..]);
    }
    if sentence_tokens.is_empty() {
        return Ok(None);
    }

    let mut prefix_effects = Vec::new();
    let mut prefix_tokens: Vec<OwnedLexToken> = Vec::new();
    let consult_tokens = if let Some(then_idx) =
        find_index(&sentence_tokens, |token: &OwnedLexToken| {
            token.is_word("then")
        }) {
        prefix_tokens = trim_commas(&sentence_tokens[..then_idx]);
        if prefix_tokens.is_empty() {
            return Ok(None);
        }
        prefix_effects = parse_exile_top_library_prefix(&prefix_tokens)
            .or_else(|| parse_effect_sentence_lexed(&prefix_tokens).ok())
            .or_else(|| parse_effect_chain(&prefix_tokens).ok())
            .unwrap_or_default();
        if prefix_effects.is_empty() {
            return Ok(None);
        }
        trim_commas(&sentence_tokens[then_idx + 1..])
    } else {
        sentence_tokens
    };
    if consult_tokens.is_empty() {
        return Ok(None);
    }

    let Some(consult_verb_idx) = find_index(&consult_tokens, |token: &OwnedLexToken| {
        token.is_word("reveal")
            || token.is_word("reveals")
            || token.is_word("exile")
            || token.is_word("exiles")
    }) else {
        return Ok(None);
    };
    let player = if consult_verb_idx == 0 {
        infer_consult_player_from_prefix(&prefix_tokens).unwrap_or(PlayerAst::You)
    } else {
        match parse_subject(&consult_tokens[..consult_verb_idx]) {
            SubjectAst::Player(player) => player,
            _ => return Ok(None),
        }
    };
    let mode = if consult_tokens[consult_verb_idx].is_word("reveal")
        || consult_tokens[consult_verb_idx].is_word("reveals")
    {
        LibraryConsultModeAst::Reveal
    } else {
        LibraryConsultModeAst::Exile
    };

    let Some(until_idx) = find_index(&consult_tokens, |token: &OwnedLexToken| {
        token.is_word("until")
    }) else {
        return Ok(None);
    };
    if until_idx <= consult_verb_idx + 1 {
        return Ok(None);
    }

    let prefix_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(
        &consult_tokens[consult_verb_idx + 1..until_idx],
    )
    .into_iter()
    .filter(|word| !is_article(word))
    .collect();
    if !slice_starts_with(&prefix_words, &["cards", "from", "top", "of"])
        || !slice_ends_with(&prefix_words, &["library"])
    {
        return Ok(None);
    }

    let until_tokens = trim_commas(&consult_tokens[until_idx + 1..]);
    let Some(match_verb_idx) = find_index(&until_tokens, |token: &OwnedLexToken| {
        token.is_word("reveal")
            || token.is_word("reveals")
            || token.is_word("exile")
            || token.is_word("exiles")
    }) else {
        return Ok(None);
    };
    if match_verb_idx == 0 || match_verb_idx + 1 >= until_tokens.len() {
        return Ok(None);
    }

    let mut filter_tokens = trim_commas(&until_tokens[match_verb_idx + 1..]).to_vec();
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let stop_rule = if let Some((count, used)) = parse_number(&filter_tokens) {
        let remaining = trim_commas(&filter_tokens[used..]).to_vec();
        if remaining.is_empty() {
            return Ok(None);
        }
        filter_tokens = remaining;
        LibraryConsultStopRuleAst::MatchCount(Value::Fixed(count as i32))
    } else {
        LibraryConsultStopRuleAst::FirstMatch
    };

    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        match parse_object_filter(&filter_tokens, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        }
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let all_tag = helper_tag_for_tokens(
        tokens,
        match mode {
            LibraryConsultModeAst::Reveal => "revealed",
            LibraryConsultModeAst::Exile => "exiled",
        },
    );
    let match_tag = helper_tag_for_tokens(tokens, "chosen");
    let mut effects = prefix_effects;
    effects.push(EffectAst::ConsultTopOfLibrary {
        player,
        mode,
        filter,
        stop_rule,
        all_tag: all_tag.clone(),
        match_tag: match_tag.clone(),
    });

    Ok(Some(ConsultSentenceParts {
        effects,
        player,
        all_tag,
        match_tag,
    }))
}

fn infer_consult_player_from_prefix(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let prefix_tokens = trim_commas(tokens);
    let (_, verb_idx) = find_verb(&prefix_tokens)?;
    match parse_subject(&prefix_tokens[..verb_idx]) {
        SubjectAst::Player(player) => Some(player),
        _ => None,
    }
}

fn parse_consult_remainder_order(words: &[&str]) -> Option<LibraryBottomOrderAst> {
    if !slice_contains(words, &"bottom") || !slice_contains(words, &"library") {
        return None;
    }
    if find_window_index(words, &["random", "order"]).is_some() {
        return Some(LibraryBottomOrderAst::Random);
    }
    if find_window_index(words, &["any", "order"]).is_some() {
        return Some(LibraryBottomOrderAst::ChooserChooses);
    }
    None
}

fn consult_stop_rule_is_single_match(stop_rule: &LibraryConsultStopRuleAst) -> bool {
    matches!(
        stop_rule,
        LibraryConsultStopRuleAst::FirstMatch
            | LibraryConsultStopRuleAst::MatchCount(Value::Fixed(1))
    )
}

fn parse_consult_condition_value(tokens: &[&str]) -> Option<Value> {
    if matches!(tokens, ["this's", "power"]) {
        return Some(Value::SourcePower);
    }

    let value_tokens = tokens
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if let Some((value, used)) = parse_value_from_lexed(&value_tokens)
        && crate::cards::builders::parser::token_word_refs(&value_tokens[used..]).is_empty()
    {
        return Some(value);
    }

    let (filter_tokens, had_number_prefix) = if slice_starts_with(tokens, &["the", "number", "of"])
    {
        (&tokens[3..], true)
    } else if slice_starts_with(tokens, &["number", "of"]) {
        (&tokens[2..], true)
    } else {
        (tokens, false)
    };
    if !had_number_prefix || filter_tokens.is_empty() {
        return None;
    }

    let filter_tokens = filter_tokens
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

fn strip_prefix_phrases<'a>(
    tokens: &'a [OwnedLexToken],
    phrases: &[&'static [&'static str]],
) -> Option<(&'static [&'static str], &'a [OwnedLexToken])> {
    phrases.iter().find_map(|phrase| {
        grammar::parse_prefix(tokens, grammar::phrase(phrase)).map(|(_, rest)| (*phrase, rest))
    })
}

fn parse_consult_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConsultCastManaValueCondition> {
    let (.., after_prefix) = strip_prefix_phrases(
        tokens,
        &[
            &["if", "it's", "a", "spell", "with", "mana", "value"][..],
            &["if", "it", "is", "a", "spell", "with", "mana", "value"][..],
            &["if", "the", "spell's", "mana", "value"][..],
            &["if", "the", "spells", "mana", "value"][..],
            &["if", "that", "spell's", "mana", "value"][..],
            &["if", "that", "spells", "mana", "value"][..],
            &["if", "its", "mana", "value"][..],
        ],
    )?;

    let (operator, right_tokens) = parse_value_comparison_tokens(after_prefix)?;
    let right_word_view = CompatWordIndex::new(right_tokens);
    let right_word_refs = right_word_view.word_refs();
    let right = parse_consult_condition_value(&right_word_refs)?;
    Some(ConsultCastManaValueCondition { operator, right })
}

fn parse_consult_cast_clause(tokens: &[OwnedLexToken]) -> Option<ConsultCastClause> {
    let mut second_tokens = trim_commas(tokens);
    let mut timing = ConsultCastTiming::Immediate;
    if let Some((TurnDurationPhrase::UntilEndOfTurn, remainder)) =
        parse_turn_duration_prefix(&second_tokens)
    {
        second_tokens = trim_commas(remainder);
        timing = ConsultCastTiming::UntilEndOfTurn;
    }

    let may_idx = find_index(&second_tokens, |token: &OwnedLexToken| token.is_word("may"))?;
    if may_idx == 0 || may_idx + 1 >= second_tokens.len() {
        return None;
    }

    let caster = match parse_subject(&second_tokens[..may_idx]) {
        SubjectAst::Player(player) => player,
        _ => return None,
    };
    let tail_tokens = &second_tokens[may_idx + 1..];
    let (matched_phrase, remainder_tokens) = strip_prefix_phrases(
        tail_tokens,
        &[
            &["cast", "that", "card"],
            &["cast", "it"],
            &["cast", "that", "exiled", "card"],
            &["cast", "the", "exiled", "card"],
            &["play", "that", "card"],
            &["play", "it"],
        ],
    )?;
    let allow_land = matches!(matched_phrase, ["play", ..]);
    let remainder_word_view = CompatWordIndex::new(remainder_tokens);
    let remainder = remainder_word_view.word_refs();
    if remainder == ["this", "turn"] {
        return Some(ConsultCastClause {
            caster,
            allow_land,
            timing: ConsultCastTiming::UntilEndOfTurn,
            cost: ConsultCastCost::Normal,
            mana_value_condition: None,
        });
    }

    if remainder
        == [
            "by", "paying", "life", "equal", "to", "the", "spell's", "mana", "value", "rather",
            "than", "paying", "its", "mana", "cost",
        ]
    {
        return Some(ConsultCastClause {
            caster,
            allow_land,
            timing,
            cost: ConsultCastCost::PayLifeEqualToManaValue,
            mana_value_condition: None,
        });
    }

    if !slice_starts_with(&remainder, &["without", "paying", "its", "mana", "cost"]) {
        return None;
    }

    let mana_value_condition = if remainder.len() == 5 {
        None
    } else {
        let condition_start = token_index_for_word_index(remainder_tokens, 5)?;
        let condition_tokens = &remainder_tokens[condition_start..];
        Some(parse_consult_mana_value_condition_tokens(condition_tokens)?)
    };

    Some(ConsultCastClause {
        caster,
        allow_land,
        timing,
        cost: ConsultCastCost::WithoutPayingManaCost,
        mana_value_condition,
    })
}

fn parse_consult_bottom_remainder_clause(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<LibraryBottomOrderAst> {
    let mut clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }

    let Some(order) = parse_consult_remainder_order(&clause_words) else {
        return None;
    };
    let mode_word = match mode {
        LibraryConsultModeAst::Reveal => "revealed",
        LibraryConsultModeAst::Exile => "exiled",
    };
    if !slice_contains(&clause_words, &mode_word) {
        return None;
    }
    let mentions_cast_window = find_window_index(&clause_words, &["not", "cast", "this"]).is_some()
        || find_window_by(&clause_words, 4, |window| {
            window == ["werent", "cast", "this", "way"]
                || window == ["weren't", "cast", "this", "way"]
        })
        .is_some()
        || find_window_index(&clause_words, &["were", "not", "cast", "this", "way"]).is_some();
    let mentions_remainder =
        slice_contains(&clause_words, &"rest") || slice_contains(&clause_words, &"other");

    (mentions_cast_window || mentions_remainder).then_some(order)
}

fn parse_if_declined_put_match_into_hand(
    tokens: &[OwnedLexToken],
    match_tag: TagKey,
) -> Option<Vec<EffectAst>> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let moves_to_hand = clause_words == ["put", "that", "card", "into", "your", "hand"]
        || clause_words == ["put", "the", "exiled", "card", "into", "your", "hand"]
        || clause_words == ["put", "it", "into", "your", "hand"]
        || clause_words
            == [
                "put", "that", "card", "into", "your", "hand", "if", "it", "wasnt", "cast", "this",
                "way",
            ]
        || clause_words
            == [
                "put", "that", "card", "into", "your", "hand", "if", "it", "wasn't", "cast",
                "this", "way",
            ]
        || clause_words
            == [
                "put", "the", "exiled", "card", "into", "your", "hand", "if", "it", "wasnt",
                "cast", "this", "way",
            ]
        || clause_words
            == [
                "put", "the", "exiled", "card", "into", "your", "hand", "if", "it", "wasn't",
                "cast", "this", "way",
            ]
        || clause_words
            == [
                "put", "it", "into", "your", "hand", "if", "it", "wasnt", "cast", "this", "way",
            ]
        || clause_words
            == [
                "put", "it", "into", "your", "hand", "if", "it", "wasn't", "cast", "this", "way",
            ]
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "dont", "put", "that", "card", "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "dont", "put", "the", "exiled", "card", "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &["if", "you", "dont", "put", "it", "into", "your", "hand"],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "don't", "put", "that", "card", "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "don't", "put", "the", "exiled", "card", "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &["if", "you", "don't", "put", "it", "into", "your", "hand"],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "do", "not", "put", "that", "card", "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "do", "not", "put", "the", "exiled", "card", "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "do", "not", "put", "it", "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "dont", "cast", "that", "card", "this", "way", "put", "it", "into",
                "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "dont", "cast", "the", "exiled", "card", "this", "way", "put", "it",
                "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "don't", "cast", "that", "card", "this", "way", "put", "it", "into",
                "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "don't", "cast", "the", "exiled", "card", "this", "way", "put", "it",
                "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "do", "not", "cast", "that", "card", "this", "way", "put", "it",
                "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "do", "not", "cast", "the", "exiled", "card", "this", "way", "put",
                "it", "into", "your", "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "dont", "cast", "it", "this", "way", "put", "it", "into", "your",
                "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "don't", "cast", "it", "this", "way", "put", "it", "into", "your",
                "hand",
            ],
        )
        || slice_starts_with(
            &clause_words,
            &[
                "if", "you", "do", "not", "cast", "it", "this", "way", "put", "it", "into", "your",
                "hand",
            ],
        );
    if !moves_to_hand {
        return None;
    }

    Some(vec![EffectAst::MoveToZone {
        target: TargetAst::Tagged(match_tag, None),
        zone: Zone::Hand,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    }])
}

fn consult_cast_effects(
    clause: &ConsultCastClause,
    match_tag: TagKey,
) -> Result<Vec<EffectAst>, CardTextError> {
    if clause.allow_land && !matches!(clause.cost, ConsultCastCost::Normal) {
        return Err(CardTextError::ParseError(
            "playing a land without paying its mana cost is unsupported".to_string(),
        ));
    }

    let mut cast_effects = match clause.cost {
        ConsultCastCost::Normal | ConsultCastCost::WithoutPayingManaCost => {
            let without_paying_mana_cost =
                matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost);
            if clause.allow_land || matches!(clause.timing, ConsultCastTiming::UntilEndOfTurn) {
                vec![EffectAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                    allow_land: clause.allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast: false,
                }]
            } else {
                vec![EffectAst::May {
                    effects: vec![EffectAst::CastTagged {
                        tag: match_tag.clone(),
                        allow_land: false,
                        as_copy: false,
                        without_paying_mana_cost,
                        cost_reduction: None,
                    }],
                }]
            }
        }
        ConsultCastCost::PayLifeEqualToManaValue => {
            if clause.allow_land {
                return Err(CardTextError::ParseError(
                    "pay-life consult cast clauses cannot allow lands".to_string(),
                ));
            }
            vec![
                EffectAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                    allow_land: false,
                    without_paying_mana_cost: false,
                    allow_any_color_for_cast: false,
                },
                EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                },
            ]
        }
    };

    if let Some(condition) = &clause.mana_value_condition {
        cast_effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(match_tag))),
                operator: condition.operator,
                right: condition.right.clone(),
            },
            if_true: cast_effects,
            if_false: Vec::new(),
        }]
    }

    Ok(cast_effects)
}

fn parse_consult_match_move_and_bottom_remainder(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = crate::cards::builders::parser::token_word_refs(&second_tokens);
    let (zone, battlefield_tapped) = if slice_starts_with(
        &second_words,
        &["put", "that", "card", "into", "your", "hand"],
    ) || slice_starts_with(
        &second_words,
        &["put", "it", "into", "your", "hand"],
    ) {
        (Zone::Hand, false)
    } else if slice_starts_with(
        &second_words,
        &[
            "put",
            "that",
            "card",
            "onto",
            "the",
            "battlefield",
            "tapped",
        ],
    ) || slice_starts_with(
        &second_words,
        &["put", "it", "onto", "the", "battlefield", "tapped"],
    ) || slice_starts_with(
        &second_words,
        &["put", "that", "card", "onto", "battlefield", "tapped"],
    ) || slice_starts_with(
        &second_words,
        &["put", "it", "onto", "battlefield", "tapped"],
    ) {
        (Zone::Battlefield, true)
    } else if slice_starts_with(
        &second_words,
        &["put", "that", "card", "onto", "the", "battlefield"],
    ) || slice_starts_with(&second_words, &["put", "it", "onto", "the", "battlefield"])
        || slice_starts_with(
            &second_words,
            &["put", "that", "card", "onto", "battlefield"],
        )
        || slice_starts_with(&second_words, &["put", "it", "onto", "battlefield"])
    {
        (Zone::Battlefield, false)
    } else {
        return Ok(None);
    };

    if !slice_contains(&second_words, &"rest") && !slice_contains(&second_words, &"other") {
        return Ok(None);
    }
    let Some(order) = parse_consult_remainder_order(&second_words) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(parts.match_tag.clone(), None),
        zone,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped,
        attached_to: None,
    });
    effects.push(EffectAst::PutTaggedRemainderOnBottomOfLibrary {
        tag: parts.all_tag,
        keep_tagged: Some(parts.match_tag),
        order,
        player: parts.player,
    });
    Ok(Some(effects))
}

fn parse_consult_match_into_hand_exile_others(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::ConsultTopOfLibrary {
            mode: LibraryConsultModeAst::Reveal,
            ..
        })
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words = crate::cards::builders::parser::token_word_refs(&second_tokens);
    let moves_to_hand =
        slice_starts_with(
            &second_words,
            &["put", "that", "card", "into", "your", "hand"],
        ) || slice_starts_with(&second_words, &["put", "it", "into", "your", "hand"]);
    let exiles_rest = slice_contains(&second_words, &"exile")
        && slice_contains(&second_words, &"other")
        && slice_contains(&second_words, &"cards");
    if !moves_to_hand || !exiles_rest {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(parts.match_tag.clone(), None),
        zone: Zone::Hand,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    });
    effects.push(EffectAst::ForEachTagged {
        tag: parts.all_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                TagKey::from(IT_TAG),
                ObjectFilter::tagged(parts.match_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::Exile {
                target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                face_down: false,
            }],
        }],
    });
    Ok(Some(effects))
}

fn parse_tainted_pact_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(&first_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if first_words.as_slice() != ["exile", "top", "card", "of", "your", "library"] {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let second_matches = second_words.as_slice()
        == [
            "you", "may", "put", "that", "card", "into", "your", "hand", "unless", "it", "has",
            "same", "name", "as", "another", "card", "exiled", "this", "way",
        ]
        || second_words.as_slice()
            == [
                "you", "may", "put", "it", "into", "your", "hand", "unless", "it", "has", "same",
                "name", "as", "another", "card", "exiled", "this", "way",
            ];
    if !second_matches {
        return Ok(None);
    }

    let third_tokens = trim_commas(third);
    let third_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(&third_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let third_matches = third_words.as_slice()
        == [
            "repeat",
            "this",
            "process",
            "until",
            "you",
            "put",
            "card",
            "into",
            "your",
            "hand",
            "or",
            "you",
            "exile",
            "two",
            "cards",
            "with",
            "same",
            "name",
            "whichever",
            "comes",
            "first",
        ];
    if !third_matches {
        return Ok(None);
    }

    let current_tag = TagKey::from("tainted_pact_current");
    let exiled_tag = TagKey::from("tainted_pact_exiled");
    let all_exiled_filter = ObjectFilter::tagged(exiled_tag.clone()).in_zone(Zone::Exile);
    Ok(Some(vec![EffectAst::RepeatProcess {
        effects: vec![
            EffectAst::ExileTopOfLibrary {
                count: Value::Fixed(1),
                player: PlayerAst::You,
                tags: vec![current_tag.clone()],
                accumulated_tags: vec![exiled_tag.clone()],
            },
            EffectAst::Conditional {
                predicate: PredicateAst::And(
                    Box::new(PredicateAst::TaggedMatches(
                        current_tag.clone(),
                        ObjectFilter::default().in_zone(Zone::Exile),
                    )),
                    Box::new(PredicateAst::ValueComparison {
                        left: Value::Count(all_exiled_filter.clone()),
                        operator: crate::effect::ValueComparisonOperator::Equal,
                        right: Value::DistinctNames(all_exiled_filter),
                    }),
                ),
                if_true: vec![EffectAst::MayMoveToZone {
                    target: TargetAst::Tagged(current_tag.clone(), None),
                    zone: Zone::Hand,
                    player: PlayerAst::You,
                }],
                if_false: Vec::new(),
            },
        ],
        continue_effect_index: 1,
        continue_predicate: IfResultPredicate::WasDeclined,
    }]))
}

fn prepend_prefix_sentence_to_consult_pair(
    prefix: &[OwnedLexToken],
    consult: &[OwnedLexToken],
    followup: &[OwnedLexToken],
    pair_rule: PairSentenceRule,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let prefix_effects =
        parse_effect_sentence_lexed(prefix).or_else(|_| parse_effect_chain(prefix))?;
    if prefix_effects.is_empty() {
        return Ok(None);
    }

    let Some(mut combined) = pair_rule(consult, followup)? else {
        return Ok(None);
    };
    let mut effects = prefix_effects;
    effects.append(&mut combined);
    Ok(Some(effects))
}

fn parse_prefix_then_consult_match_move_and_bottom_remainder(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    prepend_prefix_sentence_to_consult_pair(
        first,
        second,
        third,
        parse_consult_match_move_and_bottom_remainder,
    )
}

fn parse_prefix_then_consult_match_into_hand_exile_others(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    prepend_prefix_sentence_to_consult_pair(
        first,
        second,
        third,
        parse_consult_match_into_hand_exile_others,
    )
}

struct SentenceInput {
    lowered: OnceCell<Vec<OwnedLexToken>>,
    lexed: Vec<OwnedLexToken>,
}

impl SentenceInput {
    fn from_lexed(tokens: &[OwnedLexToken]) -> Self {
        Self {
            lowered: OnceCell::new(),
            lexed: tokens.to_vec(),
        }
    }

    fn lowered(&self) -> &[OwnedLexToken] {
        self.lowered
            .get_or_init(|| normalize_parser_tokens(&self.lexed))
            .as_slice()
    }

    fn lexed(&self) -> &[OwnedLexToken] {
        self.lexed.as_slice()
    }
}

fn future_zone_replacement_from_sentence_text(sentence_text: &str) -> Option<EffectAst> {
    let normalized = sentence_text.to_ascii_lowercase();
    let target = TargetAst::Tagged(TagKey::from(IT_TAG), None);

    if str_contains(&normalized, "countered this way")
        && str_contains(&normalized, "instead of putting it into")
        && str_contains(&normalized, "graveyard")
    {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: Some(Zone::Stack),
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    if str_contains(&normalized, "would die this turn") && str_contains(&normalized, "exile") {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: Some(Zone::Battlefield),
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    if str_contains(&normalized, "would be put into")
        && str_contains(&normalized, "graveyard")
        && str_contains(&normalized, "this turn")
        && str_contains(&normalized, "exile")
    {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: None,
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    None
}

fn maybe_rewrite_future_zone_replacement_sentence(
    sentence_effects: &mut Vec<EffectAst>,
    sentence_text: &str,
) {
    if !matches!(
        classify_instead_followup_text(sentence_text),
        InsteadSemantics::FutureReplacement
    ) {
        return;
    }

    let Some(replacement) = future_zone_replacement_from_sentence_text(sentence_text) else {
        return;
    };

    if sentence_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ExileInsteadOfGraveyardThisTurn { .. }
                | EffectAst::PreventNextTimeDamage { .. }
                | EffectAst::RedirectNextTimeDamageToSource { .. }
        )
    }) {
        return;
    }

    if sentence_effects.len() == 1 {
        if let Some(EffectAst::IfResult { effects, .. }) = sentence_effects.first_mut() {
            *effects = vec![replacement];
            return;
        }
        *sentence_effects = vec![replacement];
    }
}

fn parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = crate::cards::builders::parser::token_word_refs(&first_tokens);
    let count_word_idx = if slice_starts_with(&first_words, &["reveal", "the", "top"]) {
        3usize
    } else if slice_starts_with(&first_words, &["reveal", "top"]) {
        2usize
    } else {
        return Ok(None);
    };

    let count_tokens = first_words[count_word_idx..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if count_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "card" || word == "cards")
    {
        return Ok(None);
    }
    let Some((count, used)) = parse_number(&count_tokens) else {
        return Ok(None);
    };
    if count_tokens
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Ok(None);
    }
    let reveal_tail = crate::cards::builders::parser::token_word_refs(&count_tokens[used + 1..]);
    if reveal_tail != ["of", "your", "library"] {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words = crate::cards::builders::parser::token_word_refs(&second_tokens);
    if !matches!(
        second_words.get(..2),
        Some(["put", "all"] | ["puts", "all"])
    ) {
        return Ok(None);
    }
    let Some(revealed_idx) = find_window_index(&second_words, &["revealed", "this", "way"]) else {
        return Ok(None);
    };
    if revealed_idx <= 2 {
        return Ok(None);
    }

    let Some(filter_start) = token_index_for_word_index(&second_tokens, 2) else {
        return Ok(None);
    };
    let filter_end =
        token_index_for_word_index(&second_tokens, revealed_idx).unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_revealed = &second_words[revealed_idx + 3..];
    let has_hand_clause = find_window_index(after_revealed, &["into", "your", "hand"]).is_some();
    let has_rest_clause =
        find_window_index(after_revealed, &["and", "the", "rest", "into", "your"]).is_some()
            && slice_contains(after_revealed, &"graveyard");
    if !has_hand_clause || !has_rest_clause {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard {
            player: PlayerAst::You,
            count,
            filter,
        },
    ]))
}

fn parse_delayed_dies_exile_top_power_choose_play(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = crate::cards::builders::parser::token_word_refs(&first_tokens);
    if !slice_starts_with(
        &first_words,
        &["when", "that", "creature", "dies", "this", "turn"],
    ) {
        return Ok(None);
    }

    let Some(comma_idx) = find_index(&first_tokens, |token: &OwnedLexToken| token.is_comma())
    else {
        return Ok(None);
    };
    let action_tokens = trim_commas(&first_tokens[comma_idx + 1..]);
    let action_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(&action_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let starts_with_exile_top_power = slice_starts_with(
        &action_words,
        &[
            "exile", "number", "of", "cards", "from", "top", "of", "your", "library", "equal",
            "to", "its", "power",
        ],
    );
    let ends_with_choose_exiled =
        slice_ends_with(&action_words, &["choose", "card", "exiled", "this", "way"]);
    if !starts_with_exile_top_power || !ends_with_choose_exiled {
        return Ok(None);
    }

    let second_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(second)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let is_until_next_turn_play_clause = second_words.as_slice()
        == [
            "until", "end", "of", "your", "next", "turn", "you", "may", "play", "that", "card",
        ];
    if !is_until_next_turn_play_clause {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(first, "looked");
    let chosen_tag = helper_tag_for_tokens(first, "chosen");
    let mut exiled_filter = ObjectFilter::default();
    exiled_filter.zone = Some(Zone::Exile);
    exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: None,
        effects: vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::You,
                count: Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)))),
                tag: looked_tag.clone(),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(looked_tag, None),
                face_down: false,
            },
            EffectAst::ChooseObjects {
                filter: exiled_filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::GrantPlayTaggedUntilYourNextTurn {
                tag: chosen_tag,
                player: PlayerAst::You,
                allow_land: true,
            },
        ],
    }]))
}

fn parse_pair_sentence_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, PairSentenceRule); 12] = [
        (
            "damage-prevention-then-put-counters",
            parse_damage_prevention_then_put_counters,
        ),
        (
            "delayed-dies-exile-top-power-choose-play",
            parse_delayed_dies_exile_top_power_choose_play,
        ),
        (
            "target-gains-flashback-until-eot-targets-mana-cost",
            parse_target_gains_flashback_until_eot_with_targets_mana_cost,
        ),
        (
            "mill-then-put-from-among-into-hand",
            parse_mill_then_may_put_from_among_into_hand,
        ),
        (
            "exile-until-match-grant-play-this-turn",
            parse_exile_until_match_grant_play_this_turn,
        ),
        (
            "target-chooses-other-cant-block",
            parse_target_player_chooses_then_other_cant_block,
        ),
        (
            "tap-all-then-they-dont-untap-while-source-tapped",
            parse_tap_all_then_they_dont_untap_while_source_tapped,
        ),
        (
            "choose-card-type-then-reveal-and-put",
            parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
        ),
        (
            "choose-creature-type-then-become-type",
            parse_choose_creature_type_then_become_type,
        ),
        (
            "reveal-top-matching-into-hand-rest-graveyard",
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
        ),
        (
            "consult-match-move-bottom-remainder",
            parse_consult_match_move_and_bottom_remainder,
        ),
        (
            "consult-match-into-hand-exile-others",
            parse_consult_match_into_hand_exile_others,
        ),
    ];

    fn run_pair_rules(
        rules: &[(&'static str, PairSentenceRule)],
        first: &[OwnedLexToken],
        second: &[OwnedLexToken],
    ) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
        for (name, rule) in rules {
            if let Some(combined) = rule(first, second)? {
                return Ok(Some((*name, combined)));
            }
        }

        Ok(None)
    }

    let preferred_rules: &[(&str, PairSentenceRule)] = match lexed_head_words(first) {
        Some(("prevent", _)) => &[(
            "damage-prevention-then-put-counters",
            parse_damage_prevention_then_put_counters,
        )],
        Some(("when", Some("that"))) => &[(
            "delayed-dies-exile-top-power-choose-play",
            parse_delayed_dies_exile_top_power_choose_play,
        )],
        Some(("target", _)) => &[
            (
                "target-gains-flashback-until-eot-targets-mana-cost",
                parse_target_gains_flashback_until_eot_with_targets_mana_cost,
            ),
            (
                "target-chooses-other-cant-block",
                parse_target_player_chooses_then_other_cant_block,
            ),
        ],
        Some(("mill", _)) => &[(
            "mill-then-put-from-among-into-hand",
            parse_mill_then_may_put_from_among_into_hand,
        )],
        Some(("exile", _)) => &[
            (
                "exile-until-match-grant-play-this-turn",
                parse_exile_until_match_grant_play_this_turn,
            ),
            (
                "consult-match-move-bottom-remainder",
                parse_consult_match_move_and_bottom_remainder,
            ),
            (
                "consult-match-into-hand-exile-others",
                parse_consult_match_into_hand_exile_others,
            ),
        ],
        Some(("tap", _)) => &[(
            "tap-all-then-they-dont-untap-while-source-tapped",
            parse_tap_all_then_they_dont_untap_while_source_tapped,
        )],
        Some(("choose", _)) => &[
            (
                "choose-card-type-then-reveal-and-put",
                parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
            ),
            (
                "choose-creature-type-then-become-type",
                parse_choose_creature_type_then_become_type,
            ),
        ],
        Some(("reveal", Some("the"))) | Some(("reveal", Some("top"))) => &[(
            "reveal-top-matching-into-hand-rest-graveyard",
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
        )],
        Some(("look", Some("at"))) => &[
            (
                "consult-match-move-bottom-remainder",
                parse_consult_match_move_and_bottom_remainder,
            ),
            (
                "consult-match-into-hand-exile-others",
                parse_consult_match_into_hand_exile_others,
            ),
        ],
        _ => &[],
    };

    if let Some(hit) = run_pair_rules(preferred_rules, first, second)? {
        return Ok(Some(hit));
    }

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

fn parse_damage_prevention_then_put_counters(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let Some(first_effect) = first_effects.first() else {
        return Ok(None);
    };
    if first_effects.len() != 1 {
        return Ok(None);
    }

    let (amount, target, duration) = match first_effect {
        EffectAst::PreventDamage {
            amount,
            target,
            duration,
        } => (Some(amount.clone()), target.clone(), duration.clone()),
        EffectAst::PreventAllDamageToTarget { target, duration } => {
            (None, target.clone(), duration.clone())
        }
        _ => return Ok(None),
    };

    let second_tokens = trim_commas(second);
    let second_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !slice_starts_with(
        &second_words,
        &["for", "each", "1", "damage", "prevented", "this", "way"],
    ) || !slice_contains(&second_words, &"put")
        || !slice_contains(&second_words, &"+1/+1")
        || !slice_contains(&second_words, &"counter")
        || !slice_contains(&second_words, &"on")
    {
        return Ok(None);
    }

    let Some(on_idx) = find_index(&second_words, |word| *word == "on") else {
        return Ok(None);
    };
    let target_words = &second_words[on_idx + 1..];
    let valid_target_tail = matches!(
        target_words,
        ["that", "creature"] | ["it"] | ["that", "permanent"] | ["that", "object"]
    );
    if !valid_target_tail {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::PreventDamageToTargetPutCounters {
        amount,
        target,
        duration,
        counter_type: CounterType::PlusOnePlusOne,
    }]))
}

fn parse_target_gains_flashback_until_eot_with_targets_mana_cost(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = crate::cards::builders::parser::token_word_refs(&first_tokens);
    let Some(gain_idx) = find_index(&first_words, |word| matches!(*word, "gain" | "gains")) else {
        return Ok(None);
    };
    if first_words[gain_idx + 1..] != ["flashback", "until", "end", "of", "turn"] {
        return Ok(None);
    }

    let Some(gain_token_idx) = token_index_for_word_index(&first_tokens, gain_idx) else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&first_tokens[..gain_token_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(&target_tokens)?;

    let second_tokens = trim_commas(second);
    let second_words = crate::cards::builders::parser::token_word_refs(&second_tokens);
    let valid_followup = second_words.as_slice()
        == [
            "the",
            "flashback",
            "cost",
            "is",
            "equal",
            "to",
            "its",
            "mana",
            "cost",
        ]
        || second_words.as_slice()
            == [
                "that",
                "cards",
                "flashback",
                "cost",
                "is",
                "equal",
                "to",
                "its",
                "mana",
                "cost",
            ];
    if !valid_followup {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::GrantToTarget {
        target,
        grantable: crate::grant::Grantable::flashback_from_cards_mana_cost(),
        duration: crate::grant::GrantDuration::UntilEndOfTurn,
    }]))
}

fn parse_tap_all_then_they_dont_untap_while_source_tapped(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::TapAll { filter }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = crate::cards::builders::parser::token_word_refs(&second_tokens);
    let starts_with_supported_pronoun_clause =
        slice_starts_with(&second_words, &["they", "dont", "untap", "during"])
            || slice_starts_with(&second_words, &["they", "do", "not", "untap", "during"]);
    let has_source_tapped_duration = find_window_index(&second_words, &["for", "as", "long", "as"])
        .is_some()
        && slice_contains(&second_words, &"remains")
        && slice_contains(&second_words, &"tapped")
        && (slice_contains(&second_words, &"this")
            || slice_contains(&second_words, &"thiss")
            || slice_contains(&second_words, &"source")
            || slice_contains(&second_words, &"artifact")
            || slice_contains(&second_words, &"creature")
            || slice_contains(&second_words, &"permanent"));
    if !starts_with_supported_pronoun_clause || !has_source_tapped_duration {
        return Ok(None);
    }

    let Some((duration, clause_tokens)) = parse_restriction_duration(&second_tokens)? else {
        return Ok(None);
    };
    let clause_words = crate::cards::builders::parser::token_word_refs(&clause_tokens);
    let valid_untap_clause = slice_starts_with(&clause_words, &["they", "dont", "untap", "during"])
        || slice_starts_with(&clause_words, &["they", "do", "not", "untap", "during"]);
    if !valid_untap_clause {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::TapAll {
            filter: filter.clone(),
        },
        EffectAst::Cant {
            restriction: crate::effect::Restriction::untap(filter.clone()),
            duration,
            condition: Some(crate::ConditionExpr::SourceIsTapped),
        },
    ]))
}

fn parse_exile_until_match_grant_play_this_turn(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::ConsultTopOfLibrary {
            mode: LibraryConsultModeAst::Exile,
            stop_rule,
            ..
        }) if consult_stop_rule_is_single_match(stop_rule)
    ) {
        return Ok(None);
    }

    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag)?);
    Ok(Some(effects))
}

fn parse_look_at_top_reveal_match_put_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { player, count, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = crate::cards::builders::parser::token_word_refs(&second_tokens);
    if second_words.is_empty() {
        return Ok(None);
    }

    let (chooser, reveal_word_idx) = if slice_starts_with(&second_words, &["you", "may", "reveal"])
    {
        (PlayerAst::You, 2usize)
    } else if slice_starts_with(&second_words, &["that", "player", "may", "reveal"]) {
        (PlayerAst::That, 3usize)
    } else if slice_starts_with(&second_words, &["they", "may", "reveal"]) {
        (PlayerAst::That, 2usize)
    } else if slice_starts_with(&second_words, &["may", "reveal"]) {
        (*player, 1usize)
    } else if slice_starts_with(&second_words, &["reveal"]) {
        (*player, 0usize)
    } else {
        return Ok(None);
    };

    let from_among_word_idx = find_window_index(&second_words, &["from", "among", "them"])
        .or_else(|| find_window_index(&second_words, &["from", "among", "those", "cards"]));
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= reveal_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&second_tokens, reveal_word_idx + 1)
        .unwrap_or(second_tokens.len());
    let filter_end = token_index_for_word_index(&second_tokens, from_among_word_idx)
        .unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_word_idx =
        if find_window_index(&second_words, &["from", "among", "those", "cards"]).is_some() {
            from_among_word_idx + 4
        } else {
            from_among_word_idx + 3
        };
    let after_from_words = &second_words[after_from_word_idx..];
    let puts_into_hand = (slice_starts_with(after_from_words, &["and", "put", "it", "into"])
        || slice_starts_with(after_from_words, &["put", "it", "into"]))
        && slice_contains(after_from_words, &"hand");
    if !puts_into_hand {
        return Ok(None);
    }

    let third_words = crate::cards::builders::parser::token_word_refs(third);
    let puts_rest_bottom = matches!(third_words.first().copied(), Some("put" | "puts"))
        && slice_contains(&third_words, &"rest")
        && slice_contains(&third_words, &"bottom")
        && slice_contains(&third_words, &"library");
    if !puts_rest_bottom {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::LookAtTopCards {
        player: *player,
        count: count.clone(),
        tag: TagKey::from(IT_TAG),
    }];
    effects.push(
        EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
            player: chooser,
            filter,
            reveal: true,
            if_not_chosen: Vec::new(),
        },
    );
    Ok(Some(effects))
}

fn parse_top_cards_view_sentence(tokens: &[OwnedLexToken]) -> Option<(PlayerAst, Value, bool)> {
    let tokens = trim_commas(tokens);
    let clause_words = crate::cards::builders::parser::token_word_refs(&tokens);
    if clause_words.is_empty() {
        return None;
    }

    let (count_word_idx, revealed) =
        if slice_starts_with(&clause_words, &["look", "at", "the", "top"]) {
            (4usize, false)
        } else if slice_starts_with(&clause_words, &["look", "at", "top"]) {
            (3usize, false)
        } else if slice_starts_with(&clause_words, &["reveal", "the", "top"]) {
            (3usize, true)
        } else if slice_starts_with(&clause_words, &["reveal", "top"]) {
            (2usize, true)
        } else {
            return None;
        };

    let count_start = token_index_for_word_index(&tokens, count_word_idx)?;
    let count_tokens = &tokens[count_start..];
    let (count, used) = parse_number(count_tokens)?;
    let count_tail = crate::cards::builders::parser::token_word_refs(&count_tokens[used..]);
    if !matches!(count_tail.first().copied(), Some("card" | "cards")) {
        return None;
    }

    let owner_tail = &count_tail[1..];
    if owner_tail != ["of", "your", "library"] {
        return None;
    }

    Some((PlayerAst::You, Value::Fixed(count as i32), revealed))
}

fn parse_counted_looked_cards_into_your_hand_words(words: &[&str]) -> Option<u32> {
    if !slice_starts_with(words, &["put"]) {
        return None;
    }

    let count_tokens = words[1..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_number(&count_tokens)?;

    let mut idx = 1 + used;
    if words.get(idx).copied() == Some("of") {
        idx += 1;
    }

    match words.get(idx).copied() {
        Some("them") => idx += 1,
        Some("those") => {
            idx += 1;
            if words.get(idx).copied() == Some("card") || words.get(idx).copied() == Some("cards") {
                idx += 1;
            }
        }
        _ => return None,
    }

    if words.get(idx..idx + 3) != Some(&["into", "your", "hand"]) {
        return None;
    }
    idx += 3;

    if idx == words.len() {
        return Some(count as u32);
    }
    if idx + 1 == words.len() && words[idx] == "instead" {
        return Some(count as u32);
    }

    None
}

fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let trimmed = trim_commas(tokens);
    let clause_words = crate::cards::builders::parser::token_word_refs(&trimmed);
    if !slice_starts_with(&clause_words, &["if", "this", "spell", "was", "kicked"]) {
        return None;
    }

    let tail_start = token_index_for_word_index(&trimmed, 5).unwrap_or(trimmed.len());
    let tail = trim_commas(&trimmed[tail_start..]);
    let tail_words = crate::cards::builders::parser::token_word_refs(&tail);
    parse_counted_looked_cards_into_your_hand_words(&tail_words)
}

fn parse_may_put_filtered_looked_card_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_words = crate::cards::builders::parser::token_word_refs(&sentence_tokens);
    if sentence_words.is_empty() {
        return Ok(None);
    }

    let (chooser, action_word_idx) = if slice_starts_with(&sentence_words, &["you", "may", "put"]) {
        (PlayerAst::You, 2usize)
    } else if slice_starts_with(&sentence_words, &["that", "player", "may", "put"]) {
        (PlayerAst::That, 3usize)
    } else if slice_starts_with(&sentence_words, &["they", "may", "put"]) {
        (PlayerAst::That, 2usize)
    } else if slice_starts_with(&sentence_words, &["may", "put"]) {
        (PlayerAst::You, 1usize)
    } else {
        return Ok(None);
    };

    let from_among_word_idx = find_window_index(&sentence_words, &["from", "among", "them"])
        .or_else(|| find_window_index(&sentence_words, &["from", "among", "those", "cards"]));
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= action_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&sentence_tokens, action_word_idx + 1)
        .unwrap_or(sentence_tokens.len());
    let filter_end = token_index_for_word_index(&sentence_tokens, from_among_word_idx)
        .unwrap_or(sentence_tokens.len());
    let filter_tokens = trim_commas(&sentence_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_word_idx =
        if find_window_index(&sentence_words, &["from", "among", "those", "cards"]).is_some() {
            from_among_word_idx + 4
        } else {
            from_among_word_idx + 3
        };
    let after_from_words = &sentence_words[after_from_word_idx..];
    let tapped = match after_from_words {
        ["onto", "the", "battlefield"] | ["onto", "battlefield"] => false,
        ["onto", "the", "battlefield", "tapped"] | ["onto", "battlefield", "tapped"] => true,
        _ => return Ok(None),
    };

    Ok(Some((chooser, filter, tapped)))
}

fn parse_if_you_dont_put_card_from_among_them_into_your_hand(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    let words: Vec<&str> = crate::cards::builders::parser::token_word_refs(&trimmed)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    words.as_slice()
        == [
            "if", "you", "dont", "put", "card", "from", "among", "them", "into", "your", "hand",
        ]
        || words.as_slice()
            == [
                "if", "you", "don't", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "do", "not", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "dont", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "don't", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "do", "not", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
}

fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    let words = crate::cards::builders::parser::token_word_refs(&trimmed);
    matches!(words.first().copied(), Some("put" | "puts"))
        && slice_contains(&words, &"rest")
        && slice_contains(&words, &"bottom")
        && slice_contains(&words, &"library")
}

fn parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { player, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let Some(base_count) = parse_counted_looked_cards_into_your_hand_words(
        &crate::cards::builders::parser::token_word_refs(&trim_commas(second)),
    ) else {
        return Ok(None);
    };
    let Some(kicked_count) = parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(third)
    else {
        return Ok(None);
    };
    if !is_put_rest_on_bottom_of_library_sentence(fourth) {
        return Ok(None);
    }

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::Conditional {
            predicate: crate::cards::builders::PredicateAst::ThisSpellWasKicked,
            if_true: vec![EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: *player,
                count: kicked_count,
            }],
            if_false: vec![EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: *player,
                count: base_count,
            }],
        },
    ]))
}

fn parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let Some((chooser, battlefield_filter, tapped)) =
        parse_may_put_filtered_looked_card_onto_battlefield(second)?
    else {
        return Ok(None);
    };
    if !parse_if_you_dont_put_card_from_among_them_into_your_hand(third) {
        return Ok(None);
    }
    if !is_put_rest_on_bottom_of_library_sentence(fourth) {
        return Ok(None);
    }

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            player: chooser,
            battlefield_filter,
            tapped,
        },
    ]))
}

fn parse_top_cards_put_match_into_hand_rest_graveyard(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) = parse_top_cards_view_sentence(first) else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = crate::cards::builders::parser::token_word_refs(&second_tokens);
    if second_words.is_empty() {
        return Ok(None);
    }

    let (chooser, action_word_idx, reveal_chosen) =
        if slice_starts_with(&second_words, &["you", "may", "reveal"]) {
            (PlayerAst::You, 2usize, true)
        } else if slice_starts_with(&second_words, &["you", "may", "put"]) {
            (PlayerAst::You, 2usize, false)
        } else if slice_starts_with(&second_words, &["that", "player", "may", "reveal"]) {
            (PlayerAst::That, 3usize, true)
        } else if slice_starts_with(&second_words, &["that", "player", "may", "put"]) {
            (PlayerAst::That, 3usize, false)
        } else if slice_starts_with(&second_words, &["they", "may", "reveal"]) {
            (PlayerAst::That, 2usize, true)
        } else if slice_starts_with(&second_words, &["they", "may", "put"]) {
            (PlayerAst::That, 2usize, false)
        } else if slice_starts_with(&second_words, &["may", "reveal"]) {
            (player, 1usize, true)
        } else if slice_starts_with(&second_words, &["may", "put"]) {
            (player, 1usize, false)
        } else if slice_starts_with(&second_words, &["reveal"]) {
            (player, 0usize, true)
        } else if slice_starts_with(&second_words, &["put"]) {
            (player, 0usize, false)
        } else {
            return Ok(None);
        };

    let from_among_word_idx = find_window_index(&second_words, &["from", "among", "them"])
        .or_else(|| find_window_index(&second_words, &["from", "among", "those", "cards"]));
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= action_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&second_tokens, action_word_idx + 1)
        .unwrap_or(second_tokens.len());
    let filter_end = token_index_for_word_index(&second_tokens, from_among_word_idx)
        .unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_word_idx =
        if find_window_index(&second_words, &["from", "among", "those", "cards"]).is_some() {
            from_among_word_idx + 4
        } else {
            from_among_word_idx + 3
        };
    let after_from_words = &second_words[after_from_word_idx..];
    let moves_into_hand = if reveal_chosen {
        (slice_starts_with(after_from_words, &["and", "put", "it", "into"])
            || slice_starts_with(after_from_words, &["put", "it", "into"]))
            && slice_contains(after_from_words, &"hand")
    } else {
        slice_starts_with(after_from_words, &["into"]) && slice_contains(after_from_words, &"hand")
    };
    if !moves_into_hand {
        return Ok(None);
    }

    let third_words = crate::cards::builders::parser::token_word_refs(third);
    let puts_rest_graveyard = matches!(third_words.first().copied(), Some("put" | "puts"))
        && slice_contains(&third_words, &"rest")
        && slice_contains(&third_words, &"graveyard");
    if !puts_rest_graveyard {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::LookAtTopCards {
        player,
        count,
        tag: TagKey::from(IT_TAG),
    }];
    if reveal_top {
        effects.push(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    effects.push(EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
        player: chooser,
        filter,
        reveal: reveal_chosen,
        if_not_chosen: Vec::new(),
    });
    Ok(Some(effects))
}

fn parse_exile_until_match_cast_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };
    if !matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost) {
        return Ok(None);
    }
    let Some(order) = parse_consult_bottom_remainder_clause(
        third,
        match parts.effects.last() {
            Some(EffectAst::ConsultTopOfLibrary { mode, .. }) => *mode,
            _ => return Ok(None),
        },
    ) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag.clone())?);
    effects.push(EffectAst::PutTaggedRemainderOnBottomOfLibrary {
        tag: parts.all_tag,
        keep_tagged: None,
        order,
        player: parts.player,
    });
    Ok(Some(effects))
}

fn parse_exile_until_match_cast_else_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    let Some(EffectAst::ConsultTopOfLibrary {
        mode: LibraryConsultModeAst::Exile,
        stop_rule,
        ..
    }) = parts.effects.last()
    else {
        return Ok(None);
    };
    if !consult_stop_rule_is_single_match(stop_rule) {
        return Ok(None);
    }
    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };
    if !matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost) || clause.allow_land {
        return Ok(None);
    }
    let Some(hand_effects) = parse_if_declined_put_match_into_hand(third, parts.match_tag.clone())
    else {
        return Ok(None);
    };

    let cast_effects = consult_cast_effects(&clause, parts.match_tag)?;
    let mut effects = parts.effects;
    if cast_effects.len() == 1 {
        let single_effect = cast_effects.into_iter().next().ok_or_else(|| {
            CardTextError::ParseError("missing cast effect for consult follow-up".to_string())
        })?;
        let EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } = single_effect
        else {
            effects.push(single_effect);
            effects.push(EffectAst::IfResult {
                predicate: IfResultPredicate::WasDeclined,
                effects: hand_effects,
            });
            return Ok(Some(effects));
        };
        let mut gated_if_true = if_true;
        gated_if_true.push(EffectAst::IfResult {
            predicate: IfResultPredicate::WasDeclined,
            effects: hand_effects.clone(),
        });
        let mut gated_if_false = if_false;
        gated_if_false.extend(hand_effects);
        effects.push(EffectAst::Conditional {
            predicate,
            if_true: gated_if_true,
            if_false: gated_if_false,
        });
    } else {
        effects.extend(cast_effects);
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::WasDeclined,
            effects: hand_effects,
        });
    }
    Ok(Some(effects))
}

fn title_case_words(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut titled = String::new();
            titled.extend(first.to_uppercase());
            titled.push_str(chars.as_str());
            titled
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_named_card_filter_segment(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut segment_words = crate::cards::builders::parser::token_word_refs(tokens);
    while segment_words.first().is_some_and(|word| is_article(word)) {
        segment_words.remove(0);
    }
    if matches!(segment_words.last().copied(), Some("card" | "cards")) {
        segment_words.pop();
    }
    if segment_words.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_words(&segment_words));
    Some(filter)
}

fn split_reveal_filter_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current: Vec<OwnedLexToken> = Vec::new();
    let has_noncomparison_or = tokens
        .iter()
        .enumerate()
        .any(|(idx, token)| token.is_word("or") && !is_comparison_or_delimiter(tokens, idx));
    for (idx, token) in tokens.iter().enumerate() {
        let is_separator = (token.is_word("or") && !is_comparison_or_delimiter(tokens, idx))
            || (has_noncomparison_or && token.is_comma());
        if is_separator {
            while current.last().is_some_and(|entry| entry.is_word("and")) {
                current.pop();
            }
            let trimmed = trim_commas(&current);
            if !trimmed.is_empty() {
                segments.push(trimmed.to_vec());
            }
            current.clear();
            continue;
        }
        current.push(token.clone());
    }
    while current.last().is_some_and(|entry| entry.is_word("and")) {
        current.pop();
    }
    let trimmed = trim_commas(&current);
    if !trimmed.is_empty() {
        segments.push(trimmed.to_vec());
    }
    segments
}

fn parse_looked_card_reveal_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut filter_tokens = trim_commas(tokens).to_vec();
    let raw_word_view = CompatWordIndex::new(&filter_tokens);
    let raw_word_refs = raw_word_view.word_refs();
    let same_name_suffix_len = if raw_word_refs.len() >= 3
        && raw_word_refs[raw_word_refs.len() - 3..] == ["with", "that", "name"]
    {
        Some(3usize)
    } else if raw_word_refs.len() >= 4
        && raw_word_refs[raw_word_refs.len() - 4..] == ["with", "the", "chosen", "name"]
    {
        Some(4usize)
    } else if raw_word_refs.len() >= 3
        && raw_word_refs[raw_word_refs.len() - 3..] == ["with", "chosen", "name"]
    {
        Some(3usize)
    } else {
        None
    };
    if let Some(suffix_len) = same_name_suffix_len {
        let keep_word_count = raw_word_refs.len().saturating_sub(suffix_len);
        let base_end = if keep_word_count == 0 {
            0
        } else {
            raw_word_view
                .token_start_indices()
                .get(keep_word_count)
                .copied()
                .unwrap_or(filter_tokens.len())
        };
        filter_tokens = trim_commas(&filter_tokens[..base_end]).to_vec();
    }

    let words_all = crate::cards::builders::parser::token_word_refs(&filter_tokens);
    let non_article_words = words_all
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    if matches!(
        non_article_words.as_slice(),
        ["chosen", "card"] | ["chosen", "cards"]
    ) {
        let mut filter = ObjectFilter::default();
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
        return Some(filter);
    }
    if matches!(non_article_words.as_slice(), ["card"] | ["cards"]) {
        let mut filter = ObjectFilter::default();
        if same_name_suffix_len.is_some() {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }
    if matches!(
        words_all.as_slice(),
        ["permanent", "card"] | ["permanent", "cards"]
    ) {
        let mut filter = ObjectFilter::permanent_card();
        if same_name_suffix_len.is_some() {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }

    let has_noncomparison_or = filter_tokens.iter().enumerate().any(|(idx, token)| {
        token.is_word("or") && !is_comparison_or_delimiter(&filter_tokens, idx)
    });
    if has_noncomparison_or {
        let shared_card_suffix = matches!(words_all.last().copied(), Some("card" | "cards"));
        let segments = split_reveal_filter_segments(&filter_tokens);
        if segments.len() >= 2 {
            let mut branches = Vec::new();
            for mut segment in segments {
                if shared_card_suffix
                    && !matches!(
                        segment.last().and_then(OwnedLexToken::as_word),
                        Some("card" | "cards")
                    )
                {
                    segment.push(OwnedLexToken::word(
                        "card".to_string(),
                        TextSpan::synthetic(),
                    ));
                }
                let parsed = parse_object_filter(&segment, false)
                    .ok()
                    .filter(|filter| *filter != ObjectFilter::default())
                    .or_else(|| parse_named_card_filter_segment(&segment));
                let Some(parsed) = parsed else {
                    return None;
                };
                branches.push(parsed);
            }
            let mut filter = ObjectFilter::default();
            filter.any_of = branches;
            if same_name_suffix_len.is_some() {
                filter = filter.match_tagged(
                    TagKey::from(CHOSEN_NAME_TAG),
                    TaggedOpbjectRelation::SameNameAsTagged,
                );
            }
            return Some(filter);
        }
    }

    let mut filter = parse_search_library_disjunction_filter(&filter_tokens)
        .or_else(|| parse_object_filter(&filter_tokens, false).ok())?;
    if same_name_suffix_len.is_some() {
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
    }
    Some(filter)
}

fn parse_may_put_filtered_card_from_among_into_hand(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
    zone: Zone,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_words = crate::cards::builders::parser::token_word_refs(&sentence_tokens);
    if sentence_words.is_empty() {
        return Ok(None);
    }

    let (chooser, action_word_idx) = if slice_starts_with(&sentence_words, &["you", "may", "put"]) {
        (PlayerAst::You, 2usize)
    } else if slice_starts_with(&sentence_words, &["that", "player", "may", "put"]) {
        (PlayerAst::That, 3usize)
    } else if slice_starts_with(&sentence_words, &["they", "may", "put"]) {
        (PlayerAst::That, 2usize)
    } else if slice_starts_with(&sentence_words, &["may", "put"]) {
        (default_player, 1usize)
    } else if slice_starts_with(&sentence_words, &["put"]) {
        (default_player, 0usize)
    } else {
        return Ok(None);
    };

    let from_among_word_idx = find_window_index(&sentence_words, &["from", "among", "them"])
        .or_else(|| find_window_index(&sentence_words, &["from", "among", "those", "cards"]));
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= action_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&sentence_tokens, action_word_idx + 1)
        .unwrap_or(sentence_tokens.len());
    let filter_end = token_index_for_word_index(&sentence_tokens, from_among_word_idx)
        .unwrap_or(sentence_tokens.len());
    let filter_tokens = trim_commas(&sentence_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = Some(zone);

    let after_from_word_idx =
        if find_window_index(&sentence_words, &["from", "among", "those", "cards"]).is_some() {
            from_among_word_idx + 4
        } else {
            from_among_word_idx + 3
        };
    let after_from_words = &sentence_words[after_from_word_idx..];
    let moves_into_hand =
        slice_starts_with(after_from_words, &["into"]) && slice_contains(after_from_words, &"hand");
    if !moves_into_hand {
        return Ok(None);
    }

    Ok(Some((chooser, filter)))
}

fn parse_mill_then_may_put_from_among_into_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::Mill { player, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let Some((chooser, filter)) =
        parse_may_put_filtered_card_from_among_into_hand(second, *player, Zone::Graveyard)?
    else {
        return Ok(None);
    };

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
            player: chooser,
            filter,
            reveal: false,
            if_not_chosen: Vec::new(),
        },
    ]))
}

fn parse_mill_then_may_put_from_among_into_hand_then_if_you_dont(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = parse_mill_then_may_put_from_among_into_hand(first, second)? else {
        return Ok(None);
    };
    let Some(if_not_chosen) = parse_if_you_dont_sentence(third)? else {
        return Ok(None);
    };

    let Some(EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
        if_not_chosen: existing,
        ..
    }) = effects.get_mut(1)
    else {
        return Ok(None);
    };
    *existing = if_not_chosen;
    Ok(Some(effects))
}

fn parse_triple_sentence_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, TripleSentenceRule); 10] = [
        (
            "mill-then-put-from-among-into-hand-then-if-you-dont",
            parse_mill_then_may_put_from_among_into_hand_then_if_you_dont,
        ),
        (
            "search-face-down-exile-conditional-cast-else-hand",
            parse_search_face_down_exile_conditional_cast_else_hand,
        ),
        (
            "search-then-next-upkeep-unless-pays-lose-game",
            parse_search_then_delayed_next_upkeep_unless_pays_lose_game,
        ),
        (
            "exile-until-match-cast-rest-bottom",
            parse_exile_until_match_cast_rest_bottom,
        ),
        (
            "exile-until-match-cast-else-hand",
            parse_exile_until_match_cast_else_hand,
        ),
        (
            "top-cards-put-match-into-hand-rest-graveyard",
            parse_top_cards_put_match_into_hand_rest_graveyard,
        ),
        (
            "look-at-top-reveal-match-put-rest-bottom",
            parse_look_at_top_reveal_match_put_rest_bottom,
        ),
        (
            "prefix-then-consult-match-move-bottom-remainder",
            parse_prefix_then_consult_match_move_and_bottom_remainder,
        ),
        (
            "prefix-then-consult-match-into-hand-exile-others",
            parse_prefix_then_consult_match_into_hand_exile_others,
        ),
        ("tainted-pact-sequence", parse_tainted_pact_sequence),
    ];

    fn run_triple_rules(
        rules: &[(&'static str, TripleSentenceRule)],
        first: &[OwnedLexToken],
        second: &[OwnedLexToken],
        third: &[OwnedLexToken],
    ) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
        for (name, rule) in rules {
            if let Some(combined) = rule(first, second, third)? {
                return Ok(Some((*name, combined)));
            }
        }

        Ok(None)
    }

    let preferred_rules: &[(&str, TripleSentenceRule)] = match lexed_head_words(first) {
        Some(("mill", _)) => &[(
            "mill-then-put-from-among-into-hand-then-if-you-dont",
            parse_mill_then_may_put_from_among_into_hand_then_if_you_dont,
        )],
        Some(("search", _)) => &[
            (
                "search-face-down-exile-conditional-cast-else-hand",
                parse_search_face_down_exile_conditional_cast_else_hand,
            ),
            (
                "search-then-next-upkeep-unless-pays-lose-game",
                parse_search_then_delayed_next_upkeep_unless_pays_lose_game,
            ),
        ],
        Some(("exile", _)) => &[
            (
                "exile-until-match-cast-rest-bottom",
                parse_exile_until_match_cast_rest_bottom,
            ),
            (
                "exile-until-match-cast-else-hand",
                parse_exile_until_match_cast_else_hand,
            ),
            ("tainted-pact-sequence", parse_tainted_pact_sequence),
        ],
        Some(("look", Some("at"))) => &[
            (
                "look-at-top-reveal-match-put-rest-bottom",
                parse_look_at_top_reveal_match_put_rest_bottom,
            ),
            (
                "prefix-then-consult-match-move-bottom-remainder",
                parse_prefix_then_consult_match_move_and_bottom_remainder,
            ),
            (
                "prefix-then-consult-match-into-hand-exile-others",
                parse_prefix_then_consult_match_into_hand_exile_others,
            ),
        ],
        Some(("reveal", Some("the"))) | Some(("reveal", Some("top"))) => &[(
            "top-cards-put-match-into-hand-rest-graveyard",
            parse_top_cards_put_match_into_hand_rest_graveyard,
        )],
        _ => &[],
    };

    if let Some(hit) = run_triple_rules(preferred_rules, first, second, third)? {
        return Ok(Some(hit));
    }

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second, third)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

fn parse_search_face_down_exile_conditional_cast_else_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_effects = parse_effect_chain(first)?;
    let searched_tag: TagKey = "searched_face_down".into();
    let has_face_down_search = first_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ChooseObjectsAcrossZones { tag, .. } if *tag == searched_tag
        ) || matches!(
            effect,
            EffectAst::ChooseObjects { tag, .. } if *tag == searched_tag
        )
    }) && first_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::Exile {
                target: TargetAst::Tagged(tag, _),
                face_down: true,
            } if *tag == searched_tag
        )
    });
    if !has_face_down_search {
        return Ok(None);
    }

    let Some(hand_effects) = parse_if_declined_put_match_into_hand(third, searched_tag.clone())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = crate::cards::builders::parser::token_word_refs(&second_tokens);
    if !slice_starts_with(&second_words, &["if", "this", "spell", "was", "bargained"]) {
        return Ok(None);
    }
    let Some(effect_start_word_idx) = find_index(&second_words, |word| *word == "you") else {
        return Ok(None);
    };
    let effect_words = &second_words[effect_start_word_idx..];
    let Some((operator, right)) = parse_face_down_search_cast_mana_value_gate(effect_words) else {
        return Ok(None);
    };
    let combined_predicate = PredicateAst::And(
        Box::new(PredicateAst::ThisSpellPaidLabel("Bargain".to_string())),
        Box::new(PredicateAst::ValueComparison {
            left: Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(
                searched_tag.clone(),
            ))),
            operator,
            right,
        }),
    );
    let mut effects = first_effects;
    effects.push(EffectAst::Conditional {
        predicate: combined_predicate,
        if_true: vec![
            EffectAst::May {
                effects: vec![EffectAst::CastTagged {
                    tag: searched_tag.clone(),
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
                }],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::WasDeclined,
                effects: hand_effects.clone(),
            },
        ],
        if_false: hand_effects,
    });
    Ok(Some(effects))
}

fn parse_face_down_search_cast_mana_value_gate(
    words: &[&str],
) -> Option<(crate::effect::ValueComparisonOperator, Value)> {
    let remainder = if slice_starts_with(words, &["you", "may", "cast", "the", "exiled", "card"]) {
        &words[6..]
    } else if slice_starts_with(words, &["you", "may", "cast", "that", "card"]) {
        &words[5..]
    } else if slice_starts_with(words, &["you", "may", "cast", "it"]) {
        &words[4..]
    } else {
        return None;
    };

    if !slice_starts_with(
        remainder,
        &["without", "paying", "its", "mana", "cost", "if"],
    ) {
        return None;
    }
    let condition = &remainder[5..];
    if (slice_starts_with(condition, &["if", "that", "spell's", "mana", "value", "is"])
        || slice_starts_with(condition, &["if", "that", "spells", "mana", "value", "is"]))
        && condition.len() == 9
        && condition[7] == "or"
        && condition[8] == "less"
    {
        let value = condition[6].parse::<i32>().ok()?;
        return Some((
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            Value::Fixed(value),
        ));
    }
    if (slice_starts_with(
        condition,
        &[
            "if", "that", "spell's", "mana", "value", "is", "less", "than", "or", "equal", "to",
        ],
    ) || slice_starts_with(
        condition,
        &[
            "if", "that", "spells", "mana", "value", "is", "less", "than", "or", "equal", "to",
        ],
    )) && condition.len() == 12
    {
        let value = condition[11].parse::<i32>().ok()?;
        return Some((
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            Value::Fixed(value),
        ));
    }
    None
}

fn parse_search_then_delayed_next_upkeep_unless_pays_lose_game(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_effects = parse_effect_chain(first)?;
    let first_words = crate::cards::builders::parser::token_word_refs(first);
    if first_effects.is_empty() || !slice_starts_with(&first_words, &["search", "your", "library"])
    {
        return Ok(None);
    }

    let upkeep_tokens = trim_commas(second);
    let upkeep_words = crate::cards::builders::parser::token_word_refs(&upkeep_tokens);
    let pay_idx = if slice_starts_with(
        &upkeep_words,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "your",
            "next",
            "upkeep",
            "pay",
        ],
    ) {
        7usize
    } else if slice_starts_with(
        &upkeep_words,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "next",
            "upkeep",
            "pay",
        ],
    ) {
        7usize
    } else {
        return Ok(None);
    };
    let Some(pay_token_idx) = token_index_for_word_index(&upkeep_tokens, pay_idx) else {
        return Ok(None);
    };
    let mana_tokens = trim_commas(&upkeep_tokens[pay_token_idx + 1..]);
    if mana_tokens.is_empty() {
        return Ok(None);
    }

    let mut mana = Vec::<ManaSymbol>::new();
    for token in mana_tokens {
        if let Some(pips) = mana_pips_from_token(&token) {
            mana.extend(pips);
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if let Ok(generic) = word.parse::<u8>() {
            mana.push(ManaSymbol::Generic(generic));
            continue;
        }
        return Ok(None);
    }
    if mana.is_empty() {
        return Ok(None);
    }

    let lose_tokens = trim_commas(third);
    let lose_words = crate::cards::builders::parser::token_word_refs(&lose_tokens);
    let valid_lose_clause = lose_words == ["if", "you", "dont", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "don't", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "do", "not", "you", "lose", "the", "game"];
    if !valid_lose_clause {
        return Ok(None);
    }

    let mut effects = first_effects;
    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::LoseGame {
                player: PlayerAst::You,
            }],
            player: PlayerAst::You,
            mana,
        }],
    });
    Ok(Some(effects))
}

fn parse_if_no_card_into_hand_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let has_expected_prefix = slice_starts_with(
        &words,
        &[
            "if", "you", "didnt", "put", "card", "into", "your", "hand", "this", "way",
        ],
    ) || slice_starts_with(
        &words,
        &[
            "if", "you", "didn't", "put", "card", "into", "your", "hand", "this", "way",
        ],
    ) || slice_starts_with(
        &words,
        &[
            "if", "you", "did", "not", "put", "card", "into", "your", "hand", "this", "way",
        ],
    );
    if !has_expected_prefix {
        return Ok(None);
    }

    let Some(comma_idx) = find_index(tokens, |token: &OwnedLexToken| token.is_comma()) else {
        return Ok(None);
    };
    if comma_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

fn parse_if_you_dont_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let has_expected_prefix = slice_starts_with(&words, &["if", "you", "dont"])
        || slice_starts_with(&words, &["if", "you", "don't"])
        || slice_starts_with(&words, &["if", "you", "do", "not"]);
    if !has_expected_prefix {
        return Ok(None);
    }

    let Some(comma_idx) = find_index(tokens, |token: &OwnedLexToken| token.is_comma()) else {
        return Ok(None);
    };
    if comma_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

fn parse_look_at_top_reveal_match_put_rest_bottom_then_if_not_into_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = parse_look_at_top_reveal_match_put_rest_bottom(first, second, third)?
    else {
        return Ok(None);
    };
    let Some(if_not_chosen) = parse_if_no_card_into_hand_this_way_sentence(fourth)? else {
        return Ok(None);
    };

    let Some(EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
        if_not_chosen: existing,
        ..
    }) = effects.get_mut(1)
    else {
        return Ok(None);
    };
    *existing = if_not_chosen;
    Ok(Some(effects))
}

fn parse_quad_sentence_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, QuadSentenceRule); 3] = [
        (
            "look-at-top-put-counted-into-hand-rest-bottom-kicker-override",
            parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override,
        ),
        (
            "look-at-top-may-put-match-onto-battlefield-if-not-put-into-hand-rest-bottom",
            parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom,
        ),
        (
            "look-at-top-reveal-match-put-rest-bottom-if-not-into-hand",
            parse_look_at_top_reveal_match_put_rest_bottom_then_if_not_into_hand,
        ),
    ];

    if !matches!(lexed_head_words(first), Some(("look", Some("at")))) {
        return Ok(None);
    }

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second, third, fourth)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

fn parse_effect_sentences_from_sentence_inputs(
    sentences: Vec<SentenceInput>,
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = try_parse_divvy_sentence_sequence(&sentences)? {
        return Ok(effects);
    }

    let mut effects = Vec::new();
    let mut sentence_idx = 0usize;
    let mut carried_context: Option<CarryContext> = None;

    fn effect_contains_search_library(effect: &EffectAst) -> bool {
        if matches!(effect, EffectAst::SearchLibrary { .. }) {
            return true;
        }

        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            if !found {
                found = nested.iter().any(effect_contains_search_library);
            }
        });
        found
    }

    fn effect_needs_followup_library_shuffle(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::ChooseObjectsAcrossZones { zones, .. } if slice_contains(zones, &Zone::Library)
        ) {
            return true;
        }

        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            if !found {
                found = nested.iter().any(effect_needs_followup_library_shuffle);
            }
        });
        found
    }

    fn is_if_you_search_library_this_way_shuffle_sentence(tokens: &[OwnedLexToken]) -> bool {
        let words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        // "If you search your library this way, shuffle."
        words.as_slice()
            == [
                "if", "you", "search", "your", "library", "this", "way", "shuffle",
            ]
            || words.as_slice()
                == [
                    "if", "you", "search", "your", "library", "this", "way", "shuffles",
                ]
    }

    while sentence_idx < sentences.len() {
        let sentence = sentences[sentence_idx].lowered();
        if sentence.is_empty() {
            sentence_idx += 1;
            continue;
        }

        if sentence_idx + 3 < sentences.len()
            && let Some((rule_name, mut combined)) = parse_quad_sentence_sequence(
                sentence,
                sentences[sentence_idx + 1].lowered(),
                sentences[sentence_idx + 2].lowered(),
                sentences[sentence_idx + 3].lowered(),
            )?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 4;
            continue;
        }

        if sentence_idx + 2 < sentences.len()
            && let Some((rule_name, mut combined)) = parse_triple_sentence_sequence(
                sentence,
                sentences[sentence_idx + 1].lowered(),
                sentences[sentence_idx + 2].lowered(),
            )?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 3;
            continue;
        }

        if sentence_idx + 1 < sentences.len()
            && let Some((rule_name, mut combined)) =
                parse_pair_sentence_sequence(sentence, sentences[sentence_idx + 1].lowered())?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 2;
            continue;
        }
        let mut sentence_tokens = strip_embedded_token_rules_text(sentence);
        sentence_tokens = trim_edge_punctuation(&sentence_tokens);
        if sentence_tokens.is_empty()
            || crate::cards::builders::parser::token_word_refs(&sentence_tokens).is_empty()
        {
            sentence_idx += 1;
            continue;
        }
        sentence_tokens = rewrite_when_one_or_more_this_way_clause_prefix(&sentence_tokens);

        // Oracle frequently splits shuffle followups as a standalone sentence:
        // "If you search your library this way, shuffle." This clause is redundant when the
        // preceding sentence already compiles a library-search effect that shuffles.
        if is_if_you_search_library_this_way_shuffle_sentence(&sentence_tokens) {
            if effects.iter().any(effect_needs_followup_library_shuffle) {
                parser_trace(
                    "parse_effect_sentences:append:if-you-search-library-this-way-shuffle",
                    &sentence_tokens,
                );
                effects.push(EffectAst::ShuffleLibrary {
                    player: PlayerAst::You,
                });
                sentence_idx += 1;
                continue;
            }
            if effects.iter().any(effect_contains_search_library) {
                parser_trace(
                    "parse_effect_sentences:skip:if-you-search-library-this-way-shuffle",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
        }

        let sentence_words = crate::cards::builders::parser::token_word_refs(&sentence_tokens);
        let is_still_lands_followup = matches!(
            sentence_words.as_slice(),
            ["theyre", "still", "land"]
                | ["theyre", "still", "lands"]
                | ["its", "still", "a", "land"]
                | ["its", "still", "land"]
        );
        if is_still_lands_followup
            && effects
                .last()
                .is_some_and(|effect| matches!(effect, EffectAst::BecomeBasePtCreature { .. }))
        {
            parser_trace(
                "parse_effect_sentences:skip:still-lands-followup",
                &sentence_tokens,
            );
            sentence_idx += 1;
            continue;
        }

        let mut wraps_as_if_did_not = false;
        if let Some(without_otherwise) = strip_otherwise_sentence_prefix(&sentence_tokens) {
            sentence_tokens = rewrite_otherwise_referential_subject(without_otherwise);
            wraps_as_if_did_not = true;
        }
        parser_trace("parse_effect_sentences:sentence", &sentence_tokens);

        // "Destroy ... . It/They can't be regenerated." followups.
        if is_cant_be_regenerated_followup_sentence(&sentence_tokens) {
            if apply_cant_be_regenerated_to_last_destroy_effect(&mut effects) {
                parser_trace(
                    "parse_effect_sentences:cant-be-regenerated-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            if is_cant_be_regenerated_this_turn_followup_sentence(&sentence_tokens)
                && apply_cant_be_regenerated_to_last_target_effect(&mut effects)
            {
                parser_trace(
                    "parse_effect_sentences:cant-be-regenerated-this-turn-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone cant-be-regenerated clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(&sentence_tokens).join(" ")
            )));
        }

        if let Some((mut copy_effects, spec)) =
            parse_same_sentence_copy_and_may_cast_copy(&sentence_tokens)?
        {
            parser_trace(
                "parse_effect_sentences:copy-reference-same-sentence-may-cast-copy",
                &sentence_tokens,
            );
            effects.append(&mut copy_effects);
            effects.push(build_may_cast_tagged_effect(&spec));
            sentence_idx += 1;
            continue;
        }

        if sentence_idx + 1 < sentences.len() && is_simple_copy_reference_sentence(&sentence_tokens)
        {
            let next_tokens = strip_embedded_token_rules_text(sentences[sentence_idx + 1].lexed());
            if let Some(spec) = parse_may_cast_it_sentence(&next_tokens)
                && spec.as_copy
            {
                parser_trace(
                    "parse_effect_sentences:copy-reference-next-may-cast-copy",
                    &sentence_tokens,
                );
                effects.extend(parse_effect_sentence_lexed(&sentence_tokens)?);
                effects.push(build_may_cast_tagged_effect(&spec));
                sentence_idx += 2;
                continue;
            }
        }

        if let Some(reduction) =
            super::super::activation_and_restrictions::parse_copy_reference_cost_reduction_sentence(
                &sentence_tokens,
            )
        {
            if attach_copy_cost_reduction_to_effects(&mut effects, &reduction) {
                parser_trace(
                    "parse_effect_sentences:copy-reference-cost-reduction-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone copy cost-reduction clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(&sentence_tokens).join(" ")
            )));
        }

        if let Some(spec) = parse_may_cast_it_sentence(&sentence_tokens) {
            parser_trace(
                "parse_effect_sentences:may-cast-it-sentence",
                &sentence_tokens,
            );
            effects.push(build_may_cast_tagged_effect(&spec));
            sentence_idx += 1;
            continue;
        }

        if is_spawn_scion_token_mana_reminder(&sentence_tokens) {
            if effects
                .last()
                .is_some_and(effect_creates_eldrazi_spawn_or_scion)
            {
                parser_trace(
                    "parse_effect_sentences:spawn-scion-reminder",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token mana reminder clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(&sentence_tokens).join(" ")
            )));
        }
        if let Some(effect) =
            parse_sentence_exile_that_token_when_source_leaves(&sentence_tokens, &effects)
        {
            parser_trace(
                "parse_effect_sentences:linked-token-exile-when-source-leaves",
                &sentence_tokens,
            );
            effects.push(effect);
            sentence_idx += 1;
            continue;
        }
        if let Some(effect) =
            parse_sentence_sacrifice_source_when_that_token_leaves(&sentence_tokens, &effects)
        {
            parser_trace(
                "parse_effect_sentences:linked-token-sacrifice-source-when-token-leaves",
                &sentence_tokens,
            );
            effects.push(effect);
            sentence_idx += 1;
            continue;
        }
        if is_generic_token_reminder_sentence(&sentence_tokens)
            && effects.last().is_some_and(effect_creates_any_token)
        {
            if append_token_reminder_to_last_create_effect(&mut effects, &sentence_tokens) {
                parser_trace(
                    "parse_effect_sentences:token-reminder-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token reminder clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(&sentence_tokens).join(" ")
            )));
        }
        if is_generic_token_reminder_sentence(&sentence_tokens) {
            let reminder_words = crate::cards::builders::parser::token_word_refs(&sentence_tokens);
            let delayed_pronoun_lifecycle =
                matches!(reminder_words.first().copied(), Some("exile" | "sacrifice"))
                    && (slice_contains(&reminder_words, &"it")
                        || slice_contains(&reminder_words, &"them"));
            let pronoun_followup_clause = slice_starts_with(&reminder_words, &["when", "it"])
                || slice_starts_with(&reminder_words, &["whenever", "it"])
                || slice_starts_with(&reminder_words, &["when", "they"])
                || slice_starts_with(&reminder_words, &["whenever", "they"]);
            if delayed_pronoun_lifecycle || pronoun_followup_clause {
                // Keep standalone pronoun-led followups on the normal parser path.
                // They can be genuine tagged-object effects, not just token reminder text.
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported standalone token reminder clause (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(&sentence_tokens).join(" ")
                )));
            }
        }

        if let Some(effect) = parse_choose_target_prelude_sentence(&sentence_tokens)? {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        let mut sentence_effects =
            if let Some(followup) = parse_token_copy_followup_sentence(&sentence_tokens) {
                if try_apply_token_copy_followup(&mut effects, followup)? {
                    parser_trace(
                        "parse_effect_sentences:token-copy-followup",
                        &sentence_tokens,
                    );
                    sentence_idx += 1;
                    continue;
                }
                apply_unapplied_token_copy_followup(sentence, &sentence_tokens, followup)?
            } else if sentence_tokens.as_slice() == sentences[sentence_idx].lexed() {
                parse_effect_sentence_lexed(sentences[sentence_idx].lexed())?
            } else {
                parse_effect_sentence_lexed(&sentence_tokens)?
            };
        if wraps_as_if_did_not {
            sentence_effects = vec![EffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: sentence_effects,
            }];
            carried_context = None;
        }
        collapse_token_copy_next_end_step_exile_followup(&mut sentence_effects, &sentence_tokens);
        collapse_token_copy_end_of_combat_exile_followup(&mut sentence_effects, &sentence_tokens);
        if is_that_turn_end_step_sentence(&sentence_tokens)
            && let Some(extra_turn_player) = most_recent_extra_turn_player(&effects)
            && !sentence_effects.is_empty()
        {
            sentence_effects = vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
                player: extra_turn_player,
                effects: sentence_effects,
            }];
        }
        if crate::cards::builders::parser::token_word_refs(&sentence_tokens)
            .first()
            .copied()
            == Some("you")
        {
            carried_context = None;
        }
        if sentence_effects.is_empty()
            && !is_round_up_each_time_sentence(&sentence_tokens)
            && !is_nonsemantic_restriction_sentence(&sentence_tokens)
        {
            return Err(CardTextError::ParseError(format!(
                "sentence parsed to no semantic effects (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(&sentence_tokens).join(" ")
            )));
        }
        for effect in &mut sentence_effects {
            if let Some(context) = carried_context {
                maybe_apply_carried_player_with_clause(effect, context, &sentence_tokens);
            }
            if let Some(context) = explicit_player_for_carry(effect) {
                carried_context = Some(context);
            }
        }
        if sentence_effects.len() == 1
            && let Some(previous_effect) = effects.last()
            && let Some(effect) = sentence_effects.first_mut()
            && let EffectAst::IfResult {
                predicate,
                effects: if_result_effects,
            } = effect
        {
            if matches!(*predicate, IfResultPredicate::Did)
                && matches!(previous_effect, EffectAst::UnlessPays { .. })
            {
                *predicate = IfResultPredicate::DidNot;
            }
            if let Some(previous_target) = primary_damage_target_from_effect(previous_effect) {
                replace_it_damage_target_in_effects(
                    if_result_effects.as_mut_slice(),
                    &previous_target,
                );
            }
        }
        let sentence_text =
            crate::cards::builders::parser::token_word_refs(&sentence_tokens).join(" ");
        maybe_rewrite_future_zone_replacement_sentence(&mut sentence_effects, &sentence_text);
        if matches!(
            classify_instead_followup_text(&sentence_text),
            InsteadSemantics::SelfReplacement
        ) && sentence_effects.len() == 1
            && effects.len() >= 1
        {
            if matches!(
                sentence_effects.first(),
                Some(EffectAst::Conditional { .. })
            ) {
                let Some(previous) = effects.pop() else {
                    return Err(CardTextError::InvariantViolation(
                        "expected previous effect for 'instead' conditional rewrite".to_string(),
                    ));
                };
                let previous_target = primary_target_from_effect(&previous);
                let previous_damage_target = primary_damage_target_from_effect(&previous);
                if let Some(EffectAst::Conditional {
                    predicate,
                    mut if_true,
                    mut if_false,
                }) = sentence_effects.pop()
                {
                    if let Some(target) = previous_target {
                        replace_it_target_in_effects(&mut if_true, &target);
                    }
                    if let Some(target) = previous_damage_target {
                        replace_it_damage_target_in_effects(&mut if_true, &target);
                        replace_placeholder_damage_target_in_effects(&mut if_true, &target);
                    }
                    if_false.insert(0, previous);
                    effects.push(EffectAst::SelfReplacement {
                        predicate,
                        if_true,
                        if_false,
                    });
                    sentence_idx += 1;
                    continue;
                }
            }
        }

        effects.extend(sentence_effects);
        sentence_idx += 1;
    }

    if let Some(last_sentence) = sentences.last() {
        parser_trace("parse_effect_sentences:done", last_sentence.lowered());
    }
    Ok(effects)
}

pub(crate) fn parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = parse_exact_card_effect_bundle_lexed(tokens) {
        return Ok(effects);
    }

    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    parse_effect_sentences_from_sentence_inputs(sentences)
}

pub(crate) fn is_cant_be_regenerated_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated"]
            | ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn is_cant_be_regenerated_this_turn_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn apply_cant_be_regenerated_to_last_destroy_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(last) = effects.last_mut() else {
        return false;
    };
    apply_cant_be_regenerated_to_effect(last)
}

pub(crate) fn apply_cant_be_regenerated_to_last_target_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(previous_target) = effects.last().and_then(primary_target_from_effect) else {
        return false;
    };
    let Some(mut filter) = target_ast_to_object_filter(previous_target) else {
        return false;
    };
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

    effects.push(EffectAst::Cant {
        restriction: crate::effect::Restriction::be_regenerated(filter),
        duration: Until::EndOfTurn,
        condition: None,
    });
    true
}

fn apply_cant_be_regenerated_to_effect(effect: &mut EffectAst) -> bool {
    match effect {
        EffectAst::Destroy { target } => {
            let target = target.clone();
            *effect = EffectAst::DestroyNoRegeneration { target };
            true
        }
        EffectAst::DestroyAll { filter } => {
            let filter = filter.clone();
            *effect = EffectAst::DestroyAllNoRegeneration { filter };
            true
        }
        EffectAst::DestroyAllOfChosenColor { filter } => {
            let filter = filter.clone();
            *effect = EffectAst::DestroyAllOfChosenColorNoRegeneration { filter };
            true
        }
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, true, |nested| {
                if !applied {
                    applied = apply_cant_be_regenerated_to_effects_tail(nested);
                }
            });
            applied
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::effect::{Value, ValueComparisonOperator};
    use crate::filter::TaggedOpbjectRelation;

    use super::super::super::lexer::lex_line;
    use super::{
        ConsultCastCost, ConsultCastTiming, parse_consult_cast_clause,
        parse_consult_mana_value_condition_tokens, parse_looked_card_reveal_filter,
    };

    #[test]
    fn consult_mana_value_condition_normalizes_spell_apostrophe_prefix() {
        let tokens = lex_line("if that spell's mana value is 3 or less", 0)
            .expect("rewrite lexer should classify consult mana-value condition");

        let parsed = parse_consult_mana_value_condition_tokens(&tokens)
            .expect("consult mana-value condition should parse");

        assert_eq!(parsed.operator, ValueComparisonOperator::LessThanOrEqual);
        assert_eq!(parsed.right, Value::Fixed(3));
    }

    #[test]
    fn consult_cast_clause_keeps_this_turn_remainder_without_word_view() {
        let tokens = lex_line("You may cast it this turn", 0)
            .expect("rewrite lexer should classify consult cast clause");

        let parsed = parse_consult_cast_clause(&tokens).expect("consult cast clause should parse");

        assert_eq!(parsed.caster, crate::cards::builders::PlayerAst::You);
        assert!(!parsed.allow_land);
        assert_eq!(parsed.timing, ConsultCastTiming::UntilEndOfTurn);
        assert_eq!(parsed.cost, ConsultCastCost::Normal);
        assert!(parsed.mana_value_condition.is_none());
    }

    #[test]
    fn looked_card_reveal_filter_strips_same_name_suffix_without_word_view() {
        let tokens = lex_line("card with that name", 0)
            .expect("rewrite lexer should classify looked-card reveal filter");

        let parsed = parse_looked_card_reveal_filter(&tokens)
            .expect("looked-card reveal filter should parse");

        assert_eq!(parsed.tagged_constraints.len(), 1);
        assert_eq!(
            parsed.tagged_constraints[0].relation,
            TaggedOpbjectRelation::SameNameAsTagged
        );
    }
}

fn apply_cant_be_regenerated_to_effects_tail(effects: &mut [EffectAst]) -> bool {
    for effect in effects.iter_mut().rev() {
        if apply_cant_be_regenerated_to_effect(effect) {
            return true;
        }
    }
    false
}

pub(crate) fn primary_damage_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. } | EffectAst::DealDamageEqualToPower { target, .. } => {
            Some(target.clone())
        }
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_damage_target_from_effect);
                }
            });
            found
        }
    }
}

pub(crate) fn primary_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. }
        | EffectAst::DealDamageEqualToPower { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::CounterUnlessPays { target, .. }
        | EffectAst::Explore { target }
        | EffectAst::Connive { target }
        | EffectAst::Detain { target }
        | EffectAst::Goad { target }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::RemoveFromCombat { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::DestroyNoRegeneration { target }
        | EffectAst::Exile { target, .. }
        | EffectAst::ExileWhenSourceLeaves { target }
        | EffectAst::SacrificeSourceWhenLeaves { target }
        | EffectAst::ExileUntilSourceLeaves { target, .. }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Convert { target }
        | EffectAst::Flip { target }
        | EffectAst::Regenerate { target }
        | EffectAst::PhaseOut { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::ReturnToHand { target, .. }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToZone { target, .. }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::PutOrRemoveCounters { target, .. }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::GrantToTarget { target, .. }
        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. }
        | EffectAst::GrantProtectionChoice { target, .. }
        | EffectAst::PreventDamage { target, .. }
        | EffectAst::PreventAllDamageToTarget { target, .. }
        | EffectAst::PreventDamageToTargetPutCounters { target, .. }
        | EffectAst::PreventAllCombatDamageFromSource { source: target, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { target, .. }
        | EffectAst::RedirectNextTimeDamageToSource { target, .. }
        | EffectAst::GainControl { target, .. } => Some(target.clone()),
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_target_from_effect);
                }
            });
            found
        }
    }
}

pub(crate) fn replace_it_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_damage_target(effect, target);
    }
}

pub(crate) fn replace_it_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_target(effect, target);
    }
}

pub(crate) fn is_placeholder_damage_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
    )
}

pub(crate) fn replace_placeholder_damage_target_in_effects(
    effects: &mut [EffectAst],
    target: &TargetAst,
) {
    for effect in effects {
        replace_placeholder_damage_target(effect, target);
    }
}

pub(crate) fn replace_placeholder_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: damage_target,
            ..
        } => {
            if is_placeholder_damage_target(damage_target) {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_placeholder_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_unbound_x_in_damage_effects(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_damage_effect(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_damage_effect(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::LoseLife { amount, .. } => {
            if value_contains_unbound_x(amount) {
                *amount = replace_unbound_x_with_value(amount.clone(), replacement, clause)?;
            }
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_damage_effects(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effects_anywhere(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_effect_anywhere(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effect_anywhere(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    fn replace_in_comparison(
        comparison: &mut crate::filter::Comparison,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        use crate::filter::Comparison;

        let value = match comparison {
            Comparison::EqualExpr(value)
            | Comparison::NotEqualExpr(value)
            | Comparison::LessThanExpr(value)
            | Comparison::LessThanOrEqualExpr(value)
            | Comparison::GreaterThanExpr(value)
            | Comparison::GreaterThanOrEqualExpr(value) => value,
            _ => return Ok(()),
        };

        if value_contains_unbound_x(value) {
            **value = replace_unbound_x_with_value((**value).clone(), replacement, clause)?;
        }
        Ok(())
    }

    fn replace_in_filter(
        filter: &mut ObjectFilter,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        if let Some(power) = filter.power.as_mut() {
            replace_in_comparison(power, replacement, clause)?;
        }
        if let Some(toughness) = filter.toughness.as_mut() {
            replace_in_comparison(toughness, replacement, clause)?;
        }
        if let Some(mana_value) = filter.mana_value.as_mut() {
            replace_in_comparison(mana_value, replacement, clause)?;
        }
        if let Some(targets_object) = filter.targets_object.as_mut() {
            replace_in_filter(targets_object, replacement, clause)?;
        }
        if let Some(targets_only_object) = filter.targets_only_object.as_mut() {
            replace_in_filter(targets_only_object, replacement, clause)?;
        }
        for nested in &mut filter.any_of {
            replace_in_filter(nested, replacement, clause)?;
        }
        Ok(())
    }

    fn replace_value(
        value: &mut Value,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        if value_contains_unbound_x(value) {
            *value = replace_unbound_x_with_value(value.clone(), replacement, clause)?;
        }
        Ok(())
    }

    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::PutCounters { count: amount, .. }
        | EffectAst::PutCountersAll { count: amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::Discard { count: amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::LookAtTopCards { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. }
        | EffectAst::CopySpell { count: amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::Monstrosity { amount } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::Pump {
            power, toughness, ..
        }
        | EffectAst::SetBasePowerToughness {
            power, toughness, ..
        }
        | EffectAst::BecomeBasePtCreature {
            power, toughness, ..
        }
        | EffectAst::PumpAll {
            power, toughness, ..
        } => {
            replace_value(power, replacement, clause)?;
            replace_value(toughness, replacement, clause)?;
        }
        EffectAst::SetBasePower { power, .. } => {
            replace_value(power, replacement, clause)?;
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            replace_value(put_count, replacement, clause)?;
            replace_value(remove_count, replacement, clause)?;
        }
        EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::CreateTokenCopy { count, .. }
        | EffectAst::CreateTokenCopyFromSource { count, .. } => {
            replace_value(count, replacement, clause)?;
        }
        EffectAst::SearchLibrary { filter, .. } => {
            replace_in_filter(filter, replacement, clause)?;
        }
        EffectAst::CreateTokenWithMods {
            count,
            dynamic_power_toughness,
            ..
        } => {
            replace_value(count, replacement, clause)?;
            if let Some((power, toughness)) = dynamic_power_toughness {
                replace_value(power, replacement, clause)?;
                replace_value(toughness, replacement, clause)?;
            }
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            if let Some(life) = life.as_mut() {
                replace_value(life, replacement, clause)?;
            }
            if let Some(generic) = additional_generic.as_mut() {
                replace_value(generic, replacement, clause)?;
            }
        }
        EffectAst::PumpForEach { count, .. } => {
            replace_value(count, replacement, clause)?;
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_effects_anywhere(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn apply_where_x_to_damage_amounts(
    tokens: &[OwnedLexToken],
    effects: &mut [EffectAst],
) -> Result<(), CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let has_deal_x = find_window_by(&clause_words, 3, |window| {
        (window[0] == "deal" || window[0] == "deals") && window[1] == "x" && window[2] == "damage"
    })
    .is_some();
    let has_x_life = find_window_by(&clause_words, 3, |window| {
        (window[0] == "gain" || window[0] == "gains" || window[0] == "lose" || window[0] == "loses")
            && window[1] == "x"
            && window[2] == "life"
    })
    .is_some();
    if !has_deal_x && !has_x_life {
        return Ok(());
    }
    let Some(where_idx) = find_window_index(&clause_words, &["where", "x", "is"]) else {
        return Ok(());
    };
    let Some(where_token_idx) = token_index_for_word_index(tokens, where_idx) else {
        return Ok(());
    };
    let where_tokens = &tokens[where_token_idx..];
    let Some(where_value) = parse_where_x_value_clause(where_tokens) else {
        return Ok(());
    };
    replace_unbound_x_in_damage_effects(effects, &where_value, &clause_words.join(" "))
}

pub(crate) fn replace_it_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        } => {
            if target_references_it(damage_target) {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_it_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: effect_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: effect_target,
            ..
        }
        | EffectAst::Counter {
            target: effect_target,
        }
        | EffectAst::CounterUnlessPays {
            target: effect_target,
            ..
        }
        | EffectAst::Explore {
            target: effect_target,
        }
        | EffectAst::Connive {
            target: effect_target,
        }
        | EffectAst::Detain {
            target: effect_target,
        }
        | EffectAst::Goad {
            target: effect_target,
        }
        | EffectAst::Tap {
            target: effect_target,
        }
        | EffectAst::Untap {
            target: effect_target,
        }
        | EffectAst::PhaseOut {
            target: effect_target,
        }
        | EffectAst::RemoveFromCombat {
            target: effect_target,
        }
        | EffectAst::TapOrUntap {
            target: effect_target,
        }
        | EffectAst::Destroy {
            target: effect_target,
        }
        | EffectAst::DestroyNoRegeneration {
            target: effect_target,
        }
        | EffectAst::Exile {
            target: effect_target,
            ..
        }
        | EffectAst::ExileWhenSourceLeaves {
            target: effect_target,
        }
        | EffectAst::SacrificeSourceWhenLeaves {
            target: effect_target,
        }
        | EffectAst::ExileUntilSourceLeaves {
            target: effect_target,
            ..
        }
        | EffectAst::LookAtHand {
            target: effect_target,
        }
        | EffectAst::Transform {
            target: effect_target,
        }
        | EffectAst::Convert {
            target: effect_target,
        }
        | EffectAst::Flip {
            target: effect_target,
        }
        | EffectAst::Regenerate {
            target: effect_target,
        }
        | EffectAst::TargetOnly {
            target: effect_target,
        }
        | EffectAst::ReturnToHand {
            target: effect_target,
            ..
        }
        | EffectAst::ReturnToBattlefield {
            target: effect_target,
            ..
        }
        | EffectAst::MoveToZone {
            target: effect_target,
            ..
        }
        | EffectAst::PutCounters {
            target: effect_target,
            ..
        }
        | EffectAst::PutOrRemoveCounters {
            target: effect_target,
            ..
        }
        | EffectAst::RemoveUpToAnyCounters {
            target: effect_target,
            ..
        }
        | EffectAst::Pump {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesChoiceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantProtectionChoice {
            target: effect_target,
            ..
        }
        | EffectAst::PreventDamage {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllDamageToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::PreventDamageToTargetPutCounters {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllCombatDamageFromSource {
            source: effect_target,
            ..
        }
        | EffectAst::RedirectNextDamageFromSourceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::RedirectNextTimeDamageToSource {
            target: effect_target,
            ..
        }
        | EffectAst::GainControl {
            target: effect_target,
            ..
        } => {
            if target_references_it(effect_target) {
                *effect_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn target_references_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        TargetAst::WithCount(inner, _) => target_references_it(inner),
        _ => false,
    }
}

pub(crate) fn is_that_turn_end_step_sentence(tokens: &[OwnedLexToken]) -> bool {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    slice_starts_with(
        &clause_words,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "that",
            "turn",
            "end",
            "step",
        ],
    ) || slice_starts_with(
        &clause_words,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "that",
            "turns",
            "end",
            "step",
        ],
    )
}

pub(crate) fn most_recent_extra_turn_player(effects: &[EffectAst]) -> Option<PlayerAst> {
    effects.iter().rev().find_map(|effect| {
        if let EffectAst::ExtraTurnAfterTurn { player, .. } = effect {
            Some(*player)
        } else {
            None
        }
    })
}

pub(crate) fn rewrite_when_one_or_more_this_way_clause_prefix(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    // Generic "When one or more ... this way, ..." follow-ups are semantically
    // "If you do, ..." against the immediately previous effect result.
    let has_this_way = find_window_index(&clause_words, &["this", "way"]).is_some();
    if (slice_starts_with(&clause_words, &["when", "one", "or", "more"])
        || slice_starts_with(&clause_words, &["whenever", "one", "or", "more"]))
        && has_this_way
    {
        let Some(comma_idx) = find_index(tokens, |token: &OwnedLexToken| token.is_comma()) else {
            return tokens.to_vec();
        };
        let mut rewritten = Vec::new();

        let mut if_token = tokens[0].clone();
        if_token.replace_word("if");
        rewritten.push(if_token);

        let mut you_token = tokens.get(1).cloned().unwrap_or_else(|| tokens[0].clone());
        you_token.replace_word("you");
        rewritten.push(you_token);

        let mut do_token = tokens.get(2).cloned().unwrap_or_else(|| tokens[0].clone());
        do_token.replace_word("do");
        rewritten.push(do_token);

        rewritten.push(tokens[comma_idx].clone());
        rewritten.extend_from_slice(&tokens[comma_idx + 1..]);
        return rewritten;
    }

    tokens.to_vec()
}

pub(crate) fn strip_otherwise_sentence_prefix(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("otherwise"))
    {
        return None;
    }

    let mut idx = 1usize;
    while tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("then")) {
        idx += 1;
    }
    while tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }

    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        None
    } else {
        Some(remainder)
    }
}

pub(crate) fn rewrite_otherwise_referential_subject(
    tokens: Vec<OwnedLexToken>,
) -> Vec<OwnedLexToken> {
    let clause_words = crate::cards::builders::parser::token_word_refs(&tokens);
    let is_referential_get = clause_words.len() >= 3
        && clause_words[0] == "that"
        && matches!(clause_words[1], "creature" | "permanent")
        && matches!(clause_words[2], "gets" | "get" | "gains" | "gain");
    if !is_referential_get {
        return tokens;
    }

    let mut rewritten = tokens;
    if let Some(first) = rewritten.get_mut(0) {
        first.replace_word("target");
    }
    rewritten
}

pub(crate) fn is_nonsemantic_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    is_activate_only_restriction_sentence(tokens) || is_trigger_only_restriction_sentence(tokens)
}

fn token_copy_followup_container_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::WhenResult { effects, .. }
        | EffectAst::ResolvedIfResult { effects, .. }
        | EffectAst::ResolvedWhenResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::RepeatProcess { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilNextDrawStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => Some(effects),
        _ => None,
    }
}

pub(crate) fn parse_token_copy_followup_sentence(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if matches!(
        filtered.as_slice(),
        [
            "sacrifice",
            "that",
            "token",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ] | [
            "sacrifice",
            "those",
            "tokens",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ]
    ) {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }

    parse_token_copy_modifier_sentence(tokens)
        .or_else(|| {
            is_exile_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            is_sacrifice_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

pub(crate) fn parse_token_copy_followup_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if matches!(
        filtered.as_slice(),
        [
            "sacrifice",
            "that",
            "token",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ] | [
            "sacrifice",
            "those",
            "tokens",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ]
    ) {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }

    super::parse_token_copy_modifier_sentence_lexed(tokens)
        .or_else(|| {
            super::is_exile_that_token_at_end_of_combat_lexed(tokens)
                .then_some(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            super::is_sacrifice_that_token_at_end_of_combat_lexed(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

fn apply_unapplied_token_copy_followup(
    sentence: &[OwnedLexToken],
    _sentence_tokens: &[OwnedLexToken],
    followup: TokenCopyFollowup,
) -> Result<Vec<EffectAst>, CardTextError> {
    let span = span_from_tokens(sentence);
    let effects = match followup {
        TokenCopyFollowup::HasHaste => vec![EffectAst::GrantAbilitiesToTarget {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
            abilities: vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
            duration: Until::Forever,
        }],
        TokenCopyFollowup::GainHasteUntilEndOfTurn => vec![EffectAst::GrantAbilitiesToTarget {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
            abilities: vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
            duration: Until::EndOfTurn,
        }],
        TokenCopyFollowup::EnterTappedAndAttacking => {
            return Err(CardTextError::ParseError(
                "standalone 'enters tapped and attacking' follow-up requires a preceding token-copy, populate, or meld effect".to_string(),
            ));
        }
        TokenCopyFollowup::SacrificeAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::Sacrifice {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                player: PlayerAst::Implicit,
                count: 1,
            }],
        }],
        TokenCopyFollowup::ExileAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::Exile {
                target: TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                face_down: false,
            }],
        }],
        TokenCopyFollowup::ExileAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::Exile {
                target: TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                face_down: false,
            }],
        }],
        TokenCopyFollowup::SacrificeAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::Sacrifice {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                player: PlayerAst::Implicit,
                count: 1,
            }],
        }],
    };
    Ok(effects)
}

pub(crate) fn try_apply_token_copy_followup(
    effects: &mut [EffectAst],
    followup: TokenCopyFollowup,
) -> Result<bool, CardTextError> {
    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    match last {
        EffectAst::CreateTokenCopy {
            has_haste,
            enters_tapped,
            enters_attacking,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            enters_tapped,
            enters_attacking,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::Populate {
            has_haste,
            enters_tapped,
            enters_attacking,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            match followup {
                TokenCopyFollowup::HasHaste => *has_haste = true,
                TokenCopyFollowup::EnterTappedAndAttacking => {
                    *enters_tapped = true;
                    *enters_attacking = true;
                }
                TokenCopyFollowup::SacrificeAtNextEndStep => *sacrifice_at_next_end_step = true,
                TokenCopyFollowup::ExileAtNextEndStep => *exile_at_next_end_step = true,
                TokenCopyFollowup::ExileAtEndOfCombat => *exile_at_end_of_combat = true,
                TokenCopyFollowup::GainHasteUntilEndOfTurn
                | TokenCopyFollowup::SacrificeAtEndOfCombat => return Ok(false),
            }
            Ok(true)
        }
        EffectAst::Meld {
            enters_tapped,
            enters_attacking,
            ..
        } => match followup {
            TokenCopyFollowup::EnterTappedAndAttacking => {
                *enters_tapped = true;
                *enters_attacking = true;
                Ok(true)
            }
            _ => Ok(false),
        },
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            if try_apply_token_copy_followup(if_true.as_mut_slice(), followup)? {
                return Ok(true);
            }
            if try_apply_token_copy_followup(if_false.as_mut_slice(), followup)? {
                return Ok(true);
            }
            Ok(false)
        }
        EffectAst::CreateTokenWithMods {
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            ..
        } => {
            match followup {
                TokenCopyFollowup::ExileAtEndOfCombat => *exile_at_end_of_combat = true,
                TokenCopyFollowup::SacrificeAtEndOfCombat => *sacrifice_at_end_of_combat = true,
                TokenCopyFollowup::HasHaste
                | TokenCopyFollowup::EnterTappedAndAttacking
                | TokenCopyFollowup::GainHasteUntilEndOfTurn
                | TokenCopyFollowup::SacrificeAtNextEndStep
                | TokenCopyFollowup::ExileAtNextEndStep => return Ok(false),
            }
            Ok(true)
        }
        _ => {
            let Some(nested_effects) = token_copy_followup_container_effects_mut(last) else {
                return Ok(false);
            };
            if nested_effects.is_empty() {
                return Ok(false);
            }
            try_apply_token_copy_followup(nested_effects.as_mut_slice(), followup)
        }
    }
}
