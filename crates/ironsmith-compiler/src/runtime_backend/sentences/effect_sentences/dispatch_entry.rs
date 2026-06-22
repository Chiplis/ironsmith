use winnow::Parser as _;
use winnow::combinator::{alt, cut_err, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::{any, take_till};

use self::subject_verb_followups::{
    PostParseFollowupResult, PreParseFollowupResult, is_still_lands_followup_sentence,
    previous_sentence_is_temporary_land_animation, run_post_parse_followup_registry,
    run_pre_parse_followup_registry,
};
use super::super::activation_and_restrictions::{
    parse_choose_card_type_phrase_words, parse_mana_usage_restriction_sentence_lexed,
    parse_target_player_choose_objects_clause, parse_you_choose_objects_clause,
};
use super::super::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut, try_for_each_nested_effects_mut,
};
use super::super::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed as parse_spell_filter_lexed;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::split_leading_result_prefix_lexed;
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{
    LexStream, LexedClause, OwnedLexToken, TokenKind, contains_token_word_sequence, lex_line,
    split_lexed_sentences, token_slice_at_is, word_slice_contains_any_phrase,
    word_slice_contains_phrase, word_slice_eq, word_slice_eq_any, word_slice_find_phrase_start,
    word_slice_first_is, word_slice_starts_with_any,
};
use super::super::object_filters::{
    is_comparison_or_delimiter, parse_object_filter, parse_object_filter_lexed,
};
use super::super::permission_helpers::{
    parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::token_primitives::{
    LeadingMayActor, TurnDurationPhrase, find_index, find_window_by,
    parse_leading_may_action_lexed, parse_turn_duration_prefix, parse_value_comparison_tokens,
    slice_contains, slice_ends_with, slice_starts_with, strip_leading_if_you_do_lexed,
    word_view_has_any_prefix, word_view_has_prefix,
};
use super::super::util::{
    helper_tag_for_tokens, is_article, mana_pips_from_token, parse_counter_type_words,
    parse_number, parse_subject, parse_target_phrase, span_from_tokens, token_index_for_word_index,
    trim_commas, words,
};
use super::super::value_helpers::parse_value_from_lexed;
use super::bundle_rules::{
    parse_exact_card_effect_bundle_lexed, parse_same_sentence_copy_and_may_cast_copy,
};
use super::consult_family;
use super::divvy::try_parse_divvy_sentence_sequence;
use super::looked_cards_family;
use super::sentence_helpers::*;
use super::sequence_rules::{subject_verb_sequence_route, try_parse_subject_verb_sequence_rule};
use super::zone_handlers::parse_exile_top_library_clause;
use super::{
    SubjectVerbPrimitiveClause, find_verb, parse_effect_sentence_lexed, parse_restriction_duration,
    parse_search_library_disjunction_filter, parse_token_copy_modifier_sentence,
    trim_edge_punctuation, try_build_unless,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, CarryContext, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate,
    InsteadSemantics, KeywordAction, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, ReturnControllerAst, SubjectAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, TokenCopyFollowup, Verb,
    ZoneReplacementDurationAst,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::filter::Comparison;
use crate::mana::ManaSymbol;
use crate::parse_trace;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;
use std::cell::OnceCell;

mod subject_verb_followups;

const THAT_OBJECT_POWER_DAMAGE_PHRASES: &[&[&str]] = &[
    &[
        "that", "creature", "deals", "damage", "equal", "to", "its", "power",
    ],
    &[
        "that",
        "permanent",
        "deals",
        "damage",
        "equal",
        "to",
        "its",
        "power",
    ],
];
const COUNTERED_THIS_WAY_PHRASE: &[&str] = &["countered", "this", "way"];
const INSTEAD_OF_PHRASE: &[&str] = &["instead", "of"];
const GRAVEYARD_PHRASE: &[&str] = &["graveyard"];
const EXILE_PHRASE: &[&str] = &["exile"];
const HAND_PHRASE: &[&str] = &["hand"];
const LIBRARY_PHRASE: &[&str] = &["library"];
const WOULD_DIE_THIS_TURN_PHRASE: &[&str] = &["would", "die", "this", "turn"];
const DEALT_DAMAGE_THIS_WAY_PHRASE: &[&str] = &["dealt", "damage", "this", "way"];
const DEALT_DAMAGE_BY_PHRASE: &[&str] = &["dealt", "damage", "by"];
const PERMANENT_DEALT_DAMAGE_PHRASE: &[&str] = &["permanent", "dealt", "damage"];
const WOULD_BE_PUT_INTO_PHRASE: &[&str] = &["would", "be", "put", "into"];
const THIS_TURN_PHRASE: &[&str] = &["this", "turn"];
const YOUR_GRAVEYARD_PHRASE: &[&str] = &["your", "graveyard"];
const EXILE_THAT_CARD_INSTEAD_PHRASE: &[&str] = &["exile", "that", "card", "instead"];
const THE_NEXT_TIME_PHRASE: &[&str] = &["the", "next", "time"];
const SOURCE_OF_YOUR_CHOICE_PHRASE: &[&str] = &["source", "of", "your", "choice"];
const WOULD_DEAL_DAMAGE_TO_YOU_THIS_TURN_PHRASE: &[&str] =
    &["would", "deal", "damage", "to", "you", "this", "turn"];
const PREVENT_THAT_DAMAGE_PHRASE: &[&str] = &["prevent", "that", "damage"];
const DAMAGE_IS_PREVENTED_THIS_WAY_PHRASE: &[&str] = &["damage", "is", "prevented", "this", "way"];
const DEALS_THAT_MUCH_DAMAGE_TO_THAT_SOURCE_PHRASE: &[&str] =
    &["deals", "that", "much", "damage", "to", "that", "source"];
const CONTROLLER_PHRASE: &[&str] = &["controller"];
const CAST_INSTANT_OR_SORCERY_FROM_HAND_PHRASES: &[&[&str]] = &[
    &["cast", "an", "instant", "or", "sorcery", "spell"],
    &["from", "your", "hand"],
];
const PUT_THAT_CARD_INTO_YOUR_HAND_PHRASE: &[&str] =
    &["put", "that", "card", "into", "your", "hand"];
const INSTEAD_OF_INTO_YOUR_GRAVEYARD_PHRASE: &[&str] =
    &["instead", "of", "into", "your", "graveyard"];
const WOULD_ENTER_BATTLEFIELD_UNDER_OPPONENT_PHRASE: &[&str] = &[
    "would",
    "enter",
    "the",
    "battlefield",
    "under",
    "an",
    "opponent",
];
const ENTERS_UNDER_YOUR_CONTROL_INSTEAD_PHRASE: &[&str] =
    &["enters", "under", "your", "control", "instead"];
const THIS_OBJECT_DAMAGE_TARGET_PHRASES: &[&[&str]] =
    &[&["to", "this", "creature"], &["to", "this", "permanent"]];
const TO_THAT_PLAYER_PHRASE: &[&str] = &["to", "that", "player"];
const WITH_WORD: &str = "with";
const LEARN_WORDS: &[&str] = &["learn"];
const TIME_TRAVEL_WORDS: &[&str] = &["time", "travel"];
const OUTSIDE_GAME_ART_RATING_PHRASES: &[&[&str]] = &[
    &["ask", "a", "person", "outside", "the", "game", "to", "rate"],
    &["when", "they", "rate", "the", "art"],
];
const DESTROYED_THIS_WAY_SUBJECT_PREFIXES: &[&[&str]] = &[
    &["creature", "destroyed", "this", "way"],
    &["creatures", "destroyed", "this", "way"],
    &["a", "creature", "destroyed", "this", "way"],
];

fn split_leading_unless_payment_search_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !words.first().is_some_and(|word| *word == "unless") {
        return None;
    }
    let pay_idx = words
        .iter()
        .position(|word| matches!(*word, "pay" | "pays"))?;
    let effect_word_idx = words
        .iter()
        .enumerate()
        .skip(pay_idx + 1)
        .find_map(|(idx, word)| (*word == "search").then_some(idx))?;
    let effect_token_idx = token_index_for_word_index(tokens, effect_word_idx)?;
    Some((
        trim_edge_punctuation(&tokens[..effect_token_idx]),
        trim_edge_punctuation(&tokens[effect_token_idx..]),
    ))
}

fn apply_leading_duration_to_become_effect(effect: &mut EffectAst, duration: &Until) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::BecomeBasePtCreature {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::SetBasePowerToughness {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::BecomeBasicLandType {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::BecomeBasicLandTypeChoice {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::BecomeColorChoice {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::BecomeCopy {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::BecomeAuraEnchantment {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::MakeColorless {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::AddColors {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::AddCardTypes {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::AddSubtypes {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::SetColors {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::GrantAbilitiesToTarget {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                duration: effect_duration,
                ..
            } => {
                *effect_duration = duration.clone();
                true
            }
            _ => false,
        },
        EffectAst::Sequence { effects } => {
            let mut applied = false;
            for effect in effects {
                applied |= apply_leading_duration_to_become_effect(effect, duration);
            }
            applied
        }
        _ => false,
    }
}

fn should_apply_leading_duration_become_shortcut(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words
        .first()
        .is_some_and(|word| matches!(*word, "at" | "when" | "whenever"))
    {
        return false;
    }
    if words.iter().any(|word| *word == "if") {
        return false;
    }
    if words.windows(2).any(|window| {
        matches!(
            window,
            ["and", "become" | "becomes"] | ["and", "attacks" | "blocks"]
        )
    }) {
        return false;
    }
    words
        .iter()
        .any(|word| matches!(*word, "become" | "becomes"))
}

const BE_REGENERATED_SUFFIX: &[&str] = &["be", "regenerated"];
const CANT_BE_REGENERATED_SPLIT_PHRASE: &[&str] = &["can", "t", "be", "regenerated"];
const DEAL_X_DAMAGE_PHRASES: &[&[&str]] = &[&["deal", "x", "damage"], &["deals", "x", "damage"]];
const X_LIFE_CHANGE_PHRASES: &[&[&str]] = &[
    &["gain", "x", "life"],
    &["gains", "x", "life"],
    &["lose", "x", "life"],
    &["loses", "x", "life"],
];
const OTHERWISE_WORD: &str = "otherwise";
const REFERENTIAL_THAT_WORD: &str = "that";
const REFERENTIAL_NOUN_WORDS: &[&str] = &["creature", "permanent"];
const GET_GAIN_WORDS: &[&str] = &["get", "gets", "gain", "gains"];
const NONSEMANTIC_X_CANT_BE_ZERO_PHRASES: &[&[&str]] =
    &[&["x", "cant", "be", "0"], &["x", "can't", "be", "0"]];
const COUNTER_OR_COUNTERS_WORDS: &[&str] = &["counter", "counters"];
const OR_OR_MORE_WORDS: &[&str] = &["or", "more"];
const CANT_WORDS: &[&str] = &["cant", "can't", "cannot"];
const ON_WORD: &str = "on";
const IT_OR_THEM_WORDS: &[&str] = &["it", "them"];
const NO_WORD: &str = "no";
const X_WORD: &str = "x";
const WHERE_X_IS_WORDS: &[&str] = &["where", "x", "is"];
const SIMPLE_CANT_BE_REGENERATED_PHRASES: &[&[&str]] = &[
    &["it", "cant", "be", "regenerated"],
    &["it", "cant", "be", "regenerated", "this", "turn"],
    &["they", "cant", "be", "regenerated"],
    &["they", "cant", "be", "regenerated", "this", "turn"],
    &[
        "creature",
        "destroyed",
        "this",
        "way",
        "cant",
        "be",
        "regenerated",
    ],
    &[
        "creature",
        "destroyed",
        "this",
        "way",
        "can't",
        "be",
        "regenerated",
    ],
    &[
        "creatures",
        "destroyed",
        "this",
        "way",
        "cant",
        "be",
        "regenerated",
    ],
    &[
        "creatures",
        "destroyed",
        "this",
        "way",
        "can't",
        "be",
        "regenerated",
    ],
    &[
        "a",
        "creature",
        "destroyed",
        "this",
        "way",
        "cant",
        "be",
        "regenerated",
    ],
    &[
        "a",
        "creature",
        "destroyed",
        "this",
        "way",
        "can't",
        "be",
        "regenerated",
    ],
];
const CANT_BE_REGENERATED_THIS_TURN_PHRASES: &[&[&str]] = &[
    &["it", "cant", "be", "regenerated", "this", "turn"],
    &["they", "cant", "be", "regenerated", "this", "turn"],
];

fn summarize_effects(effects: &[EffectAst]) -> String {
    effects
        .iter()
        .map(|effect| {
            let debug = format!("{effect:?}");
            debug
                .split(|ch: char| ch == ' ' || ch == '{' || ch == '(')
                .next()
                .unwrap_or("Effect")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn repair_that_object_power_damage_subject(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
    previous_damage_target: Option<TargetAst>,
) {
    let token_words = crate::runtime_backend::token_word_refs(tokens);
    let looks_like_that_object_power_damage =
        word_slice_contains_any_phrase(&token_words, THAT_OBJECT_POWER_DAMAGE_PHRASES)
            && word_slice_contains_any_phrase(&token_words, THIS_OBJECT_DAMAGE_TARGET_PHRASES);
    if !looks_like_that_object_power_damage {
        return;
    }
    let source_target = previous_damage_target
        .unwrap_or_else(|| TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)));
    for effect in effects {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            continue;
        };
        match &subject_verb.action {
            SubjectVerbActionAst::DealDamage { amount, target, .. }
                if matches!(amount, Value::PowerOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source))
                    && matches!(target, TargetAst::Source(_)) =>
            {
                subject_verb.action = SubjectVerbActionAst::DealDamageEqualToPower {
                    source: source_target.clone(),
                    amount: Value::PowerOf(Box::new(ChooseSpec::Source)),
                    target: target.clone(),
                };
            }
            SubjectVerbActionAst::DealDamageEqualToPower {
                source,
                amount,
                target,
            } if (matches!(source, TargetAst::Source(_))
                || matches!(source, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG))
                && matches!(target, TargetAst::Source(_)) =>
            {
                subject_verb.action = SubjectVerbActionAst::DealDamageEqualToPower {
                    source: source_target.clone(),
                    amount: amount.clone(),
                    target: target.clone(),
                };
            }
            _ => {}
        }
    }
}

fn repair_target_controlled_source_damage_to_that_player(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) {
    if !word_slice_contains_phrase(
        &crate::runtime_backend::token_word_refs(tokens),
        TO_THAT_PLAYER_PHRASE,
    ) {
        return;
    }

    for effect in effects {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            continue;
        };
        let SubjectVerbActionAst::DealDamageEqualToPower { source, target, .. } =
            &mut subject_verb.action
        else {
            continue;
        };
        let TargetAst::Object(source_filter, _, _) = source else {
            continue;
        };
        if !source_filter.controller.as_ref().is_some_and(|controller| {
            matches!(controller, PlayerFilter::Opponent | PlayerFilter::NotYou)
        }) {
            continue;
        }
        if matches!(
            target,
            TargetAst::Player(PlayerFilter::Target(inner), _)
                if matches!(inner.as_ref(), PlayerFilter::Any)
        ) {
            *target = TargetAst::Player(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
                span_from_tokens(tokens),
            );
        }
    }
}

fn apply_trailing_counter_constraint_to_destroy_all(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) {
    let token_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(counter_idx) = token_words
        .iter()
        .position(|word| COUNTER_OR_COUNTERS_WORDS.contains(word))
    else {
        return;
    };
    if token_words.get(counter_idx + 1) != Some(&ON_WORD)
        || !token_words
            .get(counter_idx + 2)
            .is_some_and(|word| IT_OR_THEM_WORDS.contains(word))
    {
        return;
    }
    let descriptor_start = token_words[..counter_idx]
        .iter()
        .rposition(|word| *word == WITH_WORD)
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let descriptor_words = token_words[descriptor_start..counter_idx]
        .iter()
        .copied()
        .filter(|word| {
            !OR_OR_MORE_WORDS.contains(word) && ironsmith_core::parse_cardinal_word(word).is_none()
        })
        .collect::<Vec<_>>();
    if word_slice_first_is(&descriptor_words, NO_WORD) {
        return;
    }
    let counter_constraint = if descriptor_words.is_empty() {
        crate::filter::CounterConstraint::Any
    } else {
        let descriptor_words = descriptor_words
            .iter()
            .copied()
            .chain(std::iter::once("counter"))
            .collect::<Vec<_>>();
        let Some(counter_type) = parse_counter_type_words(&descriptor_words) else {
            return;
        };
        crate::filter::CounterConstraint::Typed(counter_type)
    };
    if !token_words[..counter_idx]
        .iter()
        .any(|word| *word == WITH_WORD)
    {
        return;
    }

    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DestroyAll { filter, .. }
                    | SubjectVerbActionAst::ExileAll { filter, .. },
                ..
            }) => {
                if filter.with_counter.is_none() {
                    filter.with_counter = Some(counter_constraint);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn leading_may_actor_to_player(
    actor: LeadingMayActor,
    default_player: PlayerAst,
) -> PlayerAst {
    match actor {
        LeadingMayActor::You => PlayerAst::You,
        LeadingMayActor::ThatPlayer => PlayerAst::That,
        LeadingMayActor::Default => default_player,
    }
}

fn attach_copy_cost_reduction_to_effect(
    effect: &mut EffectAst,
    reduction: &crate::mana::ManaCost,
) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CastTagged {
                    as_copy,
                    cost_reduction,
                    ..
                },
            ..
        }) if *as_copy => {
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

const PRONOUN_TRIGGER_PREFIXES: &[&[&str]] = &[
    &["when", "it"],
    &["whenever", "it"],
    &["when", "they"],
    &["whenever", "they"],
];

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
pub(super) struct ConsultSentenceParts {
    pub(super) effects: Vec<EffectAst>,
    pub(super) player: PlayerAst,
    pub(super) all_tag: TagKey,
    pub(super) match_tag: TagKey,
}

pub(super) struct ConsultCastClause {
    pub(super) caster: PlayerAst,
    pub(super) allow_land: bool,
    pub(super) timing: ConsultCastTiming,
    pub(super) cost: ConsultCastCost,
    pub(super) mana_value_condition: Option<ConsultCastManaValueCondition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConsultCastTiming {
    Immediate,
    UntilEndOfTurn,
    UntilYourNextTurnEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConsultCastCost {
    Normal,
    WithoutPayingManaCost,
    PayLifeEqualToManaValue,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ConsultCastManaValueCondition {
    pub(super) operator: crate::effect::ValueComparisonOperator,
    pub(super) right: Value,
}

pub(super) fn parse_prefixed_top_of_your_library_count<T: Copy>(
    tokens: &[OwnedLexToken],
    prefixes: &[(&[&str], T)],
) -> Option<(T, u32)> {
    let tokens = trim_commas(tokens);
    let word_view = TokenWordView::new(&tokens);
    let (count_word_idx, marker) = prefixes.iter().find_map(|(prefix, marker)| {
        word_view_has_prefix(&word_view, prefix).then_some((prefix.len(), *marker))
    })?;
    let count_start = word_view.token_index_for_word_index(count_word_idx)?;
    let count_tokens = &tokens[count_start..];
    let (count, used) = parse_number(count_tokens)?;
    let tail_word_view = TokenWordView::new(&count_tokens[used..]);
    let tail_words = tail_word_view.word_refs();
    matches!(
        tail_words.as_slice(),
        ["card", "of", "your", "library"] | ["cards", "of", "your", "library"]
    )
    .then_some((marker, count))
}

pub(super) fn find_from_among_looked_cards_phrase(
    word_view: &TokenWordView<'_>,
) -> Option<(usize, usize)> {
    word_view
        .find_phrase_start(&["from", "among", "those", "cards"])
        .map(|idx| (idx, 4usize))
        .or_else(|| {
            word_view
                .find_phrase_start(&["from", "among", "the", "cards", "milled", "this", "way"])
                .map(|idx| (idx, 7usize))
        })
        .or_else(|| {
            word_view
                .find_phrase_start(&["from", "among", "the", "milled", "cards"])
                .map(|idx| (idx, 5usize))
        })
        .or_else(|| {
            word_view
                .find_phrase_start(&["from", "among", "them"])
                .map(|idx| (idx, 3usize))
        })
}

pub(super) fn parse_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<ConsultSentenceParts>, CardTextError> {
    consult_family::parse_consult_traversal_sentence(tokens)
}

pub(super) fn parse_consult_remainder_order(words: &[&str]) -> Option<LibraryBottomOrderAst> {
    consult_family::parse_consult_remainder_order(words)
}

pub(super) fn consult_stop_rule_is_single_match(stop_rule: &LibraryConsultStopRuleAst) -> bool {
    matches!(
        stop_rule,
        LibraryConsultStopRuleAst::FirstMatch
            | LibraryConsultStopRuleAst::MatchCount(Value::Fixed(1))
    )
}

#[cfg(test)]
fn parse_consult_condition_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    consult_family::parse_consult_condition_value(tokens)
}

pub(super) fn parse_bargained_face_down_cast_mana_value_gate(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::ValueComparisonOperator, Value)>, CardTextError> {
    consult_family::parse_bargained_face_down_cast_mana_value_gate(tokens)
}

#[cfg(test)]
fn parse_consult_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConsultCastManaValueCondition> {
    consult_family::parse_consult_mana_value_condition_tokens(tokens)
}

pub(super) fn parse_consult_cast_clause(tokens: &[OwnedLexToken]) -> Option<ConsultCastClause> {
    consult_family::parse_consult_cast_clause(tokens)
}

pub(super) fn parse_consult_bottom_remainder_clause(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<LibraryBottomOrderAst> {
    consult_family::parse_consult_bottom_remainder_clause(tokens, mode)
}

pub(super) fn parse_if_declined_put_match_into_hand(
    tokens: &[OwnedLexToken],
    match_tag: TagKey,
) -> Option<Vec<EffectAst>> {
    consult_family::parse_if_declined_put_match_into_hand(tokens, match_tag)
}

pub(super) fn consult_cast_effects(
    clause: &ConsultCastClause,
    match_tag: TagKey,
) -> Result<Vec<EffectAst>, CardTextError> {
    consult_family::consult_cast_effects(clause, match_tag)
}

pub(crate) struct SentenceInput {
    lowered: OnceCell<Vec<OwnedLexToken>>,
    lexed: Vec<OwnedLexToken>,
}

impl SentenceInput {
    pub(crate) fn from_lexed(tokens: &[OwnedLexToken]) -> Self {
        Self {
            lowered: OnceCell::new(),
            lexed: tokens.to_vec(),
        }
    }

    pub(crate) fn lowered(&self) -> &[OwnedLexToken] {
        self.lowered
            .get_or_init(|| normalize_parser_tokens(&self.lexed))
            .as_slice()
    }

    pub(crate) fn lexed(&self) -> &[OwnedLexToken] {
        self.lexed.as_slice()
    }
}

struct SentenceDispatchState<'a> {
    effects: &'a mut Vec<EffectAst>,
    carried_context: &'a mut Option<CarryContext>,
}

struct SentenceParsePlan {
    tokens: Vec<OwnedLexToken>,
    wrap_if_result: Option<IfResultPredicate>,
    direct_effects: Option<Vec<EffectAst>>,
    consumed_sentences: usize,
}

impl SentenceParsePlan {
    fn new(tokens: Vec<OwnedLexToken>) -> Self {
        Self {
            tokens,
            wrap_if_result: None,
            direct_effects: None,
            consumed_sentences: 1,
        }
    }
}

fn sentence_contains(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    contains_token_word_sequence(tokens, phrase)
}

fn reflected_prevent_next_damage_from_tokens(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    if sentence_contains(tokens, THE_NEXT_TIME_PHRASE)
        && sentence_contains(tokens, SOURCE_OF_YOUR_CHOICE_PHRASE)
        && sentence_contains(tokens, WOULD_DEAL_DAMAGE_TO_YOU_THIS_TURN_PHRASE)
        && sentence_contains(tokens, PREVENT_THAT_DAMAGE_PHRASE)
        && sentence_contains(tokens, DAMAGE_IS_PREVENTED_THIS_WAY_PHRASE)
        && sentence_contains(tokens, DEALS_THAT_MUCH_DAMAGE_TO_THAT_SOURCE_PHRASE)
        && sentence_contains(tokens, CONTROLLER_PHRASE)
    {
        return Some(
            EffectAst::subject_verb_prevent_next_time_damage_with_reflection(
                PreventNextTimeDamageSourceAst::Choice,
                PreventNextTimeDamageTargetAst::You,
                true,
            ),
        );
    }
    None
}

fn future_zone_replacement_from_sentence_text(sentence_text: &str) -> Option<EffectAst> {
    let tokens = lex_line(sentence_text, 0).ok()?;
    future_zone_replacement_from_sentence_tokens(&tokens)
}

fn future_zone_replacement_from_sentence_tokens(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let target = || TargetAst::Tagged(TagKey::from(IT_TAG), None);
    if sentence_contains(tokens, COUNTERED_THIS_WAY_PHRASE)
        && sentence_contains(tokens, INSTEAD_OF_PHRASE)
        && sentence_contains(tokens, GRAVEYARD_PHRASE)
        && sentence_contains(tokens, EXILE_PHRASE)
    {
        return Some(EffectAst::subject_verb_register_zone_replacement(
            target(),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
        ));
    }

    if sentence_contains(tokens, COUNTERED_THIS_WAY_PHRASE)
        && sentence_contains(tokens, INSTEAD_OF_PHRASE)
        && sentence_contains(tokens, GRAVEYARD_PHRASE)
        && sentence_contains(tokens, HAND_PHRASE)
    {
        return Some(EffectAst::subject_verb_register_zone_replacement(
            target(),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Hand,
            ZoneReplacementDurationAst::OneShot,
        ));
    }

    if sentence_contains(tokens, COUNTERED_THIS_WAY_PHRASE)
        && sentence_contains(tokens, INSTEAD_OF_PHRASE)
        && sentence_contains(tokens, GRAVEYARD_PHRASE)
        && sentence_contains(tokens, LIBRARY_PHRASE)
    {
        return Some(EffectAst::subject_verb_register_zone_replacement(
            target(),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Library,
            ZoneReplacementDurationAst::OneShot,
        ));
    }

    if sentence_contains(tokens, WOULD_DIE_THIS_TURN_PHRASE)
        && sentence_contains(tokens, EXILE_PHRASE)
    {
        if sentence_contains(tokens, DEALT_DAMAGE_THIS_WAY_PHRASE)
            || sentence_contains(tokens, DEALT_DAMAGE_BY_PHRASE)
        {
            let filter = if sentence_contains(tokens, PERMANENT_DEALT_DAMAGE_PHRASE) {
                ObjectFilter::permanent()
            } else {
                ObjectFilter::creature()
            };
            return Some(
                EffectAst::subject_verb_register_damaged_by_source_zone_replacement(
                    filter,
                    Some(Zone::Battlefield),
                    Some(Zone::Graveyard),
                    Zone::Exile,
                    ZoneReplacementDurationAst::OneShot,
                ),
            );
        }

        return Some(EffectAst::subject_verb_register_zone_replacement(
            target(),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
        ));
    }

    if sentence_contains(tokens, WOULD_BE_PUT_INTO_PHRASE)
        && sentence_contains(tokens, GRAVEYARD_PHRASE)
        && sentence_contains(tokens, THIS_TURN_PHRASE)
        && sentence_contains(tokens, EXILE_PHRASE)
    {
        if sentence_contains(tokens, YOUR_GRAVEYARD_PHRASE)
            && sentence_contains(tokens, EXILE_THAT_CARD_INSTEAD_PHRASE)
        {
            crate::parse_trace::event(
                "effect-route: subject-verb verb=Exile subject=implicit recognizer=instead-replacement",
            );
            return Some(
                EffectAst::subject_verb_exile_instead_of_graveyard_this_turn(PlayerAst::You),
            );
        }
        return Some(EffectAst::subject_verb_register_zone_replacement(
            target(),
            None,
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
        ));
    }

    if let Some(effect) = reflected_prevent_next_damage_from_tokens(tokens) {
        return Some(effect);
    }

    if sentence_contains(tokens, THE_NEXT_TIME_PHRASE)
        && CAST_INSTANT_OR_SORCERY_FROM_HAND_PHRASES
            .iter()
            .all(|phrase| sentence_contains(tokens, phrase))
        && sentence_contains(tokens, THIS_TURN_PHRASE)
        && sentence_contains(tokens, PUT_THAT_CARD_INTO_YOUR_HAND_PHRASE)
        && sentence_contains(tokens, INSTEAD_OF_INTO_YOUR_GRAVEYARD_PHRASE)
    {
        return Some(EffectAst::subject_verb_register_future_zone_replacement(
            ObjectFilter::instant_or_sorcery().cast_by_you(),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Hand,
            ZoneReplacementDurationAst::OneShot,
        ));
    }

    if sentence_contains(tokens, WOULD_ENTER_BATTLEFIELD_UNDER_OPPONENT_PHRASE)
        && sentence_contains(tokens, THIS_TURN_PHRASE)
        && sentence_contains(tokens, ENTERS_UNDER_YOUR_CONTROL_INSTEAD_PHRASE)
    {
        let mut filter = ObjectFilter::creature();
        filter.controller = Some(PlayerFilter::Opponent);
        return Some(
            EffectAst::subject_verb_register_enter_under_control_replacement(
                filter,
                ZoneReplacementDurationAst::OneShot,
            ),
        );
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
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn
                    | SubjectVerbActionAst::PreventNextTimeDamage { .. }
                    | SubjectVerbActionAst::RedirectNextTimeDamageToSource { .. }
                    | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. },
                ..
            })
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

fn try_merge_otherwise_into_previous_conditional(
    effects: &mut [EffectAst],
    sentence_effects: &[EffectAst],
) -> bool {
    let [
        EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: otherwise_effects,
        },
    ] = sentence_effects
    else {
        return false;
    };
    let Some(EffectAst::Conditional {
        if_true, if_false, ..
    }) = effects.last_mut()
    else {
        return false;
    };
    if !if_false.is_empty() {
        return false;
    }
    *if_false = otherwise_effects.clone();
    if matches!(
        if_true.as_slice(),
        [EffectAst::May { .. } | EffectAst::MayByPlayer { .. }]
    ) {
        if_true.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: otherwise_effects.clone(),
        });
    }
    true
}

fn try_append_to_previous_numeric_result_branch(
    effects: &mut [EffectAst],
    sentence_effects: &[EffectAst],
    sentence_tokens: &[OwnedLexToken],
    result_branch_line: Option<usize>,
) -> bool {
    if sentence_effects.is_empty()
        || split_leading_result_prefix_lexed(sentence_tokens).is_some()
        || result_branch_line != sentence_tokens.first().map(|token| token.span.line)
    {
        return false;
    }
    let Some(EffectAst::IfResult {
        predicate: IfResultPredicate::Value(_),
        effects: branch_effects,
    }) = effects.last_mut()
    else {
        return false;
    };
    branch_effects.extend(sentence_effects.iter().cloned());
    true
}

fn numeric_result_branch_line(
    sentence_effects: &[EffectAst],
    sentence_tokens: &[OwnedLexToken],
) -> Option<usize> {
    if split_leading_result_prefix_lexed(sentence_tokens).is_none() {
        return None;
    }
    match sentence_effects {
        [
            EffectAst::IfResult {
                predicate: IfResultPredicate::Value(_),
                ..
            },
        ] => sentence_tokens.first().map(|token| token.span.line),
        _ => None,
    }
}

fn maybe_append_trailing_that_much_life_loss(
    sentence_effects: &mut Vec<EffectAst>,
    sentence_tokens: &[OwnedLexToken],
) {
    if !grammar::contains_phrase(sentence_tokens, &["then", "lose", "that", "much", "life"]) {
        return;
    }

    let life_loss = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::LoseLife {
            amount: Value::EventValue(EventValueSpec::Amount),
        },
    );
    if let [EffectAst::IfResult { effects, .. }] = sentence_effects.as_mut_slice() {
        if !effects.iter().any(effect_is_life_loss) {
            effects.push(life_loss);
        }
        return;
    }
    if !sentence_effects.iter().any(effect_is_life_loss) {
        sentence_effects.push(life_loss);
    }
}

fn effect_is_life_loss(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LoseLife { .. },
            ..
        })
    )
}

fn maybe_repair_that_player_gain_control_if_do_rewards(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    if !grammar::contains_phrase(tokens, &["that", "player", "gains", "control", "of"])
        || !grammar::contains_phrase(tokens, &["if", "they", "do"])
        || effects.is_empty()
        || effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::GainControl { .. },
                    ..
                })
            )
        })
    {
        return;
    }

    let rewards = std::mem::take(effects);
    effects.push(EffectAst::subject_verb_gain_control(
        PlayerAst::That,
        TargetAst::Source(None),
        Until::Forever,
    ));
    effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: rewards,
    });
}

pub(super) fn parse_top_cards_view_sentence(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerAst, Value, bool)> {
    looked_cards_family::parse_top_cards_view_sentence(tokens)
}

pub(super) fn parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::sequence_rules::generic_subject_verb_sequences::pairs::parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
        &[
            SentenceInput::from_lexed(first),
            SentenceInput::from_lexed(second),
        ],
        0,
    )
}

#[cfg(test)]
pub(super) fn parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::sequence_rules::generic_subject_verb_sequences::triples::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
        &[
            SentenceInput::from_lexed(first),
            SentenceInput::from_lexed(second),
            SentenceInput::from_lexed(third),
        ],
        0,
    )
}

#[cfg(test)]
pub(super) fn parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::sequence_rules::generic_subject_verb_sequences::triples::parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(
        &[
            SentenceInput::from_lexed(first),
            SentenceInput::from_lexed(second),
            SentenceInput::from_lexed(third),
        ],
        0,
    )
}

pub(super) fn parse_looked_card_choice_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    looked_cards_family::parse_looked_card_choice_filter(tokens)
}

pub(super) fn parse_counted_looked_cards_into_your_hand_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    looked_cards_family::parse_counted_looked_cards_into_your_hand_tokens(tokens)
}

pub(super) fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    looked_cards_family::parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(tokens)
}

pub(super) fn parse_may_put_filtered_looked_card_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool)>, CardTextError> {
    looked_cards_family::parse_may_put_filtered_looked_card_onto_battlefield(tokens)
}

pub(super) fn parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool, ObjectFilter)>, CardTextError> {
    looked_cards_family::parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
        tokens,
    )
}

pub(super) fn parse_if_you_dont_put_card_from_among_them_into_your_hand(
    tokens: &[OwnedLexToken],
) -> bool {
    looked_cards_family::parse_if_you_dont_put_card_from_among_them_into_your_hand(tokens)
}

pub(super) fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    looked_cards_family::is_put_rest_on_bottom_of_library_sentence(tokens)
}

pub(super) fn parse_looked_card_reveal_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    looked_cards_family::parse_looked_card_reveal_filter(tokens)
}

pub(super) fn parse_if_you_dont_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    consult_family::parse_if_you_dont_sentence(tokens)
}

pub(super) fn parse_if_you_cant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    consult_family::parse_if_you_cant_sentence(tokens)
}

fn parse_effect_sentences_from_sentence_inputs(
    sentences: Vec<SentenceInput>,
) -> Result<Vec<EffectAst>, CardTextError> {
    fn where_x_value_from_tokens(tokens: &[OwnedLexToken]) -> Option<Value> {
        let word_view = TokenWordView::new(tokens);
        let words = word_view.word_refs();
        let where_idx = word_slice_find_phrase_start(&words, WHERE_X_IS_WORDS)?;
        let where_token_idx = token_index_for_word_index(tokens, where_idx)?;
        parse_value_binding_clause(&tokens[where_token_idx..])
            .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
    }

    if let Some(effects) = try_parse_divvy_sentence_sequence(&sentences)? {
        return Ok(effects);
    }

    let mut effects = Vec::new();
    let mut sentence_idx = 0usize;
    let mut carried_context: Option<CarryContext> = None;
    let mut carried_where_x: Option<Value> = None;
    let mut last_numeric_result_branch_line: Option<usize> = None;

    while sentence_idx < sentences.len() {
        let sentence = sentences[sentence_idx].lowered();
        if sentence.is_empty() {
            sentence_idx += 1;
            continue;
        }
        if is_outside_game_art_rating_sentence(sentence) {
            sentence_idx += 1;
            continue;
        }
        let sentence_text = crate::runtime_backend::token_word_refs(sentence).join(" ");
        let _sentence_scope = parse_trace::scope(format!("effect sentence: \"{}\"", sentence_text));

        let leading_unless_tokens = trim_edge_punctuation(sentence);
        if leading_unless_tokens
            .first()
            .is_some_and(|token| token.is_word("unless"))
        {
            let clause = SubjectVerbPrimitiveClause::new(&leading_unless_tokens);
            let split_tokens = clause
                .split_once_on_comma()
                .map(|(unless_clause, effect_clause)| {
                    (
                        unless_clause.tokens().to_vec(),
                        effect_clause.tokens().to_vec(),
                    )
                })
                .or_else(|| split_leading_unless_payment_search_tokens(&leading_unless_tokens));
            if let Some((unless_tokens, effect_tokens)) = split_tokens {
                let unless_clause = SubjectVerbPrimitiveClause::new(&unless_tokens);
                let inner_effects = parse_effect_sentences_lexed(&effect_tokens)?;
                if !inner_effects.is_empty()
                    && let Some(unless_effect) = try_build_unless(inner_effects, unless_clause, 0)?
                {
                    effects.push(unless_effect);
                    carried_context = None;
                    sentence_idx += 1;
                    continue;
                }
            }
        }

        let leading_duration_tokens = trim_edge_punctuation(sentence);
        if let Some((duration, remainder)) = parse_restriction_duration(&leading_duration_tokens)?
            && should_apply_leading_duration_become_shortcut(&remainder)
        {
            let mut inner_effects = parse_effect_sentences_lexed(&remainder)?;
            let mut applied = false;
            for effect in &mut inner_effects {
                applied |= apply_leading_duration_to_become_effect(effect, &duration);
            }
            if applied {
                effects.append(&mut inner_effects);
                carried_context = None;
                sentence_idx += 1;
                continue;
            }
        }

        if let Some(mut matched) = try_parse_subject_verb_sequence_rule(&sentences, sentence_idx)? {
            let sequence_where_x = (0..matched.consumed_sentences).find_map(|offset| {
                sentences
                    .get(sentence_idx + offset)
                    .and_then(|sentence| where_x_value_from_tokens(sentence.lowered()))
            });
            if let Some(where_value) = sequence_where_x.as_ref() {
                let mut sequence_words = Vec::new();
                for offset in 0..matched.consumed_sentences {
                    if let Some(sentence) = sentences.get(sentence_idx + offset) {
                        sequence_words
                            .extend(crate::runtime_backend::token_word_refs(sentence.lowered()));
                    }
                }
                replace_unbound_x_in_effects_anywhere(
                    &mut matched.effects,
                    where_value,
                    &sequence_words.join(" "),
                )?;
            }
            let stage = if let Some(feature_tag) = matched.feature_tag {
                format!(
                    "parse_effect_sentences:subject-verb-sequence:{}:{feature_tag}",
                    matched.name
                )
            } else {
                format!(
                    "parse_effect_sentences:subject-verb-sequence:{}",
                    matched.name
                )
            };
            parser_trace(stage.as_str(), sentence);
            parse_trace::event(format!(
                "sequence subject/verb rule: {} -> {}",
                matched.name,
                summarize_effects(&matched.effects)
            ));
            parse_trace::event(format!(
                "effect-route: {}",
                subject_verb_sequence_route(matched.name)
            ));
            if let Some(where_value) = sequence_where_x {
                carried_where_x = Some(where_value);
            }
            effects.append(&mut matched.effects);
            sentence_idx += matched.consumed_sentences;
            continue;
        }

        let mut sentence_tokens = strip_embedded_token_rules_text(sentence);
        sentence_tokens = trim_edge_punctuation(&sentence_tokens);
        if sentence_tokens.is_empty()
            || crate::runtime_backend::token_word_refs(&sentence_tokens).is_empty()
        {
            sentence_idx += 1;
            continue;
        }
        sentence_tokens = rewrite_when_one_or_more_this_way_clause_prefix(&sentence_tokens);

        if word_slice_eq(
            &crate::runtime_backend::token_word_refs(&sentence_tokens),
            LEARN_WORDS,
        ) {
            effects.push(EffectAst::subject_verb_learn(PlayerAst::You));
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        if word_slice_eq(
            &crate::runtime_backend::token_word_refs(&sentence_tokens),
            TIME_TRAVEL_WORDS,
        ) {
            effects.push(time_travel_effect_ast());
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        if sentence_tokens
            .first()
            .is_some_and(|token| token.is_word("unless"))
        {
            let clause = SubjectVerbPrimitiveClause::new(&sentence_tokens);
            if let Some((unless_clause, effect_clause)) = clause.split_once_on_comma() {
                let inner_effects = parse_effect_sentences_lexed(effect_clause.tokens())?;
                if !inner_effects.is_empty()
                    && let Some(unless_effect) = try_build_unless(inner_effects, unless_clause, 0)?
                {
                    effects.push(unless_effect);
                    carried_context = None;
                    sentence_idx += 1;
                    continue;
                }
            }
        }

        if let Some(restriction) = parse_mana_usage_restriction_sentence_lexed(&sentence_tokens) {
            apply_mana_usage_restriction_to_previous_effect(
                &mut effects,
                restriction,
                &sentence_tokens,
            )?;
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        if is_still_lands_followup_sentence(&sentence_tokens)
            && previous_sentence_is_temporary_land_animation(&sentences, sentence_idx)
        {
            sentence_idx += 1;
            continue;
        }

        if let Some(replacement) = future_zone_replacement_from_sentence_tokens(&sentence_tokens) {
            effects.push(replacement);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        let mut parse_plan = {
            let mut state = SentenceDispatchState {
                effects: &mut effects,
                carried_context: &mut carried_context,
            };
            match run_pre_parse_followup_registry(
                &mut state,
                &sentences,
                sentence_idx,
                &sentence_tokens,
            )? {
                Some(PreParseFollowupResult::Handled {
                    consumed_sentences,
                    route,
                }) => {
                    parse_trace::event(format!(
                        "pre-parse followup handled sentence(s): {consumed_sentences}"
                    ));
                    parse_trace::event(format!(
                        "effect-route: {}",
                        route.unwrap_or(
                            "subject-verb verb=Do subject=implicit recognizer=pre-parse-followup",
                        )
                    ));
                    sentence_idx += consumed_sentences;
                    continue;
                }
                Some(PreParseFollowupResult::Plan(plan)) => plan,
                None => SentenceParsePlan::new(sentence_tokens.clone()),
            }
        };
        parser_trace("parse_effect_sentences:sentence", &parse_plan.tokens);
        let sentence_where_x = where_x_value_from_tokens(&parse_plan.tokens);

        let mut sentence_effects = if let Some(direct_effects) = parse_plan.direct_effects.take() {
            parse_trace::event(format!(
                "pre-parse plan supplied effects: {}",
                summarize_effects(&direct_effects)
            ));
            direct_effects
        } else if parse_plan.tokens.as_slice() == sentences[sentence_idx].lexed() {
            parse_effect_sentence_lexed(sentences[sentence_idx].lexed())?
        } else {
            parse_effect_sentence_lexed(&parse_plan.tokens)?
        };
        if let Some(predicate) = parse_plan.wrap_if_result {
            sentence_effects = vec![EffectAst::IfResult {
                predicate,
                effects: sentence_effects,
            }];
            carried_context = None;
        }
        if sentence_where_x.is_none()
            && let Some(where_value) = carried_where_x.as_ref()
        {
            replace_unbound_x_in_effects_anywhere(
                &mut sentence_effects,
                where_value,
                &crate::runtime_backend::token_word_refs(&parse_plan.tokens).join(" "),
            )?;
        }
        maybe_append_trailing_that_much_life_loss(&mut sentence_effects, &parse_plan.tokens);
        let previous_damage_target = effects.last().and_then(primary_damage_target_from_effect);
        repair_that_object_power_damage_subject(
            &mut sentence_effects,
            &sentence_tokens,
            previous_damage_target,
        );
        repair_target_controlled_source_damage_to_that_player(
            &mut sentence_effects,
            &sentence_tokens,
        );
        if crate::runtime_backend::token_word_refs(&parse_plan.tokens)
            .first()
            .copied()
            == Some("you")
        {
            carried_context = None;
        }
        if sentence_effects.is_empty()
            && !is_round_up_each_time_sentence(&parse_plan.tokens)
            && !is_nonsemantic_restriction_sentence(&parse_plan.tokens)
        {
            return Err(CardTextError::ParseError(format!(
                "sentence parsed to no semantic effects (clause: '{}')",
                crate::runtime_backend::token_word_refs(&parse_plan.tokens).join(" ")
            )));
        }
        for effect in &mut sentence_effects {
            if let Some(context) = carried_context {
                maybe_apply_carried_player_with_clause(effect, context, &parse_plan.tokens);
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
        {
            let mut state = SentenceDispatchState {
                effects: &mut effects,
                carried_context: &mut carried_context,
            };
            if let Some(PostParseFollowupResult::Handled { consumed_sentences }) =
                run_post_parse_followup_registry(
                    &mut state,
                    &sentences,
                    sentence_idx,
                    &parse_plan.tokens,
                    &mut sentence_effects,
                )?
            {
                parse_trace::event(format!(
                    "post-parse followup handled sentence(s): {consumed_sentences}"
                ));
                sentence_idx += consumed_sentences;
                continue;
            }
        }

        if try_merge_otherwise_into_previous_conditional(&mut effects, &sentence_effects) {
            sentence_idx += parse_plan.consumed_sentences;
            continue;
        }

        if try_append_to_previous_numeric_result_branch(
            &mut effects,
            &sentence_effects,
            &sentence_tokens,
            last_numeric_result_branch_line,
        ) {
            sentence_idx += parse_plan.consumed_sentences;
            continue;
        }

        parse_trace::event(format!("effects: {}", summarize_effects(&sentence_effects)));
        last_numeric_result_branch_line =
            numeric_result_branch_line(&sentence_effects, &sentence_tokens);
        if let Some(where_value) = sentence_where_x {
            carried_where_x = Some(where_value);
        }
        effects.extend(sentence_effects);
        sentence_idx += parse_plan.consumed_sentences;
    }

    if let Some(last_sentence) = sentences.last() {
        parser_trace("parse_effect_sentences:done", last_sentence.lowered());
    }
    Ok(effects)
}

fn is_outside_game_art_rating_sentence(tokens: &[OwnedLexToken]) -> bool {
    word_slice_contains_any_phrase(
        &crate::runtime_backend::token_word_refs(tokens),
        OUTSIDE_GAME_ART_RATING_PHRASES,
    )
}

pub(crate) fn parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    stacker::maybe_grow(8 * 1024 * 1024, 16 * 1024 * 1024, || {
        parse_effect_sentences_lexed_inner(tokens)
    })
}

fn apply_mana_usage_restriction_to_previous_effect(
    effects: &mut Vec<EffectAst>,
    restriction: crate::ability::ManaUsageRestriction,
    tokens: &[OwnedLexToken],
) -> Result<(), CardTextError> {
    let Some(previous) = effects.pop() else {
        return Err(CardTextError::ParseError(format!(
            "mana restriction has no preceding mana effect (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    if !effect_ast_can_produce_mana(&previous) {
        effects.push(previous);
        return Err(CardTextError::ParseError(format!(
            "mana restriction does not follow a mana-producing effect (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let wrapped = match previous {
        EffectAst::ManaRestricted {
            effects,
            mut restrictions,
        } => {
            restrictions.push(restriction);
            EffectAst::ManaRestricted {
                effects,
                restrictions,
            }
        }
        previous => EffectAst::ManaRestricted {
            effects: vec![previous],
            restrictions: vec![restriction],
        },
    };
    effects.push(wrapped);
    Ok(())
}

fn effect_ast_can_produce_mana(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => matches!(
            &subject_verb.action,
            SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaColorsAmong { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::AddManaImprintedColors
        ),
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            (!if_true.is_empty() && if_true.iter().all(effect_ast_can_produce_mana))
                || (!if_false.is_empty() && if_false.iter().all(effect_ast_can_produce_mana))
        }
        EffectAst::ManaRestricted { effects, .. } => {
            !effects.is_empty() && effects.iter().all(effect_ast_can_produce_mana)
        }
        _ => false,
    }
}

fn parse_effect_sentences_lexed_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effect) = reflected_prevent_next_damage_from_tokens(tokens) {
        return Ok(vec![effect]);
    }

    if let Some(mut effects) = parse_exact_card_effect_bundle_lexed(tokens) {
        apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
        maybe_repair_that_player_gain_control_if_do_rewards(&mut effects, tokens);
        parse_trace::event(format!(
            "exact effect bundle -> {}",
            summarize_effects(&effects)
        ));
        return Ok(effects);
    }

    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    let mut effects = parse_effect_sentences_from_sentence_inputs(sentences)?;
    group_this_way_copy_cast_followups(tokens, &mut effects);
    apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
    maybe_repair_that_player_gain_control_if_do_rewards(&mut effects, tokens);
    Ok(effects)
}

fn is_copy_reference_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CreateTokenCopy { .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { .. }
                | SubjectVerbActionAst::CopySpell { .. }
                | SubjectVerbActionAst::CopySpellForEachTarget { .. },
            ..
        })
    )
}

fn is_may_cast_copy_effect(effect: &EffectAst) -> bool {
    let EffectAst::May { effects } = effect else {
        return false;
    };
    matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CastTagged { as_copy: true, .. },
            ..
        })]
    )
}

fn group_this_way_copy_cast_followups(tokens: &[OwnedLexToken], effects: &mut Vec<EffectAst>) {
    if !(grammar::contains_phrase(tokens, &["one", "or", "more"])
        && grammar::contains_phrase(tokens, &["this", "way"]))
    {
        return;
    }

    let Some(if_idx) = effects.iter().position(|effect| {
        matches!(
            effect,
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                ..
            }
        )
    }) else {
        return;
    };

    let mut followups = Vec::new();
    while effects
        .get(if_idx + 1)
        .is_some_and(|effect| is_copy_reference_effect(effect) || is_may_cast_copy_effect(effect))
    {
        followups.push(effects.remove(if_idx + 1));
    }
    if followups.is_empty() {
        return;
    }

    if let EffectAst::IfResult {
        effects: nested, ..
    } = &mut effects[if_idx]
    {
        nested.extend(followups);
    }
}

pub(crate) fn is_cant_be_regenerated_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    let destroyed_this_way_subject =
        word_slice_starts_with_any(&words, DESTROYED_THIS_WAY_SUBJECT_PREFIXES);
    if destroyed_this_way_subject
        && words.ends_with(BE_REGENERATED_SUFFIX)
        && (words.iter().any(|word| CANT_WORDS.contains(word))
            || word_slice_contains_phrase(&words, CANT_BE_REGENERATED_SPLIT_PHRASE))
    {
        return true;
    }
    word_slice_eq_any(&words, SIMPLE_CANT_BE_REGENERATED_PHRASES)
}

pub(crate) fn is_cant_be_regenerated_this_turn_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    word_slice_eq_any(&words, CANT_BE_REGENERATED_THIS_TURN_PHRASES)
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

    effects.push(EffectAst::subject_verb_cant(
        crate::effect::Restriction::be_regenerated(filter),
        Until::EndOfTurn,
        None,
    ));
    true
}

fn apply_cant_be_regenerated_to_effect(effect: &mut EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Destroy {
                no_regeneration, ..
            }
            | SubjectVerbActionAst::DestroyAll {
                no_regeneration, ..
            }
            | SubjectVerbActionAst::DestroyAllOfChosenColor {
                no_regeneration, ..
            } => {
                *no_regeneration = true;
                true
            }
            _ => false,
        },
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
    use crate::cards::builders::find_verb;
    use crate::effect::{Value, ValueComparisonOperator};
    use crate::filter::TaggedOpbjectRelation;
    use crate::target::PlayerFilter;

    use super::super::super::grammar::structure::split_lexed_sentences;
    use super::super::super::lexer::lex_line;
    use super::super::super::permission_helpers::parse_until_end_of_turn_may_play_tagged_clause;
    use super::super::super::util::{parse_subject, trim_commas};
    use super::super::zone_handlers::parse_exile_top_library_clause;
    use super::super::{parse_effect_chain, parse_effect_sentence_lexed};
    use super::{
        ConsultCastCost, ConsultCastTiming, Verb, parse_bargained_face_down_cast_mana_value_gate,
        parse_consult_cast_clause, parse_consult_condition_value,
        parse_consult_mana_value_condition_tokens,
        parse_counted_looked_cards_into_your_hand_tokens, parse_exact_card_effect_bundle_lexed,
        parse_if_you_dont_sentence, parse_looked_card_reveal_filter,
        parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
        parse_top_cards_view_sentence,
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

    #[test]
    fn consult_condition_value_reads_source_power_from_token_view() {
        let tokens = lex_line("this's power", 0)
            .expect("rewrite lexer should classify consult value clause");

        let parsed =
            parse_consult_condition_value(&tokens).expect("consult value clause should parse");

        assert_eq!(parsed, Value::SourcePower);
    }

    #[test]
    fn top_cards_view_sentence_reads_reveal_count_from_token_view() {
        let tokens = lex_line("Reveal the top two cards of your library", 0)
            .expect("rewrite lexer should classify top-cards reveal clause");

        let parsed =
            parse_top_cards_view_sentence(&tokens).expect("top-cards reveal clause should parse");

        assert_eq!(
            parsed,
            (
                crate::cards::builders::PlayerAst::You,
                Value::Fixed(2),
                true
            )
        );
    }

    #[test]
    fn counted_looked_cards_into_hand_tokens_parse_those_cards_instead() {
        let tokens = lex_line("Put two of those cards into your hand instead", 0)
            .expect("rewrite lexer should classify counted looked-cards clause");

        let parsed = parse_counted_looked_cards_into_your_hand_tokens(&tokens)
            .expect("counted looked-cards clause should parse");

        assert_eq!(parsed, 2);
    }

    #[test]
    fn reveal_top_put_all_matching_into_hand_rest_graveyard_stays_token_aware() {
        let first = lex_line("Reveal the top three cards of your library", 0)
            .expect("rewrite lexer should classify reveal-top clause");
        let second = lex_line(
            "Put all land cards revealed this way into your hand and the rest into your graveyard",
            0,
        )
        .expect("rewrite lexer should classify reveal follow-up clause");

        let parsed =
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(&first, &second)
                .expect("reveal-top follow-up parser should not error")
                .expect("reveal-top follow-up should parse");

        assert!(matches!(
            parsed.first(),
            Some(crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    subject: crate::cards::builders::SubjectVerbSubjectAst {
                        player: crate::cards::builders::PlayerAst::You,
                        ..
                    },
                    action: crate::cards::builders::SubjectVerbActionAst::LookAtTopCards { .. },
                },
            ))
        ));
        // Now composed from reusable primitives; rest->graveyard is a per-card split.
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::ForEachTagged { .. }
        )));
    }

    #[test]
    fn reveal_top_put_all_matching_into_hand_rest_bottom_keeps_order() {
        let first = lex_line("Reveal the top five cards of your library", 0)
            .expect("rewrite lexer should classify reveal-top clause");
        let second = lex_line(
            "Put all creature cards revealed this way into your hand and the rest on the bottom of your library in any order",
            0,
        )
        .expect("rewrite lexer should classify reveal follow-up clause");

        let parsed =
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(&first, &second)
                .expect("reveal-top follow-up parser should not error")
                .expect("reveal-top bottom follow-up should parse");

        // Now composed from reusable primitives: look + reveal-tagged + tag-matching +
        // move-group-to-hand + remainder-to-bottom (order preserved on the remainder).
        assert!(matches!(
            parsed.first(),
            Some(crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    subject: crate::cards::builders::SubjectVerbSubjectAst {
                        player: crate::cards::builders::PlayerAst::You,
                        ..
                    },
                    action: crate::cards::builders::SubjectVerbActionAst::LookAtTopCards { .. },
                },
            ))
        ));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                            order: crate::cards::builders::LibraryBottomOrderAst::ChooserChooses,
                            ..
                        },
                    ..
                },
            )
        )));
    }

    #[test]
    fn parse_turnabout_mass_tap_sentence_uses_tap_or_untap_all_ast() {
        let tokens = lex_line(
            "Tap all untapped permanents of the chosen type target player controls, or untap all tapped permanents of that type that player controls",
            0,
        )
        .expect("rewrite lexer should classify turnabout mass-tap clause");

        let parsed =
            parse_effect_sentence_lexed(&tokens).expect("turnabout mass-tap clause should parse");

        let [crate::cards::builders::EffectAst::SubjectVerb(subject_verb)] = parsed.as_slice()
        else {
            panic!("expected shared tap-or-untap-all ast, got {parsed:?}");
        };
        let crate::cards::builders::SubjectVerbActionAst::TapOrUntapAll {
            tap_filter,
            untap_filter,
        } = &subject_verb.action
        else {
            panic!("expected shared tap-or-untap-all action, got {parsed:?}");
        };

        assert_eq!(tap_filter.controller, Some(PlayerFilter::target_player()));
        assert_eq!(untap_filter.controller, Some(PlayerFilter::target_player()));
        assert!(tap_filter.chosen_creature_type, "{tap_filter:?}");
        assert!(untap_filter.chosen_creature_type, "{untap_filter:?}");
    }

    #[test]
    fn choose_then_for_each_of_those_bundle_builds_for_each_tagged_loop() {
        let tokens = lex_line(
            "Choose five permanents you control. For each of those permanents, you may search your library for a card with the same name as that permanent. Put those cards onto the battlefield tapped, then shuffle.",
            0,
        )
        .expect("rewrite lexer should classify choose/for-each bundle");

        let parsed = parse_exact_card_effect_bundle_lexed(&tokens)
            .expect("choose/for-each bundle should parse");

        assert!(matches!(
            parsed.as_slice(),
            [
                crate::cards::builders::EffectAst::ChooseObjects { .. },
                crate::cards::builders::EffectAst::ForEachTagged { .. },
                ..,
            ]
        ));
    }

    #[test]
    fn subject_first_exile_top_library_then_play_bundle_parses_directly() {
        let tokens = lex_line(
            "That player exiles the top two cards of their library. Until end of turn, you may play those cards without paying their mana costs.",
            0,
        )
        .expect("rewrite lexer should classify Fallen Shinobi style bundle");

        let sentences = split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 2, "{sentences:#?}");
        let first = sentences[0];
        let second = sentences[1];

        let (verb, verb_idx) = find_verb(first).expect("first sentence should have a verb");
        assert_eq!(verb, Verb::Exile);
        let subject = parse_subject(&trim_commas(&first[..verb_idx]));
        let exile_tokens = trim_commas(&first[verb_idx + 1..]);
        let exile_effect = parse_exile_top_library_clause(&exile_tokens, Some(subject));
        assert!(exile_effect.is_some(), "expected exile clause to parse");

        let permission_effect = parse_until_end_of_turn_may_play_tagged_clause(second)
            .expect("permission clause should not error");
        assert!(
            permission_effect.is_some(),
            "expected permission clause to parse"
        );

        let parsed = parse_exact_card_effect_bundle_lexed(&tokens)
            .expect("subject-first exile/play bundle should parse directly");

        let debug = format!("{parsed:#?}").to_ascii_lowercase();
        assert!(
            debug.contains("exiletopoflibrary"),
            "expected exile-top-library effect, got {debug}"
        );
        assert!(
            debug.contains("grantplaytaggeduntilendofturn"),
            "expected play permission effect, got {debug}"
        );
    }

    #[test]
    fn exile_then_source_leaves_return_bundle_collapses_to_until_source_leaves() {
        let tokens = lex_line(
            "If there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.",
            0,
        )
        .expect("rewrite lexer should classify source-leaves exile bundle");

        let parsed = parse_exact_card_effect_bundle_lexed(&tokens)
            .or_else(|| parse_effect_chain(&tokens).ok())
            .expect("source-leaves exile bundle should parse through a supported sentence path");

        let debug = format!("{parsed:#?}").to_ascii_lowercase();
        assert!(
            debug.contains("exileuntilsourceleaves")
                || (debug.contains("exile {") && debug.contains("__it__")),
            "expected source-leaves exile bundle or equivalent tagged exile scaffold, got {debug}"
        );
        assert!(
            !debug.contains("returnfromgraveyardtobattlefield"),
            "expected source-leaves bundle not to lower into graveyard-return, got {debug}"
        );
    }

    #[test]
    fn reveal_top_then_for_each_card_type_bundle_parses_directly() {
        let tokens = lex_line(
            "Reveal the top five cards of your library. For each card type among noncreature spells you've cast this turn, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("rewrite lexer should classify Hurkyl reveal bundle");

        let sentences = split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 3, "{sentences:#?}");

        let parsed =
            super::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
                sentences[0],
                sentences[1],
                sentences[2],
            )
            .expect("Hurkyl reveal bundle helper should not error")
            .expect("Hurkyl reveal bundle helper should parse");

        // Now composed from reusable primitives: look + reveal-tagged + per-card-type
        // conditional choose (gated on a matching spell cast this turn) +
        // move-group-to-hand + remainder-to-bottom.
        assert!(matches!(
            parsed.first(),
            Some(crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::LookAtTopCards { .. },
                    ..
                }
            ))
        ));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::ValueComparison { .. },
                ..
            }
        )));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
                    ..
                }
            )
        )));
    }

    #[test]
    fn reveal_top_then_for_each_card_type_bundle_parses_atraxa_variant() {
        let tokens = lex_line(
            "Reveal the top ten cards of your library. For each card type, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("rewrite lexer should classify Atraxa reveal bundle");

        let sentences = split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 3, "{sentences:#?}");

        let parsed = super::parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(
            sentences[0],
            sentences[1],
            sentences[2],
        )
        .expect("Atraxa reveal bundle helper should not error")
        .expect("Atraxa reveal bundle helper should parse");

        // Now composed from reusable primitives: look + reveal-tagged + per-card-type
        // choose-across-zones (ungated) + move-group-to-hand + remainder-to-bottom.
        assert!(matches!(
            parsed.first(),
            Some(crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::LookAtTopCards { .. },
                    ..
                }
            ))
        ));
        assert!(matches!(
            parsed.get(1),
            Some(crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::RevealTagged { .. },
                    ..
                }
            ))
        ));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::ChooseObjectsAcrossZones { .. }
        )));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
                    ..
                }
            )
        )));
    }

    #[test]
    fn bargained_face_down_cast_gate_parses_with_winnow_clause_parser() {
        let tokens = lex_line(
            "If this spell was bargained, you may cast the exiled card without paying its mana cost if that spell's mana value is 3 or less",
            0,
        )
        .expect("rewrite lexer should classify bargained face-down cast clause");

        let parsed = parse_bargained_face_down_cast_mana_value_gate(&tokens)
            .expect("bargained face-down cast clause should not error")
            .expect("bargained face-down cast clause should parse");

        assert_eq!(parsed.0, ValueComparisonOperator::LessThanOrEqual);
        assert_eq!(parsed.1, Value::Fixed(3));
    }

    #[test]
    fn if_you_dont_clause_reports_missing_comma_after_matched_prefix() {
        let tokens = lex_line("If you don't draw a card", 0)
            .expect("rewrite lexer should classify if-you-don't clause");

        let err = parse_if_you_dont_sentence(&tokens)
            .expect_err("matched if-you-don't clause without comma should cut");

        assert!(
            err.to_string().contains("comma after if-you-don't clause"),
            "unexpected error: {err}"
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
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDistributedDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. } => Some(target.clone()),
            _ => None,
        },
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
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDistributedDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. }
            | SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::LookAtHand { target }
            | SubjectVerbActionAst::Counter { target }
            | SubjectVerbActionAst::CounterUnlessPays { target, .. }
            | SubjectVerbActionAst::PutCounters { target, .. }
            | SubjectVerbActionAst::PutCounterChoice { target, .. }
            | SubjectVerbActionAst::ReturnToHand { target, .. }
            | SubjectVerbActionAst::Detain { target }
            | SubjectVerbActionAst::Goad { target }
            | SubjectVerbActionAst::Suspect { target }
            | SubjectVerbActionAst::RemoveFromCombat { target }
            | SubjectVerbActionAst::Flip { target }
            | SubjectVerbActionAst::Regenerate { target, .. }
            | SubjectVerbActionAst::TapOrUntap { target }
            | SubjectVerbActionAst::PhaseOut { target }
            | SubjectVerbActionAst::PhaseIn { target }
            | SubjectVerbActionAst::Transform { target }
            | SubjectVerbActionAst::Convert { target }
            | SubjectVerbActionAst::Explore { target }
            | SubjectVerbActionAst::Endure { target, .. }
            | SubjectVerbActionAst::Connive { target, .. }
            | SubjectVerbActionAst::MoveToLibraryNthFromTop { target, .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target }
            | SubjectVerbActionAst::RemoveUpToAnyCounters { target, .. }
            | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { target, .. }
            | SubjectVerbActionAst::PutCounterOfChosenKind { target }
            | SubjectVerbActionAst::PutSticker { target, .. }
            | SubjectVerbActionAst::SwitchPowerToughness { target, .. }
            | SubjectVerbActionAst::GrantProtectionChoice { target, .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSource { source: target, .. }
            | SubjectVerbActionAst::ExileWhenSourceLeaves { target }
            | SubjectVerbActionAst::SacrificeSourceWhenLeaves { target }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { target, .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            }
            | SubjectVerbActionAst::PreventDamage { target, .. }
            | SubjectVerbActionAst::PreventAllDamageToTarget { target, .. }
            | SubjectVerbActionAst::PreventDamageToTargetPutCounters { target, .. }
            | SubjectVerbActionAst::PutOrRemoveCounters { target, .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. }
            | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
            | SubjectVerbActionAst::MoveToZone { target, .. }
            | SubjectVerbActionAst::TargetOnly { target }
            | SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::SetBasePowerToughness { target, .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { target, .. }
            | SubjectVerbActionAst::SetBasePower { target, .. }
            | SubjectVerbActionAst::PumpForEach { target, .. }
            | SubjectVerbActionAst::PumpByLastEffect { target, .. }
            | SubjectVerbActionAst::GainControl { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. } => {
                Some(target.clone())
            }
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            } => protected_target
                .as_ref()
                .or(destination_target.as_ref())
                .cloned(),
            _ => None,
        },
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

fn time_travel_effect_ast() -> EffectAst {
    let permanent_with_time_counter = ObjectFilter::permanent()
        .you_control()
        .with_counter_type(crate::object::CounterType::Time);
    let suspended_card_with_time_counter = ObjectFilter::default()
        .in_zone(Zone::Exile)
        .owned_by(PlayerFilter::You)
        .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
        .with_counter_type(crate::object::CounterType::Time);
    let target = TargetAst::Object(
        ObjectFilter {
            any_of: vec![
                permanent_with_time_counter,
                suspended_card_with_time_counter,
            ],
            ..ObjectFilter::default()
        },
        None,
        None,
    );
    EffectAst::subject_verb_fixed_counter_kind_put_or_remove(
        target,
        crate::object::CounterType::Time,
        true,
    )
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
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::DealDamage {
                target: damage_target,
                ..
            }
            | SubjectVerbActionAst::DealDamageEqualToPower {
                target: damage_target,
                ..
            } => {
                if is_placeholder_damage_target(damage_target) {
                    *damage_target = target.clone();
                }
            }
            _ => {}
        },
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
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::DealDamage { amount, .. }
            | SubjectVerbActionAst::DealDistributedDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEach { amount, .. } => {
                if value_contains_unbound_x(amount) {
                    *amount = replace_unbound_x_with_value(amount.clone(), replacement, clause)?;
                }
            }
            _ => {}
        },
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

    fn replace_in_target(
        target: &mut TargetAst,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        match target {
            TargetAst::Object(filter, _, _) => replace_in_filter(filter, replacement, clause)?,
            TargetAst::WithCount(inner, _) => replace_in_target(inner, replacement, clause)?,
            TargetAst::WithCountValue(inner, _, value) => {
                replace_in_target(inner, replacement, clause)?;
                replace_value(value, replacement, clause)?;
            }
            _ => {}
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

    fn replace_values_in_cost_component(
        component: &mut crate::costs::Cost,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        match component {
            crate::costs::Cost::DynamicMana(dynamic) => {
                if dynamic.base.has_x() && dynamic.x_value.is_none() {
                    dynamic.x_value = Some(replacement.clone());
                } else if let Some(value) = dynamic.x_value.as_mut() {
                    replace_value(value, replacement, clause)?;
                }
                if let Some(value) = dynamic.additional_generic.as_mut() {
                    replace_value(value, replacement, clause)?;
                }
                if let Some(value) = dynamic.multiplier.as_mut() {
                    replace_value(value, replacement, clause)?;
                }
            }
            crate::costs::Cost::Energy(value)
            | crate::costs::Cost::Mill(value)
            | crate::costs::Cost::Life(value) => replace_value(value, replacement, clause)?,
            _ => {}
        }
        Ok(())
    }

    fn replace_values_in_total_cost(
        cost: &mut crate::cost::TotalCost,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        match cost.kind() {
            ironsmith_core::TotalCostKind::All(_) => {
                let mut components = cost.costs().to_vec();
                for component in &mut components {
                    replace_values_in_cost_component(component, replacement, clause)?;
                }
                *cost = crate::cost::TotalCost::from_costs(components);
            }
            ironsmith_core::TotalCostKind::OneOf(branches) => {
                let mut branches = branches.to_vec();
                for branch in &mut branches {
                    replace_values_in_total_cost(branch, replacement, clause)?;
                }
                *cost = crate::cost::TotalCost::one_of(branches);
            }
        }
        Ok(())
    }

    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Draw { count: amount }
            | SubjectVerbActionAst::ExileTopOfLibrary { count: amount, .. }
            | SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::Mill { count: amount }
            | SubjectVerbActionAst::Scry { count: amount }
            | SubjectVerbActionAst::Surveil { count: amount }
            | SubjectVerbActionAst::Proliferate { count: amount }
            | SubjectVerbActionAst::Investigate { count: amount }
            | SubjectVerbActionAst::Amass { amount, .. }
            | SubjectVerbActionAst::Monstrosity { amount }
            | SubjectVerbActionAst::Discover { count: amount }
            | SubjectVerbActionAst::Fateseal { count: amount }
            | SubjectVerbActionAst::Populate { count: amount, .. }
            | SubjectVerbActionAst::Connive { count: amount, .. }
            | SubjectVerbActionAst::DealDamage { amount, .. }
            | SubjectVerbActionAst::DealDistributedDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEach { amount, .. }
            | SubjectVerbActionAst::PreventDamage { amount, .. }
            | SubjectVerbActionAst::PreventDamageEach { amount, .. }
            | SubjectVerbActionAst::CopySpell { count: amount, .. }
            | SubjectVerbActionAst::PutCounters { count: amount, .. }
            | SubjectVerbActionAst::PutCounterChoice { count: amount, .. }
            | SubjectVerbActionAst::PutCountersAll { count: amount, .. }
            | SubjectVerbActionAst::RemoveUpToAnyCounters { amount, .. }
            | SubjectVerbActionAst::RemoveCountersAll { amount, .. }
            | SubjectVerbActionAst::Discard { count: amount, .. }
            | SubjectVerbActionAst::PoisonCounters { count: amount }
            | SubjectVerbActionAst::EnergyCounters { count: amount }
            | SubjectVerbActionAst::ExperienceCounters { count: amount }
            | SubjectVerbActionAst::TicketCounters { count: amount }
            | SubjectVerbActionAst::PayEnergy { amount }
            | SubjectVerbActionAst::SetLifeTotal { amount }
            | SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount }
            | SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { amount, .. }
            | SubjectVerbActionAst::LookAtTopCards { count: amount, .. }
            | SubjectVerbActionAst::MoveToLibraryNthFromTop {
                position: amount, ..
            }
            | SubjectVerbActionAst::AdditionalLandPlays { count: amount, .. } => {
                replace_value(amount, replacement, clause)?;
            }
            SubjectVerbActionAst::Incubate { amount, count } => {
                replace_value(amount, replacement, clause)?;
                replace_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::CounterUnlessPays { cost, .. } => {
                replace_values_in_total_cost(cost, replacement, clause)?;
            }
            SubjectVerbActionAst::PreventDamageToTargetPutCounters {
                amount: Some(amount),
                ..
            } => {
                replace_value(amount, replacement, clause)?;
            }
            SubjectVerbActionAst::PutOrRemoveCounters {
                put_count,
                remove_count,
                ..
            } => {
                replace_value(put_count, replacement, clause)?;
                replace_value(remove_count, replacement, clause)?;
            }
            SubjectVerbActionAst::Pump {
                power, toughness, ..
            }
            | SubjectVerbActionAst::SetBasePowerToughness {
                power, toughness, ..
            }
            | SubjectVerbActionAst::BecomeBasePtCreature {
                power, toughness, ..
            }
            | SubjectVerbActionAst::PumpAll {
                power, toughness, ..
            } => {
                replace_value(power, replacement, clause)?;
                replace_value(toughness, replacement, clause)?;
            }
            SubjectVerbActionAst::SetBasePower { power, .. } => {
                replace_value(power, replacement, clause)?;
            }
            SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn { reduction, .. } => {
                replace_value(reduction, replacement, clause)?;
            }
            SubjectVerbActionAst::PumpForEach { count, .. } => {
                replace_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::ConsultTopOfLibrary {
                filter, stop_rule, ..
            } => {
                replace_in_filter(filter, replacement, clause)?;
                if let LibraryConsultStopRuleAst::MatchCount(count) = stop_rule {
                    replace_value(count, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::DealDamageEqualToPower { .. }
            | SubjectVerbActionAst::DrawForEachTaggedMatching { .. }
            | SubjectVerbActionAst::RevealHand
            | SubjectVerbActionAst::RevealTagged { .. }
            | SubjectVerbActionAst::PutOntoBattlefield { .. }
            | SubjectVerbActionAst::RevealCardsFromHand { .. }
            | SubjectVerbActionAst::LookAtObjects { .. }
            | SubjectVerbActionAst::LookAtTarget { .. }
            | SubjectVerbActionAst::EmitKeywordAction { .. }
            | SubjectVerbActionAst::Bolster { .. }
            | SubjectVerbActionAst::Support { .. }
            | SubjectVerbActionAst::Adapt { .. }
            | SubjectVerbActionAst::Explore { .. }
            | SubjectVerbActionAst::Endure { .. }
            | SubjectVerbActionAst::Exploit
            | SubjectVerbActionAst::ConniveIterated
            | SubjectVerbActionAst::OpenAttraction
            | SubjectVerbActionAst::ManifestTopCardOfLibrary
            | SubjectVerbActionAst::ManifestCardFromHand
            | SubjectVerbActionAst::ManifestDread
            | SubjectVerbActionAst::Earthbend { .. }
            | SubjectVerbActionAst::Behold { .. }
            | SubjectVerbActionAst::Fight { .. }
            | SubjectVerbActionAst::FightIterated { .. }
            | SubjectVerbActionAst::Clash { .. }
            | SubjectVerbActionAst::FlipCoin
            | SubjectVerbActionAst::RollDie { .. }
            | SubjectVerbActionAst::RollDiceChooseResult { .. }
            | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
            | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary
            | SubjectVerbActionAst::ReorderGraveyard
            | SubjectVerbActionAst::ChooseColor
            | SubjectVerbActionAst::ChooseCardType { .. }
            | SubjectVerbActionAst::ChooseNamedOption { .. }
            | SubjectVerbActionAst::ChooseCreatureType { .. }
            | SubjectVerbActionAst::ChooseCardName { .. }
            | SubjectVerbActionAst::ChoosePlayer { .. }
            | SubjectVerbActionAst::NoteLifeTotal
            | SubjectVerbActionAst::AddMana { .. }
            | SubjectVerbActionAst::ExchangeLifeTotals { .. }
            | SubjectVerbActionAst::ExchangeTextBoxes { .. }
            | SubjectVerbActionAst::ExchangeZones { .. }
            | SubjectVerbActionAst::PutRestOnBottomOfLibrary
            | SubjectVerbActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn
            | SubjectVerbActionAst::ExchangeValues { .. }
            | SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn
            | SubjectVerbActionAst::ControlCombatChoicesThisTurn { .. }
            | SubjectVerbActionAst::GainControl { .. }
            | SubjectVerbActionAst::AddManaColorsAmong { .. }
            | SubjectVerbActionAst::AddManaImprintedColors
            | SubjectVerbActionAst::DoubleManaPool
            | SubjectVerbActionAst::EmptyManaPool
            | SubjectVerbActionAst::EndTurn
            | SubjectVerbActionAst::SkipTurn
            | SubjectVerbActionAst::SkipCombatPhases
            | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
            | SubjectVerbActionAst::SkipMainPhasesThisTurn
            | SubjectVerbActionAst::SkipCombatPhasesThisTurn
            | SubjectVerbActionAst::SkipDrawStep
            | SubjectVerbActionAst::PlayFromGraveyardUntilEot
            | SubjectVerbActionAst::ControlPlayer { .. }
            | SubjectVerbActionAst::ReduceNextSpellCostThisTurn { .. }
            | SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { .. }
            | SubjectVerbActionAst::RingTemptsYou
            | SubjectVerbActionAst::VentureIntoDungeon { .. }
            | SubjectVerbActionAst::BecomeMonarch
            | SubjectVerbActionAst::TakeInitiative
            | SubjectVerbActionAst::CreateEmblem { .. }
            | SubjectVerbActionAst::LoseGame
            | SubjectVerbActionAst::WinGame
            | SubjectVerbActionAst::PayAnyEnergy { .. }
            | SubjectVerbActionAst::PayAnyLife { .. }
            | SubjectVerbActionAst::PayMana { .. }
            | SubjectVerbActionAst::DiscardHand
            | SubjectVerbActionAst::Detain { .. }
            | SubjectVerbActionAst::Goad { .. }
            | SubjectVerbActionAst::Suspect { .. }
            | SubjectVerbActionAst::ClearSuspected { .. }
            | SubjectVerbActionAst::RemoveFromCombat { .. }
            | SubjectVerbActionAst::Flip { .. }
            | SubjectVerbActionAst::Regenerate { .. }
            | SubjectVerbActionAst::RegenerateAll { .. }
            | SubjectVerbActionAst::TapAll { .. }
            | SubjectVerbActionAst::UntapAll { .. }
            | SubjectVerbActionAst::TapOrUntap { .. }
            | SubjectVerbActionAst::TapOrUntapAll { .. }
            | SubjectVerbActionAst::PhaseOut { .. }
            | SubjectVerbActionAst::PhaseOutAll { .. }
            | SubjectVerbActionAst::PhaseIn { .. }
            | SubjectVerbActionAst::PhaseInAll { .. }
            | SubjectVerbActionAst::Transform { .. }
            | SubjectVerbActionAst::Convert { .. }
            | SubjectVerbActionAst::Tap { .. }
            | SubjectVerbActionAst::Untap { .. }
            | SubjectVerbActionAst::Destroy { .. }
            | SubjectVerbActionAst::DestroyAll { .. }
            | SubjectVerbActionAst::DestroyAllOfChosenColor { .. }
            | SubjectVerbActionAst::Exile { .. }
            | SubjectVerbActionAst::ExileAll { .. }
            | SubjectVerbActionAst::LookAtHand { .. }
            | SubjectVerbActionAst::Counter { .. }
            | SubjectVerbActionAst::ReturnToHand { .. }
            | SubjectVerbActionAst::ReturnAllToHand { .. }
            | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { .. }
            | SubjectVerbActionAst::DoubleCountersOnEach { .. }
            | SubjectVerbActionAst::MoveAllCounters { .. }
            | SubjectVerbActionAst::MoveOneCounter { .. }
            | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { .. }
            | SubjectVerbActionAst::PutCounterOfChosenKind { .. }
            | SubjectVerbActionAst::Sacrifice { .. }
            | SubjectVerbActionAst::SacrificeAll { .. }
            | SubjectVerbActionAst::RevealTop
            | SubjectVerbActionAst::PutIntoHand { .. }
            | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
            | SubjectVerbActionAst::RearrangeLookedCardsInLibrary { .. }
            | SubjectVerbActionAst::ReorderTopOfLibrary { .. }
            | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { .. }
            | SubjectVerbActionAst::PutSticker { .. }
            | SubjectVerbActionAst::SwitchPowerToughness { .. }
            | SubjectVerbActionAst::ScalePowerToughnessAll { .. }
            | SubjectVerbActionAst::ScaleXValue { .. }
            | SubjectVerbActionAst::GrantProtectionChoice { .. }
            | SubjectVerbActionAst::PreventAllCombatDamage { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSource { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageToPlayers { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageToYou { .. }
            | SubjectVerbActionAst::PreventNextTimeDamage { .. }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget { .. }
            | SubjectVerbActionAst::PreventAllDamageToTarget { .. }
            | SubjectVerbActionAst::PreventAllDamageToTargetFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventAllDamageFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventDamageToTargetPutCounters { amount: None, .. }
            | SubjectVerbActionAst::Meld { .. }
            | SubjectVerbActionAst::SearchLibrarySlotsToHand { .. }
            | SubjectVerbActionAst::RetargetStackObject { .. }
            | SubjectVerbActionAst::GrantAbilityToSource { .. }
            | SubjectVerbActionAst::ExchangeControl { .. }
            | SubjectVerbActionAst::ExchangeControlHeterogeneous { .. }
            | SubjectVerbActionAst::DestroyAllAttachedTo { .. }
            | SubjectVerbActionAst::ExileAllAttachedTo { .. }
            | SubjectVerbActionAst::Attach { .. }
            | SubjectVerbActionAst::Unattach { .. }
            | SubjectVerbActionAst::ExileWhenSourceLeaves { .. }
            | SubjectVerbActionAst::SacrificeSourceWhenLeaves { .. }
            | SubjectVerbActionAst::MayMoveToZone { .. }
            | SubjectVerbActionAst::RegisterZoneReplacement { .. }
            | SubjectVerbActionAst::RegisterFutureZoneReplacement { .. }
            | SubjectVerbActionAst::RegisterDrawReplacement { .. }
            | SubjectVerbActionAst::RegisterManaReplacement { .. }
            | SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement { .. }
            | SubjectVerbActionAst::Enchant { .. }
            | SubjectVerbActionAst::ChooseSpellCastHistory { .. }
            | SubjectVerbActionAst::CopySpellForEachTarget { .. }
            | SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::PutTaggedRemainderInZone { .. }
            | SubjectVerbActionAst::CastTagged { .. }
            | SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { .. }
            | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn { .. }
            | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. }
            | SubjectVerbActionAst::ReturnToBattlefield { .. }
            | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { .. }
            | SubjectVerbActionAst::TargetOnly { .. }
            | SubjectVerbActionAst::TagMatchingObjects { .. }
            | SubjectVerbActionAst::PumpByLastEffect { .. }
            | SubjectVerbActionAst::AddCardTypes { .. }
            | SubjectVerbActionAst::RemoveCardTypes { .. }
            | SubjectVerbActionAst::AddSubtypes { .. }
            | SubjectVerbActionAst::SetCreatureSubtypes { .. }
            | SubjectVerbActionAst::BecomeSaddledUntilEndOfTurn { .. }
            | SubjectVerbActionAst::AddColors { .. }
            | SubjectVerbActionAst::AddAllSubtypesOfFamily { .. }
            | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { .. }
            | SubjectVerbActionAst::BecomeAuraEnchantment { .. }
            | SubjectVerbActionAst::BecomeBasicLandType { .. }
            | SubjectVerbActionAst::SetColors { .. }
            | SubjectVerbActionAst::MakeColorless { .. }
            | SubjectVerbActionAst::BecomeBasicLandTypeChoice { .. }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice { .. }
            | SubjectVerbActionAst::BecomeColorChoice { .. }
            | SubjectVerbActionAst::BecomeCopy { .. }
            | SubjectVerbActionAst::GrantAbilitiesAll { .. }
            | SubjectVerbActionAst::RemoveAbilitiesAll { .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceAll { .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { .. }
            | SubjectVerbActionAst::GrantToTarget { .. }
            | SubjectVerbActionAst::GrantBySpec { .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { .. }
            | SubjectVerbActionAst::AdditionalPhases { .. }
            | SubjectVerbActionAst::TurnFaceUp { .. }
            | SubjectVerbActionAst::ShuffleLibrary => {}
            SubjectVerbActionAst::Cant { .. } => {}
            SubjectVerbActionAst::SearchLibrary {
                filter,
                count_value,
                library_position_from_top,
                ..
            } => {
                replace_in_filter(filter, replacement, clause)?;
                if let Some(count_value) = count_value.as_mut() {
                    replace_value(count_value, replacement, clause)?;
                }
                if let Some(position) = library_position_from_top.as_mut() {
                    replace_value(position, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::MoveToZone {
                target,
                attached_to,
                ..
            } => {
                replace_in_target(target, replacement, clause)?;
                if let Some(attached_to) = attached_to {
                    replace_in_target(attached_to, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::CreateTokenCopy { count, .. }
            | SubjectVerbActionAst::CreateTokenCopyFromSource { count, .. } => {
                replace_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::CreateTokenWithMods {
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
            SubjectVerbActionAst::Learn
            | SubjectVerbActionAst::DoubleCountersOnTarget { .. }
            | SubjectVerbActionAst::RegisterEnterUnderControlReplacement { .. } => {}
        },
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
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let has_deal_x = word_slice_contains_any_phrase(&clause_words, DEAL_X_DAMAGE_PHRASES);
    let has_x_life = word_slice_contains_any_phrase(&clause_words, X_LIFE_CHANGE_PHRASES);
    let Some(where_idx) = word_slice_find_phrase_start(&clause_words, WHERE_X_IS_WORDS) else {
        return Ok(());
    };
    let has_unbound_x_before_where = clause_words[..where_idx].iter().any(|word| *word == X_WORD);
    if !has_deal_x && !has_x_life && !has_unbound_x_before_where {
        return Ok(());
    }
    let Some(where_token_idx) = token_index_for_word_index(tokens, where_idx) else {
        return Ok(());
    };
    let where_tokens = &tokens[where_token_idx..];
    let Some(where_value) = parse_value_binding_clause(where_tokens)
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
    else {
        return Ok(());
    };
    if has_deal_x || has_x_life {
        replace_unbound_x_in_damage_effects(effects, &where_value, &clause_words.join(" "))
    } else {
        replace_unbound_x_in_effects_anywhere(effects, &where_value, &clause_words.join(" "))
    }
}

pub(crate) fn replace_it_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::DealDamage {
                target: damage_target,
                ..
            } => {
                if target_references_it(damage_target) {
                    *damage_target = target.clone();
                }
            }
            _ => {}
        },
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_it_target(effect: &mut EffectAst, target: &TargetAst) {
    fn should_replace_self_replacement_target(effect_target: &TargetAst) -> bool {
        target_references_it(effect_target)
            || matches!(
                effect_target,
                TargetAst::Tagged(_, _) | TargetAst::Source(_)
            )
    }

    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::DealDamage {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::DealDamageEqualToPower {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::Tap {
                target: effect_target,
            }
            | SubjectVerbActionAst::Untap {
                target: effect_target,
            }
            | SubjectVerbActionAst::Destroy {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::Exile {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::LookAtHand {
                target: effect_target,
            }
            | SubjectVerbActionAst::Counter {
                target: effect_target,
            }
            | SubjectVerbActionAst::CounterUnlessPays {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::PutCounters {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::PutCounterChoice {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::ReturnToHand {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::Detain {
                target: effect_target,
            }
            | SubjectVerbActionAst::Goad {
                target: effect_target,
            }
            | SubjectVerbActionAst::Suspect {
                target: effect_target,
            }
            | SubjectVerbActionAst::RemoveFromCombat {
                target: effect_target,
            }
            | SubjectVerbActionAst::Flip {
                target: effect_target,
            }
            | SubjectVerbActionAst::Regenerate {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::TapOrUntap {
                target: effect_target,
            }
            | SubjectVerbActionAst::PhaseOut {
                target: effect_target,
            }
            | SubjectVerbActionAst::PhaseIn {
                target: effect_target,
            }
            | SubjectVerbActionAst::Transform {
                target: effect_target,
            }
            | SubjectVerbActionAst::Convert {
                target: effect_target,
            }
            | SubjectVerbActionAst::Explore {
                target: effect_target,
            }
            | SubjectVerbActionAst::Endure {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::GainControl {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: effect_target,
            }
            | SubjectVerbActionAst::PreventDamage {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::PreventAllDamageToTarget {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::PreventAllDamageToTargetFromSourceFilter {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::PreventDamageToTargetPutCounters {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::PutOrRemoveCounters {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::TargetOnly {
                target: effect_target,
            }
            | SubjectVerbActionAst::Connive {
                target: effect_target,
                ..
            } => {
                if should_replace_self_replacement_target(effect_target) {
                    *effect_target = target.clone();
                }
            }
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            } => {
                for effect_target in protected_target
                    .iter_mut()
                    .chain(destination_target.iter_mut())
                {
                    if should_replace_self_replacement_target(effect_target) {
                        *effect_target = target.clone();
                    }
                }
            }
            SubjectVerbActionAst::Pump {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::SetBasePowerToughness {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::BecomeBasePtCreature {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::SetBasePower {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::PumpForEach {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::PumpByLastEffect {
                target: effect_target,
                ..
            } => {
                if should_replace_self_replacement_target(effect_target) {
                    *effect_target = target.clone();
                }
            }
            SubjectVerbActionAst::MoveToZone {
                target: effect_target,
                attached_to,
                ..
            } => {
                if should_replace_self_replacement_target(effect_target) {
                    *effect_target = target.clone();
                }
                if let Some(effect_target) = attached_to
                    && should_replace_self_replacement_target(effect_target)
                {
                    *effect_target = target.clone();
                }
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                target: effect_target,
                ..
            } => {
                if should_replace_self_replacement_target(effect_target) {
                    *effect_target = target.clone();
                }
            }
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::GrantToTarget {
                target: effect_target,
                ..
            }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                target: effect_target,
                ..
            } => {
                if target_references_it(effect_target) {
                    *effect_target = target.clone();
                }
            }
            _ => {}
        },
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
    grammar::words_match_prefix(
        tokens,
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
    )
    .is_some()
        || grammar::words_match_prefix(
            tokens,
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
        .is_some()
}

pub(crate) fn most_recent_extra_turn_player(effects: &[EffectAst]) -> Option<PlayerAst> {
    effects.iter().rev().find_map(|effect| {
        let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            subject: crate::cards::builders::SubjectVerbSubjectAst { player, .. },
            action: SubjectVerbActionAst::ExtraTurnAfterTurn { .. },
        }) = effect
        else {
            return None;
        };
        Some(*player)
    })
}

pub(crate) fn rewrite_when_one_or_more_this_way_clause_prefix(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    // Generic "When one or more ... this way, ..." follow-ups are semantically
    // "If you do, ..." against the immediately previous effect result.
    let this_way_in_prefix = grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        .map(|(before, _after)| grammar::contains_phrase(before, &["this", "way"]))
        .unwrap_or(false);
    if (grammar::strip_lexed_prefix_phrase(tokens, &["when", "one", "or", "more"]).is_some()
        || grammar::strip_lexed_prefix_phrase(tokens, &["whenever", "one", "or", "more"]).is_some())
        && this_way_in_prefix
    {
        let Some((_before, after)) =
            grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        else {
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

        rewritten.push(OwnedLexToken::comma(tokens[0].span()));
        rewritten.extend_from_slice(after);
        return rewritten;
    }

    tokens.to_vec()
}

pub(crate) fn strip_otherwise_sentence_prefix(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    if !tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == OTHERWISE_WORD)
    {
        return None;
    }

    let mut idx = 1usize;
    while tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }
    if token_slice_at_is(tokens, idx, "then") {
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
    let clause_words = crate::runtime_backend::token_word_refs(&tokens);
    let is_referential_get = clause_words.len() >= 3
        && clause_words[0] == REFERENTIAL_THAT_WORD
        && REFERENTIAL_NOUN_WORDS.contains(&clause_words[1])
        && GET_GAIN_WORDS.contains(&clause_words[2]);
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
    is_activate_only_restriction_sentence(tokens)
        || is_trigger_only_restriction_sentence(tokens)
        || word_slice_eq_any(
            &crate::runtime_backend::token_word_refs(tokens),
            NONSEMANTIC_X_CANT_BE_ZERO_PHRASES,
        )
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
    let tokens = if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "create")
    {
        &tokens[1..]
    } else {
        tokens
    };
    let filtered = crate::runtime_backend::util::non_article_token_word_refs(tokens);
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
    let tokens = if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "create")
    {
        &tokens[1..]
    } else {
        tokens
    };
    let filtered = crate::runtime_backend::util::non_article_token_word_refs(tokens);
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

pub(crate) fn parse_token_granted_ability_followup_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let words = TokenWordView::new(tokens);
    let prefix_len = if word_view_has_prefix(&words, &["it", "has"])
        || word_view_has_prefix(&words, &["that", "token", "has"])
        || word_view_has_prefix(&words, &["the", "token", "has"])
    {
        if word_view_has_prefix(&words, &["it", "has"]) {
            2
        } else {
            3
        }
    } else if word_view_has_prefix(&words, &["they", "have"])
        || word_view_has_prefix(&words, &["those", "tokens", "have"])
        || word_view_has_prefix(&words, &["the", "tokens", "have"])
    {
        if word_view_has_prefix(&words, &["they", "have"]) {
            2
        } else {
            3
        }
    } else {
        return Ok(None);
    };

    let Some(ability_start) = words.token_index_after_words(prefix_len) else {
        return Ok(None);
    };
    let ability_tokens = trim_edge_punctuation(&tokens[ability_start..]);
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let (abilities, is_choice) =
        super::parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, false)?;
    if is_choice || abilities.is_empty() {
        return Ok(None);
    }
    Ok(Some(abilities))
}

fn apply_unapplied_token_copy_followup(
    sentence: &[OwnedLexToken],
    _sentence_tokens: &[OwnedLexToken],
    followup: TokenCopyFollowup,
) -> Result<Vec<EffectAst>, CardTextError> {
    let span = span_from_tokens(sentence);
    let effects = match followup {
        TokenCopyFollowup::HasHaste => vec![EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
            Until::Forever,
        )],
        TokenCopyFollowup::GainHasteUntilEndOfTurn => {
            vec![EffectAst::subject_verb_grant_abilities_to_target(
                TargetAst::Tagged(TagKey::from(IT_TAG), span),
                vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
                Until::EndOfTurn,
            )]
        }
        TokenCopyFollowup::EnterTappedAndAttacking => {
            return Err(CardTextError::ParseError(
                "standalone 'enters tapped and attacking' follow-up requires a preceding token-copy, populate, or meld effect".to_string(),
            ));
        }
        TokenCopyFollowup::SacrificeAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::subject_verb_sacrifice(
                PlayerAst::Implicit,
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                1,
                None,
            )],
        }],
        TokenCopyFollowup::ExileAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::subject_verb_exile(
                TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                false,
            )],
        }],
        TokenCopyFollowup::ExileAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::subject_verb_exile(
                TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                false,
            )],
        }],
        TokenCopyFollowup::SacrificeAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::subject_verb_sacrifice(
                PlayerAst::Implicit,
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                1,
                None,
            )],
        }],
    };
    Ok(effects)
}

pub(crate) fn try_apply_token_granted_ability_followup(
    effects: &mut [EffectAst],
    abilities: &[GrantedAbilityAst],
) -> Result<bool, CardTextError> {
    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    match last {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenWithMods {
                    granted_abilities, ..
                },
            ..
        }) => {
            granted_abilities.extend(abilities.iter().cloned());
            Ok(true)
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            if try_apply_token_granted_ability_followup(if_true.as_mut_slice(), abilities)? {
                return Ok(true);
            }
            if try_apply_token_granted_ability_followup(if_false.as_mut_slice(), abilities)? {
                return Ok(true);
            }
            Ok(false)
        }
        _ => {
            let Some(nested_effects) = token_copy_followup_container_effects_mut(last) else {
                return Ok(false);
            };
            if nested_effects.is_empty() {
                return Ok(false);
            }
            try_apply_token_granted_ability_followup(nested_effects.as_mut_slice(), abilities)
        }
    }
}

pub(crate) fn try_apply_token_copy_followup(
    effects: &mut [EffectAst],
    followup: TokenCopyFollowup,
) -> Result<bool, CardTextError> {
    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    match last {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Populate {
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
            SubjectVerbActionAst::Meld {
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
            SubjectVerbActionAst::CreateTokenCopy {
                has_haste,
                enters_tapped,
                enters_attacking,
                exile_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                ..
            }
            | SubjectVerbActionAst::CreateTokenCopyFromSource {
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
            SubjectVerbActionAst::CreateTokenWithMods {
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
