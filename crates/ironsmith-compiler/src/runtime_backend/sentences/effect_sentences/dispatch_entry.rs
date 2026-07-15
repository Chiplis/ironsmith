use self::subject_verb_followups::{
    PostParseFollowupResult, PreParseFollowupResult, run_post_parse_followup_registry,
    run_pre_parse_followup_registry,
};
use super::super::activation_and_restrictions::{
    parse_mana_usage_restriction_sentence_lexed, parse_single_word_keyword_action,
};
use super::super::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut, try_for_each_nested_effects_mut,
};
use super::super::grammar::effects as effect_grammar;
use super::super::grammar::primitives::{self as grammar};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed,
};
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, contains_token_word_sequence, split_lexed_sentences,
    token_slice_at_is,
};
use super::super::token_primitives::{LeadingMayActor, find_window_by};
use super::super::util::{span_from_tokens, trim_commas};
use super::bundle_rules::{
    parse_same_sentence_copy_and_may_cast_copy, parse_typed_effect_bundle_lexed,
};
use super::consult_family;
use super::divvy::try_parse_divvy_sentence_sequence;
use super::looked_cards_family;
use super::sentence_helpers::*;
use super::sequence_rules::{subject_verb_sequence_route, try_parse_subject_verb_sequence_rule};
use super::{
    SubjectVerbPrimitiveClause, parse_effect_sentence_lexed, parse_restriction_duration,
    parse_token_copy_modifier_sentence, trim_edge_punctuation, try_build_unless,
};
use crate::cards::builders::{
    CardTextError, CarryContext, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate,
    InsteadSemantics, KeywordAction, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TokenCopyFollowup,
    ZoneReplacementDurationAst,
};
use crate::effect::{EventValueSpec, Until, Value};
use crate::parse_trace;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, SourceReferenceSurface, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;
use std::cell::OnceCell;

mod subject_verb_followups;

const COUNTERED_THIS_WAY_PHRASE: &[&str] = &["countered", "this", "way"];
const INSTEAD_OF_PHRASE: &[&str] = &["instead", "of"];
const GRAVEYARD_PHRASE: &[&str] = &["graveyard"];
const EXILE_PHRASE: &[&str] = &["exile"];
const HAND_PHRASE: &[&str] = &["hand"];
const LIBRARY_PHRASE: &[&str] = &["library"];
const WOULD_DIE_THIS_TURN_PHRASE: &[&str] = &["would", "die", "this", "turn"];
const A_CREATURE_WOULD_DIE_THIS_TURN_PHRASE: &[&str] =
    &["a", "creature", "would", "die", "this", "turn"];
const A_PERMANENT_YOU_CONTROL_WOULD_BE_PUT_PHRASE: &[&str] = &[
    "a",
    "permanent",
    "you",
    "control",
    "would",
    "be",
    "put",
    "into",
    "a",
    "graveyard",
    "from",
    "the",
    "battlefield",
    "this",
    "turn",
];
const WOULD_LEAVE_THE_BATTLEFIELD_PHRASE: &[&str] = &["would", "leave", "the", "battlefield"];
const DEALT_DAMAGE_THIS_WAY_PHRASE: &[&str] = &["dealt", "damage", "this", "way"];
const DEALT_DAMAGE_BY_PHRASE: &[&str] = &["dealt", "damage", "by"];
const PERMANENT_DEALT_DAMAGE_PHRASE: &[&str] = &["permanent", "dealt", "damage"];
const CREATURE_OPPONENT_CONTROLS_WOULD_DIE_PHRASE: &[&str] = &[
    "creature", "an", "opponent", "controls", "would", "die", "this", "turn",
];
const THAT_CREATURE_WOULD_DIE_THIS_TURN_PHRASE: &[&str] =
    &["that", "creature", "would", "die", "this", "turn"];
const WOULD_BE_PUT_INTO_PHRASE: &[&str] = &["would", "be", "put", "into"];
const THAT_SPELL_WOULD_PHRASE: &[&str] = &["that", "spell", "would"];
const INSTEAD_PHRASE: &[&str] = &["instead"];
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
            | SubjectVerbActionAst::SetCardTypes {
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
            | SubjectVerbActionAst::GrantAbilitiesAll {
                duration: effect_duration,
                ..
            }
            | SubjectVerbActionAst::GrantAbilitiesChoiceAll {
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
    if find_window_by(&words, 2, |window| {
        matches!(
            window,
            ["and", "become" | "becomes"] | ["and", "attacks" | "blocks"]
        )
    })
    .is_some()
    {
        return false;
    }
    words
        .iter()
        .any(|word| matches!(*word, "become" | "becomes"))
}

const OTHERWISE_WORD: &str = "otherwise";
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
    if !effect_grammar::dispatch_entry_shapes::is_that_object_power_damage_to_source_tokens(tokens)
    {
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
    if !effect_grammar::dispatch_entry_shapes::has_to_that_player_damage_target_tokens(tokens) {
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
    let Some(counter_constraint) =
        effect_grammar::dispatch_entry_shapes::parse_trailing_counter_constraint_tokens(tokens)
    else {
        return;
    };

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

pub(super) fn parse_top_of_your_library_count(
    tokens: &[OwnedLexToken],
    expected_action: effect_grammar::dispatch_entry_shapes::TopLibraryAction,
) -> Option<u32> {
    let shape = effect_grammar::dispatch_entry_shapes::parse_top_library_count_tokens(tokens)?;
    (shape.action == expected_action).then_some(shape.count)
}

pub(super) fn parse_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<ConsultSentenceParts>, CardTextError> {
    consult_family::parse_consult_traversal_sentence(tokens)
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

fn preserve_leading_result_prefix_for_sequence(
    sentence_tokens: &[OwnedLexToken],
    effects: &mut Vec<EffectAst>,
) {
    let Some(prefix) = split_leading_result_prefix_lexed(sentence_tokens) else {
        return;
    };

    match (prefix.kind, effects.as_mut_slice()) {
        (
            LeadingResultPrefixKind::If,
            [
                EffectAst::IfResult {
                    predicate,
                    effects: nested,
                },
            ],
        ) if predicate == &prefix.predicate => {
            super::preserve_result_conjunction_body_lexed(prefix.trailing_tokens, nested);
            return;
        }
        (
            LeadingResultPrefixKind::When,
            [
                EffectAst::WhenResult {
                    predicate,
                    effects: nested,
                },
            ],
        ) if predicate == &prefix.predicate => {
            super::preserve_result_conjunction_body_lexed(prefix.trailing_tokens, nested);
            return;
        }
        _ => {}
    }

    let mut nested = std::mem::take(effects);
    super::preserve_result_conjunction_body_lexed(prefix.trailing_tokens, &mut nested);
    effects.push(match prefix.kind {
        LeadingResultPrefixKind::If => EffectAst::IfResult {
            predicate: prefix.predicate,
            effects: nested,
        },
        LeadingResultPrefixKind::When => EffectAst::WhenResult {
            predicate: prefix.predicate,
            effects: nested,
        },
    });
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

fn future_zone_replacement_counters(
    tokens: &[OwnedLexToken],
) -> Vec<(crate::object::CounterType, u32)> {
    effect_grammar::dispatch_entry_shapes::parse_future_zone_counter_tokens(tokens)
        .map(|shape| vec![(shape.counter_type, shape.count)])
        .unwrap_or_default()
}

fn future_zone_replacement_from_sentence_tokens(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let target = || TargetAst::Tagged(TagKey::from(IT_TAG), None);
    if tokens.first().is_some_and(|token| token.is_word("if"))
        && sentence_contains(tokens, WOULD_LEAVE_THE_BATTLEFIELD_PHRASE)
        && sentence_contains(tokens, EXILE_PHRASE)
        && sentence_contains(tokens, INSTEAD_PHRASE)
    {
        return Some(EffectAst::subject_verb_register_zone_replacement(
            target(),
            Some(Zone::Battlefield),
            None,
            Zone::Exile,
            ZoneReplacementDurationAst::Persistent,
        ));
    }

    if sentence_contains(tokens, COUNTERED_THIS_WAY_PHRASE)
        && sentence_contains(tokens, INSTEAD_OF_PHRASE)
        && sentence_contains(tokens, GRAVEYARD_PHRASE)
        && sentence_contains(tokens, EXILE_PHRASE)
    {
        let counters = future_zone_replacement_counters(tokens);
        if !counters.is_empty() {
            return Some(
                EffectAst::subject_verb_register_zone_replacement_with_counters(
                    target(),
                    Some(Zone::Stack),
                    Some(Zone::Graveyard),
                    Zone::Exile,
                    ZoneReplacementDurationAst::OneShot,
                    counters,
                ),
            );
        }
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
        let placement =
            effect_grammar::dispatch_entry_shapes::parse_countered_spell_library_placement_tokens(
                tokens,
            )?;
        return Some(
            EffectAst::subject_verb_register_zone_replacement_with_library_placement(
                target(),
                Some(Zone::Stack),
                Some(Zone::Graveyard),
                Zone::Library,
                placement,
                ZoneReplacementDurationAst::OneShot,
            ),
        );
    }

    if sentence_contains(tokens, WOULD_DIE_THIS_TURN_PHRASE)
        && sentence_contains(tokens, EXILE_PHRASE)
    {
        if tokens.first().is_some_and(|token| token.is_word("if"))
            && sentence_contains(tokens, A_CREATURE_WOULD_DIE_THIS_TURN_PHRASE)
        {
            return Some(EffectAst::subject_verb_register_future_zone_replacement(
                ObjectFilter::creature(),
                Some(Zone::Battlefield),
                Some(Zone::Graveyard),
                Zone::Exile,
                ZoneReplacementDurationAst::UntilEndOfTurn,
                crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
                false,
            ));
        }

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

        let target = if sentence_contains(tokens, CREATURE_OPPONENT_CONTROLS_WOULD_DIE_PHRASE) {
            TargetAst::Object(
                ObjectFilter::creature()
                    .controlled_by(PlayerFilter::Opponent)
                    .match_tagged(IT_TAG, TaggedOpbjectRelation::IsTaggedObject),
                None,
                None,
            )
        } else {
            target()
        };
        return Some(EffectAst::subject_verb_register_zone_replacement(
            target,
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
        ));
    }

    if sentence_contains(tokens, THAT_SPELL_WOULD_PHRASE)
        && sentence_contains(tokens, WOULD_BE_PUT_INTO_PHRASE)
        && sentence_contains(tokens, GRAVEYARD_PHRASE)
        && sentence_contains(tokens, EXILE_PHRASE)
        && sentence_contains(tokens, INSTEAD_PHRASE)
    {
        return Some(EffectAst::subject_verb_register_zone_replacement(
            target(),
            Some(Zone::Stack),
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
        if tokens.first().is_some_and(|token| token.is_word("if"))
            && sentence_contains(tokens, A_PERMANENT_YOU_CONTROL_WOULD_BE_PUT_PHRASE)
        {
            return Some(EffectAst::subject_verb_register_future_zone_replacement(
                ObjectFilter::permanent().controlled_by(PlayerFilter::You),
                Some(Zone::Battlefield),
                Some(Zone::Graveyard),
                Zone::Exile,
                ZoneReplacementDurationAst::UntilEndOfTurn,
                crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
                false,
            ));
        }

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
            crate::cards::builders::FutureZoneReplacementCausePolicyAst::ChangedObjectIsCause,
            false,
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

fn damage_regeneration_exile_followup_from_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = effect_grammar::followup_shapes::parse_damage_regeneration_exile_followup(tokens)?;
    let replacement = future_zone_replacement_from_sentence_tokens(tokens)?;
    let tagged_target = TagKey::from(IT_TAG);
    let regeneration_filter = ObjectFilter::creature()
        .match_tagged(tagged_target.clone(), TaggedOpbjectRelation::IsTaggedObject);
    let cant_regenerate = EffectAst::subject_verb_cant(
        crate::effect::Restriction::be_regenerated(regeneration_filter),
        Until::EndOfTurn,
        None,
    );
    let predicate = match shape.gate {
        effect_grammar::followup_shapes::DamageRegenerationExileGate::DamagedObjectIsCreature => {
            PredicateAst::TaggedMatches(tagged_target, ObjectFilter::creature())
        }
        effect_grammar::followup_shapes::DamageRegenerationExileGate::ThisSpellWasKicked => {
            PredicateAst::ThisSpellWasKicked
        }
    };

    Some(vec![EffectAst::Conditional {
        predicate,
        if_true: vec![cant_regenerate, replacement],
        if_false: Vec::new(),
    }])
}

fn secondary_fight_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Fight { creature2, .. },
        ..
    }) = effect
    {
        return Some(creature2.clone());
    }

    let mut found = None;
    for_each_nested_effects(effect, false, |nested| {
        if found.is_none() {
            found = nested
                .iter()
                .rev()
                .find_map(secondary_fight_target_from_effect);
        }
    });
    found
}

fn rebind_fight_death_replacement_target(
    replacement: &mut EffectAst,
    previous_effect: Option<&EffectAst>,
    sentence_tokens: &[OwnedLexToken],
) {
    if !sentence_contains(sentence_tokens, WOULD_DIE_THIS_TURN_PHRASE)
        || !sentence_contains(sentence_tokens, EXILE_PHRASE)
        || (!sentence_contains(sentence_tokens, THAT_CREATURE_WOULD_DIE_THIS_TURN_PHRASE)
            && !sentence_contains(sentence_tokens, CREATURE_OPPONENT_CONTROLS_WOULD_DIE_PHRASE))
    {
        return;
    }
    let Some(fight_target) = previous_effect.and_then(secondary_fight_target_from_effect) else {
        return;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::RegisterZoneReplacement {
                target,
                from_zone: Some(Zone::Battlefield),
                to_zone: Some(Zone::Graveyard),
                replacement_zone: Zone::Exile,
                duration: ZoneReplacementDurationAst::OneShot,
                ..
            },
        ..
    }) = replacement
    else {
        return;
    };
    *target = fight_target;
}

fn append_replacement_to_trailing_reflexive_result(
    effects: &mut [EffectAst],
    replacement: EffectAst,
) -> Result<(), EffectAst> {
    let Some(EffectAst::WhenResult {
        effects: reflexive_effects,
        ..
    }) = effects.last_mut()
    else {
        return Err(replacement);
    };
    reflexive_effects.push(replacement);
    Ok(())
}

fn maybe_rewrite_future_zone_replacement_sentence(
    sentence_effects: &mut Vec<EffectAst>,
    sentence_tokens: &[OwnedLexToken],
) {
    if !matches!(
        classify_instead_followup_tokens(sentence_tokens),
        InsteadSemantics::FutureReplacement
    ) {
        return;
    }

    let Some(replacement) = future_zone_replacement_from_sentence_tokens(sentence_tokens) else {
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
    if !grammar::has_phrase(sentence_tokens, &["then", "lose", "that", "much", "life"]) {
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

fn maybe_append_reexile_returned_objects(
    sentence_effects: &mut Vec<EffectAst>,
    sentence_tokens: &[OwnedLexToken],
) {
    if !grammar::has_phrase(sentence_tokens, &["then", "exile", "them", "again"]) {
        return;
    }

    if let [EffectAst::IfResult { effects, .. }] = sentence_effects.as_mut_slice() {
        append_reexile_returned_objects_if_missing(effects);
        return;
    }
    append_reexile_returned_objects_if_missing(sentence_effects);
}

fn append_reexile_returned_objects_if_missing(effects: &mut Vec<EffectAst>) {
    let already_exiles = effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Exile { .. } | SubjectVerbActionAst::ExileAll { .. },
                ..
            })
        )
    });
    if already_exiles {
        return;
    }

    effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(TagKey::from(IT_TAG), None),
        false,
    ));
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
    if !grammar::has_phrase(tokens, &["that", "player", "gains", "control", "of"])
        || !grammar::has_phrase(tokens, &["if", "they", "do"])
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

pub(crate) fn with_where_x_surface_hints(
    mut value: Value,
    binding_tokens: &[OwnedLexToken],
) -> Value {
    let words = crate::runtime_backend::token_word_refs(binding_tokens);
    let explicit_count_surface = words.contains(&"number")
        && words.iter().any(|word| {
            matches!(
                *word,
                "card"
                    | "cards"
                    | "counter"
                    | "counters"
                    | "creature"
                    | "creatures"
                    | "permanent"
                    | "permanents"
            )
        });
    let counts_objects = matches!(
        value.unhinted(),
        Value::Count(_)
            | Value::CountScaled(_, _)
            | Value::EffectMetric {
                metric: ironsmith_core::EffectMetric::Count
                    | ironsmith_core::EffectMetric::ChosenCount
                    | ironsmith_core::EffectMetric::AffectedCount,
                ..
            }
            | Value::PendingEffectMetric {
                metric: ironsmith_core::EffectMetric::Count
                    | ironsmith_core::EffectMetric::ChosenCount
                    | ironsmith_core::EffectMetric::AffectedCount,
                ..
            }
    ) || (explicit_count_surface
        && matches!(
            value.unhinted(),
            Value::EffectValue(_)
                | Value::EffectValueOffset(_, _)
                | Value::PendingEffectMetric { .. }
                | Value::PendingEffectMetricOffset { .. }
        ));
    value = value.with_surface_hint(ValueSurfaceHint::WhereXIs);
    let mentions_energy = binding_tokens.iter().any(|token| {
        token.as_word() == Some("e")
            || (token.kind == TokenKind::ManaGroup && token.mana_group_inner() == Some("e"))
    });
    if mentions_energy
        && words.contains(&"paid")
        && words.contains(&"this")
        && words.contains(&"way")
    {
        value = value.with_surface_hint(ValueSurfaceHint::EnergyPaidThisWay);
    } else if words.contains(&"mana")
        && words.contains(&"value")
        && words.contains(&"permanent")
        && words.contains(&"exiled")
        && words.contains(&"way")
    {
        value = value.with_surface_hint(ValueSurfaceHint::ManaValueOfPermanentExiledThisWay);
    } else if words.contains(&"result") {
        value = value.with_surface_hint(ValueSurfaceHint::PriorEffectResult);
    } else if counts_objects && words.contains(&"revealed") && words.contains(&"way") {
        value = value.with_surface_hint(ValueSurfaceHint::CardsRevealedThisWay);
    } else if counts_objects && words.contains(&"exiled") && words.contains(&"way") {
        value = value.with_surface_hint(ValueSurfaceHint::CardsExiledThisWay);
    } else if counts_objects && words.contains(&"discarded") && words.contains(&"way") {
        value = value.with_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay);
    } else if counts_objects && words.contains(&"sacrificed") && words.contains(&"way") {
        value = value.with_surface_hint(ValueSurfaceHint::PermanentsSacrificedThisWay);
    } else if counts_objects
        && words.contains(&"counters")
        && words.contains(&"removed")
        && words.contains(&"way")
    {
        value = value.with_surface_hint(ValueSurfaceHint::CountersRemovedThisWay);
    }
    value
}

fn parse_effect_sentences_from_sentence_inputs(
    sentences: Vec<SentenceInput>,
) -> Result<Vec<EffectAst>, CardTextError> {
    fn scope_partitioned_prior_metric_followup(
        previous_effects: &[EffectAst],
        sentence_tokens: &[OwnedLexToken],
        sentence_effects: &mut Vec<EffectAst>,
    ) {
        fn pending_prior_metric_query_mut(
            value: &mut Value,
        ) -> Option<&mut ironsmith_core::PriorEffectMetricQuery> {
            match value {
                Value::PendingPriorEffectMetric(query) => Some(query),
                Value::SurfaceHinted { value, .. }
                | Value::Scaled(value, _)
                | Value::DividedRoundedDown(value, _)
                | Value::HalfRoundedDown(value) => pending_prior_metric_query_mut(value),
                _ => None,
            }
        }

        if !contains_token_word_sequence(sentence_tokens, &["that", "player"])
            || !matches!(
                previous_effects.last(),
                Some(EffectAst::ForEachPlayer { .. })
            )
        {
            return;
        }
        let [EffectAst::RepeatEffects { count, .. }] = sentence_effects.as_mut_slice() else {
            return;
        };
        let Some(query) = pending_prior_metric_query_mut(count) else {
            return;
        };
        if query.source != ironsmith_core::EffectMetricSource::AffectedObjects
            || query.metric != ironsmith_core::EffectMetric::Count
            || query.action.is_none()
            || query.player.is_some()
        {
            return;
        }

        query.player = Some(PlayerFilter::IteratedPlayer);
        let repeat = sentence_effects
            .pop()
            .expect("single repeat effect was matched above");
        sentence_effects.push(EffectAst::ForEachPlayer {
            effects: vec![repeat],
        });
        parse_trace::event(
            "partitioned prior-effect repeat scoped to the preceding each-player result",
        );
    }

    fn annotate_counter_followup_surface(effects: &mut [EffectAst], hint: ValueSurfaceHint) {
        fn annotate(effect: &mut EffectAst, hint: ValueSurfaceHint) {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutCounters { count, .. },
                ..
            }) = effect
            {
                *count = count.clone().with_surface_hint(hint);
            }
            for_each_nested_effects_mut(effect, true, |nested| {
                for child in nested {
                    annotate(child, hint);
                }
            });
        }

        for effect in effects {
            annotate(effect, hint);
        }
    }

    fn preserve_plural_counter_antecedent(
        sentence_tokens: &[OwnedLexToken],
        effects: &mut Vec<EffectAst>,
    ) {
        const PLURAL_ANTECEDENT_ALIAS: &str = "plural_antecedent_cards";

        if !contains_token_word_sequence(sentence_tokens, &["among", "those", "cards"]) {
            return;
        }

        fn bind_aggregate_filter(value: &mut Value, alias: &TagKey) -> bool {
            let filter = match value {
                Value::SurfaceHinted { value, .. } => return bind_aggregate_filter(value, alias),
                Value::GreatestPower(filter)
                | Value::GreatestToughness(filter)
                | Value::GreatestManaValue(filter)
                | Value::LeastPower(filter)
                | Value::LeastToughness(filter)
                | Value::LeastManaValue(filter)
                | Value::TotalPower(filter)
                | Value::TotalToughness(filter)
                | Value::TotalManaValue(filter)
                | Value::BasicLandTypesAmong(filter)
                | Value::CreatureTypesAmong(filter)
                | Value::CardTypesAmong(filter)
                | Value::ColorsAmong(filter) => filter,
                _ => return false,
            };

            let mut rebound = false;
            for constraint in &mut filter.tagged_constraints {
                if constraint.tag.as_str() == IT_TAG {
                    constraint.tag = alias.clone();
                    rebound = true;
                }
            }
            rebound
        }

        fn bind_effect(effect: &mut EffectAst, alias: &TagKey) -> bool {
            let mut rebound = false;
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutCounters { count, .. },
                ..
            }) = effect
            {
                rebound |= bind_aggregate_filter(count, alias);
            }
            for_each_nested_effects_mut(effect, true, |nested| {
                for child in nested {
                    rebound |= bind_effect(child, alias);
                }
            });
            rebound
        }

        let alias = TagKey::from(PLURAL_ANTECEDENT_ALIAS);
        let rebound = effects.iter_mut().fold(false, |rebound, effect| {
            rebound | bind_effect(effect, &alias)
        });
        if rebound {
            // Capture the plural discourse set before a nested create/return
            // action replaces the ordinary singular `it` antecedent.
            effects.insert(0, EffectAst::SnapshotLastObjectTag { into: alias });
        }
    }

    fn where_x_value_from_tokens(tokens: &[OwnedLexToken]) -> Option<Value> {
        let binding_tokens =
            effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(tokens)
                .map(|shape| shape.binding_tokens)
                .or_else(|| {
                    effect_grammar::sentence_predicate_shapes::parse_where_x_sentence_tokens(tokens)
                        .map(|shape| shape.where_tokens)
                })?;
        if let Some(value) = crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_value_binding(binding_tokens)
        {
            return Some(with_where_x_surface_hints(value, tokens));
        }
        if let Some((_, value)) =
            effect_grammar::sentence_predicate_shapes::parse_where_x_value_shape_tokens(
                binding_tokens,
                false,
            )
            .and_then(super::dispatch_inner::lower_where_x_shape)
        {
            return Some(with_where_x_surface_hints(value, tokens));
        }
        parse_value_binding_clause(binding_tokens)
            .map(|value| with_where_x_surface_hints(value, tokens))
    }

    fn parse_leading_flip_result_sentence(
        tokens: &[OwnedLexToken],
    ) -> Result<Option<Vec<EffectAst>>, CardTextError> {
        let Some((predicate, rest_tokens)) =
            effect_grammar::dispatch_entry_shapes::parse_flip_result_shape_tokens(tokens)
        else {
            return Ok(None);
        };
        let effects = parse_effect_sentences_lexed(rest_tokens)?;
        Ok(Some(vec![EffectAst::IfResult { predicate, effects }]))
    }

    fn parse_tagged_characteristics_and_keyword_sentence(
        tokens: &[OwnedLexToken],
    ) -> Result<Option<Vec<EffectAst>>, CardTextError> {
        let Some(shape) =
            effect_grammar::dispatch_entry_shapes::parse_tagged_characteristics_shape_tokens(
                tokens,
            )
        else {
            return Ok(None);
        };
        let Some(keyword) = parse_single_word_keyword_action(shape.ability_word) else {
            return Ok(None);
        };
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
        let mut effects = Vec::new();
        if !shape.colors.is_empty() {
            effects.push(EffectAst::subject_verb_add_colors(
                target.clone(),
                shape.colors,
                Until::Forever,
            ));
        }
        if !shape.subtypes.is_empty() {
            effects.push(EffectAst::subject_verb_add_subtypes(
                target.clone(),
                shape.subtypes,
                Until::Forever,
            ));
        }
        effects.push(EffectAst::subject_verb_grant_abilities_to_target(
            target,
            vec![GrantedAbilityAst::from(keyword)],
            Until::Forever,
        ));
        Ok(Some(effects))
    }

    fn parse_tagged_exact_type_with_quoted_ability_sentence(
        tokens: &[OwnedLexToken],
    ) -> Result<Option<Vec<EffectAst>>, CardTextError> {
        let Some(shape) = effect_grammar::sentence_predicate_shapes::parse_tagged_exact_type_with_quoted_ability_tokens(tokens)
        else {
            return Ok(None);
        };
        // Slice the quoted payload from the original token stream. This keeps
        // non-word cost tokens such as `{T}` and the comma that follows them;
        // the predicate shape only needs the payload to prove the sentence
        // form and should not be the source of the granted ability's costs.
        let first_quote = tokens
            .iter()
            .position(|token| token.kind == TokenKind::Quote)
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "exact type-setting clause is missing its opening quote".to_string(),
                )
            })?;
        let second_quote = tokens
            .iter()
            .enumerate()
            .skip(first_quote + 1)
            .find_map(|(index, token)| (token.kind == TokenKind::Quote).then_some(index))
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "exact type-setting clause is missing its closing quote".to_string(),
                )
            })?;
        let quoted_ability_tokens = &tokens[first_quote + 1..second_quote];
        let clause_words = crate::runtime_backend::token_word_refs(tokens);
        let (abilities, _) = super::gain_ability::parse_granted_abilities_for_gain_clause(
            quoted_ability_tokens,
            &clause_words,
            false,
        )?;
        if abilities.is_empty() {
            return Err(CardTextError::ParseError(
                "exact type-setting clause has an unsupported quoted ability".to_string(),
            ));
        }

        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
        let mut effects = vec![EffectAst::subject_verb_set_card_types(
            target.clone(),
            shape.card_types,
            Until::Forever,
        )];
        if !shape.subtypes.is_empty() {
            effects.push(EffectAst::subject_verb_add_subtypes(
                target.clone(),
                shape.subtypes,
                Until::Forever,
            ));
        }
        effects.push(EffectAst::subject_verb_grant_abilities_to_target(
            target,
            abilities,
            Until::Forever,
        ));
        Ok(Some(effects))
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
        if let Some(restart) = parse_restart_game_sentence(sentences[sentence_idx].lexed())? {
            effects.push(restart);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        let sentence_text = crate::runtime_backend::token_word_refs(sentence).join(" ");
        let _sentence_scope = parse_trace::scope(format!("effect sentence: \"{}\"", sentence_text));

        let leading_unless_tokens = trim_edge_punctuation(sentence);
        if let Some(split) =
            effect_grammar::parse_leading_unless_clause_split_tokens(&leading_unless_tokens)
        {
            let unless_tokens = trim_edge_punctuation(&leading_unless_tokens[split.condition]);
            let effect_tokens = trim_edge_punctuation(&leading_unless_tokens[split.effect]);
            if !unless_tokens.is_empty() && !effect_tokens.is_empty() {
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

        let direct_for_each_tokens = trim_edge_punctuation(sentence);
        if effect_grammar::dispatch_entry_shapes::is_direct_for_each_who_tokens(
            &direct_for_each_tokens,
        ) {
            if let Some(effect) = parse_for_each_opponent_clause(&direct_for_each_tokens)? {
                effects.push(effect);
                carried_context = None;
                sentence_idx += 1;
                continue;
            }
            if let Some(effect) = parse_for_each_player_clause(&direct_for_each_tokens)? {
                effects.push(effect);
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
            preserve_leading_result_prefix_for_sequence(sentence, &mut matched.effects);
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

        if let Some(mut exact_type_effects) =
            parse_tagged_exact_type_with_quoted_ability_sentence(sentence)?
        {
            // "If you do, return that card ... . It's ..." keeps the
            // characteristic-setting sentence inside the successful-result
            // branch and binds "it" to the object returned by that branch.
            if let Some(EffectAst::IfResult {
                effects: branch, ..
            }) = effects.last_mut()
            {
                branch.append(&mut exact_type_effects);
            } else {
                effects.append(&mut exact_type_effects);
            }
            carried_context = None;
            sentence_idx += 1;
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

        if let Some(action) =
            effect_grammar::dispatch_entry_shapes::parse_direct_atomic_action_tokens(
                &sentence_tokens,
            )
        {
            effects.push(match action {
                effect_grammar::dispatch_entry_shapes::DirectAtomicActionShape::Learn => {
                    EffectAst::subject_verb_learn(PlayerAst::You)
                }
                effect_grammar::dispatch_entry_shapes::DirectAtomicActionShape::TimeTravel => {
                    time_travel_effect_ast()
                }
            });
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        if let Some(flip_result_effects) = parse_leading_flip_result_sentence(&sentence_tokens)? {
            effects.extend(flip_result_effects);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        if let Some(characteristic_effects) =
            parse_tagged_characteristics_and_keyword_sentence(&sentence_tokens)?
        {
            effects.extend(characteristic_effects);
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

        if let Some(compound_rider) =
            damage_regeneration_exile_followup_from_sentence_tokens(&sentence_tokens)
        {
            effects.extend(compound_rider);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        if let Some(mut replacement) =
            future_zone_replacement_from_sentence_tokens(&sentence_tokens)
        {
            rebind_fight_death_replacement_target(
                &mut replacement,
                effects.last(),
                &sentence_tokens,
            );
            if let Err(replacement) =
                append_replacement_to_trailing_reflexive_result(&mut effects, replacement)
            {
                effects.push(replacement);
            }
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
        preserve_plural_counter_antecedent(&parse_plan.tokens, &mut sentence_effects);
        let sentence_words = crate::runtime_backend::token_word_refs(&parse_plan.tokens);
        if sentence_words.contains(&"then") {
            annotate_counter_followup_surface(
                &mut sentence_effects,
                ValueSurfaceHint::CounterFollowupThen,
            );
        } else if sentence_idx > 0 && sentence_words.first().copied() == Some("put") {
            annotate_counter_followup_surface(
                &mut sentence_effects,
                ValueSurfaceHint::CounterFollowupSeparateSentence,
            );
        }
        // `sentence_tokens` may intentionally have had inline token rules
        // stripped before outer subject/verb dispatch. The resulting create
        // action is now known, so attach every quoted token ability from the
        // untouched source sentence under the created token's own identity.
        super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
            &mut sentence_effects,
            sentence,
        );
        if let Some(predicate) = parse_plan.wrap_if_result {
            sentence_effects = vec![EffectAst::IfResult {
                predicate,
                effects: sentence_effects,
            }];
            carried_context = None;
        }
        if let Some(where_value) = sentence_where_x.as_ref() {
            replace_unbound_x_in_effects_anywhere(
                &mut sentence_effects,
                where_value,
                &crate::runtime_backend::token_word_refs(&parse_plan.tokens).join(" "),
            )?;
        } else if let Some(where_value) = carried_where_x.as_ref() {
            replace_unbound_x_in_effects_anywhere(
                &mut sentence_effects,
                where_value,
                &crate::runtime_backend::token_word_refs(&parse_plan.tokens).join(" "),
            )?;
        }
        maybe_append_trailing_that_much_life_loss(&mut sentence_effects, &parse_plan.tokens);
        maybe_append_reexile_returned_objects(&mut sentence_effects, &parse_plan.tokens);
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
            if matches!(&*predicate, IfResultPredicate::Did)
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

        scope_partitioned_prior_metric_followup(
            &effects,
            &parse_plan.tokens,
            &mut sentence_effects,
        );

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
    effect_grammar::dispatch_entry_shapes::is_outside_game_art_rating_tokens(tokens)
}

pub(crate) fn parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    stacker::maybe_grow(8 * 1024 * 1024, 16 * 1024 * 1024, || {
        let mut effects = parse_effect_sentences_lexed_inner(tokens)?;
        transport_optional_search_partition_followup(&mut effects);
        transport_coin_flip_outcomes_into_owner(&mut effects);
        preserve_linked_target_fanout_group(tokens, &mut effects);
        preserve_tapped_this_way_group_for_later_distribution(tokens, &mut effects);
        let instead_shape = effect_grammar::parse_instead_followup_shape_tokens(tokens);
        if instead_shape.conditional_intro
            && instead_shape.semantics == InsteadSemantics::SelfReplacement
        {
            for effect in &mut effects {
                if let EffectAst::SelfReplacement {
                    attach_to_previous_ability,
                    ..
                } = effect
                {
                    *attach_to_previous_ability = true;
                }
            }
        }
        Ok(effects)
    })
}

fn contains_tagged_battlefield_partition(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TagMatchingObjects { tag, .. },
            ..
        }) => tag.as_str().starts_with("partition_pool"),
        EffectAst::Sequence { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. } => {
            effects.iter().any(contains_tagged_battlefield_partition)
        }
        _ => false,
    }
}

fn append_effects_to_optional_search(
    effects: &mut [EffectAst],
    mut followups: Vec<EffectAst>,
) -> bool {
    let [optional] = effects else {
        return false;
    };
    let body = match optional {
        EffectAst::May { effects } | EffectAst::MayByPlayer { effects, .. } => effects,
        _ => return false,
    };
    body.append(&mut followups);
    true
}

/// Keep an optional per-player search, a later partition of the searched
/// collection, and the corresponding "player who searched" shuffle in the
/// same iteration. Lowering them as sibling effects loses both the searcher's
/// identity and the optional-choice scope, and can make the shuffle depend on
/// whether a later move changed the game state rather than whether the player
/// chose to search.
fn transport_optional_search_partition_followup(effects: &mut Vec<EffectAst>) {
    let mut index = 0;
    while index + 2 < effects.len() {
        let mut partition_effects = match effects.get(index + 1) {
            Some(EffectAst::ForEachOpponent { effects })
                if effects.iter().any(contains_tagged_battlefield_partition) =>
            {
                effects.clone()
            }
            _ => {
                index += 1;
                continue;
            }
        };
        if let [EffectAst::Sequence { effects: nested }] = partition_effects.as_slice() {
            partition_effects = nested.clone();
        }
        let shuffle_effects = match effects.get(index + 2) {
            Some(EffectAst::ForEachPlayerDid {
                effects,
                predicate: None,
                result_predicate: IfResultPredicate::SearchedLibrary,
            }) => effects.clone(),
            _ => {
                index += 1;
                continue;
            }
        };
        let Some(EffectAst::ForEachOpponent {
            effects: search_effects,
        }) = effects.get_mut(index)
        else {
            index += 1;
            continue;
        };

        let mut followups = partition_effects;
        followups.extend(shuffle_effects);
        if !append_effects_to_optional_search(search_effects, followups) {
            index += 1;
            continue;
        }
        effects.drain(index + 1..=index + 2);
        index += 1;
    }
}

fn is_direct_coin_flip(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::FlipCoin,
            ..
        })
    )
}

fn coin_flip_owner_body_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
    let effects = match effect {
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::AnyPlayerMay { effects }
        | EffectAst::ForEachObject { effects, .. } => effects,
        _ => return None,
    };
    if !effects.last().is_some_and(is_direct_coin_flip) {
        return None;
    }
    Some(effects)
}

fn is_coin_flip_outcome(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did | IfResultPredicate::DidNot,
            ..
        }
    )
}

/// Keep outcome clauses with the coin flip that establishes their result.
///
/// Sentence parsing initially produces siblings for constructs such as
/// "you may flip ... If you win ..." and "for each creature, flip ... If
/// you win ...". Leaving those siblings in place makes lowering bind the
/// result to the `May`/`ForEachObject` wrapper instead of to the flip itself;
/// it also lets declining an optional flip masquerade as losing it. Moving
/// only contiguous win/lose branches into an owner whose final action is a
/// coin flip preserves both the optional and per-iteration scopes.
fn transport_coin_flip_outcomes_into_owner(effects: &mut Vec<EffectAst>) {
    let mut owner_index = 0;
    while owner_index < effects.len() {
        let owns_coin_flip = coin_flip_owner_body_mut(&mut effects[owner_index]).is_some();
        if owns_coin_flip {
            let mut end = owner_index + 1;
            while effects.get(end).is_some_and(is_coin_flip_outcome) {
                end += 1;
            }
            if end > owner_index + 1 {
                let outcomes = effects.drain(owner_index + 1..end).collect::<Vec<_>>();
                coin_flip_owner_body_mut(&mut effects[owner_index])
                    .expect("coin-flip owner was matched before draining outcomes")
                    .extend(outcomes);
            }
        }

        owner_index += 1;
    }
}

fn direct_all_object_filter(effect: &EffectAst) -> Option<&ObjectFilter> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return None;
    };
    match action {
        SubjectVerbActionAst::DestroyAll { filter, .. }
        | SubjectVerbActionAst::ExileAll { filter, .. }
        | SubjectVerbActionAst::ReturnAllToHand { filter, .. }
        | SubjectVerbActionAst::UntapAll { filter }
        | SubjectVerbActionAst::PumpAll { filter, .. }
        | SubjectVerbActionAst::GrantAbilitiesAll { filter, .. } => Some(filter),
        _ => None,
    }
}

fn direct_all_object_filter_mut(effect: &mut EffectAst) -> Option<&mut ObjectFilter> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return None;
    };
    match action {
        SubjectVerbActionAst::DestroyAll { filter, .. }
        | SubjectVerbActionAst::ExileAll { filter, .. }
        | SubjectVerbActionAst::ReturnAllToHand { filter, .. }
        | SubjectVerbActionAst::UntapAll { filter }
        | SubjectVerbActionAst::PumpAll { filter, .. }
        | SubjectVerbActionAst::GrantAbilitiesAll { filter, .. } => Some(filter),
        _ => None,
    }
}

fn filter_has_linked_it_constraint(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == IT_TAG
            && matches!(
                constraint.relation,
                TaggedOpbjectRelation::SameNameAsTagged
                    | TaggedOpbjectRelation::SharesColorWithTagged
            )
    })
}

fn filter_has_it_reference(filter: &ObjectFilter) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
}

/// Keep a compound target-plus-linked-set subject available to later
/// demonstratives. The individual fanout action must still exclude the target,
/// while "those creatures/cards" refers to the union of both parts.
fn preserve_linked_target_fanout_group(tokens: &[OwnedLexToken], effects: &mut Vec<EffectAst>) {
    for effect in effects.iter_mut() {
        if let EffectAst::Sequence { effects: nested }
        | EffectAst::Coordinated {
            effects: nested, ..
        } = effect
        {
            preserve_linked_target_fanout_group(tokens, nested);
        }
    }
    if effects.len() < 3 {
        return;
    }

    let words = crate::runtime_backend::token_word_refs(tokens);
    let has_trailing_that_name = words
        .windows(3)
        .any(|window| window == ["with", "that", "name"]);

    for first_idx in 0..effects.len().saturating_sub(2) {
        let second_idx = first_idx + 1;
        let Some(linked_filter) = direct_all_object_filter(&effects[second_idx]) else {
            continue;
        };
        let excludes_primary = linked_filter.other
            || linked_filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            });
        if !filter_has_linked_it_constraint(linked_filter) || !excludes_primary {
            continue;
        }

        // Three-part same-name lists may spell the final reference as "with
        // that name". Preserve that structured relation instead of leaving the
        // last set as an unrestricted all-permanents action.
        if has_trailing_that_name
            && let Some(trailing_filter) = effects
                .get_mut(second_idx + 1)
                .and_then(direct_all_object_filter_mut)
            && !filter_has_it_reference(trailing_filter)
        {
            trailing_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
        }

        if !effects[second_idx + 1..]
            .iter()
            .filter_map(direct_all_object_filter)
            .any(filter_has_it_reference)
        {
            continue;
        }

        let primary_alias = TagKey::from(format!("linked_fanout_primary_{first_idx}"));
        let group_alias = TagKey::from(format!("linked_fanout_group_{first_idx}"));

        // Give the explicit target a real runtime tag before the linked
        // fanout is lowered. A lowering-only snapshot cannot safely back
        // player references or later filters because no runtime effect binds
        // that alias. `TagAffected` both preserves the affected target set at
        // resolution and makes the alias the current object reference for the
        // fanout that follows.
        let primary = effects.remove(first_idx);
        effects.insert(
            first_idx,
            EffectAst::TagAffected {
                effect: Box::new(primary),
                tag: primary_alias.clone(),
            },
        );

        let mut related_filter = direct_all_object_filter(&effects[second_idx])
            .expect("linked fanout filter was just matched")
            .clone();

        related_filter
            .tagged_constraints
            .retain(|constraint| constraint.relation != TaggedOpbjectRelation::IsNotTaggedObject);
        related_filter.other = false;
        for constraint in &mut related_filter.tagged_constraints {
            if constraint.tag.as_str() == IT_TAG {
                constraint.tag = primary_alias.clone();
            }
        }

        // The later demonstrative refers to the union of the explicit target
        // and the linked fanout, not merely to objects satisfying the fanout
        // relation. That distinction matters for a colorless Radiance target:
        // it belongs to "those creatures" even though it shares no color with
        // itself. Keep the union structural so execution and rendering agree.
        let mut primary_filter = ObjectFilter::default();
        primary_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: primary_alias,
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        let mut group_filter = related_filter.clone();
        group_filter.other = false;
        group_filter.tagged_constraints.clear();
        group_filter.any_of.clear();
        group_filter.any_of.push(primary_filter);
        group_filter.any_of.push(related_filter);
        let group_zones = group_filter.zone.into_iter().collect::<Vec<_>>();

        // The follow-up demonstrative is the union tag; the primary and
        // fanout actions themselves keep their direct target relationship.
        for effect in &mut effects[second_idx + 1..] {
            if let Some(filter) = direct_all_object_filter_mut(effect) {
                for constraint in &mut filter.tagged_constraints {
                    if constraint.tag.as_str() == IT_TAG {
                        constraint.tag = group_alias.clone();
                    }
                }
            }
        }

        effects.insert(
            second_idx + 1,
            EffectAst::subject_verb_tag_matching_objects(group_filter, group_zones, group_alias),
        );
        return;
    }
}

fn preserve_tapped_this_way_group_for_later_distribution(
    tokens: &[OwnedLexToken],
    effects: &mut Vec<EffectAst>,
) {
    const TAPPED_GROUP_ALIAS: &str = "tapped_this_way_group";

    let words = crate::runtime_backend::token_word_refs(tokens);
    if !words
        .windows(3)
        .any(|window| window == ["tapped", "this", "way"])
        || !words
            .windows(4)
            .any(|window| window == ["any", "number", "of", "those"])
        || !words.iter().any(|word| *word == "divided")
    {
        return;
    }

    let Some(tap_index) = effects.iter().position(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::TapAll { .. },
                ..
            })
        )
    }) else {
        return;
    };
    let Some(distributed_index) = effects.iter().position(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DealDistributedDamage { .. },
                ..
            })
        )
    }) else {
        return;
    };

    let alias = TagKey::from(TAPPED_GROUP_ALIAS);
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::DealDistributedDamage { target, .. },
        ..
    }) = &mut effects[distributed_index]
    {
        fn bind_target(target: &mut TargetAst, alias: &TagKey) {
            match target {
                TargetAst::Object(filter, _, _) => {
                    for constraint in &mut filter.tagged_constraints {
                        if constraint.tag.as_str() == IT_TAG {
                            constraint.tag = alias.clone();
                        }
                    }
                }
                TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
                    bind_target(inner, alias);
                }
                _ => {}
            }
        }
        bind_target(target, &alias);
    }

    effects.insert(
        tap_index + 1,
        EffectAst::SnapshotLastObjectTag { into: alias },
    );
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

    // Quoted emblem abilities may contain their own sentence boundaries and
    // activated-ability colons. Consume the typed whole-sentence shape before
    // generic sentence and subject/verb splitting sees those nested tokens.
    if effect_grammar::emblem_shapes::parse_emblem_payload_tokens(tokens)
        .is_some_and(|shape| shape.requires_whole_sentence_dispatch)
        && let Some(effect) = super::zone_handlers::parse_emblem_action(tokens, None)
    {
        return Ok(vec![effect]);
    }

    if let Some(mut effects) = parse_typed_effect_bundle_lexed(tokens) {
        apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
        maybe_repair_that_player_gain_control_if_do_rewards(&mut effects, tokens);
        parse_trace::event(format!(
            "typed effect bundle -> {}",
            summarize_effects(&effects)
        ));
        return Ok(effects);
    }

    let sentence_segments = split_leading_amass_comma_then_sentences(split_lexed_sentences(tokens));
    let sentences = sentence_segments
        .into_iter()
        .map(SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    let mut effects = parse_effect_sentences_from_sentence_inputs(sentences)?;
    group_this_way_copy_cast_followups(tokens, &mut effects);
    apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
    maybe_repair_that_player_gain_control_if_do_rewards(&mut effects, tokens);
    Ok(effects)
}

fn parse_restart_game_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let word_tokens = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| !token.parser_word_pieces().is_empty())
        .collect::<Vec<_>>();
    if word_tokens.len() < 3
        || word_tokens[0].1.parser_text() != "restart"
        || word_tokens[1].1.parser_text() != "the"
        || word_tokens[2].1.parser_text() != "game"
    {
        return Ok(None);
    }

    if word_tokens.len() == 3 {
        return Ok(Some(EffectAst::RestartGame {
            cards_left_in_exile: None,
            source_surface: None,
        }));
    }

    if word_tokens.len() < 9
        || word_tokens[3].1.parser_text() != "leaving"
        || word_tokens[4].1.parser_text() != "in"
        || word_tokens[5].1.parser_text() != "exile"
    {
        return Err(CardTextError::ParseError(
            "unsupported restart-game continuation".to_string(),
        ));
    }

    let Some(exiled_word_idx) = word_tokens[6..]
        .windows(2)
        .position(|window| {
            window[0].1.parser_text() == "exiled" && window[1].1.parser_text() == "with"
        })
        .map(|idx| idx + 6)
    else {
        return Err(CardTextError::ParseError(
            "restart-game exile exemption is missing `exiled with`".to_string(),
        ));
    };

    let object_start = word_tokens[5].0 + 1;
    let object_end = word_tokens[exiled_word_idx].0;
    let mut object_tokens = trim_edge_punctuation(&tokens[object_start..object_end]);
    if object_tokens
        .first()
        .is_some_and(|token| token.parser_text() == "all")
    {
        object_tokens.remove(0);
    }
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "restart-game exile exemption is missing a card description".to_string(),
        ));
    }

    let mut filter = super::parse_object_filter(&object_tokens, false)?;
    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let source_start = word_tokens[exiled_word_idx + 1].0 + 1;
    let source_tokens = trim_edge_punctuation(&tokens[source_start..]);
    let source_words = source_tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    if source_words.is_empty() {
        return Err(CardTextError::ParseError(
            "restart-game exile exemption is missing its source".to_string(),
        ));
    }
    let source_text = source_words.join(" ");
    let source_surface = if source_words[0].eq_ignore_ascii_case("this")
        || source_words[0].eq_ignore_ascii_case("it")
    {
        SourceReferenceSurface::ThisPermanentType(source_text)
    } else if source_words.len() == 1 {
        SourceReferenceSurface::ShortName(source_text)
    } else {
        SourceReferenceSurface::FullName(source_text)
    };

    Ok(Some(EffectAst::RestartGame {
        cards_left_in_exile: Some(ChooseSpec::All(filter)),
        source_surface: Some(source_surface),
    }))
}

fn split_leading_amass_comma_then_sentences<'a>(
    segments: Vec<&'a [OwnedLexToken]>,
) -> Vec<&'a [OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        if segment
            .iter()
            .find_map(OwnedLexToken::as_word)
            .is_some_and(|word| word.eq_ignore_ascii_case("amass"))
        {
            let split = super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![segment]);
            if split.len() > 1 {
                result.extend(split);
                continue;
            }
        }
        result.push(segment);
    }
    result
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
    if !effect_grammar::dispatch_entry_shapes::is_one_or_more_this_way_tokens(tokens) {
        return;
    }

    let mut if_idx = 0usize;
    while effects.get(if_idx).is_some_and(|effect| {
        !matches!(
            effect,
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                ..
            }
        )
    }) {
        if_idx += 1;
    }
    if if_idx >= effects.len() {
        return;
    }

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

pub(crate) fn is_cant_be_regenerated_this_turn_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::followup_shapes::parse_cant_be_regenerated_followup(tokens)
        .is_some_and(|shape| shape.this_turn)
}

#[cfg(test)]
pub(crate) fn is_cant_be_regenerated_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::followup_shapes::parse_cant_be_regenerated_followup(tokens).is_some()
}

pub(crate) fn apply_cant_be_regenerated_to_last_destroy_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(last) = effects.last_mut() else {
        return false;
    };
    apply_cant_be_regenerated_to_effect(last)
}

pub(crate) fn apply_cant_be_regenerated_to_last_destroy_group(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(last) = effects.last_mut() else {
        return false;
    };
    let EffectAst::Coordinated {
        effects: coordinated,
        ..
    } = last
    else {
        return apply_cant_be_regenerated_to_effect(last);
    };
    let mut applied = false;
    for effect in coordinated {
        applied |= apply_cant_be_regenerated_to_effect(effect);
    }
    applied
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
    use super::super::chain_carry::Verb;
    use super::super::zone_handlers::parse_exile_top_library_clause;
    use super::super::{parse_effect_chain, parse_effect_sentence_lexed};
    use super::{
        ConsultCastCost, ConsultCastTiming, parse_bargained_face_down_cast_mana_value_gate,
        parse_consult_cast_clause, parse_consult_condition_value,
        parse_consult_mana_value_condition_tokens,
        parse_counted_looked_cards_into_your_hand_tokens, parse_if_you_dont_sentence,
        parse_looked_card_reveal_filter,
        parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
        parse_top_cards_view_sentence, parse_typed_effect_bundle_lexed,
    };

    #[test]
    fn restart_game_keeps_exiled_non_aura_permanent_cards_as_a_typed_exemption() {
        let tokens = lex_line(
            "Restart the game, leaving in exile all non-Aura permanent cards exiled with Karn.",
            0,
        )
        .expect("lex restart instruction");
        let effect = super::parse_restart_game_sentence(&tokens)
            .expect("parse restart instruction")
            .expect("restart shape matched");
        let crate::cards::builders::EffectAst::RestartGame {
            cards_left_in_exile: Some(crate::target::ChooseSpec::All(filter)),
            source_surface: Some(crate::target::SourceReferenceSurface::ShortName(source_surface)),
        } = effect
        else {
            panic!("expected typed restart-game exemption");
        };

        assert_eq!(filter.zone, Some(crate::zone::Zone::Exile));
        assert!(
            filter
                .card_types
                .contains(&crate::types::CardType::Planeswalker)
        );
        assert!(
            filter
                .excluded_subtypes
                .contains(&crate::types::Subtype::Aura)
        );
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert_eq!(source_surface, "Karn");

        let full_tokens = lex_line(
            "Restart the game, leaving in exile all non-Aura permanent cards exiled with Karn. Then put those cards onto the battlefield under your control.",
            0,
        )
        .expect("lex full restart instruction");
        let effects = super::parse_effect_sentences_lexed(&full_tokens)
            .expect("parse restart and its follow-up");
        assert_eq!(
            effects.len(),
            2,
            "the post-restart instruction must survive"
        );
        assert!(matches!(
            effects.first(),
            Some(crate::cards::builders::EffectAst::RestartGame { .. })
        ));
    }

    fn contains_still_land_animation(effects: &[crate::cards::builders::EffectAst]) -> bool {
        effects.iter().any(|effect| {
            if let crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::BecomeBasePtCreature {
                            preserve_other_types,
                            type_retention_surface,
                            ..
                        },
                    ..
                },
            ) = effect
                && *preserve_other_types
                && *type_retention_surface == Some(ironsmith_core::TypeRetentionSurface::StillALand)
            {
                return true;
            }

            let mut found = false;
            super::for_each_nested_effects(effect, true, |nested| {
                found |= contains_still_land_animation(nested);
            });
            found
        })
    }

    #[test]
    fn still_lands_sentence_reaches_followup_registry() {
        let tokens = lex_line(
            "Untap up to two target lands you control. They become 5/5 Elemental creatures with flying and haste until end of turn. They're still lands.",
            0,
        )
        .expect("land animation fixture should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("land animation with retained-type followup should parse");

        assert!(
            contains_still_land_animation(&parsed),
            "the retained-land sentence must annotate the preceding animation: {parsed:#?}"
        );
    }

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

        let parsed =
            parse_typed_effect_bundle_lexed(&tokens).expect("choose/for-each bundle should parse");

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

        let parsed = parse_typed_effect_bundle_lexed(&tokens)
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

        let parsed = parse_typed_effect_bundle_lexed(&tokens)
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
    fn each_chosen_player_search_put_top_routes_before_generic_put() {
        let tokens = lex_line(
            "Choose two target players. Each of them searches their library for a card, then shuffles and puts that card on top.",
            0,
        )
        .expect("rewrite lexer should classify chosen-player search sequence");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("typed chosen-player search sequence should parse");

        let Some(crate::cards::builders::EffectAst::ForEachPlayersFiltered { filter, effects }) =
            parsed.iter().find(|effect| {
                matches!(
                    effect,
                    crate::cards::builders::EffectAst::ForEachPlayersFiltered { .. }
                )
            })
        else {
            panic!("expected filtered player iteration, got {parsed:#?}");
        };
        assert_eq!(filter, &PlayerFilter::target_player());
        assert!(matches!(
            effects.as_slice(),
            [crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::SearchLibrary { .. },
                    ..
                }
            )]
        ));
    }

    #[test]
    fn clash_win_branch_keeps_additional_pump_and_keyword_grant_together() {
        let tokens = lex_line(
            "Target creature gets +2/+2 until end of turn. Clash with an opponent. If you win, that creature gets an additional +2/+2 and gains trample until end of turn.",
            0,
        )
        .expect("rewrite lexer should classify the clash sequence");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("typed clash result sequence should parse");
        assert!(matches!(
            parsed.get(1),
            Some(crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::Clash { .. },
                    ..
                }
            ))
        ));
        let Some(crate::cards::builders::EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::WonClash,
            effects,
        }) = parsed.last()
        else {
            panic!("expected a typed result branch, got {parsed:#?}");
        };
        let [
            crate::cards::builders::EffectAst::Coordinated {
                effects,
                leading_duration: false,
                result_conjunction: false,
            },
        ] = effects.as_slice()
        else {
            panic!("expected the conjoined rewards to retain coordination: {effects:#?}");
        };
        assert_eq!(
            effects.len(),
            2,
            "both rewards must stay gated: {effects:#?}"
        );
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::Pump { .. },
                    ..
                }
            )
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::GrantAbilitiesToTarget {
                            ..
                        },
                    ..
                }
            )
        )));

        let lowered = crate::runtime_backend::compile_support::compile_statement_effects(&parsed)
            .expect("typed clash result branch should lower normally");
        let lowered_debug = format!("{lowered:#?}");
        assert!(
            lowered_debug.contains("ClashEffect")
                && lowered_debug.contains("IfEffect")
                && lowered_debug.contains("Trample"),
            "lowered branch must retain the clash condition and both rewards: {lowered_debug}"
        );
    }

    #[test]
    fn leading_if_you_do_sequence_retains_the_conjoined_result_boundary() {
        let tokens = lex_line(
            "You may pay {1}. If you do, draw a card and gain 2 life.",
            0,
        )
        .expect("coordinated result sequence should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("coordinated result sequence should parse");
        let Some(crate::cards::builders::EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects,
        }) = parsed.last()
        else {
            panic!("expected an if-result branch, got {parsed:#?}");
        };
        let [
            crate::cards::builders::EffectAst::Coordinated {
                effects: coordinated,
                leading_duration: false,
                result_conjunction: true,
            },
        ] = effects.as_slice()
        else {
            panic!("expected one coordinated result body, got {effects:#?}");
        };
        assert_eq!(coordinated.len(), 2, "{coordinated:#?}");
    }

    #[test]
    fn leading_if_you_do_keeps_a_matched_consult_sequence_gated() {
        let tokens = lex_line(
            "You may exile it. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield and put all other cards revealed this way into your graveyard.",
            0,
        )
        .expect("Gamekeeper-style result sequence should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("Gamekeeper-style result sequence should parse");
        let [crate::cards::builders::EffectAst::MayByPlayer { .. }, gated] = parsed.as_slice()
        else {
            panic!("expected an optional antecedent followed by one gated sequence: {parsed:#?}");
        };
        let crate::cards::builders::EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects,
        } = gated
        else {
            panic!("expected the consult procedure to remain under `If you do`: {gated:#?}");
        };
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::ConsultTopOfLibrary { .. },
                    ..
                }
            )
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::MoveToZone {
                        zone: crate::zone::Zone::Battlefield,
                        ..
                    },
                    ..
                }
            )
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::ForEachTagged { .. }
        )));
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

    #[test]
    fn prior_token_instead_followup_builds_typed_self_replacement() {
        let tokens = lex_line(
            "Create a tapped 1/1 black Skeleton creature token. If a creature died this turn, create two of those tokens instead.",
            0,
        )
        .expect("lex prior-token replacement");
        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("parse typed prior-token replacement");
        let [
            crate::cards::builders::EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                attach_to_previous_ability,
            },
        ] = parsed.as_slice()
        else {
            panic!("expected typed self-replacement, got {parsed:#?}");
        };
        assert!(matches!(
            predicate,
            crate::cards::builders::PredicateAst::CreatureDiedThisTurn
        ));
        assert_eq!(if_true.len(), 1, "{if_true:#?}");
        assert_eq!(if_false.len(), 1, "{if_false:#?}");
        assert!(!attach_to_previous_ability);

        let lowered =
            crate::runtime_backend::compile_support::compile_statement_effects_with_imports(
                &parsed,
                &crate::runtime_backend::reference_model::ReferenceImports::default(),
            )
            .expect("lower typed prior-token replacement");
        let debug = format!("{lowered:#?}");
        assert!(debug.contains("self_replacements"), "{debug}");
        assert!(debug.contains("CreatureDiedThisTurn"), "{debug}");
    }

    #[test]
    fn fight_death_replacement_tracks_the_secondary_fighter() {
        for text in [
            "This creature fights target creature you don't control. If that creature would die this turn, exile it instead.",
            "This creature fights up to one target creature an opponent controls. If that creature would die this turn, exile it instead.",
            "Target creature you control fights target creature an opponent controls. If the creature an opponent controls would die this turn, exile it instead.",
        ] {
            let tokens = lex_line(text, 0).expect("lex fight death-replacement sequence");
            let parsed = super::parse_effect_sentences_lexed(&tokens)
                .expect("parse fight death-replacement sequence");

            let fight_target = parsed
                .first()
                .and_then(super::secondary_fight_target_from_effect)
                .unwrap_or_else(|| panic!("expected a fight effect for {text:?}: {parsed:#?}"));
            let replacement_target = parsed.iter().find_map(|effect| {
                let crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action:
                            crate::cards::builders::SubjectVerbActionAst::RegisterZoneReplacement {
                                target,
                                ..
                            },
                        ..
                    },
                ) = effect
                else {
                    return None;
                };
                Some(target)
            });

            assert_eq!(
                replacement_target,
                Some(&fight_target),
                "replacement must follow fight's secondary target for {text:?}: {parsed:#?}"
            );

            let lowered =
                crate::runtime_backend::compile_support::compile_statement_effects(&parsed)
                    .unwrap_or_else(|error| {
                        panic!("lower fight replacement for {text:?}: {error}")
                    });
            assert!(
                format!("{lowered:#?}").contains("RegisterZoneReplacementEffect"),
                "expected event-layer replacement for {text:?}: {lowered:#?}"
            );
        }
    }

    #[test]
    fn compound_damage_regeneration_exile_keeps_its_gate() {
        for (text, kicked) in [
            (
                "If it's a creature, it can't be regenerated this turn, and if it would die this turn, exile it instead.",
                false,
            ),
            (
                "If this spell was kicked, that creature can't be regenerated this turn and if it would die this turn, exile it instead.",
                true,
            ),
        ] {
            let tokens = lex_line(text, 0).expect("lex compound regeneration/exile rider");
            let parsed = super::damage_regeneration_exile_followup_from_sentence_tokens(&tokens)
                .expect("parse compound regeneration/exile rider");
            let [
                crate::cards::builders::EffectAst::Conditional {
                    predicate,
                    if_true,
                    if_false,
                },
            ] = parsed.as_slice()
            else {
                panic!("expected one gated compound rider for {text:?}: {parsed:#?}");
            };

            if kicked {
                assert!(matches!(
                    predicate,
                    crate::cards::builders::PredicateAst::ThisSpellWasKicked
                ));
            } else {
                let crate::cards::builders::PredicateAst::TaggedMatches(tag, filter) = predicate
                else {
                    panic!("expected creature gate for {text:?}: {parsed:#?}");
                };
                assert_eq!(tag.as_str(), crate::cards::builders::IT_TAG);
                assert_eq!(filter, &crate::target::ObjectFilter::creature());
            }
            assert!(if_false.is_empty());
            assert!(matches!(
                if_true.as_slice(),
                [
                    crate::cards::builders::EffectAst::SubjectVerb(
                        crate::cards::builders::SubjectVerbEffectAst {
                            action: crate::cards::builders::SubjectVerbActionAst::Cant { .. },
                            ..
                        },
                    ),
                    crate::cards::builders::EffectAst::SubjectVerb(
                        crate::cards::builders::SubjectVerbEffectAst {
                            action:
                                crate::cards::builders::SubjectVerbActionAst::RegisterZoneReplacement {
                                    from_zone: Some(crate::zone::Zone::Battlefield),
                                    to_zone: Some(crate::zone::Zone::Graveyard),
                                    replacement_zone: crate::zone::Zone::Exile,
                                    duration:
                                        crate::cards::builders::ZoneReplacementDurationAst::OneShot,
                                    ..
                                },
                            ..
                        },
                    ),
                ]
            ));
        }
    }

    #[test]
    fn bare_imperative_choose_does_not_inherit_a_previous_opponent_loop() {
        let tokens = lex_line(
            "Exile all opponents' graveyards. Choose a nonland card exiled this way.",
            0,
        )
        .expect("multi-sentence opponent-exile choice should lex");
        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("multi-sentence opponent-exile choice should parse");

        assert!(
            matches!(
                parsed.get(1),
                Some(crate::cards::builders::EffectAst::ChooseObjects {
                    player: crate::cards::builders::PlayerAst::You,
                    ..
                })
            ),
            "{parsed:#?}"
        );
    }

    #[test]
    fn passive_voter_owner_survives_inside_a_vote_option() {
        let tokens = lex_line(
            "Each player votes for time or money. For each money vote, choose a permanent owned by the voter and gain control of it.",
            0,
        )
        .expect("vote option with a voter-owned choice should lex");
        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("vote option with a voter-owned choice should parse");
        let Some(crate::cards::builders::EffectAst::VoteOption { effects, .. }) = parsed.get(1)
        else {
            panic!("expected a typed vote option, got {parsed:#?}");
        };
        let Some(crate::cards::builders::EffectAst::ChooseObjects { filter, .. }) = effects.first()
        else {
            panic!("expected the vote option to start with an object choice: {effects:#?}");
        };

        assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
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
            | SubjectVerbActionAst::Goad { target, .. }
            | SubjectVerbActionAst::Suspect { target }
            | SubjectVerbActionAst::RemoveFromCombat { target }
            | SubjectVerbActionAst::Flip { target }
            | SubjectVerbActionAst::Regenerate { target, .. }
            | SubjectVerbActionAst::TapOrUntap { target }
            | SubjectVerbActionAst::PhaseOut { target, .. }
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
            | SubjectVerbActionAst::AssignNoCombatDamage { source: target, .. }
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
            | SubjectVerbActionAst::TargetOnly { target, .. }
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
        if let Some(attached_to) = filter.attached_to_object.as_mut() {
            replace_in_filter(attached_to, replacement, clause)?;
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
        } else if value.unhinted() == replacement.unhinted()
            && replacement.has_surface_hint(ValueSurfaceHint::WhereXIs)
            && !value.has_surface_hint(ValueSurfaceHint::WhereXIs)
        {
            *value = replacement.clone();
        }
        Ok(())
    }

    fn replace_values_in_cost_component(
        component: &mut crate::costs::Cost,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        match component {
            crate::costs::Cost::Mana(mana) if mana.has_x() => {
                *component = crate::costs::Cost::dynamic_mana(
                    ironsmith_core::DynamicManaCost::from_x(mana.clone(), replacement.clone()),
                );
            }
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
        EffectAst::UnlessPays { effects, cost, .. } => {
            let consequence_references_it =
                crate::runtime_backend::compile_support::effects_reference_it_tag(effects);
            replace_values_in_total_cost(cost, replacement, clause)?;
            replace_unbound_x_in_effects_anywhere(effects, replacement, clause)?;
            if consequence_references_it {
                super::rewrite_unless_cost_source_values_to_it_tag(effect);
            }
        }
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value,
            ..
        }
        | EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value,
            ..
        }
        | EffectAst::ChooseObjectsBottomOfLibrary {
            filter,
            count,
            count_value,
            ..
        }
        | EffectAst::ChooseObjectsTopOfLibrary {
            filter,
            count,
            count_value,
            ..
        } => {
            replace_in_filter(filter, replacement, clause)?;
            if let Some(value) = count_value.as_mut() {
                replace_value(value, replacement, clause)?;
            } else if count.dynamic_x {
                *count_value = Some(replacement.clone());
            }
        }
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
            SubjectVerbActionAst::PayMana { cost, x_value } => {
                if cost.has_x() && x_value.is_none() {
                    *x_value = Some(replacement.clone());
                } else if let Some(x_value) = x_value.as_mut() {
                    replace_value(x_value, replacement, clause)?;
                }
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
                filter,
                stop_rule,
                max_exposed,
                ..
            } => {
                replace_in_filter(filter, replacement, clause)?;
                if let LibraryConsultStopRuleAst::MatchCount(count) = stop_rule {
                    replace_value(count, replacement, clause)?;
                }
                if let Some(max_exposed) = max_exposed {
                    replace_value(max_exposed, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::ReturnToHand { target, .. }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target } => {
                replace_in_target(target, replacement, clause)?;
            }
            SubjectVerbActionAst::ReturnAllToHand { filter, .. }
            | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter }
            | SubjectVerbActionAst::DestroyAll { filter, .. }
            | SubjectVerbActionAst::DestroyAllOfChosenColor { filter, .. }
            | SubjectVerbActionAst::ExileAll { filter, .. } => {
                replace_in_filter(filter, replacement, clause)?;
            }
            SubjectVerbActionAst::RevealCardsFromHand {
                count,
                count_value,
                ..
            } => {
                if count.dynamic_x {
                    if let Some(value) = count_value {
                        replace_value(value, replacement, clause)?;
                    } else {
                        *count_value = Some(replacement.clone());
                    }
                }
            }
            SubjectVerbActionAst::DealDamageEqualToPower { .. }
            | SubjectVerbActionAst::DrawForEachTaggedMatching { .. }
            | SubjectVerbActionAst::RevealHand
            | SubjectVerbActionAst::RevealTagged { .. }
            | SubjectVerbActionAst::PutOntoBattlefield { .. }
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
            | SubjectVerbActionAst::CloakTopCardOfLibrary
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
            | SubjectVerbActionAst::ChooseLandType { .. }
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
            | SubjectVerbActionAst::LookAtHand { .. }
            | SubjectVerbActionAst::Counter { .. }
            | SubjectVerbActionAst::DoubleCountersOnEach { .. }
            | SubjectVerbActionAst::MoveAllCounters { .. }
            | SubjectVerbActionAst::MoveOneCounter { .. }
            | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { .. }
            | SubjectVerbActionAst::PutCounterOfChosenKind { .. }
            | SubjectVerbActionAst::Sacrifice { .. }
            | SubjectVerbActionAst::SacrificeAll { .. }
            | SubjectVerbActionAst::RevealTop
            | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
            | SubjectVerbActionAst::ReorderTopOfLibrary { .. }
            | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { .. }
            | SubjectVerbActionAst::PutSticker { .. }
            | SubjectVerbActionAst::SwitchPowerToughness { .. }
            | SubjectVerbActionAst::ScalePowerToughnessAll { .. }
            | SubjectVerbActionAst::ScaleXValue { .. }
            | SubjectVerbActionAst::GrantProtectionChoice { .. }
            | SubjectVerbActionAst::PreventAllCombatDamage { .. }
            | SubjectVerbActionAst::AssignNoCombatDamage { .. }
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
            | SubjectVerbActionAst::SetCardTypes { .. }
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
    let Some(shape) =
        effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(tokens)
    else {
        return Ok(());
    };
    let Some(where_value) = crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_value_binding(shape.binding_tokens)
        .or_else(|| parse_value_binding_clause(shape.binding_tokens))
        .map(|value| with_where_x_surface_hints(value, tokens))
    else {
        return Ok(());
    };
    let clause_text = LexedClause::new(tokens).text();
    match shape.scope {
        effect_grammar::dispatch_entry_shapes::WhereXReplacementScope::DamageOrLife => {
            replace_unbound_x_in_damage_effects(effects, &where_value, &clause_text)
        }
        effect_grammar::dispatch_entry_shapes::WhereXReplacementScope::AnyEffect => {
            replace_unbound_x_in_effects_anywhere(effects, &where_value, &clause_text)
        }
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
                ..
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
                ..
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
                ..
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
    grammar::match_word_prefix(
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
        || grammar::match_word_prefix(
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
        .map(|(before, _after)| grammar::has_phrase(before, &["this", "way"]))
        .unwrap_or(false);
    let action_result_followup = grammar::strip_lexed_prefix_phrase(tokens, &["when", "you"])
        .or_else(|| grammar::strip_lexed_prefix_phrase(tokens, &["whenever", "you"]))
        .is_some_and(|rest| {
            rest.first()
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| matches!(word, "discard" | "exile" | "mill" | "sacrifice"))
                && grammar::strip_lexed_prefix_phrase(&rest[1..], &["one", "or", "more"]).is_some()
        });
    if action_result_followup && this_way_in_prefix {
        let Some((_before, after)) =
            grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        else {
            return tokens.to_vec();
        };
        let mut rewritten = Vec::new();

        let mut when_token = tokens[0].clone();
        when_token.replace_word("when");
        rewritten.push(when_token);

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
    if !effect_grammar::dispatch_entry_shapes::has_otherwise_referential_subject_tokens(&tokens) {
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
        || effect_grammar::dispatch_entry_shapes::is_x_cant_be_zero_tokens(tokens)
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
        | EffectAst::DelayedUntilNextMainPhase { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectLeavesBattlefield { effects, .. }
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
            "upkeep"
        ] | [
            "sacrifice",
            "those",
            "tokens",
            "at",
            "beginning",
            "of",
            "next",
            "upkeep"
        ]
    ) {
        return Some(TokenCopyFollowup::SacrificeAtNextUpkeep);
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
            "upkeep"
        ] | [
            "sacrifice",
            "those",
            "tokens",
            "at",
            "beginning",
            "of",
            "next",
            "upkeep"
        ]
    ) {
        return Some(TokenCopyFollowup::SacrificeAtNextUpkeep);
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
    let Some(ability_tokens) =
        effect_grammar::dispatch_entry_shapes::parse_token_granted_ability_tokens(tokens)
    else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let (abilities, is_choice) =
        super::parse_granted_abilities_for_gain_clause(ability_tokens, &clause_words, false)?;
    if is_choice || abilities.is_empty() {
        return Ok(None);
    }
    Ok(Some(abilities))
}

fn apply_unapplied_token_copy_followup(
    sentence: &[OwnedLexToken],
    sentence_tokens: &[OwnedLexToken],
    followup: TokenCopyFollowup,
    bind_leading_it_to_source: bool,
) -> Result<Vec<EffectAst>, CardTextError> {
    let span = span_from_tokens(sentence);
    let leading_it_span = || {
        let tokens = trim_edge_punctuation(sentence_tokens);
        let first = tokens.first()?;
        if first.as_word()? != "it" {
            return None;
        }
        Some(first.span)
    };
    let fallback_target = || {
        let leading_it_span = leading_it_span();
        if let Some(it_span) = leading_it_span {
            let it_span = Some(it_span);
            crate::runtime_backend::util::record_source_reference_surface(
                it_span,
                SourceReferenceSurface::ThisPermanentType("it".to_string()),
            );
            crate::runtime_backend::util::record_source_reference_surface(
                span,
                SourceReferenceSurface::ThisPermanentType("it".to_string()),
            );
            if bind_leading_it_to_source {
                return TargetAst::Source(it_span);
            }
        }
        TargetAst::Tagged(TagKey::from(IT_TAG), span)
    };
    let effects = match followup {
        TokenCopyFollowup::HasHaste => vec![EffectAst::subject_verb_grant_abilities_to_target(
            fallback_target(),
            vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
            Until::Forever,
        )],
        TokenCopyFollowup::GainHasteUntilEndOfTurn => {
            vec![EffectAst::subject_verb_grant_abilities_to_target(
                fallback_target(),
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
        TokenCopyFollowup::SacrificeAtNextUpkeep => vec![EffectAst::DelayedUntilNextUpkeep {
            player: PlayerAst::Any,
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
                    granted_abilities,
                    ability_presentation,
                    ..
                },
            ..
        }) => {
            granted_abilities.extend(abilities.iter().cloned());
            *ability_presentation =
                Some(ironsmith_core::TokenAbilityPresentation::SeparateSentence);
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
                    | TokenCopyFollowup::SacrificeAtNextUpkeep
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
                    | TokenCopyFollowup::SacrificeAtNextUpkeep
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
                    | TokenCopyFollowup::SacrificeAtNextUpkeep
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
