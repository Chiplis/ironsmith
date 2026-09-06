use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::DamagePreventionActionAst;
use self::subject_verb_followups::{
    PostParseFollowupResult, PreParseFollowupResult, is_conditional_token_entry_followup_sentence,
    run_post_parse_followup_registry, run_pre_parse_followup_registry,
    try_bind_conditional_token_entry_followup,
};
use super::super::activation_and_restrictions::{
    parse_mana_usage_restriction_sentence_lexed, parse_single_word_keyword_action,
};
use super::super::effect_ast_traversal::{
    TerminalResultProducer, for_each_nested_effects, for_each_nested_effects_mut,
    terminal_result_producer, try_for_each_nested_effects_mut,
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
use super::bundle_rules::parse_same_sentence_copy_and_may_cast_copy;
use super::consult_family;
use super::divvy::try_parse_divvy_sentence_sequence;
use super::looked_cards_family;
use super::sentence_helpers::*;
use super::sequence_rules::try_parse_document_program;
use super::{
    SubjectVerbPrimitiveClause, parse_effect_sentence_lexed, parse_token_copy_modifier_sentence,
    trim_edge_punctuation, try_build_unless,
};
use crate::cards::builders::{
    CardTextError, CarryContext, EffectAst, GrantedAbilityAst, IfResultPredicate, InsteadSemantics,
    KeywordAction, LibraryBottomOrderAst, LibraryConsultModeAst, LibraryConsultStopRuleAst,
    PlayerAst, PredicateAst, PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst,
    ReturnControllerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, TagKey, TargetAst, TokenCopyFollowup, ZoneReplacementDurationAst, CounterActionAst, GrantActionAst, ManaActionAst, LibraryActionAst, ReplacementActionAst, ExchangeActionAst, TurnStructureActionAst, CharacteristicActionAst, KeywordActionAst, ZoneMoveActionAst, PermanentStateActionAst, RevealLookActionAst, RandomActionAst, LifeResourceActionAst, ChoiceActionAst, DamageActionAst, StatChangeActionAst, StackActionAst, TokenActionAst, ControlActionAst, GameActionAst, DelayedEffectAst, ObjectChoiceEffectAst, VoteEffectAst, ConditionalEffectAst, PermissionEffectAst,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::parse_trace;
use crate::recognition::ParseOutcome;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, SourceReferenceSurface, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};
use crate::types::CardType;
use crate::zone::Zone;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;
use ironsmith_core::ValueSurfaceHint;
use std::cell::OnceCell;
use winnow::Parser as _;

mod subject_verb_followups;

/// Keep a retarget of a newly copied stack object in the delayed trigger that
/// creates that copy. Trigger-line parsing has its own public-root path, so it
/// applies this typed normalization after constructing its raw `LineAst` too.
pub fn transport_copy_retarget_into_trailing_delayed_trigger(effects: &mut Vec<EffectAst>) {
    subject_verb_followups::transport_copy_retarget_into_trailing_delayed_trigger(effects);
    subject_verb_followups::transport_copy_retarget_into_trailing_optional_copy(effects);
}

/// Parse a complete quantified token-creation sentence before any quoted
/// token rule can be mistaken for the outer action. The unquoted prefix proves
/// the participant and creation shape; the untouched tokens are then used to
/// attach each quoted rule to the created token.
pub fn parse_quantified_token_creation_with_embedded_rules(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let full_tokens = trim_edge_punctuation(tokens);
    let outer_tokens = strip_embedded_token_rules_text(&full_tokens);
    if outer_tokens == full_tokens {
        return Ok(None);
    }
    let words = crate::lexer::token_word_refs(&full_tokens);
    let has_quantified_player = crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[
            &["each", "opponent"],
            &["each", "player"],
            &["for", "each", "opponent"],
            &["for", "each", "player"],
        ],
    );
    if !has_quantified_player
        || !crate::word_primitives::any_sequence_occurs(&words, &[&["create"], &["creates"]])
        || !crate::word_primitives::sequence_occurs(&words, &["token"])
    {
        return Ok(None);
    }

    let effect = if crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[&["each", "opponent"], &["for", "each", "opponent"]],
    ) {
        parse_for_each_opponent_clause(&outer_tokens)?
    } else {
        parse_for_each_player_clause(&outer_tokens)?
    };
    let Some(effect) = effect else {
        return Ok(None);
    };
    let mut effects = vec![effect];
    super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
        &mut effects,
        &full_tokens,
    );
    Ok(effects.pop())
}

/// Recover the compared revealed set after a complete effect-body parse.
/// Some document routes add prior-action surface provenance to the generic
/// `for each card revealed this way` repeat after the sentence followup
/// registry has run. The original two-sentence source and the typed reveal
/// tag still prove the exact same-mana-value relation here.
fn preserve_revealed_same_mana_value_as_another_iterator(
    tokens: &[OwnedLexToken],
    effects: &mut [EffectAst],
) {
    let sentences = split_lexed_sentences(tokens);
    let Some(comparison_sentence) = sentences.last().copied() else {
        return;
    };
    let words = crate::lexer::token_word_refs(comparison_sentence);
    const PREFIX: &[&str] = &[
        "for", "each", "of", "those", "cards", "that", "has", "the", "same", "mana", "value", "as",
        "another", "card", "revealed", "this", "way",
    ];
    if sentences.len() < 2 || !crate::word_primitives::parse_sequence_prefix(&words, PREFIX) {
        return;
    }

    let Some(revealed_tag) = effects.iter().rev().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { tag }),
            ..
        }) => Some(tag.clone()),
        _ => None,
    }) else {
        return;
    };
    let Some(iterator) = effects.last_mut() else {
        return;
    };
    let conditional_effects = match iterator {
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged { tag, effects })
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && !effects.is_empty() =>
        {
            std::mem::take(effects)
        }
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, effects })
            if !effects.is_empty()
                && matches!(
                    count.unhinted(),
                    Value::PendingPriorEffectMetric(query)
                        if query.source == ironsmith_core::EffectMetricSource::AffectedObjects
                            && query.metric == ironsmith_core::EffectMetric::Count
                            && query.player.is_none()
                            && matches!(
                                query.action,
                                None | Some(ironsmith_core::PriorEffectAction::Revealed)
                            )
                            && query.counter_type.is_none()
                ) =>
        {
            std::mem::take(effects)
        }
        _ => return,
    };
    let filter = ObjectFilter::default().match_tagged(
        revealed_tag.clone(),
        TaggedOpbjectRelation::SameManaValueAsAnotherTagged,
    );
    *iterator = EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: revealed_tag,
        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: PredicateAst::ItMatches(filter),
            effects: conditional_effects,
        })],
    });
}

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
pub fn apply_leading_duration_to_become_effect(effect: &mut EffectAst, duration: &Until) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                duration: effect_duration,
                animation_duration_surface,
                ..
            }) => {
                *effect_duration = duration.clone();
                *animation_duration_surface =
                    Some(ironsmith_core::AnimationDurationSurface::Leading);
                true
            }
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                duration: effect_duration,
                ..
            })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
                duration: effect_duration,
                ..
            }) => {
                *effect_duration = duration.clone();
                true
            }
            _ => false,
        },
        EffectAst::Sequence { effects } | EffectAst::Coordinated { effects, .. } => {
            let mut applied = false;
            for effect in effects {
                applied |= apply_leading_duration_to_become_effect(effect, duration);
            }
            applied
        }
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        }) => {
            let mut applied = false;
            for branch_effect in if_true.iter_mut().chain(if_false.iter_mut()) {
                applied |= apply_leading_duration_to_become_effect(branch_effect, duration);
            }
            applied
        }
        _ => false,
    }
}

fn apply_leading_duration_to_entire_effect(effect: &mut EffectAst, duration: &Until) -> bool {
    match effect {
        EffectAst::Sequence { effects } | EffectAst::Coordinated { effects, .. } => {
            !effects.is_empty()
                && effects
                    .iter_mut()
                    .all(|effect| apply_leading_duration_to_entire_effect(effect, duration))
        }
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        }) => {
            let branch_count = if_true.len() + if_false.len();
            branch_count > 0
                && if_true
                    .iter_mut()
                    .chain(if_false.iter_mut())
                    .all(|effect| apply_leading_duration_to_entire_effect(effect, duration))
        }
        _ => apply_leading_duration_to_become_effect(effect, duration),
    }
}

fn preserve_fully_scoped_leading_duration_coordination(effects: Vec<EffectAst>) -> Vec<EffectAst> {
    let mut flattened = Vec::new();
    let mut had_coordination = false;
    for effect in effects {
        match effect {
            EffectAst::Coordinated {
                effects,
                result_conjunction: false,
                ..
            } => {
                had_coordination = true;
                flattened.extend(effects);
            }
            other => flattened.push(other),
        }
    }
    if flattened.len() > 1 || had_coordination {
        vec![EffectAst::Coordinated {
            effects: flattened,
            leading_duration: true,
            result_conjunction: false,
        }]
    } else {
        flattened
    }
}

fn should_apply_leading_duration_become_shortcut(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::token_word_refs(tokens);
    if crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[&["at"], &["when"], &["whenever"]],
    ) {
        return false;
    }
    if crate::word_primitives::sequence_occurs(&words, &["if"]) {
        return false;
    }
    if crate::word_primitives::any_sequence_occurs(
        &words,
        &[
            &["and", "become"],
            &["and", "becomes"],
            &["and", "attacks"],
            &["and", "blocks"],
        ],
    ) {
        return false;
    }
    crate::word_primitives::any_sequence_occurs(&words, &[&["become"], &["becomes"]])
}

const OTHERWISE_WORD: &str = "otherwise";
pub(super) fn summarize_effects(effects: &[EffectAst]) -> String {
    effects
        .iter()
        .map(|effect| {
            let debug = format!("{effect:?}");
            debug
                .split([' ', '{', '('])
                .next()
                .unwrap_or("Effect")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn bind_that_object_power_damage_subject(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
    previous_damage_target: Option<TargetAst>,
) {
    if !effect_grammar::dispatch_entry_shapes::is_that_object_power_damage_to_source_tokens(tokens)
    {
        return;
    }
    let source_target = previous_damage_target
        .or_else(|| effects.iter().find_map(primary_damage_target_from_effect))
        .unwrap_or_else(|| {
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(tokens),
            )
        });
    fn bind_source_target_in_effect(effect: &mut EffectAst, source_target: &TargetAst) {
        if let EffectAst::SubjectVerb(subject_verb) = effect {
            match &subject_verb.action {
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                    amount,
                    target,
                    unpreventable,
                }) if matches!(amount, Value::PowerOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source))
                    && matches!(target, TargetAst::Source(_)) =>
                {
                    subject_verb.action = SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                        source: source_target.clone(),
                        amount: Value::PowerOf(Box::new(ChooseSpec::Source)),
                        target: target.clone(),
                        unpreventable: *unpreventable,
                    });
                }
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    source,
                    amount,
                    target,
                    unpreventable,
                }) if (matches!(source, TargetAst::Source(_))
                    || matches!(source, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()))
                    && matches!(target, TargetAst::Source(_)) =>
                {
                    subject_verb.action = SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                        source: source_target.clone(),
                        amount: amount.clone(),
                        target: target.clone(),
                        unpreventable: *unpreventable,
                    });
                }
                _ => {}
            }
        }

        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                bind_source_target_in_effect(nested_effect, source_target);
            }
        });
    }

    for effect in effects {
        bind_source_target_in_effect(effect, &source_target);
    }
}

fn bind_target_controlled_source_damage_to_that_player(
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
        let SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { source, target, .. }) =
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
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. }),
            ..
        }) = effect
            && filter.with_counter.is_none()
        {
            filter.with_counter = Some(counter_constraint);
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
                SubjectVerbActionAst::Stack(StackActionAst::CastTagged {
                    as_copy,
                    cost_reduction,
                    ..
                }),
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

fn trim_effect_sentence_edge_punctuation(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    // A terminal quoted grant ends in `."`, so the ordinary punctuation
    // trimmer would remove both the closing quote and its period while leaving
    // the opening quote embedded in the sentence. Keep balanced quote pairs
    // intact so the grant parser can retain authored quoted-ability semantics.
    let quote_count = tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Quote)
        .count();
    if quote_count < 2 || quote_count % 2 != 0 {
        return trim_edge_punctuation(tokens);
    }

    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
        )
    {
        end -= 1;
    }
    tokens[start..end].to_vec()
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
    pub(super) surface: ironsmith_core::GrantPlayTaggedSurface,
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

pub struct SentenceInput {
    lowered: OnceCell<Vec<OwnedLexToken>>,
    lexed: Vec<OwnedLexToken>,
}

impl SentenceInput {
    pub fn from_lexed(tokens: &[OwnedLexToken]) -> Self {
        Self {
            lowered: OnceCell::new(),
            lexed: tokens.to_vec(),
        }
    }

    pub fn lowered(&self) -> &[OwnedLexToken] {
        self.lowered
            .get_or_init(|| normalize_parser_tokens(&self.lexed))
            .as_slice()
    }

    pub fn lexed(&self) -> &[OwnedLexToken] {
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
                EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    predicate,
                    effects: nested,
                }),
            ],
        ) if predicate == &prefix.predicate => {
            super::preserve_result_conjunction_body_lexed(prefix.trailing_tokens, nested);
            return;
        }
        (
            LeadingResultPrefixKind::When,
            [
                EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                    predicate,
                    effects: nested,
                }),
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
        LeadingResultPrefixKind::If => EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: prefix.predicate,
            effects: nested,
        }),
        LeadingResultPrefixKind::When => EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
            predicate: prefix.predicate,
            effects: nested,
        }),
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

pub fn future_zone_replacement_from_sentence_tokens(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let target = || TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
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
                    .match_tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        TaggedOpbjectRelation::IsTaggedObject,
                    ),
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
            // The target can die after this resolving spell has left the
            // stack. Keep the replacement through the turn rather than tying
            // its lifetime to the source spell's one-shot effects.
            ZoneReplacementDurationAst::UntilEndOfTurn,
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

/// Parses a counter-result replacement followed by an immediate permission
/// for the same tagged spell.  Keeping the two actions together matters: the
/// replacement must be installed around the preceding counter effect, while
/// the permission must run only after that counter has moved the spell to its
/// replacement zone.
fn future_zone_replacement_with_may_cast_followup(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let (replacement_tokens, cast_tokens) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())?;
    let replacement_tokens = trim_commas(replacement_tokens);
    let cast_tokens = trim_commas(cast_tokens);
    let replacement = future_zone_replacement_from_sentence_tokens(&replacement_tokens)?;
    let cast = parse_may_cast_it_sentence(&cast_tokens)?;
    Some(vec![replacement, build_may_cast_tagged_effect(&cast)])
}

fn damage_regeneration_exile_followup_from_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = effect_grammar::followup_shapes::parse_damage_regeneration_exile_followup(tokens)?;
    let replacement = future_zone_replacement_from_sentence_tokens(tokens)?;
    let tagged_target = crate::tag::CompilerReferenceTag::It.bind();
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

    Some(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate,
        if_true: vec![cant_regenerate, replacement],
        if_false: Vec::new(),
    })])
}

fn secondary_fight_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { creature2, .. }),
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
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                target,
                from_zone: Some(Zone::Battlefield),
                to_zone: Some(Zone::Graveyard),
                replacement_zone: Zone::Exile,
                duration: ZoneReplacementDurationAst::UntilEndOfTurn,
                ..
            }),
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
    let Some(EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
        effects: reflexive_effects,
        ..
    })) = effects.last_mut()
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
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileInsteadOfGraveyardThisTurn)
                    | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventNextTimeDamage { .. })
                    | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource { .. })
                    | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. }),
                ..
            })
        )
    }) {
        return;
    }

    if sentence_effects.len() == 1 {
        if let Some(EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })) = sentence_effects.first_mut() {
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
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Otherwise,
            effects: otherwise_effects,
        }),
    ] = sentence_effects
    else {
        return false;
    };
    let Some(previous) = effects.last_mut() else {
        return false;
    };
    let conditional = match previous {
        conditional @ EffectAst::Conditionals(ConditionalEffectAst::Conditional { .. }) => conditional,
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Value(_),
            effects,
        }) => {
            let Some(conditional @ EffectAst::Conditionals(ConditionalEffectAst::Conditional { .. })) = effects.last_mut() else {
                return false;
            };
            conditional
        }
        _ => return false,
    };
    let EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_false, .. }) = conditional else {
        unreachable!("conditional shape was proven above")
    };
    if !if_false.is_empty() {
        return false;
    }
    // "Otherwise" negates the authored condition. An optional action in the
    // true arm does not make the condition false when its player declines it;
    // explicit "if you don't" wording is handled by the result-followup path.
    *if_false = otherwise_effects.clone();
    true
}

#[cfg(test)]
mod nested_numeric_otherwise_tests {
    use crate::cards::builders::SourcePredicateAst;
    use super::*;

    fn conditional(if_false: Vec<EffectAst>) -> EffectAst {
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::Source(SourcePredicateAst::SourceIsTapped),
            if_true: vec![EffectAst::SolveCase],
            if_false,
        })
    }

    fn otherwise() -> Vec<EffectAst> {
        vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Otherwise,
            effects: vec![EffectAst::RestartGame {
                cards_left_in_exile: None,
                source_surface: None,
            }],
        })]
    }

    #[test]
    fn otherwise_can_fill_the_conditional_inside_one_numeric_result_row() {
        let mut prior = vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Value(crate::effect::Comparison::Equal(20)),
            effects: vec![EffectAst::SolveCase, conditional(Vec::new())],
        })];
        assert!(try_merge_otherwise_into_previous_conditional(
            &mut prior,
            &otherwise()
        ));
        let [EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })] = prior.as_slice() else {
            panic!("numeric branch changed shape: {prior:#?}");
        };
        let Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_false, .. })) = effects.last() else {
            panic!("conditional tail changed shape: {effects:#?}");
        };
        assert!(matches!(
            if_false.as_slice(),
            [EffectAst::RestartGame { .. }]
        ));
    }

    #[test]
    fn nested_otherwise_does_not_overwrite_a_populated_false_arm() {
        let mut prior = vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Value(crate::effect::Comparison::Equal(20)),
            effects: vec![conditional(vec![EffectAst::SolveCase])],
        })];
        assert!(!try_merge_otherwise_into_previous_conditional(
            &mut prior,
            &otherwise()
        ));
    }
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
    let Some(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::Value(_),
        effects: branch_effects,
    })) = effects.last_mut()
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
    split_leading_result_prefix_lexed(sentence_tokens)?;
    match sentence_effects {
        [
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Value(_),
                ..
            }),
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
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
            amount: Value::EventValue(EventValueSpec::Amount),
        }),
    );
    if let [EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })] = sentence_effects.as_mut_slice() {
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

    if let [EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })] = sentence_effects.as_mut_slice() {
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
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { .. }) | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { .. }),
                ..
            })
        )
    });
    if already_exiles {
        return;
    }

    effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        false,
    ));
}

fn effect_is_life_loss(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { .. }),
            ..
        })
    )
}

fn maybe_bind_that_player_gain_control_if_do_rewards(
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
                    action: SubjectVerbActionAst::Control(ControlActionAst::GainControl { .. }),
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
    effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: rewards,
    }));
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
    super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
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
    super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
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
    super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(
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

pub fn with_where_x_surface_hints(mut value: Value, binding_tokens: &[OwnedLexToken]) -> Value {
    let words = crate::lexer::token_word_refs(binding_tokens);
    let has_word = |word| crate::word_primitives::sequence_occurs(&words, &[word]);
    let explicit_count_surface = has_word("number")
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
    let counts_objects = counts_objects
        || matches!(
            value.unhinted(),
            Value::PendingPriorEffectMetric(query) | Value::PriorEffectMetric { query, .. }
                if query.metric == ironsmith_core::EffectMetric::Count
        );
    let counts_objects_with_an_ability = match value.unhinted() {
        Value::Count(filter) | Value::CountScaled(filter, _) => {
            !filter.ability_markers.is_empty() || !filter.static_abilities.is_empty()
        }
        _ => false,
    };
    let aggregates_objects = matches!(
        value.unhinted(),
        Value::TotalPower(_) | Value::TotalToughness(_) | Value::TotalManaValue(_)
    );
    value = value.without_surface_hint(ValueSurfaceHint::EqualTo)
        .with_surface_hint(ValueSurfaceHint::WhereXIs);
    if counts_objects_with_an_ability
        && words
            .iter()
            .any(|word| matches!(*word, "ability" | "abilities"))
    {
        value = value.with_surface_hint(ValueSurfaceHint::ExplicitAbilityNoun);
    }
    let mentions_energy = binding_tokens.iter().any(|token| {
        token.as_word() == Some("e")
            || (token.kind == TokenKind::ManaGroup && token.mana_group_inner() == Some("e"))
    });
    if mentions_energy && has_word("paid") && has_word("this") && has_word("way") {
        value = value.with_surface_hint(ValueSurfaceHint::EnergyPaidThisWay);
    } else if has_word("mana")
        && has_word("value")
        && has_word("permanent")
        && has_word("exiled")
        && has_word("way")
    {
        value = value.with_surface_hint(ValueSurfaceHint::ManaValueOfPermanentExiledThisWay);
    } else if has_word("result") {
        value = value.with_surface_hint(ValueSurfaceHint::PriorEffectResult);
    } else if counts_objects && has_word("revealed") && has_word("way") {
        value = value.with_surface_hint(ValueSurfaceHint::CardsRevealedThisWay);
    } else if counts_objects && has_word("exiled") && has_word("way") {
        value = value.with_surface_hint(ValueSurfaceHint::CardsExiledThisWay);
    } else if counts_objects && has_word("discarded") && has_word("way") {
        value = value.with_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay);
    } else if counts_objects && has_word("died") && has_word("way") {
        value = value.with_surface_hint(ValueSurfaceHint::DiedThisWay);
    } else if (counts_objects || aggregates_objects) && has_word("sacrificed") && has_word("way") {
        value = value.with_surface_hint(ValueSurfaceHint::PermanentsSacrificedThisWay);
    } else if counts_objects && has_word("counters") && has_word("removed") && has_word("way") {
        value = value.with_surface_hint(ValueSurfaceHint::CountersRemovedThisWay);
    }
    value
}

fn into_exact_single_conditional(mut parsed: Vec<EffectAst>) -> Option<EffectAst> {
    if parsed.len() != 1 {
        return None;
    }
    match parsed.pop()? {
        conditional @ EffectAst::Conditionals(ConditionalEffectAst::Conditional { .. }) => Some(conditional),
        EffectAst::Sequence { effects } | EffectAst::Coordinated { effects, .. } => {
            into_exact_single_conditional(effects)
        }
        _ => None,
    }
}

fn parse_effect_sentences_from_sentence_inputs(
    sentences: Vec<SentenceInput>,
) -> Result<Vec<EffectAst>, CardTextError> {
    fn bind_definite_player_damage_to_carried_participant(
        carried_context: CarryContext,
        sentence_tokens: &[OwnedLexToken],
        effect: &mut EffectAst,
    ) {
        let sentence_words = crate::lexer::parser_token_word_refs(sentence_tokens);
        if carried_context != CarryContext::Player(PlayerAst::That)
            || !crate::word_primitives::sequence_occurs(&sentence_words, &["the", "player"])
        {
            return;
        }

        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { target, .. }),
            ..
        }) = effect
            && let TargetAst::Player(player, _) = target
            && *player == PlayerFilter::Any
        {
            // A previous effect can establish the participant through an
            // object-controller relation (for example, tap permanents "that
            // player controls"). In the following sentence, definite "the
            // player" names that participant rather than a fresh arbitrary
            // player.
            *player = PlayerFilter::IteratedPlayer;
        }
    }

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

        let sentence_words = crate::lexer::parser_token_word_refs(sentence_tokens);
        if !crate::word_primitives::sequence_occurs(&sentence_words, &["that", "player"])
            || !matches!(
                previous_effects.last(),
                Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { .. }))
            )
        {
            return;
        }
        let [EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, .. })] = sentence_effects.as_mut_slice() else {
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
        sentence_effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![repeat],
        }));
        parse_trace::event(
            "partitioned prior-effect repeat scoped to the preceding each-player result",
        );
    }

    fn annotate_counter_followup_surface(effects: &mut [EffectAst], hint: ValueSurfaceHint) {
        fn annotate(effect: &mut EffectAst, hint: ValueSurfaceHint) {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { count, .. }),
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

    fn starts_with_linked_ability_grant(tokens: &[OwnedLexToken]) -> bool {
        let words = crate::lexer::parser_token_word_refs(tokens);
        let first = words.first().copied();
        let second = words.get(1).copied();
        let third = words.get(2).copied();
        (matches!(first, Some("it")) && matches!(second, Some("gains" | "has")))
            || (matches!(first, Some("they")) && matches!(second, Some("gain" | "have")))
            || (matches!(first, Some("that"))
                && matches!(
                    second,
                    Some(
                        "creature" | "permanent" | "artifact" | "enchantment" | "land" | "vehicle"
                    )
                )
                && matches!(third, Some("gains" | "has")))
            || (matches!(first, Some("those"))
                && matches!(
                    second,
                    Some(
                        "creatures"
                            | "permanents"
                            | "artifacts"
                            | "enchantments"
                            | "lands"
                            | "vehicles"
                    )
                )
                && matches!(third, Some("gain" | "have")))
    }

    fn preserve_plural_counter_antecedent(
        sentence_tokens: &[OwnedLexToken],
        effects: &mut Vec<EffectAst>,
    ) {
        let sentence_words = crate::lexer::parser_token_word_refs(sentence_tokens);
        if !crate::word_primitives::sequence_occurs(&sentence_words, &["among", "those", "cards"]) {
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
                if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                    constraint.tag = alias.clone();
                    rebound = true;
                }
            }
            rebound
        }

        fn bind_effect(effect: &mut EffectAst, alias: &TagKey) -> bool {
            let mut rebound = false;
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { count, .. }),
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

        let alias = crate::tag::CompilerReferenceTag::PluralAntecedentCards.bind();
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
        let binding_tokens = crate::util::trim_edge_punctuation_tokens(binding_tokens);
        if let Some(value) =
            crate::keyword_static::parse_where_x_is_aggregate_filter_value(binding_tokens)
        {
            return Some(with_where_x_surface_hints(value, tokens));
        }
        if let Some(value) =
            crate::grammar::shared_util::value_semantics::parse_turn_history_value_binding(
                binding_tokens,
            )
        {
            return Some(with_where_x_surface_hints(value, tokens));
        }
        // Preserve typed `number of ...` aggregates before the generic exact
        // value shape can reduce their trailing scope to a plain object count.
        // For example, the count in "number of abilities from among ...
        // found among creatures you control" is the distinct ability set,
        // not the creatures that carry those abilities.
        if let Some(value) =
            crate::keyword_static::parse_where_x_is_number_of_filter_value(binding_tokens)
        {
            return Some(with_where_x_surface_hints(value, tokens));
        }
        if let Some(value) = parse_exact_where_x_value_expression(binding_tokens) {
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
        Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, effects })]))
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
        let target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
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
        let first_quote = crate::slice_primitives::select_position(tokens, |token| {
            token.kind == TokenKind::Quote
        })
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
        let clause_words = crate::lexer::token_word_refs(tokens);
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

        let target = TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(tokens),
        );
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
        let authored_sentence = sentences[sentence_idx].lexed();
        let comma_then =
            super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![authored_sentence]);
        if let [target_clause, flip_clause] = comma_then.as_slice()
            && super::super::grammar::effects::clause_dispatch_shapes::parse_choose_target_shape(
                target_clause,
            )
            .is_some()
            && matches!(
                crate::lexer::parser_token_word_refs(flip_clause).as_slice(),
                ["flip", "a", "coin"] | ["you", "flip", "a", "coin"]
            )
        {
            effects.push(EffectAst::CommaThen {
                effects: vec![
                    super::parse_effect_clause_lexed(target_clause)?,
                    super::parse_effect_clause_lexed(flip_clause)?,
                ],
            });
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        // `SentenceInput::lowered()` intentionally removes presentation
        // punctuation, including the quote boundaries around embedded token
        // rules. Keep the grammar-proven quantified create action ahead of
        // that normalization: otherwise a quoted `can't block` rule can be
        // claimed as the outer restriction and the token creation is lost.
        // The helper parses the actor/create prefix from the rule-free slice
        // and reattaches every rule from this untouched lexed sentence.
        if let Some(effect) =
            parse_quantified_token_creation_with_embedded_rules(sentences[sentence_idx].lexed())?
        {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        let sentence = sentences[sentence_idx].lowered();
        if sentence.is_empty() {
            sentence_idx += 1;
            continue;
        }
        // A sentence that says what happens to the damage the preceding shield
        // prevents binds to that shield before anything else reads it: it
        // names the prevention event's amount, which nothing else knows.
        if super::chain_carry::bind_prevention_followup(&mut effects, sentence)
            || super::chain_carry::bind_tap_lock(&mut effects, sentence)
            || super::chain_carry::bind_self_animate_after_life_gain(&mut effects, sentence)
            || super::chain_carry::bind_destroy_typed_subset(&mut effects, sentence)
            || super::chain_carry::bind_return_exiled_to_owners_hands(&mut effects, sentence)
        {
            parser_trace("parse_effect_sentences:rider:bound-followup", sentence);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        if authored_sentence
            .first()
            .is_some_and(|token| token.is_word("if"))
            && !authored_sentence
                .iter()
                .any(|token| token.is_word("instead"))
            && authored_sentence
                .iter()
                .any(|token| token.kind == TokenKind::ManaGroup)
            && authored_words_contain_spent_to_cast(authored_sentence)
            && let Some(conditional) = effect_grammar::parse_conditional_sentence_family_lexed(
                authored_sentence,
                super::parse_effect_chain_lexed,
            )?
        {
            effects.extend(conditional);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        // Counter-unless is one typed verb phrase. The generic sentence
        // planner splits `unless` alternatives before subject/verb
        // primitives run, which can expose only the counter prefix and later
        // reinterpret the payment as a modal choice. Claim the complete
        // authored sentence while the counter target and payment are still
        // correlated.
        let authored_words = crate::lexer::parser_token_word_refs(authored_sentence);
        if crate::word_primitives::parse_sequence_prefix(&authored_words, &["counter"])
            && crate::word_primitives::sequence_occurs(&authored_words, &["unless"])
        {
            let authored_clause = trim_edge_punctuation(authored_sentence);
            effects.push(super::verb_handlers::parse_counter(&authored_clause[1..])?);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        // A complete target declaration can contain a historical `put`
        // relative clause. A multi-sentence program reaches this loop without
        // the outer single-sentence boundary, so apply the same typed ownership
        // proof here. The embedded history verb is then part of the target
        // declaration rather than a second zone-change action.
        if let Some(declarations) =
            super::clause_pattern_helpers::parse_choose_target_prelude_sentence(authored_sentence)?
        {
            effects.extend(declarations);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        if let Some(shape) =
            super::super::grammar::effects::clause_dispatch_shapes::parse_choose_target_shape(
                authored_sentence,
            )
            && !super::super::grammar::effects::chain_splitting::
                has_authored_comma_then_surface_tokens(authored_sentence)
            && !crate::word_primitives::sequence_occurs(
                &crate::lexer::parser_token_word_refs(authored_sentence),
                &["then"],
            )
            && super::super::util::parse_target_phrase(shape.target_tokens).is_ok()
        {
            effects.push(super::parse_effect_clause_lexed(authored_sentence)?);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        // A modal bullet can contain an ordinary effect followed by a
        // one-shot "the next time one or more ... enter" registration. The
        // complete bullet therefore enters this public multi-sentence loop;
        // keep the typed registration ahead of the broad single-sentence
        // grant parser when we reach its second sentence.
        if let Some(effect) = parse_next_batch_enter_with_counters(sentence)? {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        if let Some(rating_effects) = parse_outside_game_art_rating_sentence(sentence)? {
            effects.extend(rating_effects);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        if is_play_magic_subgame_sentence(sentences[sentence_idx].lexed()) {
            let consumes_nonwinner_sentence = sentences
                .get(sentence_idx + 1)
                .is_some_and(|sentence| is_subgame_half_life_nonwinner_sentence(sentence.lexed()));
            let nonwinner_effects = if consumes_nonwinner_sentence {
                {
                    vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::That,
                        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                            amount: Value::HalfLifeTotalRoundedUp(PlayerFilter::IteratedPlayer),
                        }),
                    )]
                }
            } else {
                Default::default()
            };
            effects.push(EffectAst::PlaySubgame { nonwinner_effects });
            carried_context = None;
            sentence_idx += if consumes_nonwinner_sentence { 2 } else { 1 };
            continue;
        }
        if let Some(restart) = parse_restart_game_sentence(sentences[sentence_idx].lexed())? {
            effects.push(restart);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        let sentence_text = crate::lexer::token_word_refs(sentence).join(" ");
        let _sentence_scope = parse_trace::scope(format!("effect sentence: \"{}\"", sentence_text));

        // A named vote-option sentence owns its comma-delimited `for each
        // <option> vote` prefix. In a multi-sentence program, `SentenceInput`
        // normalization can otherwise expose that prefix to generic sequence
        // splitting before the existing typed vote rule sees the complete
        // authored sentence. This is especially visible when the option
        // sentence starts with the ordering connective `Then`.
        if effect_grammar::parse_named_vote_option_effects_shape(authored_sentence).is_some()
            && let Some(effect) = super::dispatch_inner::parse_vote_subject_verb(authored_sentence)?
        {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        // A paid-cost conditional can also be a token-entry follow-up. Bind
        // that exact producer-relative shape before the generic paid-label
        // chain parser turns the entry words into a permanent ability grant.
        if try_bind_conditional_token_entry_followup(&mut effects, authored_sentence)? {
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        // The public multi-sentence family has several deliberately broad
        // whole-sentence and sequence probes before its ordinary
        // single-sentence fallback. A typed optional-cost condition must own
        // its complete consequence before any of those probes can claim a
        // later verb and discard the gate. Reuse the exact predicate proof
        // from the chain dispatcher rather than matching Gift (or any other
        // optional cost) by surface text.
        if super::chain_carry::leading_condition_is_paid_label(sentence)
            && let Ok(parsed) = super::parse_effect_chain_lexed(sentence)
            && let Some(conditional) = into_exact_single_conditional(parsed)
        {
            let mut conditional_effects = vec![conditional];
            let handled = {
                let mut state = SentenceDispatchState {
                    effects: &mut effects,
                    carried_context: &mut carried_context,
                };
                subject_verb_followups::post_rule_future_zone_and_self_replacement(
                    &mut state,
                    &sentences,
                    sentence_idx,
                    sentence,
                    &mut conditional_effects,
                )?
                .is_some()
            };
            if !handled {
                effects.append(&mut conditional_effects);
            }
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

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
        if let Some(duration_shape) =
            effect_grammar::parse_search_restriction_duration_shape_lexed(&leading_duration_tokens)?
            && duration_shape.placement
                == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            && should_apply_leading_duration_become_shortcut(&duration_shape.remainder)
        {
            let mut inner_effects = parse_effect_sentences_lexed(&duration_shape.remainder)?;
            let fully_scoped = !inner_effects.is_empty()
                && inner_effects.iter_mut().all(|effect| {
                    apply_leading_duration_to_entire_effect(effect, &duration_shape.duration)
                });
            if fully_scoped {
                effects.extend(preserve_fully_scoped_leading_duration_coordination(
                    inner_effects,
                ));
                carried_context = None;
                sentence_idx += 1;
                continue;
            }
            let mut applied = false;
            for effect in &mut inner_effects {
                applied |=
                    apply_leading_duration_to_become_effect(effect, &duration_shape.duration);
            }
            if applied {
                effects.append(&mut inner_effects);
                carried_context = None;
                sentence_idx += 1;
                continue;
            }
        }

        let direct_for_each_tokens = trim_edge_punctuation(sentence);
        let direct_for_each_words = crate::lexer::token_word_refs(&direct_for_each_tokens);
        let direct_other_player_stack_copy = crate::word_primitives::parse_sequence_prefix(
            &direct_for_each_words,
            &["each", "other", "player", "may", "copy"],
        ) && crate::word_primitives::sequence_occurs(
            &direct_for_each_words,
            &["copy", "that", "spell"],
        ) && crate::word_primitives::sequence_occurs(
            &direct_for_each_words,
            &["choose", "new", "targets", "for"],
        );
        let direct_quantified_token_creation_with_rules =
            parse_quantified_token_creation_with_embedded_rules(&direct_for_each_tokens)?;
        // Keep an authored per-player optional stack-copy loop outside the
        // generic subject/verb sequence routes. Those routes can legally type
        // the individual copy and retarget actions, but binding `each other
        // player` directly into their singular player fields loses both the
        // iteration and each player's optional choice at runtime.
        if direct_other_player_stack_copy
            && let Some(effect) = parse_for_each_player_clause(&direct_for_each_tokens)?
        {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        if let Some(effect) = direct_quantified_token_creation_with_rules {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        if crate::word_primitives::parse_any_sequence_prefix(
            &direct_for_each_words,
            &[
                &["each", "player", "return"],
                &["each", "player", "returns"],
            ],
        ) && let Some(effect) = parse_for_each_player_clause(&direct_for_each_tokens)?
        {
            // A destination-first return can contain a comma-separated card
            // type union. Give the quantified-player route the complete
            // sentence before generic coordination treats those type arms as
            // independent effect clauses.
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
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

        // Strip a token blueprint's quoted rule before broad sequence
        // recognition. A nested activated rule has its own colon and verbs;
        // allowing the outer sequence registry to inspect those tokens can
        // claim or reject the create sentence before the dedicated create
        // parser gets the rule-free token definition. The untouched `sentence`
        // is retained below so the quoted rule can be attached afterward.
        let embedded_rule_free_sentence = strip_embedded_token_rules_text(sentence);
        let stripped_embedded_rule = embedded_rule_free_sentence.as_slice() != sentence;
        let conditional_token_entry_boundary =
            is_conditional_token_entry_followup_sentence(authored_sentence)
                || sentences
                    .get(sentence_idx + 1)
                    .is_some_and(|next| is_conditional_token_entry_followup_sentence(next.lexed()));
        let joint_object_result_boundary = {
            let continuation = trim_edge_punctuation(
                super::super::token_primitives::strip_leading_if_you_do_lexed(authored_sentence),
            );
            continuation.len() < authored_sentence.len()
                && super::super::grammar::effects::subject_verb_registry_shapes::parse_joint_object_each_actions_shape(
                    &continuation,
                )
                .is_some()
        };
        if joint_object_result_boundary {
            let mut result_effects = super::parse_effect_chain_lexed(authored_sentence)?;
            if !matches!(result_effects.as_slice(), [EffectAst::Conditionals(ConditionalEffectAst::IfResult { .. })]) {
                return Err(CardTextError::ParseError(format!(
                    "joint-object result followup lost its result gate (clause: '{}')",
                    LexedClause::new(authored_sentence).text()
                )));
            }
            effects.append(&mut result_effects);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        // A choice-complement owns its comma-separated keep slots and the
        // sacrifice of the unchosen remainder even when it is the first
        // statement in a larger program. Preserve that typed statement here
        // so a later conditional can replace only its chooser.
        if let Some(effect) =
            super::dispatch_inner::parse_choice_complement_subject_verb(authored_sentence)?
        {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        if !stripped_embedded_rule
            && !conditional_token_entry_boundary
            && let Some(mut matched) = try_parse_document_program(&sentences, sentence_idx)?
        {
            let sequence_where_x = (0..matched.consumed_sentences).find_map(|offset| {
                sentences
                    .get(sentence_idx + offset)
                    .and_then(|sentence| where_x_value_from_tokens(sentence.lowered()))
            });
            if let Some(where_value) = sequence_where_x.as_ref() {
                let mut sequence_words = Vec::new();
                for offset in 0..matched.consumed_sentences {
                    if let Some(sentence) = sentences.get(sentence_idx + offset) {
                        sequence_words.extend(crate::lexer::token_word_refs(sentence.lowered()));
                    }
                }
                replace_unbound_x_in_effects_anywhere(
                    &mut matched.effects,
                    where_value,
                    &sequence_words.join(" "),
                )?;
            }
            super::chain_carry::bind_adjacent_shared_x_life_stat_values(
                &mut matched.effects,
                sentence,
            );
            super::chain_carry::dedupe_shared_target_player_draw_lose_x(
                &mut matched.effects,
                sentence,
            );
            preserve_leading_result_prefix_for_sequence(sentence, &mut matched.effects);
            let stage = if let Some(feature_tag) = matched.feature_tag {
                format!(
                    "parse_effect_sentences:document-program:{}:{feature_tag}",
                    matched.name
                )
            } else {
                format!("parse_effect_sentences:document-program:{}", matched.name)
            };
            parser_trace(stage.as_str(), sentence);
            parse_trace::event(format!(
                "sequence document program: {} -> {}",
                matched.name,
                summarize_effects(&matched.effects)
            ));
            parse_trace::event(format!(
                "effect-route: document-program recognizer={}",
                matched.name
            ));
            if let Some(where_value) = sequence_where_x {
                carried_where_x = Some(where_value);
            }
            effects.append(&mut matched.effects);
            sentence_idx += matched.consumed_sentences;
            continue;
        }
        if super::chain_carry::bind_no_regeneration_rider(&mut effects, sentence) {
            parser_trace("parse_effect_sentences:rider:no-regeneration", sentence);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }
        if let Some(mut exact_type_effects) =
            parse_tagged_exact_type_with_quoted_ability_sentence(sentence)?
        {
            // "If you do, return that card ... . It's ..." keeps the
            // characteristic-setting sentence inside the successful-result
            // branch and binds "it" to the object returned by that branch.
            if let Some(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                effects: branch, ..
            })) = effects.last_mut()
            {
                branch.append(&mut exact_type_effects);
            } else {
                effects.append(&mut exact_type_effects);
            }
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        let mut sentence_tokens = embedded_rule_free_sentence;
        sentence_tokens = trim_effect_sentence_edge_punctuation(&sentence_tokens);
        if sentence_tokens.is_empty() || crate::lexer::token_word_refs(&sentence_tokens).is_empty()
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

        // Conditional entry text refers back to the immediately preceding
        // typed token producer. Bind it before the broad characteristic/
        // keyword route can reinterpret "they enter tapped" as a standalone
        // granted ability and lose the producer correlation.
        if try_bind_conditional_token_entry_followup(&mut effects, &sentence_tokens)? {
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

        if let Some(mut replacement_and_cast) =
            future_zone_replacement_with_may_cast_followup(&sentence_tokens)
        {
            rebind_fight_death_replacement_target(
                &mut replacement_and_cast[0],
                effects.last(),
                &sentence_tokens,
            );
            effects.append(&mut replacement_and_cast);
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
        super::preserve_leading_result_coordination_lexed(
            &parse_plan.tokens,
            &mut sentence_effects,
        );
        preserve_plural_counter_antecedent(&parse_plan.tokens, &mut sentence_effects);
        let sentence_words = crate::lexer::token_word_refs(&parse_plan.tokens);
        let counter_verb = sentence_words.iter().position(|word| matches!(*word, "put" | "distribute"));
        let counter_follows_then = counter_verb.map_or_else(
            || sentence_words.contains(&"then"),
            |verb| sentence_words[..verb].iter().rev()
                .find(|word| matches!(**word, "then" | "and"))
                // With no preceding connector, retain a trailing "then"
                // joining the counter placement to its following action.
                .map_or_else(|| sentence_words.contains(&"then"), |word| *word == "then"),
        );
        if counter_follows_then {
            annotate_counter_followup_surface(
                &mut sentence_effects,
                ValueSurfaceHint::CounterFollowupThen,
            );
        } else if sentence_idx > 0 {
            // The parser already knows this effect came from a later authored
            // sentence. Preserve that boundary for every counter-producing
            // verb ("put", "distribute", and future equivalents); the
            // annotation helper is a no-op for non-counter effects.
            annotate_counter_followup_surface(
                &mut sentence_effects,
                ValueSurfaceHint::CounterFollowupSeparateSentence,
            );
        }
        if sentences
            .get(sentence_idx + parse_plan.consumed_sentences)
            .is_some_and(|next| starts_with_linked_ability_grant(next.lexed()))
        {
            annotate_counter_followup_surface(
                &mut sentence_effects,
                ValueSurfaceHint::CounterGrantSeparateSentence,
            );
        }
        // `sentence_tokens` may intentionally have had inline token rules
        // stripped before outer subject/verb dispatch. The resulting create
        // action is now known, so attach every quoted token ability from the
        // untouched source sentence under the created token's own identity.
        super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
            &mut sentence_effects,
            authored_sentence,
        );
        if let Some(predicate) = parse_plan.wrap_if_result {
            sentence_effects = vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate,
                effects: sentence_effects,
            })];
            carried_context = None;
        }
        if let Some(where_value) = sentence_where_x.as_ref() {
            replace_unbound_x_in_effects_anywhere(
                &mut sentence_effects,
                where_value,
                &crate::lexer::token_word_refs(&parse_plan.tokens).join(" "),
            )?;
        } else if let Some(where_value) = carried_where_x.as_ref() {
            replace_unbound_x_in_effects_anywhere(
                &mut sentence_effects,
                where_value,
                &crate::lexer::token_word_refs(&parse_plan.tokens).join(" "),
            )?;
        }
        super::chain_carry::bind_adjacent_shared_x_life_stat_values(
            &mut sentence_effects,
            &parse_plan.tokens,
        );
        super::chain_carry::dedupe_shared_target_player_draw_lose_x(
            &mut sentence_effects,
            &parse_plan.tokens,
        );
        maybe_append_trailing_that_much_life_loss(&mut sentence_effects, &parse_plan.tokens);
        maybe_append_reexile_returned_objects(&mut sentence_effects, &parse_plan.tokens);
        let previous_damage_target = effects.last().and_then(primary_damage_target_from_effect);
        bind_that_object_power_damage_subject(
            &mut sentence_effects,
            &sentence_tokens,
            previous_damage_target,
        );
        bind_target_controlled_source_damage_to_that_player(
            &mut sentence_effects,
            &sentence_tokens,
        );
        if crate::lexer::token_word_refs(&parse_plan.tokens)
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
                crate::lexer::token_word_refs(&parse_plan.tokens).join(" ")
            )));
        }
        for effect in &mut sentence_effects {
            if let Some(context) = carried_context {
                bind_definite_player_damage_to_carried_participant(
                    context,
                    &parse_plan.tokens,
                    effect,
                );
                maybe_apply_carried_player_with_clause(effect, context, &parse_plan.tokens);
            }
            if let Some(context) = explicit_player_for_carry(effect) {
                carried_context = Some(context);
            }
        }
        if sentence_effects.len() == 1
            && let Some(previous_effect) = effects.last()
            && let Some(effect) = sentence_effects.first_mut()
            && let EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate,
                effects: if_result_effects,
            }) = effect
        {
            if matches!(
                previous_effect,
                EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { .. })
                    | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { .. }),
                        ..
                    })
            ) {
                *predicate = match &*predicate {
                    // An UnlessPays effect happens when the payer declines and
                    // its consequence is carried out.  Modal result wording
                    // refers to the payment instead, so both polarities must
                    // be inverted before the result is bound to that effect.
                    IfResultPredicate::Did => IfResultPredicate::DidNot,
                    IfResultPredicate::DidNot => IfResultPredicate::Did,
                    other => other.clone(),
                };
            }
            if let Some(previous_target) = primary_damage_target_from_effect(previous_effect) {
                replace_it_damage_target_in_effects(
                    if_result_effects.as_mut_slice(),
                    &previous_target,
                );
                if matches!(previous_target, TargetAst::AnyTarget(_))
                    && crate::word_primitives::sequence_occurs(
                        &crate::lexer::token_word_refs(&parse_plan.tokens),
                        &["the", "permanent", "or", "player"],
                    )
                {
                    replace_definite_prior_damage_recipient_in_effects(
                        if_result_effects.as_mut_slice(),
                    );
                }
            }
        }
        let sentence_words = crate::lexer::token_word_refs(&parse_plan.tokens);
        let is_if_player_does = crate::word_primitives::parse_sequence_prefix(
            &sentence_words,
            &["if", "a", "player", "does"],
        );
        if is_if_player_does
            && matches!(effects.last(), Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { .. })))
            && let [effect] = sentence_effects.as_mut_slice()
            && let EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate,
                effects: followups,
            }) = effect.clone()
        {
            // Preserve the participant identity from an each-player action.
            // The resulting per-player branch is lowered with that player as
            // IteratedPlayer, and the runtime can correlate it with the
            // antecedent's PlayerCounts outcome.
            *effect = EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDid {
                effects: followups,
                predicate: None,
                result_predicate: predicate,
            });
        }
        {
            let mut state = SentenceDispatchState {
                effects: &mut effects,
                carried_context: &mut carried_context,
            };
            match run_post_parse_followup_registry(
                &mut state,
                &sentences,
                sentence_idx,
                &sentence_tokens,
                &mut sentence_effects,
            )? {
                Some(PostParseFollowupResult::Handled { consumed_sentences }) => {
                    parse_trace::event(format!(
                        "post-parse followup handled sentence(s): {consumed_sentences}"
                    ));
                    sentence_idx += consumed_sentences;
                    continue;
                }
                Some(PostParseFollowupResult::Annotated) | None => {}
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

fn authored_words_contain_spent_to_cast(tokens: &[OwnedLexToken]) -> bool {
    crate::word_primitives::sequence_occurs(
        &crate::lexer::parser_token_word_refs(tokens),
        &["spent", "to", "cast", "this", "spell"],
    )
}

fn parse_outside_game_art_rating_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use effect_grammar::dispatch_entry_shapes::OutsideGameArtRatingSentenceKind;

    let Some(kind) =
        effect_grammar::dispatch_entry_shapes::parse_outside_game_art_rating_tokens(tokens)
    else {
        return Ok(None);
    };

    match kind {
        OutsideGameArtRatingSentenceKind::Request => {
            Ok(Some(vec![EffectAst::subject_verb_choose_named_option(
                PlayerAst::You,
                (1..=5).map(|rating| rating.to_string()).collect(),
            )]))
        }
        OutsideGameArtRatingSentenceKind::ResultTrigger => {
            let Some(comma) = tokens
                .iter()
                .position(|token| token.kind == TokenKind::Comma)
            else {
                return Err(CardTextError::ParseError(format!(
                    "art-rating result trigger is missing its effect separator (clause: '{}')",
                    crate::lexer::render_token_slice(tokens)
                )));
            };
            let result_tokens = trim_edge_punctuation(&tokens[comma + 1..]);
            if result_tokens.is_empty() {
                return Err(CardTextError::ParseError(
                    "art-rating result trigger is missing its effect".to_string(),
                ));
            }
            let result_effects = parse_effect_sentences_lexed(&result_tokens)?;
            Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                predicate: IfResultPredicate::Did,
                effects: result_effects,
            })]))
        }
    }
}

fn parse_delegated_categorical_library_choice(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    const CATEGORIES: &[&str] = &[
        "an",
        "opponent",
        "chooses",
        "from",
        "among",
        "them",
        "a",
        "creature",
        "card",
        "a",
        "land",
        "card",
        "and",
        "a",
        "noncreature",
        "nonland",
        "card",
    ];
    // A categorical "from among them" list selects one card from each named
    // bucket. Keep those as three executable choices that append to one
    // result tag; a union would select only one card and make the first two
    // categories an impossible type intersection.
    let complete_program = crate::word_primitives::parse_sequence_prefix(
        &source_words,
        &["reveal", "the", "cards", "in", "your", "library"],
    ) && crate::word_primitives::parse_sequence_suffix(
        &source_words,
        &[
            "you", "put", "the", "chosen", "cards", "into", "your", "hand", "then", "shuffle",
        ],
    ) && crate::word_primitives::sequence_occurs(&source_words, CATEGORIES);
    if !complete_program
        && !crate::word_primitives::parse_sequence_suffix(&source_words, CATEGORIES)
        && !crate::word_primitives::parse_sequence_suffix(
            &source_words,
            &[
                "opponent",
                "chooses",
                "from",
                "among",
                "them",
                "a",
                "creature",
                "card",
                "a",
                "land",
                "card",
                "and",
                "a",
                "noncreature",
                "nonland",
                "card",
            ],
        )
        && !crate::word_primitives::parse_sequence_suffix(
            &source_words,
            &[
                "chooses",
                "from",
                "among",
                "them",
                "a",
                "creature",
                "card",
                "a",
                "land",
                "card",
                "and",
                "a",
                "noncreature",
                "nonland",
                "card",
            ],
        )
        && !crate::word_primitives::parse_sequence_suffix(
            &source_words,
            &[
                "from",
                "among",
                "them",
                "a",
                "creature",
                "card",
                "a",
                "land",
                "card",
                "and",
                "a",
                "noncreature",
                "nonland",
                "card",
            ],
        )
    {
        return None;
    }
    let pool = crate::tag::CompilerReferenceTag::RevealedLibrary.bind();
    let result = crate::tag::CompilerReferenceTag::ChosenObjects.bind();
    let chooser_tag = crate::tag::CompilerReferenceTag::DelegatedLibraryChooser.bind();
    let tagged_pool = |filter: ObjectFilter| {
        filter
            .in_zone(Zone::Library)
            .match_tagged(pool.clone(), TaggedOpbjectRelation::IsTaggedObject)
    };
    let choice = |filter| EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter: tagged_pool(filter),
        count: ChoiceCount::exactly(1),
        count_value: None,
        player: PlayerAst::That,
        tag: result.clone(),
    });
    let delegated_choices = vec![
        EffectAst::subject_verb_choose_player(
            PlayerAst::You,
            PlayerFilter::Opponent,
            chooser_tag,
            false,
            0,
        ),
        EffectAst::Sequence {
            effects: vec![
                choice(ObjectFilter::default().with_type(CardType::Creature)),
                choice(ObjectFilter::default().with_type(CardType::Land)),
                choice(
                    ObjectFilter::default()
                        .without_type(CardType::Creature)
                        .without_type(CardType::Land),
                ),
            ],
        },
    ];
    if !complete_program {
        return Some(delegated_choices);
    }

    let mut effects = vec![
        EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::default().owned_by(PlayerFilter::You),
            vec![Zone::Library],
            pool.clone(),
        ),
        EffectAst::subject_verb_reveal_tagged(pool),
    ];
    effects.extend(delegated_choices);
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(result, None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
    ));
    Some(effects)
}

fn reveal_collection_tag(effects: &[EffectAst]) -> Option<TagKey> {
    fn from_effect(effect: &EffectAst) -> Option<TagKey> {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { tag }),
                ..
            }) => Some(tag.clone().into()),
            EffectAst::Sequence { effects }
            | EffectAst::CommaThen { effects }
            | EffectAst::Coordinated { effects, .. }
            | EffectAst::SourceSentence { effects, .. } => effects.iter().find_map(from_effect),
            _ => None,
        }
    }
    effects.iter().find_map(from_effect)
}

/// Preserve a searched/revealed pool across a delegated two-card partition.
/// The ordinary subject parser treats “that player chooses two of them” as
/// cards controlled by the opponent; here the explicit pronoun instead binds
/// the choice to the revealed collection, and the terminal “rest” becomes an
/// executable set difference rather than an unpopulated literal tag.
fn parse_complete_delegated_search_partition(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [search_sentence, choice_sentence, movement_sentence] = sentences.as_slice() else {
        return Ok(None);
    };
    let choice_words = crate::lexer::parser_token_word_refs(choice_sentence);
    let choice_surface = if crate::word_primitives::parse_sequence_complete(
        &choice_words,
        &["an", "opponent", "chooses", "two", "of", "them"],
    ) {
        true
    } else if crate::word_primitives::parse_sequence_complete(
        &choice_words,
        &["an", "opponent", "chooses", "two", "of", "those", "cards"],
    ) {
        false
    } else {
        return Ok(None);
    };
    let movement_words = crate::lexer::parser_token_word_refs(movement_sentence);
    let chosen_to_hand = if crate::word_primitives::parse_sequence_complete(
        &movement_words,
        &[
            "put", "the", "chosen", "cards", "into", "your", "hand", "and", "shuffle", "the",
            "rest", "into", "your", "library",
        ],
    ) {
        true
    } else if crate::word_primitives::parse_sequence_complete(
        &movement_words,
        &[
            "shuffle", "the", "chosen", "cards", "into", "your", "library", "and", "put", "the",
            "rest", "into", "your", "hand",
        ],
    ) {
        false
    } else {
        return Ok(None);
    };
    let first_words = crate::lexer::parser_token_word_refs(search_sentence);
    if !crate::word_primitives::parse_sequence_prefix(&first_words, &["search", "your", "library"])
        || !crate::word_primitives::any_sequence_occurs(&first_words, &[&["reveal"], &["reveals"]])
    {
        return Ok(None);
    }

    let mut effects = parse_effect_sentences_lexed(search_sentence)?;
    let Some(pool) = reveal_collection_tag(&effects) else {
        return Ok(None);
    };
    let chooser_tag = crate::tag::CompilerReferenceTag::DelegatedLibraryChooser.bind();
    let chosen = crate::tag::CompilerReferenceTag::ChosenObjects.bind();
    effects.push(EffectAst::subject_verb_choose_player(
        PlayerAst::You,
        PlayerFilter::Opponent,
        chooser_tag,
        false,
        0,
    ));
    effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter: ObjectFilter::tagged(pool.clone()),
        count: ChoiceCount::exactly(2),
        count_value: None,
        player: PlayerAst::That,
        tag: chosen.clone(),
    }));

    let rest = TargetAst::Object(
        ObjectFilter::tagged(pool).not_tagged(chosen.clone()),
        None,
        None,
    );
    let chosen_target = TargetAst::Tagged(chosen, None);
    let hand_move = |target| {
        EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )
    };
    let library_move =
        |target| EffectAst::subject_verb_shuffle_objects_into_library(PlayerAst::You, target);
    let (first_move, second_move) = if chosen_to_hand {
        (hand_move(chosen_target), library_move(rest))
    } else {
        (library_move(chosen_target), hand_move(rest))
    };
    effects.push(EffectAst::Coordinated {
        effects: vec![first_move, second_move],
        leading_duration: false,
        result_conjunction: false,
    });

    // This boolean is intentionally consumed only as a lexical proof that the
    // two supported authored antecedents were distinguished above. The first
    // sentence's sequence surface remains the renderer's source of “them” vs
    // “those cards”.
    let _ = choice_surface;
    Ok(Some(effects))
}

#[cfg(test)]
mod delegated_categorical_library_choice_tests {
    use super::*;

    #[test]
    fn full_sentence_keeps_three_choices_in_one_shared_result_collection() {
        let tokens = crate::lexer::lex_line(
            "An opponent chooses from among them a creature card, a land card, and a noncreature, nonland card.",
            0,
        )
        .expect("categorical delegated choice should lex");
        let effects = parse_effect_sentences_lexed(&tokens)
            .expect("categorical delegated choice should parse");
        let debug = format!("{effects:#?}");

        assert_eq!(debug.matches("ChooseObjects").count(), 3, "{debug}");
        assert_eq!(debug.matches("__chosen_objects__").count(), 3, "{debug}");
        assert_eq!(debug.matches("__revealed_library__").count(), 3, "{debug}");
        assert!(debug.contains("ChoosePlayer"), "{debug}");
    }
}

/// Preserve the player identity established by an each-player physical coin
/// flip for a following heads/tails clause. This sequence has a distinct
/// result model from a called coin flip: heads is the producer result and
/// tails is its complement for the same iterated player.
pub(super) fn parse_each_player_coin_face_sequence(
    sentence_parts: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [flip_sentence, followup_sentence] = sentence_parts else {
        return Ok(None);
    };
    let followup_words = crate::lexer::parser_token_word_refs(followup_sentence);
    let result_predicate = match followup_words.get(..7) {
        Some(["each", "player", "whose", "coin", "comes", "up", "heads"]) => IfResultPredicate::Did,
        Some(["each", "player", "whose", "coin", "comes", "up", "tails"]) => {
            IfResultPredicate::DidNot
        }
        _ => return Ok(None),
    };

    let mut flip = parse_effect_sentences_lexed_inner(flip_sentence)?;
    let view = crate::lexer::TokenWordView::new(followup_sentence);
    let tail_start = view.token_index_after_words(7).ok_or_else(|| {
        CardTextError::ParseError("coin-face follow-up is missing its action".to_string())
    })?;
    let mut normalized_followup = followup_sentence[..2].to_vec();
    normalized_followup.extend_from_slice(&followup_sentence[tail_start..]);
    let followup = parse_effect_sentences_lexed_inner(&normalized_followup)?;

    let [EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects: flip_body })] = flip.as_mut_slice() else {
        return Ok(None);
    };
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. })] = flip_body.as_mut_slice()
    else {
        return Ok(None);
    };
    if !matches!(action, SubjectVerbActionAst::Random(RandomActionAst::FlipCoin)) {
        return Ok(None);
    }
    let [
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: followup_body,
        }),
    ] = followup.as_slice()
    else {
        return Ok(None);
    };

    *action = SubjectVerbActionAst::Random(RandomActionAst::FlipCoinFaceOnly);
    flip.push(EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDid {
        effects: followup_body.clone(),
        predicate: None,
        result_predicate,
    }));
    Ok(Some(flip))
}

fn parse_complete_simple_draw_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(tokens).is_some() {
        return Ok(None);
    }
    // `You may draw ... if ...` has a permission envelope outside the
    // trailing condition. The standalone draw shortcut otherwise parses
    // `you may` as an ordinary subject and silently drops the optionality.
    if effect_grammar::clause_dispatch_shapes::parse_leading_may_shape(tokens).is_some() {
        return Ok(None);
    }
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(tokens)
        || super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![tokens]).len() > 1
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens).len() > 1
        // A comma-separated sibling action ("draw two cards, lose 2 life")
        // continues the chain; this shortcut owns only a single draw
        // sentence, and its trailing-clause grammar cannot read that action.
        || super::lex_chain_helpers::split_segments_on_comma_effect_head_lexed(vec![tokens]).len()
            > 1
    {
        return Ok(None);
    }
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(draw_idx) = crate::slice_primitives::select_position(tokens, |token| {
        token.is_any_word(&["draw", "draws"])
    }) else {
        return Ok(None);
    };
    if tokens[..draw_idx].iter().any(|token| token.kind == TokenKind::Comma
        || token.is_any_word(&["if", "unless", "when", "whenever", "until", "then", "may"]))
    {
        return Ok(None);
    }
    let subject = if draw_idx == 0 {
        None
    } else {
        let subject = crate::util::parse_subject(&tokens[..draw_idx]);
        if !matches!(
            subject,
            SubjectAst::Player(_) | SubjectAst::TriggeringSourceController
        ) {
            return Ok(None);
        }
        Some(subject)
    };
    super::verb_handlers::parse_draw(&tokens[draw_idx + 1..], subject).map(Some)
}

pub(crate) fn parse_complete_investigate_statement(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("investigate"))
    {
        return Ok(None);
    }
    super::creation_handlers::parse_investigate(&tokens[1..], None).map(|effect| Some(vec![effect]))
}

pub(crate) fn parse_complete_simple_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // A sentence with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(tokens).is_some() {
        return Ok(None);
    }
    // A participant-scoped sentence ("each player ...") is the participant loop's.
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(tokens).is_some() {
        return Ok(None);
    }
    // "N target players each ..." is the target-players loop's.
    if effect_grammar::for_each_shapes::parse_for_each_target_players_shape(tokens)
        .is_some_and(|shape| crate::util::parse_target_phrase(shape.target_tokens).is_ok())
    {
        return Ok(None);
    }
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(tokens)
        || super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![tokens]).len() > 1
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens).len() > 1
        // A comma-separated sibling action ("gain 2 life, draw two cards")
        // continues the chain; these shortcuts own only a single-action
        // sentence and would feed the later action to the first verb's
        // trailing-clause grammar.
        || super::lex_chain_helpers::split_segments_on_comma_effect_head_lexed(vec![tokens]).len()
            > 1
    {
        return Ok(None);
    }
    if let Some(effect) = parse_complete_simple_draw_sentence(tokens)? {
        return Ok(Some(effect));
    }

    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(gain_idx) = crate::slice_primitives::select_position(tokens, |token| {
        token.is_any_word(&["gain", "gains"])
    }) else {
        return Ok(None);
    };
    if tokens[..gain_idx]
        .iter()
        .any(|token| token.kind == TokenKind::Comma)
    {
        return Ok(None);
    }
    if tokens[..gain_idx].iter().any(|token| token.is_word("may")) {
        // An optional action keeps its `may` scope; the may-aware chain
        // routes own the wrapper.
        return Ok(None);
    }
    let subject = if gain_idx == 0 {
        None
    } else {
        let subject = crate::util::parse_subject(&tokens[..gain_idx]);
        if !matches!(
            subject,
            SubjectAst::Player(_) | SubjectAst::TriggeringSourceController
        ) {
            return Ok(None);
        }
        Some(subject)
    };
    if gain_idx > 3
        && !matches!(
            subject,
            Some(SubjectAst::Player(
                PlayerAst::PlayerToYourLeft | PlayerAst::PlayerToYourRight
            ))
        )
    {
        return Ok(None);
    }
    let gain_tokens = &tokens[gain_idx + 1..];
    if gain_tokens
        .first()
        .is_some_and(|token| token.is_word("control"))
    {
        return super::verb_handlers::parse_gain_control(gain_tokens, subject).map(Some);
    }
    if gain_tokens.iter().any(|token| token.is_word("life")) {
        return super::verb_handlers::parse_gain_life(gain_tokens, subject).map(Some);
    }
    Ok(None)
}

pub(super) fn parse_complete_simple_mill_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(tokens).is_some() {
        return Ok(None);
    }
    // "N target players each mill" is the target-players loop's.
    if effect_grammar::for_each_shapes::parse_for_each_target_players_shape(tokens)
        .is_some_and(|shape| crate::util::parse_target_phrase(shape.target_tokens).is_ok())
    {
        return Ok(None);
    }
    if crate::lexer::split_lexed_sentences(tokens).len() == 1
        && let Some(participant) =
            effect_grammar::for_each_shapes::parse_participant_clause_shape(tokens)
        && participant.participant_is_actor
        && participant
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_any_word(&["mill", "mills"]))
    {
        if let Some(effect) = super::for_each_helpers::parse_for_each_player_clause(tokens)? {
            return Ok(Some(effect));
        }
        if let Some(effect) = super::for_each_helpers::parse_for_each_opponent_clause(tokens)? {
            return Ok(Some(effect));
        }
    }
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(tokens).is_some() {
        // The simple mill leaf cannot bind a terminal `where X is ...`
        // definition. Leave that complete program to the typed effect-chain
        // compositor, which parses the mill action first and then replaces
        // its unbound X with the authored value.
        return Ok(None);
    }
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(tokens)
        || super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![tokens]).len() > 1
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens).len() > 1
        || super::lex_chain_helpers::split_segments_on_comma_effect_head_lexed(vec![tokens]).len() > 1
    {
        return Ok(None);
    }
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(mill_idx) = crate::slice_primitives::select_position(tokens, |token| {
        token.is_any_word(&["mill", "mills"])
    }) else {
        return Ok(None);
    };
    if tokens[..mill_idx].iter().any(|token| token.kind == TokenKind::Comma
        || token.is_any_word(&["may", "if", "unless", "when", "whenever", "then"]))
    {
        return Ok(None);
    }
    let subject = if mill_idx == 0 {
        None
    } else {
        let subject = crate::util::parse_subject(&tokens[..mill_idx]);
        if !matches!(
            subject,
            SubjectAst::Player(_) | SubjectAst::TriggeringSourceController
        ) {
            return Ok(None);
        }
        Some(subject)
    };
    super::zone_handlers::parse_mill(&tokens[mill_idx + 1..], subject).map(Some)
}

fn simple_card_type_word(word: &str) -> Option<CardType> {
    Some(match word {
        "artifact" => CardType::Artifact,
        "battle" => CardType::Battle,
        "creature" => CardType::Creature,
        "enchantment" => CardType::Enchantment,
        "instant" => CardType::Instant,
        "kindred" | "tribal" => CardType::Kindred,
        "land" => CardType::Land,
        "planeswalker" => CardType::Planeswalker,
        "sorcery" => CardType::Sorcery,
        _ => return None,
    })
}

fn push_simple_controlled_object_choice(effects: &mut Vec<EffectAst>, card_type: CardType) {
    let mut filter = ObjectFilter::default();
    filter.card_types.push(card_type);
    filter.controller = Some(PlayerFilter::You);
    effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player: PlayerAst::You,
        tag: crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
    }));
}

fn push_plain_iterated_copy_of_it(effects: &mut Vec<EffectAst>) {
    effects.push(EffectAst::subject_verb_become_copy(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        Until::EndOfTurn,
        false,
        None,
        None,
        Vec::new(),
        Vec::new(),
        Default::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        None,
    ));
}

fn push_each_other_controlled_type_copy(effects: &mut Vec<EffectAst>, card_type: CardType) {
    let mut filter = ObjectFilter::default();
    filter.card_types.push(card_type);
    filter.controller = Some(PlayerFilter::You);
    filter.other = true;
    let mut copy_effects = Vec::with_capacity(1);
    push_plain_iterated_copy_of_it(&mut copy_effects);
    effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachObject {
        filter,
        effects: copy_effects,
    }));
}

fn parse_complete_simple_controlled_object_choice(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let ["choose", article, card_type_word, "you", "control"] = words.as_slice() else {
        return None;
    };
    if !matches!(*article, "a" | "an" | "one") {
        return None;
    }
    let card_type = simple_card_type_word(card_type_word)?;
    let mut effects = Vec::with_capacity(1);
    push_simple_controlled_object_choice(&mut effects, card_type);
    Some(effects)
}

/// Parse a persistent object choice followed by a mass copy instruction.
///
/// The second sentence is represented as a `ForEachObject` around one
/// `BecomeCopy`; lowering turns that exact typed shape into one continuous
/// effect and binds both `other` and `that creature` to the preceding choice.
/// Claiming the two-sentence envelope here avoids asking the whole-program
/// compatibility registry to rediscover a relationship already proved by the
/// choice-filter and become-clause grammars.
fn controlled_type_choice_then_each_other_copy_shape(
    choice_sentence: &[OwnedLexToken],
    copy_sentence: &[OwnedLexToken],
) -> Option<(CardType, CardType)> {
    let choice_words = crate::lexer::parser_token_word_refs(choice_sentence);
    let copy_words = crate::lexer::parser_token_word_refs(copy_sentence);
    let ["choose", article, chosen_type_word, "you", "control"] = choice_words.as_slice() else {
        return None;
    };
    if !matches!(*article, "a" | "an" | "one") {
        return None;
    }
    let [
        "each",
        "other",
        affected_type_word,
        "you",
        "control",
        become_word,
        "a",
        "copy",
        "of",
        "that",
        source_type_word,
        "until",
        "end",
        "of",
        "turn",
    ] = copy_words.as_slice()
    else {
        return None;
    };
    if !matches!(*become_word, "become" | "becomes") {
        return None;
    }
    let chosen_type = simple_card_type_word(chosen_type_word)?;
    let affected_type = simple_card_type_word(affected_type_word)?;
    (simple_card_type_word(source_type_word)? == chosen_type)
        .then_some((chosen_type, affected_type))
}

pub(crate) fn is_controlled_type_choice_then_each_other_copy_shape(
    choice_sentence: &[OwnedLexToken],
    copy_sentence: &[OwnedLexToken],
) -> bool {
    controlled_type_choice_then_each_other_copy_shape(choice_sentence, copy_sentence).is_some()
}

fn parse_choose_then_each_other_becomes_copy(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [choice_sentence, copy_sentence] = sentences else {
        return Ok(None);
    };
    let Some((chosen_type, affected_type)) =
        controlled_type_choice_then_each_other_copy_shape(choice_sentence, copy_sentence)
    else {
        return Ok(None);
    };

    let mut effects = Vec::with_capacity(2);
    push_simple_controlled_object_choice(&mut effects, chosen_type);
    push_each_other_controlled_type_copy(&mut effects, affected_type);
    Ok(Some(effects))
}

fn complete_simple_face_down_exile_top_count(tokens: &[OwnedLexToken]) -> Option<i32> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    match words.as_slice() {
        [
            "exile",
            "the",
            "top",
            "card",
            "of",
            "your",
            "library",
            "face",
            "down",
        ] => Some(1),
        [
            "exile",
            "the",
            "top",
            count,
            "cards",
            "of",
            "your",
            "library",
            "face",
            "down",
        ] => crate::util::parse_number_word_i32(count).filter(|count| *count > 0),
        _ => None,
    }
}

fn push_complete_simple_face_down_exile_top(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
    count: i32,
) {
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
            count: Value::Fixed(count),
            surface: None,
            tags: vec![crate::tag::TagRef::of(crate::util::helper_tag_for_tokens(tokens, "exiled"))],
            accumulated_tags: Vec::new(),
            face_down: true,
        }),
    ));
}

fn build_complete_simple_face_down_exile_top(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let count = complete_simple_face_down_exile_top_count(tokens)?;
    let mut effects = Vec::with_capacity(1);
    push_complete_simple_face_down_exile_top(&mut effects, tokens, count);
    Some(effects)
}

fn complete_simple_otherwise_face_down_exile_top_shape(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], i32)> {
    let (otherwise_tokens, exile_tokens) = grammar::split_lexed_once_on_comma(tokens)?;
    let otherwise_words = crate::lexer::parser_token_word_refs(otherwise_tokens);
    if !crate::word_primitives::parse_sequence_complete(&otherwise_words, &["otherwise"]) {
        return None;
    }
    Some((
        exile_tokens,
        complete_simple_face_down_exile_top_count(exile_tokens)?,
    ))
}

fn build_complete_simple_otherwise_face_down_exile_top(
    exile_tokens: &[OwnedLexToken],
    count: i32,
) -> Vec<EffectAst> {
    let mut nested = Vec::with_capacity(1);
    push_complete_simple_face_down_exile_top(&mut nested, exile_tokens, count);
    vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::Otherwise,
        effects: nested,
    })]
}

fn secret_choices_match_conditional_source_type(tokens: &[OwnedLexToken]) -> Option<&str> {
    let (predicate_tokens, consequence_tokens) = grammar::split_lexed_once_on_comma(tokens)?;
    let predicate_words = crate::lexer::parser_token_word_refs(predicate_tokens);
    if !crate::word_primitives::parse_any_sequence_complete(
        &predicate_words,
        &[
            &["if", "they", "match"],
            &["if", "those", "choices", "match"],
        ],
    ) {
        return None;
    }
    let consequence_words = crate::lexer::parser_token_word_refs(consequence_tokens);
    let [
        "sacrifice",
        "this",
        source_type,
        "and",
        "put",
        "all",
        "cards",
        "exiled",
        "with",
        "it",
        "into",
        "their",
        owner_word,
        "hands",
    ] = consequence_words.as_slice()
    else {
        return None;
    };
    if !matches!(*owner_word, "owner's" | "owners" | "owners'")
        || simple_card_type_word(source_type).is_none()
    {
        return None;
    }
    Some(source_type)
}

fn push_secret_choices_match_sacrifice(
    members: &mut Vec<crate::model::CoordinationMemberAst>,
    source_type: &str,
) {
    let effects = vec![EffectAst::subject_verb_sacrifice(
        PlayerAst::You,
        ObjectFilter::source_with_surface(SourceReferenceSurface::ThisPermanentType(format!(
            "this {source_type}"
        ))),
        1,
        None,
    )];
    members.push(crate::model::CoordinationMemberAst::new(effects));
}

fn push_secret_choices_match_return(members: &mut Vec<crate::model::CoordinationMemberAst>) {
    let mut source_exiled =
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind())
            .in_zone(Zone::Exile);
    source_exiled.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::All));
    source_exiled.set_explicit_card_noun(true);
    let effects = vec![
        EffectAst::subject_verb_move_all_to_zone(
            TargetAst::Object(source_exiled, None, None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )
        .with_move_to_zone_actor_surface(PlayerAst::You),
    ];
    members.push(crate::model::CoordinationMemberAst::new(effects));
}

fn build_secret_choices_match_conditional(source_type: &str) -> EffectAst {
    let mut members = Vec::with_capacity(2);
    push_secret_choices_match_sacrifice(&mut members, source_type);
    push_secret_choices_match_return(&mut members);
    let if_true = vec![EffectAst::Coordination(crate::model::CoordinationAst {
        kind: crate::model::CoordinationKindAst::Carry,
        members,
        boundaries: vec![crate::model::CoordinationBoundaryAst {
            operator: crate::model::CoordinationOperatorAst::And,
            ordering: crate::model::EffectOrderingAst::Unordered,
            dependency: crate::model::EffectDependencyAst::Independent,
            carries: Vec::new(),
            provenance: None,
        }],
        provenance: None,
    })];
    EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::SecretChoicesMatch,
        if_true,
        if_false: Vec::new(),
    })
}

fn build_secret_choices_match_conditional_effects(source_type: &str) -> Vec<EffectAst> {
    vec![build_secret_choices_match_conditional(source_type)]
}

fn parse_complete_serial_create_statement(
    sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((verb, verb_word_idx)) = super::find_verb(sentence) else {
        return Ok(None);
    };
    if !matches!(verb, super::chain_carry::Verb::Create)
        || sentence
            .first()
            .is_some_and(|token| token.is_any_word(&["if", "when", "whenever", "at"]))
    {
        return Ok(None);
    }
    let Some(verb_token_idx) =
        crate::lexer::TokenWordView::new(sentence).map_word_to_token_start(verb_word_idx)
    else {
        return Ok(None);
    };
    if sentence[..verb_token_idx]
        .iter()
        .any(|token| token.kind == TokenKind::Comma)
    {
        return Ok(None);
    }
    let Some(operands) = effect_grammar::parse_serial_create_token_operand_list_tokens(
        &sentence[verb_token_idx + 1..],
    ) else {
        return Ok(None);
    };

    let subject_tokens = &sentence[..verb_token_idx];
    let mut members = Vec::with_capacity(operands.len());
    for operand in operands {
        let mut create_tokens = Vec::with_capacity(operand.len() + 1);
        create_tokens.push(OwnedLexToken::synthetic_word("create"));
        create_tokens.extend_from_slice(operand);
        let subject =
            (!subject_tokens.is_empty()).then(|| crate::util::parse_subject(subject_tokens));
        let effect = super::creation_handlers::parse_create(&create_tokens, subject)?;
        members.push(crate::model::CoordinationMemberAst::new(vec![effect]));
    }
    let mut boundaries = Vec::with_capacity(members.len().saturating_sub(1));
    for index in 1..members.len() {
        boundaries.push(crate::model::CoordinationBoundaryAst {
            operator: if index + 1 == members.len() {
                crate::model::CoordinationOperatorAst::And
            } else {
                crate::model::CoordinationOperatorAst::Comma
            },
            ordering: crate::model::EffectOrderingAst::Unordered,
            dependency: crate::model::EffectDependencyAst::Independent,
            carries: Vec::new(),
            provenance: None,
        });
    }
    Ok(Some(vec![EffectAst::Coordination(
        crate::model::CoordinationAst {
            kind: crate::model::CoordinationKindAst::Conjunction,
            members,
            boundaries,
            provenance: None,
        },
    )]))
}

#[inline(never)]
pub(crate) fn parse_complete_create_statement(
    sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if effect_grammar::delayed_step_shapes::parse_delayed_timing_marker_shape(sentence)
        .is_some_and(|marker| marker.start_word > 0)
    {
        return Ok(None);
    }
    // A verb inside a granted ability belongs to the quoted rule, not to
    // this statement's actor. Keep the enclosing grant intact.
    if let Some(create_idx) = sentence.iter().position(|token| token.is_any_word(&["create", "creates"]))
        && (sentence[..create_idx].iter().filter(|token| token.kind == TokenKind::Quote).count() % 2 == 1
            || sentence[..create_idx].iter().any(|token| token.is_any_word(&["choose", "chooses"]))) {
        return Ok(None);
    }
    // A die-roll row ("10—20 | Create ...") is the result-prefix statement's; read alone, the range was lost.
    if crate::grammar::structure::split_leading_numeric_result_prefix_lexed(
        crate::util::trim_edge_punctuation_tokens(sentence),
    )
    .is_some()
    {
        return Ok(None);
    }
    // A creation with a trailing condition is the conditional statement's; read alone, the condition was lost.
    if crate::grammar::structure::split_trailing_if_clause_lexed(sentence).is_some() {
        return Ok(None);
    }
    {
        let sentences = crate::lexer::split_lexed_sentences(sentence);
        if sentences.len() > 1
            && sentences.first().is_some_and(|first| {
                !first
                    .iter()
                    .any(|token| token.is_any_word(&["create", "creates"]))
            })
        {
            // The complete create program owns lines that begin with the
            // creation (optionally followed by quoted token rules). A leading
            // unrelated resolution sentence belongs to sentence dispatch.
            return Ok(None);
        }
    }
    if crate::lexer::split_lexed_sentences(sentence).len() == 1
        && let Some(participant) =
            effect_grammar::for_each_shapes::parse_participant_clause_shape(sentence)
        && participant.participant_is_actor
        && participant
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_any_word(&["create", "creates"]))
    {
        if let Some(effect) = super::for_each_helpers::parse_for_each_player_clause(sentence)? {
            return Ok(Some(vec![effect]));
        }
        if let Some(effect) = super::for_each_helpers::parse_for_each_opponent_clause(sentence)? {
            return Ok(Some(vec![effect]));
        }
    }
    if super::chain_carry::parse_leading_player_may_lexed(sentence).is_some() {
        // Permission owns the complete action envelope.  Parsing only the
        // later `create` verb would turn its player phrase into an opaque
        // subject and silently erase both `may` and the typed actor.
        return Ok(None);
    }
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(sentence).is_some() {
        // Any other participant-scoped creation ("each player who controls an
        // artifact creates ...") is the participant loop's: read here, the
        // participant was lost.
        // The target exclusion and quantified actor belong to the enclosing
        // participant loop.  Treating the prefix as one ordinary creator
        // would discard both the announced target and the fanout scope.
        return Ok(None);
    }
    if let Some(effects) = parse_complete_serial_create_statement(sentence)? {
        return Ok(Some(effects));
    }
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(sentence)
        || super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![sentence]).len() > 1
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(sentence).len() > 1
    {
        // A create action followed by another executable clause is a typed
        // chain, not one token definition. The create leaf deliberately owns
        // only the complete standalone action (plus proven reminder
        // sentences below).
        return Ok(None);
    }
    let sentences = crate::lexer::split_lexed_sentences(sentence);
    if sentences.len() > 1 {
        let Some(mut effects) = parse_complete_create_statement(sentences[0])? else {
            return Ok(None);
        };
        for followup in &sentences[1..] {
            if matches!(
                parse_token_copy_followup_sentence_lexed(followup),
                Some(TokenCopyFollowup::GainHasteUntilEndOfTurn(_))
            ) {
                // This is an effect on the newly created objects, not an
                // intrinsic part of their token definition. Leave the whole
                // program to the sentence planner so it can bind the tagged
                // followup while preserving its authored duration.
                return Ok(None);
            }
            // Eldrazi Spawn/Scion tokens carry their mana ability in the
            // token blueprint, so the authored restatement adds nothing. The
            // followup registry treats it as a no-op; this fast path must
            // agree, or the ability is appended a second time as a grant.
            if crate::activation_and_restrictions::is_spawn_scion_token_mana_reminder(followup)
                && effects.last().is_some_and(
                    crate::activation_and_restrictions::effect_creates_eldrazi_spawn_or_scion,
                )
            {
                continue;
            }
            // A complete create statement may absorb only grammar-proven
            // token reminder sentences. Conditional `create ... instead`
            // followups share token words but belong to the typed
            // self-replacement sequence rule.
            if !crate::activation_and_restrictions::is_generic_token_reminder_sentence(followup)
                || !crate::activation_and_restrictions::append_token_reminder_to_last_create_effect(
                    &mut effects,
                    followup,
                )?
            {
                return Ok(None);
            }
        }
        return Ok(Some(effects));
    }
    let outer_tokens = strip_embedded_token_rules_text(sentence);
    let has_embedded_rules = outer_tokens.as_slice() != sentence;
    let parse_tokens = outer_tokens.as_slice();
    let (parsed, quantified_opponents) = if parse_tokens
        .first()
        .is_some_and(|token| token.is_word("create"))
        && effect_grammar::parse_create_head_tokens(parse_tokens).is_some()
    {
        (
            super::creation_handlers::parse_create(parse_tokens, None)?,
            false,
        )
    } else {
        // A labeled conditional can contain a later `create` verb, but the
        // label and predicate are not an actor subject for a standalone create
        // statement. Leave that complete surface to the conditional or
        // self-replacement grammar.
        if effect_grammar::split_labeled_effect_prefix_lexed(parse_tokens).is_some() {
            return Ok(None);
        }
        let Some((verb, verb_word_idx)) = super::find_verb(parse_tokens) else {
            return Ok(None);
        };
        let Some(verb_idx) =
            crate::lexer::TokenWordView::new(parse_tokens).map_word_to_token_start(verb_word_idx)
        else {
            return Ok(None);
        };
        if verb_word_idx == 0
            || !matches!(verb, super::chain_carry::Verb::Create)
            || parse_tokens
                .first()
                .is_some_and(|token| token.is_any_word(&["if", "when", "whenever", "at"]))
            || parse_tokens[..verb_idx]
                .iter()
                .any(|token| token.kind == TokenKind::Comma)
            || effect_grammar::parse_create_head_tokens(&parse_tokens[verb_idx..]).is_none()
        {
            return Ok(None);
        }
        let quantified_opponents =
            effect_grammar::for_each_shapes::parse_quantified_opponent_presence(
                &parse_tokens[..verb_idx],
            );
        let subject = if quantified_opponents {
            crate::util::SubjectAst::Player(PlayerAst::That)
        } else {
            crate::util::parse_subject(&parse_tokens[..verb_idx])
        };
        let created = super::creation_handlers::parse_create(&parse_tokens[verb_idx..], Some(subject))?;
        let created = if !quantified_opponents
            && let Some(owner) = crate::grammar::effects::become_shapes::parse_controller_owner_subject_tokens(&parse_tokens[..verb_idx])
            && !matches!(&owner.target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
            && !matches!(&owner.target, TargetAst::Object(filter, ..) if filter.tagged_constraints.iter().any(|constraint|
                constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                    || constraint.tag.as_str() == crate::tag::CompilerReferenceTag::Triggering.as_str()))
        {
            EffectAst::Sequence { effects: vec![EffectAst::subject_verb_target_only(owner.target), created] }
        } else { created };
        (created, quantified_opponents)
    };

    let mut effects = vec![parsed];
    if has_embedded_rules
        && !super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
            &mut effects,
            sentence,
        )
    {
        return Err(CardTextError::InvariantViolation(
            "typed token creation lost its embedded rule attachment".to_string(),
        ));
    }
    if quantified_opponents {
        effects = vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })];
    }
    Ok(Some(effects))
}

#[inline(never)]
pub(super) fn parse_complete_quantified_discard_statement(
    sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // A participant-scoped discard ("each player on your team may discard") is the participant loop's.
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(sentence).is_some() {
        return Ok(None);
    }
    let Some((verb, verb_word_idx)) = super::find_verb(sentence) else {
        return Ok(None);
    };
    let Some(verb_idx) = crate::lexer::TokenWordView::new(sentence).map_word_to_token_start(verb_word_idx) else {
        return Ok(None);
    };
    if verb_idx == 0 || !matches!(verb, super::chain_carry::Verb::Discard) {
        return Ok(None);
    }
    let Some(scope) = effect_grammar::chain_carry::parse_leading_chain_scope_tokens(sentence)
    else {
        return Ok(None);
    };
    let discard = super::zone_handlers::parse_discard(
        &sentence[verb_idx + 1..],
        Some(SubjectAst::Player(PlayerAst::That)),
    )?;
    let quantified = match scope {
        effect_grammar::chain_carry::ChainPlayerScope::EachOpponent => EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![discard],
        }),
        effect_grammar::chain_carry::ChainPlayerScope::EachPlayer => EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![discard],
        }),
    };
    Ok(Some(vec![quantified]))
}

#[inline(never)]
pub(crate) fn parse_complete_get_pump_statement(
    sentence: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if sentence.first().is_some_and(|token| token.is_any_word(&["if", "unless", "instead"])) {
        return Ok(None);
    }
    if effect_grammar::clause_primitive_shapes::parse_combat_requirement_shape(sentence).is_some() {
        return Ok(None);
    }
    // A comma-then continuation belongs to the effect-chain reader, which
    // preserves both the pump and the following action.
    if super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(sentence) {
        return Ok(None);
    }
    // A pump coordinated with an ability grant is the compound gain statement's.
    if effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(sentence).is_some()
        || effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(sentence).is_some()
    {
        return Ok(None);
    }
    // A pump with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(sentence).is_some() {
        return Ok(None);
    }
    // A pump followed by a fight is the pump-then-fight statement's.
    if crate::grammar::effects::counter_marker_shapes::parse_gets_then_fights_tokens(sentence)
        .is_some()
    {
        return Ok(None);
    }
    let Some((verb, verb_word_idx)) = super::find_verb(sentence) else {
        return Ok(None);
    };
    let Some(verb_idx) = crate::lexer::TokenWordView::new(sentence).map_word_to_token_start(verb_word_idx) else {
        return Ok(None);
    };
    if verb_idx == 0
        || !matches!(verb, super::chain_carry::Verb::Get)
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(sentence).len() > 1
        || sentence.first().is_some_and(|token| {
            token.is_any_word(&["if", "when", "whenever", "unless", "at", "as", "until"])
        })
        || sentence[..verb_idx]
            .iter()
            .any(|token| token.kind == TokenKind::Comma)
        || sentence[..verb_idx]
            .iter()
            .any(|token| token.is_word("may"))
    {
        // An optional pump keeps its `may` scope; the may-aware chain routes
        // own the wrapper.
        return Ok(None);
    }
    super::clause_dispatch::parse_get_pump_clause(
        &sentence[..verb_idx],
        &sentence[verb_idx + 1..],
        sentence,
    )
}

#[inline(never)]
pub(super) fn parse_complete_become_statement(
    sentence: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if super::chain_carry::parse_leading_player_may_lexed(sentence).is_some() {
        return Ok(None);
    }
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(sentence).is_some() {
        return Ok(None);
    }
    if effect_grammar::gain_ability_shapes::parse_simple_gain_ability_shape(sentence).is_some() {
        return Ok(None);
    }
    // "can't become ..." is the can't statement's.
    if crate::lexer::parser_token_word_refs(sentence)
        .iter()
        .any(|word| *word == "cant")
    {
        return Ok(None);
    }
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(sentence).is_some_and(
        |shape| effect_grammar::for_each_shapes::starts_life_total_becomes(shape.inner_tokens),
    ) {
        // The possessive life total belongs to each quantified participant,
        // not to an object target named by the words before `becomes`.
        return Ok(None);
    }
    let Some((verb, verb_word_idx)) = super::find_verb(sentence) else {
        return Ok(None);
    };
    let Some(verb_idx) = crate::lexer::TokenWordView::new(sentence).map_word_to_token_start(verb_word_idx) else {
        return Ok(None);
    };
    if verb_idx == 0
        || !matches!(verb, super::chain_carry::Verb::Become)
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(sentence).len() > 1
        || sentence.first().is_some_and(|token| {
            token.is_any_word(&["if", "when", "whenever", "unless", "at", "as", "until"])
        })
        || sentence[..verb_idx]
            .iter()
            .any(|token| token.kind == TokenKind::Comma)
    {
        return Ok(None);
    }
    super::clause_dispatch::parse_become_clause(&sentence[..verb_idx], &sentence[verb_idx + 1..])
        .map(Some)
}

pub(super) fn parse_complete_compound_gain_statement(
    sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if crate::grammar::structure::split_leading_numeric_result_prefix_lexed(sentence).is_some() {
        return Ok(None);
    }
    if sentence.first().is_some_and(|token| token.is_word("instead"))
        || effect_grammar::split_conditional_sentence_family_head_lexed(sentence).is_some() {
        return Ok(None);
    }
    // Independently conditioned modifiers retain one predicate per member.
    // Taking the final trailing-if as the whole sentence's envelope would
    // accidentally require both conditions for the first modifier.
    if sentence.iter().filter(|token| token.is_word("if")).count() > 1
        && let ParseOutcome::Match(matched) = effect_grammar::coordination::recognize_coordination(sentence)
        && matched.value.members.len() > 1
    {
        let plan = matched.value;
        let mut members = Vec::new();
        for member in &plan.members {
            let Some(trailing) = crate::grammar::structure::split_trailing_if_clause_lexed(member.tokens) else {
                return Ok(None);
            };
            if !super::find_verb(trailing.leading_tokens).is_some_and(|(verb, _)| matches!(verb,
                crate::cards::builders::Verb::Get | crate::cards::builders::Verb::Gain | crate::cards::builders::Verb::Lose
            )) { return Ok(None); }
            let effects = super::parse_effect_sentence_lexed(trailing.leading_tokens)?;
            members.push(EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                predicate: trailing.predicate, effects,
            }));
        }
        return Ok(plan.into_ast(members).map(|coordination| vec![EffectAst::Coordination(coordination)]));
    }
    if let Some(trailing_if) = crate::grammar::structure::split_trailing_if_clause_lexed(sentence)
        && (effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(
            trailing_if.leading_tokens,
        )
        .is_some()
            || effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(
                trailing_if.leading_tokens,
            )
            .is_some())
        && let Some(effects) =
            super::gain_ability::parse_gain_ability_sentence(trailing_if.leading_tokens)?
    {
        return Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: trailing_if.predicate,
            effects,
        })]));
    }
    if effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(sentence).is_none()
        && effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(sentence).is_none()
        && !effect_grammar::gain_ability_shapes::parse_simple_gain_ability_shape(sentence)
            .is_some_and(|shape| effect_grammar::gain_ability_shapes::subject_contains_gain_base_pt(
                &crate::lexer::parser_token_word_refs(shape.subject_tokens)))
    {
        return Ok(None);
    }
    super::gain_ability::parse_gain_ability_sentence(sentence)
}

#[inline(never)]
fn parse_independent_typed_statement(
    sentence: &[OwnedLexToken],
    is_document: bool,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let input = super::statement_readings::Statement {
        sentence,
        is_document,
    };
    match super::statement_readings::read_statement(&input) {
        ParseOutcome::Match(matched) => Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => Ok(None),
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
    }
}

#[inline(never)]
fn has_adjacent_token_producer_followup(
    sentences: &[&[OwnedLexToken]],
) -> Result<bool, CardTextError> {
    for pair in sentences.windows(2) {
        let Some(created) = parse_complete_create_statement(pair[0])? else {
            continue;
        };
        let creates_token = created
            .iter()
            .any(crate::activation_and_restrictions::effect_creates_any_token);
        let is_token_followup =
            crate::activation_and_restrictions::is_generic_token_reminder_sentence(pair[1])
                || parse_token_copy_followup_sentence_lexed(pair[1]).is_some()
                || parse_token_granted_ability_followup_sentence_lexed(pair[1])?.is_some();
        if creates_token && is_token_followup {
            return Ok(true);
        }
    }
    Ok(false)
}

#[inline(never)]
pub(super) fn parse_flat_independent_statements(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if sentences.iter().skip(1).any(|sentence| {
        matches!(
            classify_instead_followup_tokens(sentence),
            InsteadSemantics::SelfReplacement
        )
    }) {
        // `instead` makes the later statement semantically dependent on its
        // predecessor, so it is not an independent flat statement sequence.
        return Ok(None);
    }
    let document_sentences = sentences
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    if document_ends_in_program(&document_sentences)? {
        return Ok(None);
    }
    if has_adjacent_token_producer_followup(sentences)? {
        // These statements are grammatically correlated. Leave them to the
        // sentence follow-up planner, which preserves both the producer
        // reference and the authored sentence surface.
        return Ok(None);
    }
    let authored_comma_then = sentences
        .iter()
        .any(|sentence| super::lex_chain_helpers::has_authored_comma_then_surface_lexed(sentence));
    if sentences.len() > 1 && authored_comma_then {
        // Coordination belongs to its source sentence. The boundary-aware
        // composer must retain it rather than extending it over the document.
        return Ok(None);
    }
    let statements =
        super::lex_chain_helpers::split_segments_on_comma_then_lexed(sentences.to_vec());
    if statements.len() < 2 {
        return Ok(None);
    }
    let mut effects = Vec::new();
    for sentence in statements {
        let Some(mut statement_effects) = parse_independent_typed_statement(sentence, true)? else {
            return Ok(None);
        };
        effects.append(&mut statement_effects);
    }
    Ok(Some(if authored_comma_then {
        vec![EffectAst::CommaThen { effects }]
    } else {
        effects
    }))
}

/// Whether a document program (or procedure) covers the document from some
/// sentence through its end, or a statement with its riders opens it. Such a document belongs to the sentence loop,
/// which reads the leading sentences one by one and the program as a whole;
/// composing its sentences independently would lose the references the
/// program carries between them.
fn document_ends_in_program(document_sentences: &[SentenceInput]) -> Result<bool, CardTextError> {
    for start in 0..document_sentences.len() {
        // A program's committed error stands at the first sentence, where this
        // probe always asked; at a later sentence an error is simply no.
        let matched = if start == 0 {
            try_parse_document_program(document_sentences, 0)?
        } else {
            crate::grammar::primitives::probe_shape(try_parse_document_program(
                document_sentences,
                start,
            ))
            .flatten()
        };
        if let Some(matched) = matched
            && (start + matched.consumed_sentences == document_sentences.len()
                || (start == 0 && matched.name == super::procedures::RIDDEN_STATEMENT))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn parse_resolving_card_countered_exile_replacement(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if words.len() != 19
        || !crate::word_primitives::parse_sequence_prefix(
            &words,
            &["exile", "that", "card", "with", "a"],
        )
        || !words.get(6..).is_some_and(|tail| {
            crate::word_primitives::parse_sequence_complete(
                tail,
                &[
                    "counter",
                    "on",
                    "it",
                    "instead",
                    "of",
                    "putting",
                    "it",
                    "into",
                    "your",
                    "graveyard",
                    "as",
                    "it",
                    "resolves",
                ],
            )
        })
    {
        return None;
    }

    let view = crate::lexer::TokenWordView::new(tokens);
    let range = view.token_span_for_words(5, 6)?;
    let counter_type = crate::grammar::filters::parse_counter_type_from_tokens(&tokens[range])?;
    Some(
        EffectAst::subject_verb_register_zone_replacement_with_counters(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
            vec![(counter_type, 1)],
        ),
    )
}

#[inline(never)]
pub(crate) fn parse_complete_delegated_partition_program(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    super::delegated_partition_programs::parse_delegated_graveyard_pair_partition_program(tokens)
        .or_else(|| {
            super::delegated_partition_programs::parse_conditional_delegated_graveyard_partition_program(
                tokens,
            )
        })
        .or_else(|| {
            super::delegated_partition_programs::parse_revealed_top_delegated_partition_program(
                tokens,
            )
        })
        .or_else(|| {
            super::delegated_partition_programs::parse_source_exiled_delegated_partition_program(
                tokens,
            )
        })
}

/// Parse a correlated delegated partition at the start of a longer authored
/// procedure, then compose independently parseable follow-up sentences in the
/// same semantic lowering slice. The prefix owns the selected/complement set
/// identities; trailing instructions may consume those identities but cannot
/// retroactively change the partition.
pub(super) fn parse_delegated_partition_program_prefix(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if sentences.len() < 3 {
        return Ok(None);
    }

    for prefix_len in (2..sentences.len()).rev() {
        let prefix_sentences = sentences[..prefix_len]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let prefix_tokens = crate::util::join_sentences_with_period(&prefix_sentences);
        let Some(mut effects) = parse_complete_delegated_partition_program(&prefix_tokens) else {
            continue;
        };

        for sentence in &sentences[prefix_len..] {
            effects.extend(parse_effect_sentences_lexed(sentence)?);
        }
        return Ok(Some(effects));
    }

    Ok(None)
}

/// Parse statement-sized typed clauses that are already independently owned
/// by leaf grammar.  A document sequence made entirely from these statements
/// composes by concatenating their AST nodes; demonstratives remain explicit
/// reference constraints for the reference phase to bind across source
/// sentence boundaries.
fn parse_composable_typed_statements(
    sentences: &[&[OwnedLexToken]],
    full_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn preserve_source_sentence_boundary(
        effects: &mut Vec<EffectAst>,
        sentence_start: usize,
        sentence: &[OwnedLexToken],
        is_document: bool,
    ) {
        if !is_document || effects.len() == sentence_start {
            return;
        }
        let sentence_effects = effects.split_off(sentence_start);
        effects.push(EffectAst::SourceSentence {
            effects: sentence_effects,
            leading_then: sentence.first().is_some_and(|token| token.is_word("then")),
            starting_with_controller: false,
        });
    }

    fn parse_statement_control_flow(
        sentence: &[OwnedLexToken],
    ) -> Result<Option<EffectAst>, CardTextError> {
        let plan = match effect_grammar::control_flow::recognize_control_flow(sentence) {
            crate::recognition::ParseOutcome::Match(matched) => matched.value,
            crate::recognition::ParseOutcome::NoMatch => return Ok(None),
            crate::recognition::ParseOutcome::Error(diagnostic) => {
                return Err(diagnostic.into_card_text_error());
            }
        };
        if plan.parse_original_with_legacy {
            return Ok(None);
        }

        // These bodies already have complete typed statement parsers.  Keep
        // control-flow recognition and body lowering phase-separated instead
        // of re-entering the aggregate chain dispatcher as its callback.
        let complete_compound_gain =
            effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(plan.body_tokens)
                .is_some()
                || effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(
                    plan.body_tokens,
                )
                .is_some();
        let complete_uncoordinated_gain =
            effect_grammar::gain_ability_shapes::parse_simple_gain_ability_shape(plan.body_tokens)
                .is_some_and(|shape| shape.complete)
                && super::lex_chain_helpers::split_effect_chain_on_and_lexed(plan.body_tokens)
                    .len()
                    == 1
                && super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![
                    plan.body_tokens,
                ])
                .len()
                    == 1;
        let body = if (complete_compound_gain || complete_uncoordinated_gain)
            && let Some(effects) =
                super::gain_ability::parse_gain_ability_sentence(plan.body_tokens)?
        {
            effects
        } else if let Some(effects) = super::parse_cant_effect_sentence_lexed(plan.body_tokens)? {
            effects
        } else if let Some(effects) =
            super::chain_carry::parse_inline_looked_card_partition_chain(plan.body_tokens)
        {
            effects
        } else {
            let body_words = crate::lexer::parser_token_word_refs(plan.body_tokens);
            if body_words.first() != Some(&"put")
                || !crate::word_primitives::sequence_occurs(&body_words, &["counter"])
            {
                return Ok(None);
            }
            parse_single_put_counters_effect_chain(plan.body_tokens)?
        };

        Ok(plan
            .into_ast(body)
            .map(|control| EffectAst::ControlFlow(Box::new(control))))
    }

    fn attach_dynamic_token_stats(
        effect: &mut EffectAst,
        power: &Value,
        toughness: &Value,
    ) -> bool {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                    dynamic_power_toughness,
                    ..
                }),
            ..
        }) = effect
        {
            *dynamic_power_toughness = Some((power.clone(), toughness.clone()));
            return true;
        }

        let mut attached = false;
        for_each_nested_effects_mut(effect, false, |nested| {
            if attached {
                return;
            }
            for nested_effect in nested.iter_mut().rev() {
                if attach_dynamic_token_stats(nested_effect, power, toughness) {
                    attached = true;
                    break;
                }
            }
        });
        attached
    }

    fn attach_dynamic_token_stats_to_latest_creation(
        effects: &mut [EffectAst],
        power: &Value,
        toughness: &Value,
    ) -> bool {
        effects
            .iter_mut()
            .rev()
            .any(|effect| attach_dynamic_token_stats(effect, power, toughness))
    }

    if sentences.is_empty() {
        return Ok(None);
    }
    if sentences.iter().skip(1).any(|sentence| {
        matches!(
            classify_instead_followup_tokens(sentence),
            InsteadSemantics::SelfReplacement
        )
    }) {
        // An authored `instead` sentence is correlated with the immediately
        // preceding executable statement. It cannot be composed as an
        // independent SourceSentence without losing the replacement arm.
        // Leave the complete program to the typed sentence-followup planner.
        return Ok(None);
    }
    if sentences.len() > 1 {
        let document_sentences = sentences
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();
        if document_ends_in_program(&document_sentences)? {
            return Ok(None);
        }
    }
    if has_adjacent_token_producer_followup(sentences)? {
        return Ok(None);
    }
    let mut effects = Vec::new();
    let is_document = sentences.len() > 1;
    for sentence in sentences {
        let sentence_start = effects.len();
        if matches!(
            crate::grammar::token_definitions::parse_token_reminder_sentence_kind_tokens(sentence),
            Some(crate::grammar::token_definitions::TokenReminderSentenceKind::PowerToughness)
        ) && let Some((power, toughness)) =
            crate::grammar::token_definitions::parse_token_dynamic_power_toughness_tokens(sentence)
        {
            if !attach_dynamic_token_stats_to_latest_creation(&mut effects, &power, &toughness) {
                return Ok(None);
            }
            continue;
        }
        if let Some(mut statement_effects) =
            parse_independent_typed_statement(sentence, is_document)?
        {
            effects.append(&mut statement_effects);
            preserve_source_sentence_boundary(&mut effects, sentence_start, sentence, is_document);
            continue;
        }
        let statement_tokens = if sentence.first().is_some_and(|token| token.is_word("then")) {
            &sentence[1..]
        } else {
            sentence
        };
        let statement_words = crate::lexer::parser_token_word_refs(statement_tokens);
        if statement_words.first() == Some(&"put")
            && crate::word_primitives::sequence_occurs(&statement_words, &["counter"])
            && super::zone_counter_helpers::has_counter_placement_head(statement_tokens)
        {
            let comma_then = LexedClause::new(statement_tokens).split_comma_then();
            let effect = if let Some((head, tail)) = comma_then {
                let head_words = crate::lexer::parser_token_word_refs(head.tokens());
                let tail_words = crate::lexer::parser_token_word_refs(tail.tokens());
                if head_words.first() == Some(&"put")
                    && tail_words.first() == Some(&"put")
                    && crate::word_primitives::sequence_occurs(&head_words, &["counter"])
                    && crate::word_primitives::sequence_occurs(&tail_words, &["counter"])
                    && let Ok(mut head_effect) =
                        super::zone_counter_helpers::parse_put_counters(head.tokens())
                    && let Ok(tail_effect) =
                        super::zone_counter_helpers::parse_put_counters(tail.tokens())
                    && let crate::recognition::ParseOutcome::Match(plan) =
                        effect_grammar::coordination::recognize_coordination(statement_tokens)
                {
                    if let (
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action:
                                SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                                    counter_type: crate::object::CounterType::PlusOnePlusOne,
                                    count,
                                    target,
                                    target_count: None,
                                    distributed: false,
                                }),
                            ..
                        }),
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action:
                                SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                                    counter_type: source_counter_type,
                                    count: Value::Fixed(1),
                                    target: TargetAst::Source(_),
                                    target_count: None,
                                    distributed: false,
                                }),
                            ..
                        }),
                    ) = (&mut head_effect, &tail_effect)
                        && let Value::CountersOn(source, Some(counted_counter_type)) =
                            count.unhinted()
                        && counted_counter_type == source_counter_type
                        && matches!(source.base(), ChooseSpec::Source)
                    {
                        let created_tag =
                            crate::util::helper_tag_for_tokens(full_tokens, "created_token");
                        if tag_first_created_token_result(&mut effects, &created_tag) {
                            *target =
                                TargetAst::Tagged(crate::tag::TagRef::of(created_tag), span_from_tokens(head.tokens()));
                        }
                    }
                    plan.value
                        .into_ast(vec![head_effect, tail_effect])
                        .map(EffectAst::Coordination)
                } else {
                    None
                }
            } else if let Some(trailing_if) =
                crate::grammar::structure::split_trailing_if_clause_lexed(statement_tokens)
                && let Ok(base_effect) =
                    super::zone_counter_helpers::parse_put_counters(trailing_if.leading_tokens)
            {
                Some(EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                    predicate: trailing_if.predicate,
                    effects: vec![base_effect],
                }))
            } else if is_document {
                Some(super::zone_counter_helpers::parse_put_counters(
                    statement_tokens,
                )?)
            } else {
                None
            };
            if let Some(effect) = effect {
                effects.push(effect);
                preserve_source_sentence_boundary(
                    &mut effects,
                    sentence_start,
                    sentence,
                    is_document,
                );
                continue;
            }
        }
        if is_document && let Some(effect) = parse_statement_control_flow(sentence)? {
            effects.push(effect);
            preserve_source_sentence_boundary(&mut effects, sentence_start, sentence, is_document);
            continue;
        }
        if is_document {
            let fight_tokens = if sentence.first().is_some_and(|token| token.is_word("then")) {
                &sentence[1..]
            } else {
                sentence
            };
            if let Some(effect) = super::clause_primitives::parse_fight_clause(fight_tokens)? {
                effects.push(effect);
                preserve_source_sentence_boundary(
                    &mut effects,
                    sentence_start,
                    sentence,
                    is_document,
                );
                continue;
            }
        }
        return Ok(None);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_complete_composable_fight_program(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() < 2
        || !sentences.iter().any(|sentence| {
            effect_grammar::clause_primitive_shapes::parse_fight_shape(sentence).is_some()
        })
    {
        return Ok(None);
    }
    parse_composable_typed_statements(&sentences, tokens)
}

/// Lower a grammar-proven coordination from its statement-sized clause
/// members. The coordination plan retains ordering and omitted-subject carry;
/// each member is lowered by its verb leaf without re-entering the aggregate
/// sentence dispatcher.
fn parse_direct_typed_coordination(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(tokens).is_some() {
        return Ok(None);
    }
    let plan = match effect_grammar::coordination::recognize_coordination(tokens) {
        crate::recognition::ParseOutcome::Match(matched) => matched.value,
        crate::recognition::ParseOutcome::NoMatch => return Ok(None),
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    };
    if plan.members.len() != 2
        || plan.members.iter().any(|member| member.head.is_none())
        || plan.boundaries.iter().any(|boundary| {
            boundary.omission != effect_grammar::coordination::CoordinationOmissionAst::Subject
        })
        || !plan.members.last().is_some_and(|member| {
            super::find_verb(member.tokens).is_some_and(|(verb, _)| {
                matches!(
                    verb,
                    crate::cards::builders::Verb::Gain | crate::cards::builders::Verb::Lose
                )
            })
        })
    {
        return Ok(None);
    }

    let mut effects = Vec::with_capacity(plan.members.len());
    for member in &plan.members {
        let member_tokens = crate::util::trim_edge_punctuation_tokens(member.tokens);
        let Some((verb, verb_word_idx)) = super::find_verb(member_tokens) else {
            return Ok(None);
        };
        let Some(verb_idx) = crate::lexer::TokenWordView::new(member_tokens).map_word_to_token_start(verb_word_idx) else {
            return Ok(None);
        };
        let subject =
            (verb_idx > 0).then(|| crate::util::parse_subject(&member_tokens[..verb_idx]));
        let Ok(effect) = super::verb_handlers::parse_effect_with_verb(
            verb,
            subject,
            &member_tokens[verb_idx + 1..],
        ) else {
            return Ok(None);
        };
        effects.push(effect);
    }

    for (boundary_index, boundary) in plan.boundaries.iter().enumerate() {
        if boundary.omission != effect_grammar::coordination::CoordinationOmissionAst::Subject {
            continue;
        }
        let Some(context) = super::chain_carry::explicit_player_for_carry(&effects[boundary_index])
        else {
            continue;
        };
        super::chain_carry::maybe_apply_carried_player(&mut effects[boundary_index + 1], context);
    }

    let Some(coordination) = plan.into_ast(effects) else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::Coordination(coordination)]))
}

/// The sentence-list rule. Memoized per card like the sentence rule: line
/// shapes that each need the effect clause parsed before they can claim the
/// line all read one parse of it.
#[track_caller]
pub fn parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    crate::sentence_memo::memoized(
        crate::sentence_memo::Rule::Sentences,
        tokens,
        std::panic::Location::caller(),
        || {
            let mut effects = parse_effect_sentences_lexed_unfinalized(tokens)?;
            transport_coin_flip_outcomes_into_owner(&mut effects);
            preserve_linked_target_fanout_group(tokens, &mut effects);
            Ok(effects)
        },
    )
}

fn parse_effect_sentences_lexed_unfinalized(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let document = super::document_readings::Document::new(tokens);
    match super::document_readings::read_document(&document) {
        ParseOutcome::Match(matched) => Ok(matched.value.value),
        // No reading claims the document: its sentences are composed as
        // independent statements when they can be, and the sentence loop
        // reads it otherwise.
        ParseOutcome::NoMatch => {
            if let Some(effects) = parse_flat_independent_statements(&document.sentences)? {
                return Ok(effects);
            }
            parse_effect_sentences_lexed_after_direct(tokens, &document.sentences)
        }
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
    }
}

pub(super) fn parse_single_put_counters_effect_chain(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(tokens)
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens).len() > 1
    {
        return super::parse_effect_sentences_lexed(tokens);
    }
    if !tokens.first().is_some_and(|token| token.is_word("put"))
        || !tokens
            .iter()
            .any(|token| token.is_any_word(&["counter", "counters"]))
    {
        return Err(CardTextError::ParseError(format!(
            "conditional counter body is not a put-counters clause (clause: '{}')",
            crate::lexer::render_token_slice(tokens)
        )));
    }
    Ok(vec![super::zone_counter_helpers::parse_put_counters(
        tokens,
    )?])
}

mod after_direct_readings;

#[inline(never)]
fn parse_effect_sentences_lexed_after_direct(
    tokens: &[OwnedLexToken],
    sentences: &[&[OwnedLexToken]],
) -> Result<Vec<EffectAst>, CardTextError> {
    let input = after_direct_readings::AfterDirect {
        tokens,
        sentences,
        read_by_cache: Default::default(),
    };
    match after_direct_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(matched.value.value),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    parse_effect_sentences_lexed_legacy(tokens, sentences)
}

mod legacy_readings;

#[inline(never)]
fn parse_effect_sentences_lexed_legacy(
    tokens: &[OwnedLexToken],
    sentences: &[&[OwnedLexToken]],
) -> Result<Vec<EffectAst>, CardTextError> {
    let input = legacy_readings::LegacyDocument { tokens, sentences };
    match legacy_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(matched.value.value),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    let mut effects = parse_effect_sentences_lexed_inner(tokens)?;
    preserve_revealed_same_mana_value_as_another_iterator(tokens, &mut effects);
    transport_optional_search_partition_followup(&mut effects);
    transport_coin_flip_outcomes_into_owner(&mut effects);
    transport_copy_retarget_into_trailing_delayed_trigger(&mut effects);
    preserve_linked_target_fanout_group(tokens, &mut effects);
    preserve_tapped_this_way_group_for_later_distribution(tokens, &mut effects);
    super::chain_carry::preserve_independent_target_player_coordination(&mut effects, tokens);
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
}

fn dynamic_base_pt_where_x_effect(effect: &EffectAst) -> bool {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                power, toughness, ..
            }),
        ..
    }) = effect
        && power.unhinted() == toughness.unhinted()
        && power.has_surface_hint(ValueSurfaceHint::WhereXIs)
    {
        return true;
    }
    let mut found = false;
    crate::model::visit::for_each_nested_effects(effect, true, |nested| {
        found |= nested.iter().any(dynamic_base_pt_where_x_effect);
    });
    found
}

fn tag_first_created_token_result(effects: &mut [EffectAst], tag: &TagKey) -> bool {
    for effect in effects {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { .. })
                    | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { .. })
                    | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { .. }),
                ..
            })
        ) {
            *effect = EffectAst::TagAffected {
                effect: Box::new(effect.clone()),
                tag: crate::tag::TagRef::of(tag.clone()),
            };
            return true;
        }

        let mut tagged = false;
        for_each_nested_effects_mut(effect, false, |nested| {
            if !tagged {
                tagged = tag_first_created_token_result(nested, tag);
            }
        });
        if tagged {
            return true;
        }
    }
    false
}

fn set_first_put_counter_target(effects: &mut [EffectAst], target: &TargetAst) -> bool {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { target: found, .. }),
            ..
        }) = effect
        {
            *found = target.clone();
            return true;
        }

        let mut replaced = false;
        for_each_nested_effects_mut(effect, false, |nested| {
            if !replaced {
                replaced = set_first_put_counter_target(nested, target);
            }
        });
        if replaced {
            return true;
        }
    }
    false
}

/// Preserve the created-token set and the independent set whose counters
/// provide the distinct kinds in `create ... . Then for each kind of counter
/// among ..., put a counter of that kind on either of those tokens.`
fn parse_created_token_counter_kind_distribution_followup(
    sentences: &[&[OwnedLexToken]],
    full_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [producer, consumer] = sentences else {
        return Ok(None);
    };
    let producer_words = crate::lexer::parser_token_word_refs(producer);
    if producer_words.first() != Some(&"create")
        || !crate::word_primitives::any_sequence_occurs(&producer_words, &[&["token"], &["tokens"]])
    {
        return Ok(None);
    }
    let Some(shape) =
        effect_grammar::counter_marker_shapes::parse_counter_kind_distribution_tokens(consumer)
    else {
        return Ok(None);
    };
    if !effect_grammar::counter_marker_shapes::parse_created_token_distribution_target(
        shape.target_tokens,
    ) {
        return Ok(None);
    }

    let mut effects = parse_effect_sentences_lexed_inner(producer)?;
    let created_tag = crate::util::helper_tag_for_tokens(full_tokens, "created_token");
    if !tag_first_created_token_result(&mut effects, &created_tag) {
        return Ok(None);
    }
    let counter_source = super::parse_object_filter(shape.counter_source_tokens, false)?;
    effects.push(
        EffectAst::subject_verb_put_each_counter_kind_from_on_one_of(
            TargetAst::Object(counter_source, None, None),
            TargetAst::Tagged(crate::tag::TagRef::of(created_tag), span_from_tokens(shape.target_tokens)),
        ),
    );
    Ok(Some(effects))
}

/// Preserve two independent antecedents in the common sequence
/// `create a token, then mill ... . Put a counter on the token if ... was
/// milled this way.` The explicit tag names the created token while the
/// ordinary result binding continues to name the mill event for the trailing
/// condition.
fn parse_created_token_mill_counter_followup(
    sentences: &[&[OwnedLexToken]],
    full_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [producer, consumer] = sentences else {
        return Ok(None);
    };
    let producer_words = crate::lexer::parser_token_word_refs(producer);
    let consumer_words = crate::lexer::parser_token_word_refs(consumer);
    if producer_words.first() != Some(&"create")
        || !crate::word_primitives::sequence_occurs(&producer_words, &["then", "mill"])
        || consumer_words.first() != Some(&"put")
        || !crate::word_primitives::sequence_occurs(
            &consumer_words,
            &["counter", "on", "the", "token", "if"],
        )
        || !crate::word_primitives::parse_any_sequence_suffix(
            &consumer_words,
            &[
                &["was", "milled", "this", "way"],
                &["were", "milled", "this", "way"],
            ],
        )
    {
        return Ok(None);
    }

    let mut effects = parse_effect_sentences_lexed_inner(producer)?;
    let created_tag = crate::util::helper_tag_for_tokens(full_tokens, "created_token");
    if !tag_first_created_token_result(&mut effects, &created_tag) {
        return Ok(None);
    }

    let mut followup = parse_effect_sentences_lexed_inner(consumer)?;
    let created_target = TargetAst::Tagged(crate::tag::TagRef::of(created_tag), span_from_tokens(consumer));
    if !set_first_put_counter_target(&mut followup, &created_target) {
        return Ok(None);
    }
    effects.extend(followup);
    Ok(Some(effects))
}

fn parse_quoted_token_rule_then_linked_counter_followup(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if !crate::word_primitives::sequence_occurs(&words, &["create"])
        || !crate::word_primitives::any_sequence_occurs(&words, &[&["token"], &["tokens"]])
    {
        return Ok(None);
    }
    let Some(opening_quote) =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Quote)
    else {
        return Ok(None);
    };
    let Some(closing_quote) = tokens
        .iter()
        .enumerate()
        .skip(opening_quote + 1)
        .find_map(|(idx, token)| (token.kind == TokenKind::Quote).then_some(idx))
    else {
        return Ok(None);
    };
    if !tokens[opening_quote + 1..closing_quote]
        .iter()
        .any(|token| token.kind == TokenKind::Period)
    {
        return Ok(None);
    }
    let trailing = &tokens[closing_quote + 1..];
    let trailing_words = crate::lexer::parser_token_word_refs(trailing);
    if trailing_words.first() != Some(&"put") {
        return Ok(None);
    }
    let clauses = super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![trailing]);
    let [first_counter_tokens, second_counter_tokens] = clauses.as_slice() else {
        return Ok(None);
    };
    let mut first_counter = match parse_put_counters(first_counter_tokens) {
        Ok(effect) => effect,
        Err(_) => return Ok(None),
    };
    let second_counter = match parse_put_counters(second_counter_tokens) {
        Ok(effect) => effect,
        Err(_) => return Ok(None),
    };
    let first_action = match &mut first_counter {
        EffectAst::SubjectVerb(effect) => &mut effect.action,
        _ => return Ok(None),
    };
    let second_action = match &second_counter {
        EffectAst::SubjectVerb(effect) => &effect.action,
        _ => return Ok(None),
    };
    let (
        SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
            counter_type: crate::object::CounterType::PlusOnePlusOne,
            count,
            target: first_target,
            target_count: None,
            distributed: false,
        }),
        SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
            counter_type: source_counter_type,
            count: Value::Fixed(1),
            target: TargetAst::Source(_),
            target_count: None,
            distributed: false,
        }),
    ) = (first_action, second_action)
    else {
        return Ok(None);
    };
    let Value::CountersOn(source, Some(counted_counter_type)) = count.unhinted() else {
        return Ok(None);
    };
    if counted_counter_type != source_counter_type || !matches!(source.base(), ChooseSpec::Source) {
        return Ok(None);
    }

    let mut create_effects = parse_effect_sentence_lexed(&tokens[..=closing_quote])?;
    let [create_effect] = create_effects.as_mut_slice() else {
        return Ok(None);
    };
    if !matches!(
        create_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { .. }),
            ..
        })
    ) {
        return Ok(None);
    }
    let created_tag = crate::util::helper_tag_for_tokens(tokens, "created_token");
    *first_target = TargetAst::Tagged(crate::tag::TagRef::of(created_tag.clone()), span_from_tokens(first_counter_tokens));

    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(create_effect.clone()),
            tag: crate::tag::TagRef::of(created_tag),
        },
        first_counter,
        second_counter,
    ]))
}

fn parse_quoted_token_rule_then_conditional_followup(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if !crate::word_primitives::sequence_occurs(&words, &["create"])
        || !crate::word_primitives::any_sequence_occurs(&words, &[&["token"], &["tokens"]])
    {
        return Ok(None);
    }
    let Some(opening_quote) =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Quote)
    else {
        return Ok(None);
    };
    let Some(closing_quote) = tokens
        .iter()
        .enumerate()
        .skip(opening_quote + 1)
        .find_map(|(idx, token)| (token.kind == TokenKind::Quote).then_some(idx))
    else {
        return Ok(None);
    };
    if !tokens[opening_quote + 1..closing_quote]
        .iter()
        .any(|token| token.kind == TokenKind::Period)
    {
        return Ok(None);
    }

    let trailing = trim_edge_punctuation(&tokens[closing_quote + 1..]);
    let trailing_words = crate::lexer::parser_token_word_refs(&trailing);
    if !crate::word_primitives::parse_sequence_prefix(&trailing_words, &["then", "if"]) {
        return Ok(None);
    }
    let mut create_effects = parse_effect_sentence_lexed(&tokens[..=closing_quote])?;
    let [create] = create_effects.as_mut_slice() else {
        return Ok(None);
    };
    if !matches!(
        create,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { .. }),
            ..
        })
    ) {
        return Ok(None);
    }
    let Some(conditional) = effect_grammar::parse_conditional_sentence_family_lexed(
        &trailing,
        super::parse_effect_chain_lexed,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(vec![
        EffectAst::SourceSentence {
            effects: vec![create.clone()],
            leading_then: false,
            starting_with_controller: false,
        },
        EffectAst::SourceSentence {
            effects: conditional,
            leading_then: true,
            starting_with_controller: false,
        },
    ]))
}

fn parse_quoted_token_rule_then_coin_flip_outcomes(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [producer, outcomes @ ..] = sentences.as_slice() else {
        return Ok(None);
    };
    if outcomes.is_empty() {
        return Ok(None);
    }

    let producer_words = crate::lexer::parser_token_word_refs(producer);
    if producer_words.first() != Some(&"create")
        || !crate::word_primitives::any_sequence_occurs(&producer_words, &[&["token"], &["tokens"]])
    {
        return Ok(None);
    }
    let Some(opening_quote) =
        crate::slice_primitives::select_position(producer, |token| token.kind == TokenKind::Quote)
    else {
        return Ok(None);
    };
    let Some(closing_quote) = producer
        .iter()
        .enumerate()
        .skip(opening_quote + 1)
        .find_map(|(idx, token)| (token.kind == TokenKind::Quote).then_some(idx))
    else {
        return Ok(None);
    };
    if !producer[opening_quote + 1..closing_quote]
        .iter()
        .any(|token| token.kind == TokenKind::Period)
    {
        return Ok(None);
    }
    let trailing = trim_edge_punctuation(&producer[closing_quote + 1..]);
    let inline_flip = crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(&trailing),
        &["then", "flip", "a", "coin"],
    );
    let outcome_start = if inline_flip {
        0
    } else if outcomes.first().is_some_and(|sentence| {
        crate::word_primitives::parse_sequence_complete(
            &crate::lexer::parser_token_word_refs(sentence),
            &["then", "flip", "a", "coin"],
        )
    }) {
        1
    } else {
        return Ok(None);
    };

    let mut create_effects = parse_effect_sentence_lexed(&producer[..=closing_quote])?;
    let [create] = create_effects.as_mut_slice() else {
        return Ok(None);
    };
    if !matches!(
        create,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { .. }),
            ..
        })
    ) {
        return Ok(None);
    }

    let mut effects = vec![
        EffectAst::SourceSentence {
            effects: vec![create.clone()],
            leading_then: false,
            starting_with_controller: false,
        },
        EffectAst::SourceSentence {
            effects: vec![EffectAst::subject_verb_flip_coin(PlayerAst::You)],
            leading_then: true,
            starting_with_controller: false,
        },
    ];
    for outcome in &outcomes[outcome_start..] {
        let Some((predicate, rest_tokens)) =
            effect_grammar::dispatch_entry_shapes::parse_flip_result_shape_tokens(outcome)
        else {
            return Ok(None);
        };
        let outcome_effects = if let Some(effect) =
            super::clause_primitives::parse_anaphoric_object_deals_damage_clause(rest_tokens)?
        {
            vec![effect]
        } else {
            parse_effect_sentences_lexed(rest_tokens)?
        };
        if outcome_effects.is_empty() {
            return Ok(None);
        }
        effects.push(EffectAst::SourceSentence {
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate,
                effects: outcome_effects,
            })],
            leading_then: false,
            starting_with_controller: false,
        });
    }

    Ok(Some(effects))
}

#[cfg(test)]
mod quoted_token_coin_flip_outcome_tests {
    use super::*;

    const EFFECTS: &str = "Create a colorless artifact token named Land Mine with \"{R}, Sacrifice this token: This token deals 2 damage to target attacking creature without flying.\" Then flip a coin. If you lose the flip, this creature deals 2 damage to itself.";

    #[test]
    fn quoted_token_rule_keeps_the_following_flip_as_the_outcome_producer() {
        let tokens = crate::lexer::lex_line(EFFECTS, 0).expect("quoted token and flip should lex");
        let effects = parse_quoted_token_rule_then_coin_flip_outcomes(&tokens)
            .expect("quoted token and flip probe should not error")
            .expect("quoted token and flip shape should match");
        let [
            EffectAst::SourceSentence {
                effects: create, ..
            },
            EffectAst::SourceSentence {
                effects: flip,
                leading_then: true,
                ..
            },
            EffectAst::SourceSentence {
                effects: outcome, ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected three typed source instructions: {effects:#?}");
        };
        assert!(matches!(
            create.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { .. }),
                ..
            })]
        ));
        assert!(matches!(
            flip.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Random(RandomActionAst::FlipCoin),
                ..
            })]
        ));
        assert!(matches!(
            outcome.as_slice(),
            [EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                ..
            })]
        ));

        let public = parse_effect_sentences_lexed(&tokens)
            .expect("public effect parser should keep the specialized sentence bundle");
        assert!(
            matches!(
                public.as_slice(),
                [
                    EffectAst::SourceSentence { .. },
                    EffectAst::SourceSentence {
                        leading_then: true,
                        ..
                    },
                    EffectAst::SourceSentence { .. }
                ]
            ),
            "{public:#?}"
        );
        let prepared = crate::lowering_support::stage_effects_for_lowering(
            &public,
            crate::cards::builders::ReferenceImports::default(),
        )
        .expect("coin-flip sentence bundle should prepare");
        assert_eq!(prepared.source_sentence_segments.len(), 3, "{prepared:#?}");

        let builder = ironsmith_compiler_lowering::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Coin Boundary Probe",
        )
        .card_types(vec![crate::types::CardType::Creature]);
        let (definition, trace) = crate::parse_trace::capture(|| {
            builder.parse_text(format!("At the beginning of your upkeep, {EFFECTS}"))
        });
        let definition = definition.unwrap_or_else(|error| {
            panic!(
                "public trigger route should compile: {error}\n{}",
                trace.render()
            )
        });
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind
        else {
            panic!("expected a triggered ability");
        };
        assert_eq!(
            triggered.effects.segments.len(),
            3,
            "{:#?}\n{}",
            triggered.effects,
            trace.render()
        );
        let [then_effect] = triggered.effects.segments[1].default_effects.as_slice() else {
            panic!("expected one leading-then effect: {:#?}", triggered.effects);
        };
        assert!(matches!(
            then_effect.downcast_ref::<crate::effects::SequenceEffect>(),
            Some(sequence)
                if sequence.surface == ironsmith_core::SequenceSurface::SentenceLeadingThen
        ));
    }

    #[test]
    fn quoted_token_rule_does_not_claim_a_different_followup_action() {
        let tokens = crate::lexer::lex_line(&EFFECTS.replace("Then flip a coin", "Then scry 1"), 0)
            .expect("changed followup should lex");
        assert!(
            parse_quoted_token_rule_then_coin_flip_outcomes(&tokens)
                .expect("changed followup probe should not error")
                .is_none()
        );
    }
}

fn parse_reveal_hand_then_put_same_name_as_permanent(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    const PREFIX: &[&str] = &["reveal", "a", "card", "in", "your", "hand", "then", "put"];
    if !crate::word_primitives::parse_sequence_prefix(&words, PREFIX) {
        return None;
    }
    let remainder = words.get(PREFIX.len()..)?;
    let referent_len =
        if crate::word_primitives::parse_sequence_prefix(remainder, &["that", "card"]) {
            2
        } else if crate::word_primitives::parse_sequence_prefix(remainder, &["it"]) {
            1
        } else {
            return None;
        };
    if !remainder.get(referent_len..).is_some_and(|tail| {
        crate::word_primitives::parse_sequence_complete(
            tail,
            &[
                "onto",
                "the",
                "battlefield",
                "if",
                "it",
                "has",
                "the",
                "same",
                "name",
                "as",
                "a",
                "permanent",
            ],
        )
    }) {
        return None;
    }

    let selected_tag = crate::util::helper_tag_for_tokens(tokens, "revealed_hand_card");
    let comparison_tag = crate::util::helper_tag_for_tokens(tokens, "same_name_permanents");
    let mut selected_filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::You);
    selected_filter.set_explicit_card_noun(true);
    let mut same_name_filter = ObjectFilter::default();
    same_name_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: comparison_tag.clone().into(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });

    Some(vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: selected_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(selected_tag.clone()),
        }),
        EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(selected_tag.clone())),
        EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::permanent(),
            vec![Zone::Battlefield],
            crate::tag::TagRef::of(comparison_tag),
        ),
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(crate::tag::TagRef::of(selected_tag.clone()), same_name_filter),
            if_true: vec![EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::TagRef::of(selected_tag),
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            })],
            if_false: Vec::new(),
        }),
    ])
}

fn contains_tagged_battlefield_partition(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TagMatchingObjects { tag, .. },
            ..
        }) => tag.as_str().starts_with("partition_pool"),
        EffectAst::Sequence { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => {
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
        EffectAst::Permissions(PermissionEffectAst::May { effects }) | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => effects,
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
            Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }))
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
            Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDid {
                effects,
                predicate: None,
                result_predicate: IfResultPredicate::SearchedLibrary,
            })) => effects.clone(),
            _ => {
                index += 1;
                continue;
            }
        };
        let Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: search_effects,
        })) = effects.get_mut(index)
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
            action: SubjectVerbActionAst::Random(RandomActionAst::FlipCoin) | SubjectVerbActionAst::Random(RandomActionAst::FlipCoinFaceOnly),
            ..
        })
    )
}

fn coin_flip_owner_body_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
    let effects = match effect {
        EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
        | EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextCleanupStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUntapStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUpkeep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextDrawStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextMainPhase { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextFirstMainPhase { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat { effects })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration { effects, .. }) => effects,
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
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did | IfResultPredicate::DidNot,
            ..
        })
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
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { filter, .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { filter, .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { filter, .. }) => Some(filter),
        _ => None,
    }
}

fn direct_all_object_filter_mut(effect: &mut EffectAst) -> Option<&mut ObjectFilter> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return None;
    };
    match action {
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { filter, .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { filter, .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { filter, .. }) => Some(filter),
        _ => None,
    }
}

fn filter_has_linked_it_constraint(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
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
        .any(|constraint| constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
}

fn linked_fanout_group_tag(effect: &EffectAst) -> Option<TagKey> {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::TagMatchingObjects { tag, .. },
        ..
    }) = effect
        && tag.as_str().starts_with("linked_fanout_group_")
    {
        return Some(tag.clone().into());
    }

    let mut found = None;
    for_each_nested_effects(effect, true, |nested| {
        if found.is_none() {
            found = nested.iter().find_map(linked_fanout_group_tag);
        }
    });
    found
}

fn retag_linked_fanout_followup(effect: &mut EffectAst, group: &TagKey) {
    if let Some(filter) = direct_all_object_filter_mut(effect) {
        for constraint in &mut filter.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                constraint.tag = group.clone();
            }
        }
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for effect in nested {
            retag_linked_fanout_followup(effect, group);
        }
    });
}

/// Keep a compound target-plus-linked-set subject available to later
/// demonstratives. The individual fanout action must still exclude the target,
/// while "those creatures/cards" refers to the union of both parts.
fn preserve_linked_target_fanout_group(tokens: &[OwnedLexToken], effects: &mut Vec<EffectAst>) {
    // Sentence splitting may put the target/fanout pair and its later plural
    // demonstrative in sibling containers. Carry the durable union tag across
    // that boundary before looking for a direct pair in this vector.
    let mut carried_group: Option<TagKey> = None;
    for effect in effects.iter_mut() {
        match effect {
            EffectAst::Sequence { effects: nested }
            | EffectAst::Coordinated {
                effects: nested, ..
            } => preserve_linked_target_fanout_group(tokens, nested),
            EffectAst::Coordination(coordination) => {
                for member in &mut coordination.members {
                    preserve_linked_target_fanout_group(tokens, &mut member.effects);
                }
                preserve_linked_target_fanout_group_across_coordination(tokens, coordination);
                transport_linked_fanout_group_across_coordination(tokens, coordination);
            }
            _ => {}
        }
        if let Some(group) = carried_group.as_ref() {
            retag_linked_fanout_followup(effect, group);
        }
        if let Some(group) = linked_fanout_group_tag(effect) {
            carried_group = Some(group);
        }
    }
    if effects.len() < 2 {
        return;
    }

    let words = crate::lexer::token_word_refs(tokens);
    let has_trailing_that_name =
        crate::word_primitives::sequence_occurs(&words, &["with", "that", "name"]);

    for first_idx in 0..effects.len().saturating_sub(1) {
        let second_idx = first_idx + 1;
        let Some(linked_filter) = direct_all_object_filter(&effects[second_idx]) else {
            continue;
        };
        let excludes_primary = linked_filter.other
            || linked_filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
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
                    tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
        }

        let primary_alias = crate::tag::CompilerIndexedTag::LinkedFanoutPrimary.key(first_idx);
        let group_alias = crate::tag::CompilerIndexedTag::LinkedFanoutGroup.key(first_idx);

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
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                constraint.tag = primary_alias.clone().into();
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
                tag: primary_alias.clone().into(),
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
                    if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                        constraint.tag = group_alias.clone().into();
                    }
                }
            }
        }

        // Capture the fanout's actual outcomes before creating the union. A
        // post-action battlefield scan loses moved objects and can include
        // objects whose zone change was replaced or prevented.
        let fanout = effects.remove(second_idx);
        effects.insert(
            second_idx,
            EffectAst::TagAffected {
                effect: Box::new(fanout),
                tag: group_alias.clone(),
            },
        );

        effects.insert(
            second_idx + 1,
            EffectAst::subject_verb_tagged_object_union(
                group_filter,
                group_zones,
                group_alias.clone(),
                vec![primary_alias, group_alias],
            ),
        );
        return;
    }
}

fn transport_linked_fanout_group_across_coordination(
    tokens: &[OwnedLexToken],
    coordination: &mut crate::model::CoordinationAst,
) {
    let words = crate::lexer::token_word_refs(tokens);
    let has_trailing_that_name =
        crate::word_primitives::sequence_occurs(&words, &["with", "that", "name"]);
    let mut carried_group: Option<TagKey> = None;

    for member in &mut coordination.members {
        for effect in &mut member.effects {
            if let Some(group) = carried_group.as_ref() {
                if has_trailing_that_name
                    && let Some(filter) = direct_all_object_filter_mut(effect)
                    && !filter_has_it_reference(filter)
                {
                    filter.tagged_constraints.push(TaggedObjectConstraint {
                        tag: group.clone(),
                        relation: TaggedOpbjectRelation::SameNameAsTagged,
                    });
                }
                retag_linked_fanout_followup(effect, group);
            }
            if let Some(group) = linked_fanout_group_tag(effect) {
                carried_group = Some(group);
            }
        }
    }
}

/// Apply the linked target/fanout ownership rewrite across the member
/// boundaries of the migrated coordination AST.
///
/// Before coordination became a typed program, all members occupied one
/// `Vec<EffectAst>` and `preserve_linked_target_fanout_group` could see the
/// explicit target, its related set, and a later demonstrative together. The
/// typed wrapper must retain the same semantic pass without flattening or
/// discarding its authored boundary metadata.
fn preserve_linked_target_fanout_group_across_coordination(
    tokens: &[OwnedLexToken],
    coordination: &mut crate::model::CoordinationAst,
) {
    if coordination.members.len() < 2 {
        return;
    }

    let words = crate::lexer::token_word_refs(tokens);
    let has_trailing_that_name =
        crate::word_primitives::sequence_occurs(&words, &["with", "that", "name"]);

    for first_member_idx in 0..coordination.members.len().saturating_sub(1) {
        let second_member_idx = first_member_idx + 1;
        let Some(primary_effect_idx) = coordination.members[first_member_idx]
            .effects
            .len()
            .checked_sub(1)
        else {
            continue;
        };
        let Some(linked_filter) = coordination.members[second_member_idx]
            .effects
            .first()
            .and_then(direct_all_object_filter)
        else {
            continue;
        };
        let excludes_primary = linked_filter.other
            || linked_filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            });
        if !filter_has_linked_it_constraint(linked_filter) || !excludes_primary {
            continue;
        }

        if has_trailing_that_name {
            'trailing: for member in coordination.members.iter_mut().skip(second_member_idx + 1) {
                for effect in &mut member.effects {
                    let Some(trailing_filter) = direct_all_object_filter_mut(effect) else {
                        continue;
                    };
                    if !filter_has_it_reference(trailing_filter) {
                        trailing_filter
                            .tagged_constraints
                            .push(TaggedObjectConstraint {
                                tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                                relation: TaggedOpbjectRelation::SameNameAsTagged,
                            });
                    }
                    break 'trailing;
                }
            }
        }

        let primary_alias =
            crate::tag::CompilerIndexedTag::LinkedFanoutPrimary.key(first_member_idx);
        let group_alias = crate::tag::CompilerIndexedTag::LinkedFanoutGroup.key(first_member_idx);

        let primary = coordination.members[first_member_idx]
            .effects
            .remove(primary_effect_idx);
        coordination.members[first_member_idx].effects.insert(
            primary_effect_idx,
            EffectAst::TagAffected {
                effect: Box::new(primary),
                tag: primary_alias.clone(),
            },
        );

        let mut related_filter = direct_all_object_filter(
            coordination.members[second_member_idx]
                .effects
                .first()
                .expect("linked coordination member has one effect"),
        )
        .expect("linked coordination filter was matched before tagging")
        .clone();
        related_filter
            .tagged_constraints
            .retain(|constraint| constraint.relation != TaggedOpbjectRelation::IsNotTaggedObject);
        related_filter.other = false;
        for constraint in &mut related_filter.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                constraint.tag = primary_alias.clone().into();
            }
        }

        let mut primary_filter = ObjectFilter::default();
        primary_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: primary_alias.clone().into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        let mut group_filter = related_filter.clone();
        group_filter.other = false;
        group_filter.tagged_constraints.clear();
        group_filter.any_of.clear();
        group_filter.any_of.push(primary_filter);
        group_filter.any_of.push(related_filter);
        let group_zones = group_filter.zone.into_iter().collect::<Vec<_>>();

        for member in coordination.members.iter_mut().skip(second_member_idx + 1) {
            for effect in &mut member.effects {
                retag_linked_fanout_followup(effect, &group_alias);
            }
        }

        let fanout = coordination.members[second_member_idx].effects.remove(0);
        coordination.members[second_member_idx].effects.insert(
            0,
            EffectAst::TagAffected {
                effect: Box::new(fanout),
                tag: group_alias.clone(),
            },
        );
        coordination.members[second_member_idx].effects.insert(
            1,
            EffectAst::subject_verb_tagged_object_union(
                group_filter,
                group_zones,
                group_alias.clone(),
                vec![primary_alias, group_alias],
            ),
        );
        return;
    }
}

fn preserve_tapped_this_way_group_for_later_distribution(
    tokens: &[OwnedLexToken],
    effects: &mut Vec<EffectAst>,
) {
    let words = crate::lexer::token_word_refs(tokens);
    if !crate::word_primitives::sequence_occurs(&words, &["tapped", "this", "way"])
        || !crate::word_primitives::sequence_occurs(&words, &["any", "number", "of", "those"])
        || !crate::word_primitives::sequence_occurs(&words, &["divided"])
    {
        return;
    }

    let Some(tap_index) = crate::slice_primitives::select_position(effects, |effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { .. }),
                ..
            })
        )
    }) else {
        return;
    };
    let Some(distributed_index) = crate::slice_primitives::select_position(effects, |effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { .. }),
                ..
            })
        )
    }) else {
        return;
    };

    let alias = crate::tag::CompilerReferenceTag::TappedThisWayGroup.bind();
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { target, .. }),
        ..
    }) = &mut effects[distributed_index]
    {
        fn bind_target(target: &mut TargetAst, alias: &TagKey) {
            match target {
                TargetAst::Object(filter, _, _) => {
                    for constraint in &mut filter.tagged_constraints {
                        if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        {
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
    restriction: crate::model::compiler_semantic::CompilerManaUsageRestriction,
    tokens: &[OwnedLexToken],
) -> Result<(), CardTextError> {
    let Some(previous) = effects.pop() else {
        return Err(CardTextError::ParseError(format!(
            "mana restriction has no preceding mana effect (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    };

    if !effect_ast_can_produce_mana(&previous) {
        effects.push(previous);
        return Err(CardTextError::ParseError(format!(
            "mana restriction does not follow a mana-producing effect (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
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
            SubjectVerbActionAst::Mana(ManaActionAst::AddMana { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddManaScaled { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyColor { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyOneColor { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddManaChosenColor { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddManaFromLandCouldProduce { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddManaColorsAmong { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddOneManaAnyColorAmong { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddManaCommanderIdentity { .. })
                | SubjectVerbActionAst::Mana(ManaActionAst::AddManaImprintedColors)
        ),
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        })
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

fn parse_next_batch_enter_with_counters(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let view = crate::lexer::TokenWordView::new(&tokens);
    if !view.parses_prefix(&["the", "next", "time", "one", "or", "more"]) {
        return Ok(None);
    }
    let Some(enter_word_idx) = view.parse_phrase_start(&["enter", "this", "turn"]) else {
        return Ok(None);
    };
    if enter_word_idx <= 6 {
        return Ok(None);
    }
    let Some(mut tail_start) = view.token_index_after_words(enter_word_idx + 3) else {
        return Ok(None);
    };
    if tokens
        .get(tail_start)
        .is_some_and(|token| token.kind == TokenKind::Comma)
    {
        tail_start += 1;
    }
    let Some(tail_tokens) = tokens.get(tail_start..) else {
        return Ok(None);
    };
    let Some(counter) =
        effect_grammar::counter_marker_shapes::parse_tagged_enters_additional_tokens(tail_tokens)
    else {
        return Ok(None);
    };
    if !counter.descriptor.additional {
        return Ok(None);
    }

    let Some(object_start) = view.token_index_after_words(6) else {
        return Ok(None);
    };
    let Some(enter_token) = view.map_word_to_token_start(enter_word_idx) else {
        return Ok(None);
    };
    let mut filter = super::parse_object_filter(&tokens[object_start..enter_token], false)?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    Ok(Some(
        EffectAst::subject_verb_register_next_batch_enter_with_counters(
            filter,
            counter.descriptor.counter_type,
            Value::Fixed(counter.descriptor.count as i32),
        ),
    ))
}

#[cfg(test)]
mod next_batch_enter_with_counters_tests {
    use super::*;
    use crate::lexer::lex_line;
    use crate::types::CardType;

    #[test]
    fn parses_next_matching_simultaneous_entry_batch_as_typed_replacement() {
        let tokens = lex_line(
            "The next time one or more enchantment creatures you control enter this turn, each enters with two additional +1/+1 counters on it.",
            0,
        )
        .unwrap();
        let effect = parse_next_batch_enter_with_counters(&tokens)
            .unwrap()
            .expect("next-batch entry replacement should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterNextBatchEnterWithCounters {
                    filter,
                    counter_type: crate::object::CounterType::PlusOnePlusOne,
                    count: Value::Fixed(2),
                }),
            ..
        }) = effect
        else {
            panic!("expected typed next-batch entry replacement: {effect:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(
            filter.all_card_types,
            [CardType::Enchantment, CardType::Creature]
        );
    }

    #[test]
    fn public_multi_sentence_route_keeps_next_batch_registration() {
        let tokens = lex_line(
            "Put two lore counters on target Saga you control. The next time one or more enchantment creatures you control enter this turn, each enters with two additional +1/+1 counters on it.",
            0,
        )
        .unwrap();
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("the complete modal bullet should parse");

        assert!(
            effects.iter().any(|effect| matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterNextBatchEnterWithCounters {
                        filter,
                        counter_type: crate::object::CounterType::PlusOnePlusOne,
                        count: Value::Fixed(2),
                    }),
                    ..
                }) if filter.zone == Some(Zone::Battlefield)
                    && filter.controller == Some(PlayerFilter::You)
                    && filter.all_card_types
                        == [CardType::Enchantment, CardType::Creature]
            )),
            "public route must not lower the second sentence as a permanent GrantAbility: {effects:#?}"
        );
    }

    #[test]
    fn does_not_promote_singular_or_persistent_entry_rules_to_batch_one_shots() {
        for text in [
            "The next time an enchantment creature you control enters this turn, it enters with two additional +1/+1 counters on it.",
            "Until end of turn, enchantment creatures you control enter with two additional +1/+1 counters on them.",
            "The next time one or more enchantment creatures you control enter this turn, each enters with two +1/+1 counters on it.",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert!(
                parse_next_batch_enter_with_counters(&tokens)
                    .unwrap()
                    .is_none(),
                "near miss must not acquire next-batch semantics: {text}"
            );
        }
    }
}

#[cfg(test)]
mod resolving_card_countered_exile_tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn resolving_card_countered_exile_and_free_cast_are_typed() {
        let replacement = lex_line(
            "Exile that card with a dream counter on it instead of putting it into your graveyard as it resolves.",
            0,
        )
        .unwrap();
        let parsed = parse_effect_sentences_lexed(&replacement).unwrap();
        assert!(
            matches!(
                parsed.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                        counters,
                        replacement_zone: Zone::Exile,
                        ..
                    }),
                    ..
                })] if counters == &vec![(crate::object::CounterType::Dream, 1)]
            ),
            "expected the resolving-card replacement to retain its Dream counter: {parsed:#?}"
        );

        let permission = lex_line(
            "You may cast a spell from among cards you own in exile with dream counters on them without paying its mana cost.",
            0,
        )
        .unwrap();
        let parsed = parse_effect_sentences_lexed(&permission).unwrap();
        assert!(
            matches!(
                parsed.as_slice(),
                [EffectAst::Permissions(PermissionEffectAst::May { effects })]
                    if matches!(effects.as_slice(), [
                        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, tag, .. }),
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::Stack(StackActionAst::CastTagged {
                                tag: cast_tag,
                                without_paying_mana_cost: true,
                                ..
                            }),
                            ..
                        })
                    ] if filter.with_counter
                        == Some(crate::filter::CounterConstraint::Typed(
                            crate::object::CounterType::Dream,
                        )) && tag == cast_tag)
            ),
            "{parsed:#?}"
        );
    }

    #[test]
    fn similar_move_and_cast_surfaces_do_not_gain_dream_replacement_semantics() {
        for text in [
            "Exile that card with a dream counter on it.",
            "Exile that card with a dream counter on it instead of putting it into your hand as it resolves.",
            "You may cast a spell from among cards you own in exile without paying its mana cost.",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let debug = format!("{:#?}", parse_effect_sentences_lexed(&tokens).unwrap());
            assert!(
                !debug.contains("RegisterZoneReplacement { target: Tagged")
                    || !debug.contains("counters: [(Dream, 1)]"),
                "overclaimed: {text}: {debug}"
            );
        }
    }
}

#[inline(never)]
pub(super) fn parse_effect_sentences_lexed_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = parse_complete_create_statement(tokens)? {
        return Ok(effects);
    }
    if effect_grammar::sentence_predicate_shapes::parse_quoted_ability_sentence_tokens(tokens)
        .is_some()
        && {
            // Count sentence boundaries only before the first quote: nested
            // quoted abilities legitimately contain their own periods, but a
            // resolution sentence ahead of the grant must reach ordinary
            // sentence dispatch.
            let first_quote = tokens
                .iter()
                .position(|token| token.kind == TokenKind::Quote)
                .unwrap_or(tokens.len());
            crate::lexer::split_lexed_sentences(&tokens[..first_quote]).len() <= 1
        }
        && !crate::word_primitives::sequence_occurs(
            &crate::lexer::parser_token_word_refs(tokens),
            &["as", "long", "as"],
        )
        && effect_grammar::delayed_sentence_shapes::parse_delayed_this_turn_shape(tokens).is_none()
        && super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens).len() == 1
        && super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![tokens]).len() == 1
        && let Some(effects) = super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::dispatch_inner::parse_attacking_doesnt_tap_if_source_untapped(tokens)?
    {
        return Ok(effects);
    }
    dispatch_effect_sentences_lexed_inner_remaining(tokens)
}

mod document_remaining_readings;

#[inline(never)]
fn dispatch_effect_sentences_lexed_inner_remaining(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let input = document_remaining_readings::RemainingDocument {
        tokens,
        read_by_cache: Default::default(),
    };
    match document_remaining_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    let sentence_segments = split_quoted_grant_then_vote_option_sentences(
        split_leading_amass_comma_then_sentences(split_lexed_sentences(tokens)),
    );
    let sentences = sentence_segments
        .into_iter()
        .map(SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    let mut effects = parse_effect_sentences_from_sentence_inputs(sentences)?;
    group_this_way_copy_cast_followups(tokens, &mut effects);
    apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
    maybe_bind_that_player_gain_control_if_do_rewards(&mut effects, tokens);
    Ok(effects)
}

/// Parse a resolving rule such as "Permanents enter tapped this turn."
/// The subject remains a normal object filter so the capability also covers
/// narrower turn-scoped entry rules without tying the effect to one card.
fn parse_turn_scoped_enter_tapped_replacement(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let view = crate::lexer::TokenWordView::new(tokens);
    let Some(enter_word) = view.parse_any_word_position(&["enter", "enters"]) else {
        return Ok(None);
    };
    if !view.parses_complete_at(enter_word + 1, &["tapped", "this", "turn"]) {
        return Ok(None);
    }
    let Some(enter_index) = view.map_word_to_token_start(enter_word) else {
        return Ok(None);
    };
    let subject_tokens = trim_edge_punctuation(&tokens[..enter_index]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = super::parse_object_filter(&subject_tokens, false)?;
    filter.zone = Some(Zone::Battlefield);
    Ok(Some(
        EffectAst::subject_verb_register_enter_tapped_replacement(
            filter,
            ZoneReplacementDurationAst::UntilEndOfTurn,
        ),
    ))
}

/// Parse a resolving effect that establishes a turn-long cost for each
/// creature declared as a blocker. The affected creature filter remains live
/// for the duration, while the activation's X value is captured at resolution.
/// "Until end of turn, if you tap a land you control for mana, it produces
/// {U} instead of any other type." (Deep Water) — a whole-sentence shape that
/// registers a turn-scoped mana-production replacement. The clause carries
/// its own scope and duration, so it must not be split into a generic
/// conditional around a verb clause.
fn parse_tapped_land_mana_replacement(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let spec = effect_grammar::parse_mana_replacement_clause_spec_lexed(tokens)?;
    Some(EffectAst::SubjectVerb(
        crate::model::ast::SubjectVerbEffectAst {
            subject: crate::model::ast::SubjectVerbSubjectAst {
                role: SubjectVerbRoleAst::Actor,
                player: PlayerAst::Implicit,
            },
            action: SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterManaReplacement {
                source_filter: crate::target::ObjectFilter::default()
                    .with_type(crate::types::CardType::Land)
                    .you_control(),
                replacement_mana: vec![spec.replacement_mana],
                mode: crate::effects::ReplacementApplyMode::UntilEndOfTurn,
            }),
        },
    ))
}

pub(super) fn parse_temporary_per_blocker_tax(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if !crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "this",
            "turn",
            "creatures",
            "can't",
            "block",
            "unless",
            "their",
            "controller",
            "pays",
            "for",
            "each",
            "blocking",
            "creature",
            "they",
            "control",
        ],
    ) {
        return Ok(None);
    }

    let view = crate::lexer::TokenWordView::new(tokens);
    let Some(pays_word) = view.parse_word_position("pays") else {
        return Ok(None);
    };
    let Some(for_word) = view.parse_any_word_position_from(&["for"], pays_word + 1) else {
        return Ok(None);
    };
    let Some(cost_start) = view.token_index_after_words(pays_word + 1) else {
        return Ok(None);
    };
    let Some(for_index) = view.map_word_to_token_start(for_word) else {
        return Ok(None);
    };
    let cost_tokens = trim_edge_punctuation(&tokens[cost_start..for_index]);
    let mana_cost = crate::grammar::values::parse_mana_cost_tokens(&cost_tokens)?;
    if !mana_cost.has_x() {
        return Ok(None);
    }
    let cost = ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::DynamicMana(
        ironsmith_core::DynamicManaCost::new(
            mana_cost,
            None,
            None,
            None,
            ironsmith_core::DynamicManaDisplayHint::Default,
        ),
    ));
    let block_cost = StaticAbility::block_cost(
        ObjectFilter::source(),
        ObjectFilter::creature(),
        cost,
        "This creature can't block unless its controller pays {X}",
    );
    Ok(Some(
        EffectAst::subject_verb_grant_abilities_all_dynamically(
            ObjectFilter::creature(),
            vec![GrantedAbilityAst::StaticAbility(Box::new(
                crate::cards::builders::StaticAbilityAst::Static(block_cost),
            ))],
            Until::EndOfTurn,
        ),
    ))
}

fn parse_restart_game_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let view = crate::lexer::TokenWordView::new(tokens);
    if !view.parses_prefix(&["restart", "the", "game"]) {
        return Ok(None);
    }

    if view.len() == 3 {
        return Ok(Some(EffectAst::RestartGame {
            cards_left_in_exile: None,
            source_surface: None,
        }));
    }

    if view.len() < 9 || !view.parses_prefix_at(3, &["leaving", "in", "exile"]) {
        return Err(CardTextError::ParseError(
            "unsupported restart-game continuation".to_string(),
        ));
    }

    let Some(exiled_word_idx) = view
        .parse_phrase_start(&["exiled", "with"])
        .filter(|word_idx| *word_idx >= 6)
    else {
        return Err(CardTextError::ParseError(
            "restart-game exile exemption is missing `exiled with`".to_string(),
        ));
    };

    let Some(object_start) = view.token_index_after_words(6) else {
        return Ok(None);
    };
    let Some(object_end) = view.map_word_to_token_start(exiled_word_idx) else {
        return Ok(None);
    };
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
        tag: (crate::tag::CompilerReferenceTag::SourceExiled.bind()).into(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let Some(source_start) = view.token_index_after_words(exiled_word_idx + 2) else {
        return Ok(None);
    };
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

fn is_play_magic_subgame_sentence(tokens: &[OwnedLexToken]) -> bool {
    crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(tokens),
        &[
            "players",
            "play",
            "a",
            "magic",
            "subgame",
            "using",
            "their",
            "libraries",
            "as",
            "their",
            "decks",
        ],
    )
}

fn is_subgame_half_life_nonwinner_sentence(tokens: &[OwnedLexToken]) -> bool {
    crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(tokens),
        &[
            "each", "player", "who", "doesn't", "win", "the", "subgame", "loses", "half", "their",
            "life", "rounded", "up",
        ],
    )
}

fn split_leading_amass_comma_then_sentences(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
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

fn split_quoted_grant_then_vote_option_sentences(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        let mut inside_quotes = false;
        let mut split = None;
        for (index, token) in segment.iter().enumerate() {
            if token.kind == TokenKind::Quote {
                inside_quotes = !inside_quotes;
                continue;
            }
            if !inside_quotes
                && index > 0
                && token.is_word("then")
                && segment[..index]
                    .iter()
                    .any(|token| token.kind == TokenKind::Period)
                && effect_grammar::parse_named_vote_option_effects_shape(&segment[index..])
                    .is_some()
            {
                split = Some(index);
                break;
            }
        }
        if let Some(index) = split {
            result.push(&segment[..index]);
            result.push(&segment[index..]);
        } else {
            result.push(segment);
        }
    }
    result
}

fn is_copy_reference_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { .. })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { .. })
                | SubjectVerbActionAst::Stack(StackActionAst::CopySpell { .. })
                | SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget { .. }),
            ..
        })
    )
}

fn is_may_cast_copy_effect(effect: &EffectAst) -> bool {
    let EffectAst::Permissions(PermissionEffectAst::May { effects }) = effect else {
        return false;
    };
    matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Stack(StackActionAst::CastTagged { as_copy: true, .. }),
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
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                ..
            })
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

    if let EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        effects: nested, ..
    }) = &mut effects[if_idx]
    {
        nested.extend(followups);
    }
}

pub fn is_cant_be_regenerated_this_turn_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::followup_shapes::parse_cant_be_regenerated_followup(tokens)
        .is_some_and(|shape| shape.this_turn)
}

#[cfg(test)]
pub fn is_cant_be_regenerated_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::followup_shapes::parse_cant_be_regenerated_followup(tokens).is_some()
}

pub fn apply_cant_be_regenerated_to_last_destroy_effect(effects: &mut [EffectAst]) -> bool {
    let Some(last) = effects.last_mut() else {
        return false;
    };
    apply_cant_be_regenerated_to_effect(last)
}

pub fn apply_cant_be_regenerated_to_last_destroy_group(effects: &mut [EffectAst]) -> bool {
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

pub fn mark_last_destroy_creature_destroyed_this_way_surface(effects: &mut [EffectAst]) -> bool {
    fn mark(effect: &mut EffectAst) -> bool {
        match effect {
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                    creature_destroyed_this_way_surface,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll {
                    creature_destroyed_this_way_surface,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor {
                    creature_destroyed_this_way_surface,
                    ..
                }) => {
                    *creature_destroyed_this_way_surface = true;
                    true
                }
                _ => false,
            },
            EffectAst::Coordinated { effects, .. } => effects
                .iter_mut()
                .fold(false, |found, effect| mark(effect) || found),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { modes }) | EffectAst::ObjectChoices(ObjectChoiceEffectAst::VillainousChoice { modes, .. }) => {
                modes.iter_mut().fold(false, |found, mode| {
                    mode.effects.last_mut().is_some_and(mark) || found
                })
            }
            _ => {
                let mut applied = false;
                for_each_nested_effects_mut(effect, true, |nested| {
                    if !applied {
                        applied = nested.last_mut().is_some_and(mark);
                    }
                });
                applied
            }
        }
    }

    effects.last_mut().is_some_and(mark)
}

#[cfg(test)]
mod tests {
    #[test]
    fn participant_actions_keep_the_opponent_iteration() {
        for text in ["Each opponent draws a card.", "Each opponent discards a card and loses 2 life."] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let effects = super::parse_effect_sentences_lexed(&tokens).unwrap();
            assert!(format!("{effects:?}").contains("ForEachOpponent"), "{text}: {effects:#?}");
        }
    }

    #[test]
    fn coordinated_modifiers_keep_independent_trailing_conditions() {
        for text in [
            "Creatures your opponents control get -1/-0 until end of turn if {U} was spent to cast this spell, and creatures you control get +1/+0 until end of turn if {R} was spent to cast this spell.",
            "Creatures your opponents control lose flying until end of turn if {G} was spent to cast this spell, and creatures you control gain flying until end of turn if {U} was spent to cast this spell.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let effects = super::parse_effect_sentences_lexed(&tokens).unwrap();
            assert!(matches!(effects.as_slice(), [crate::cards::builders::EffectAst::Coordination(_)]), "{effects:#?}");
        }
    }

    #[test]
    fn dynamic_pump_does_not_consume_a_later_optional_fight() {
        let tokens = crate::lexer::lex_line("Choose up to one target creature you don't control with flying. This creature gets +X/+0 until end of turn, where X is the number of creature cards in your graveyard, then you may have this creature fight that creature.", 0).unwrap();
        let effects = super::parse_effect_sentences_lexed(&tokens).unwrap();
        let debug = format!("{effects:#?}");
        assert!(debug.contains("Fight"), "{debug}");
        assert!(debug.contains("May"), "{debug}");
    }

    #[test]
    fn optional_viewed_card_fallback_preserves_remainder_order() {
        for (order, expected) in [("in a random order", "Random"), ("in any order", "ChooserChooses")] {
            let text = format!("Look at the top five cards of your library. You may put a creature card from among them onto the battlefield tapped. If you don't, put a card from among them into your hand. Put the rest on the bottom of your library {order}.");
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            let effects = super::parse_effect_sentences_lexed(&tokens).unwrap();
            let debug = format!("{effects:#?}");
            assert!(debug.contains("PutTaggedRemainderOnBottomOfLibrary"), "{debug}");
            assert!(debug.contains(expected), "{debug}");
        }
    }

    #[test]
    fn return_sequence_keeps_the_discard_followup() {
        for text in [
            "Return target permanent to its owner's hand, then that player discards two cards.",
            "Return target creature card from your graveyard to your hand, then discard a card.",
        ] {
        let tokens = crate::lexer::lex_line(
            text, 0,
        ).unwrap();
        let effects = super::parse_effect_sentences_lexed(&tokens).unwrap();
        let debug = format!("{effects:#?}");
        assert!(debug.contains("Return"), "{debug}");
        assert!(debug.contains("Discard"), "{debug}");
        }
    }

    #[test]
    fn conditional_counter_document_keeps_the_otherwise_branch() {
        let tokens = crate::lexer::lex_line(
            "If this creature is suspected, put two +1/+1 counters on it. Otherwise, suspect it.", 0,
        ).unwrap();
        let effects = super::parse_effect_sentences_lexed(&tokens).unwrap();
        let debug = format!("{effects:#?}");
        assert!(debug.contains("PutCounters"), "{debug}");
        assert!(debug.contains("action: Suspect("), "{debug}");
    }

    #[test]
    fn complete_pump_defers_to_a_following_action() {
        let tokens = crate::lexer::lex_line(
            "This creature gets +2/+1 until end of turn, then scry 2.", 0,
        ).unwrap();
        assert!(super::parse_complete_get_pump_statement(&tokens).unwrap().is_none());
        let effects = super::parse_effect_sentences_lexed(&tokens).unwrap();
        let debug = format!("{effects:#?}");
        assert!(debug.contains("Pump"), "{debug}");
        assert!(debug.contains("Scry"), "{debug}");
    }

    use crate::cards::builders::TurnEventPredicateAst;
    use crate::cards::builders::SourcePredicateAst;
    use crate::cards::builders::PermissionEffectAst;
    use crate::cards::builders::ConditionalEffectAst;
    use crate::cards::builders::VoteEffectAst;
    use crate::cards::builders::ObjectChoiceEffectAst;
    use crate::cards::builders::ForEachEffectAst;
    use crate::cards::builders::DelayedEffectAst;
    use crate::cards::builders::TokenActionAst;
    use crate::cards::builders::StatChangeActionAst;
    use crate::cards::builders::LifeResourceActionAst;
    use crate::cards::builders::RevealLookActionAst;
    use crate::cards::builders::PermanentStateActionAst;
    use crate::cards::builders::ZoneMoveActionAst;
    use crate::cards::builders::KeywordActionAst;
    use crate::cards::builders::CharacteristicActionAst;
    use crate::cards::builders::ReplacementActionAst;
    use crate::cards::builders::LibraryActionAst;
    use crate::cards::builders::ManaActionAst;
    use crate::cards::builders::GrantActionAst;
    use crate::cards::builders::{
        EffectAst, IfResultPredicate, PlayerAst, PredicateAst, SubjectVerbActionAst, find_verb,
    };
    use crate::effect::{Value, ValueComparisonOperator};
    use crate::filter::TaggedOpbjectRelation;
    use crate::model::visit::{TerminalResultProducer, terminal_result_producer};
    use crate::target::PlayerFilter;

    use super::super::super::grammar::structure::split_lexed_sentences;
    use super::super::super::lexer::lex_line;
    use super::super::super::permission_helpers::parse_until_end_of_turn_may_play_tagged_clause;
    use super::super::super::util::{parse_subject, trim_commas};
    use super::super::chain_carry::Verb;
    use super::super::parse_typed_effect_bundle_lexed;
    use super::super::zone_handlers::parse_exile_top_library_clause;
    use super::super::{parse_effect_chain, parse_effect_sentence_lexed};
    use super::{
        ConsultCastCost, ConsultCastTiming, parse_bargained_face_down_cast_mana_value_gate,
        parse_consult_cast_clause, parse_consult_condition_value,
        parse_consult_mana_value_condition_tokens,
        parse_counted_looked_cards_into_your_hand_tokens, parse_effect_sentences_lexed,
        parse_effect_sentences_lexed_inner, parse_if_you_dont_sentence,
        parse_looked_card_reveal_filter,
        parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
        parse_top_cards_view_sentence,
    };

    #[test]
    fn serial_creature_subtype_subject_reaches_public_effect_sentence_route() {
        let tokens = lex_line(
            "Birds, Frogs, Otters, and Rats you control get +1/+1 until end of turn. Untap them.",
            0,
        )
        .expect("serial subtype trigger body should lex");
        let effects = parse_effect_sentences_lexed_inner(&tokens)
            .expect("serial subtype trigger body should parse through public dispatch");
        let debug = format!("{effects:#?}");

        for subtype in ["Bird", "Frog", "Otter", "Rat"] {
            assert!(debug.contains(subtype), "missing {subtype}: {debug}");
        }
        assert!(debug.contains("PumpAll"), "{debug}");
        assert!(debug.contains("UntapAll"), "{debug}");
    }

    #[test]
    fn coordinated_protection_domains_reach_one_public_grant_route() {
        let tokens = lex_line(
            "Another target creature you control gains protection from colorless or from the color of your choice until end of turn.",
            0,
        )
        .expect("coordinated protection grant should lex");
        let effects = parse_effect_sentences_lexed_inner(&tokens)
            .expect("coordinated protection grant should parse through public dispatch");
        assert_eq!(effects.len(), 1, "{effects:#?}");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("GrantProtectionChoice"), "{debug}");
        assert!(debug.contains("chooser: You"), "{debug}");
        assert!(debug.contains("allow_colorless: true"), "{debug}");
    }

    #[test]
    fn temporary_flash_and_cast_trigger_grant_remain_sibling_typed_effects() {
        let tokens = lex_line(
            "You may cast Dinosaur spells this turn as though they had flash, and whenever you cast a Dinosaur spell this turn, it gains \"When this creature enters, you may have it fight another target creature.\"",
            0,
        )
        .expect("coordinated permission should lex");
        assert_eq!(
            super::super::lex_chain_helpers::split_effect_chain_on_and_lexed(&tokens).len(),
            2,
            "the permission and cast-trigger grant should be a grammar-proven conjunction"
        );
        let effects = parse_effect_sentences_lexed_inner(&tokens)
            .expect("coordinated permission should parse");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("GrantBySpec"), "{debug}");
        assert!(debug.contains("Flash"), "{debug}");
        assert!(
            debug.contains("GrantAbilitiesToTarget") || debug.contains("ApplyContinuous"),
            "{debug}"
        );
    }

    #[test]
    fn next_turn_flash_and_entry_counter_replacement_share_duration_as_siblings() {
        let tokens = lex_line(
            "Until your next turn, you may cast creature spells as though they had flash, and each creature you control enters with an additional +1/+1 counter on it.",
            0,
        )
        .expect("coordinated next-turn permission should lex");
        let effects = parse_effect_sentences_lexed_inner(&tokens)
            .expect("coordinated next-turn permission should parse");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("GrantBySpec"), "{debug}");
        assert!(debug.contains("UntilYourNextTurnEnd"), "{debug}");
        assert!(
            debug.contains("EnterWithCounters") || debug.contains("EntryCounter"),
            "{debug}"
        );
    }

    #[test]
    fn complete_target_declaration_owns_embedded_put_history_verb() {
        let tokens = lex_line(
            "Choose up to three target permanent cards in graveyards that were put there from the battlefield this turn.",
            0,
        )
        .expect("historical target declaration should lex");
        let effects = parse_effect_sentences_lexed_inner(&tokens)
            .expect("historical target declaration should use the direct target route");
        let debug = format!("{effects:#?}");

        assert_eq!(debug.matches("TargetOnly").count(), 1, "{debug}");
        assert!(debug.contains("explicit_declaration: true"), "{debug}");
        assert!(debug.contains("zone: Some("), "{debug}");
        assert!(debug.contains("Graveyard"), "{debug}");
        assert!(
            debug.contains("entered_graveyard_from_battlefield_this_turn: true"),
            "{debug}"
        );
        assert!(
            !debug.contains("MoveToZone"),
            "the embedded relative-clause verb became a second action: {debug}"
        );
    }

    #[test]
    fn multi_sentence_loop_keeps_embedded_put_history_inside_target_declaration() {
        let tokens = lex_line(
            "Choose up to three target permanent cards in graveyards that were put there from the battlefield this turn. Return them to the battlefield tapped under their owners' control. You draw a card for each opponent who controls one or more of those permanents.",
            0,
        )
        .expect("historical return program should lex");
        let effects = parse_effect_sentences_lexed(&tokens)
            .expect("the complete program should keep the historical target typed");
        let debug = format!("{effects:#?}");

        assert_eq!(debug.matches("TargetOnly").count(), 1, "{debug}");
        assert!(
            debug.contains("entered_graveyard_from_battlefield_this_turn: true"),
            "{debug}"
        );
        assert!(debug.contains("ReturnToBattlefield"), "{debug}");
        assert!(debug.contains("PlayerControls"), "{debug}");
    }

    #[test]
    fn draw_where_x_counts_distinct_graveyard_card_types() {
        let tokens = lex_line(
            "Draw X cards, where X is the number of card types among cards in your graveyard.",
            0,
        )
        .expect("dynamic draw should lex");
        let effects = parse_effect_sentences_lexed_inner(&tokens)
            .expect("dynamic draw should parse through the public sentence route");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("CardTypesInGraveyard(\n"), "{debug}");
        assert!(debug.contains("You"), "{debug}");
        assert!(!debug.contains("Count(\n"), "{debug}");
    }

    #[test]
    fn quantified_token_creation_keeps_multiple_quoted_rules_on_the_created_token() {
        let tokens = lex_line(
            "Each opponent creates a 1/1 red Pirate creature token with \"This token can't block\" and \"Creatures you control attack each combat if able.\"",
            0,
        )
        .expect("quantified token creation should lex");
        let parsed = parse_effect_sentences_lexed(&tokens)
            .expect("quantified token creation should parse through document dispatch");
        let [EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })] = parsed.as_slice() else {
            panic!("expected one opponent iteration, got {parsed:#?}");
        };
        let [EffectAst::SubjectVerb(effect)] = effects.as_slice() else {
            panic!("expected one nested token creation, got {effects:#?}");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            player,
            definition,
            granted_abilities,
            ..
        }) = &effect.action
        else {
            panic!("expected a typed token creation, got {effect:#?}");
        };
        assert_eq!(player, &PlayerAst::That);

        let ast_debug = format!("{definition:#?}\n{granted_abilities:#?}");
        assert!(ast_debug.contains("CantBlock"), "{ast_debug}");
        assert!(ast_debug.contains("MustAttack"), "{ast_debug}");
        assert!(
            !ast_debug.contains("MustBlockSpecificAttacker"),
            "quoted token rule escaped into the outer action: {ast_debug}"
        );

        let (lowered, _) = crate::compile_support::compile_effects(
            &parsed,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("quantified token creation should lower");
        let lowered_debug = format!("{lowered:#?}");
        assert!(
            lowered_debug.contains("ForPlayersEffect"),
            "{lowered_debug}"
        );
        assert!(
            lowered_debug.contains("CreateTokenEffect"),
            "{lowered_debug}"
        );
        assert!(lowered_debug.contains("CantBlock"), "{lowered_debug}");
        assert!(lowered_debug.contains("MustAttack"), "{lowered_debug}");
        assert!(
            !lowered_debug.contains("MustBlockSpecificAttacker"),
            "{lowered_debug}"
        );

        let public_dispatch = parse_effect_sentences_lexed_inner(&tokens)
            .expect("the normalized public sentence loop should preserve the token creation");
        let public_debug = format!("{public_dispatch:#?}");
        assert!(
            public_debug.contains("CreateTokenWithMods"),
            "{public_debug}"
        );
        let public_token_rules = public_dispatch
            .iter()
            .find_map(|effect| match effect {
                EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }) => effects.iter().find_map(|effect| {
                    let EffectAst::SubjectVerb(effect) = effect else {
                        return None;
                    };
                    let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                        definition,
                        granted_abilities,
                        ..
                    }) = &effect.action
                    else {
                        return None;
                    };
                    Some(format!("{definition:#?}\n{granted_abilities:#?}"))
                }),
                _ => None,
            })
            .unwrap_or_else(|| panic!("public route lost token semantics: {public_dispatch:#?}"));
        assert!(
            public_token_rules.contains("CantBlock"),
            "{public_token_rules}"
        );
        assert!(
            public_token_rules.contains("MustAttack"),
            "{public_token_rules}"
        );
        assert!(
            !public_debug.contains("MustBlockSpecificAttacker"),
            "the public loop let a quoted rule escape into the outer action: {public_debug}"
        );

        let near_miss = lex_line(
            "Each opponent creates a 1/1 red Pirate creature token with \"This token can't block.\"",
            0,
        )
        .expect("single-rule token creation should lex");
        let near_miss = parse_effect_sentences_lexed(&near_miss)
            .expect("single-rule token creation should still parse");
        let [EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })] = near_miss.as_slice() else {
            panic!("single-rule route lost opponent iteration: {near_miss:#?}");
        };
        let [EffectAst::SubjectVerb(effect)] = effects.as_slice() else {
            panic!("single-rule route lost token creation: {near_miss:#?}");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            definition,
            granted_abilities,
            ..
        }) = &effect.action
        else {
            panic!("single-rule route lost token definition: {near_miss:#?}");
        };
        let near_miss_debug = format!("{definition:#?}\n{granted_abilities:#?}");
        assert!(near_miss_debug.contains("CantBlock"), "{near_miss_debug}");
        assert!(
            !near_miss_debug.contains("MustAttack"),
            "a missing quoted rule must not be invented: {near_miss_debug}"
        );
    }

    #[test]
    fn three_sentence_created_token_lifecycle_stays_one_effect_program() {
        for text in [
            "Create a 1/1 colorless Construct artifact creature token. Exile that token when this leaves the battlefield. Sacrifice this when that token leaves the battlefield.",
            "Create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when this leaves the battlefield. Sacrifice this when that token leaves the battlefield.",
        ] {
            let tokens = lex_line(text, 0).expect("created-token lifecycle should lex");
            let effects = parse_effect_sentences_lexed(&tokens)
                .expect("created-token lifecycle should parse through the public sentence route");
            let debug = format!("{effects:#?}");

            assert_eq!(effects.len(), 3, "{text}: {debug}");
            assert!(debug.contains("CreateTokenWithMods"), "{text}: {debug}");
            assert!(debug.contains("ExileWhenSourceLeaves"), "{text}: {debug}");
            assert!(
                debug.contains("SacrificeSourceWhenLeaves"),
                "{text}: {debug}"
            );
        }
    }

    #[test]
    fn otherwise_optional_cast_stays_optional_only_in_the_false_arm() {
        let cast_tokens = lex_line("you may cast it without paying its mana cost", 0)
            .expect("optional cast clause should lex");
        let cast_effects =
            parse_effect_sentence_lexed(&cast_tokens).expect("optional cast clause should parse");
        assert!(
            matches!(
                cast_effects.as_slice(),
                [EffectAst::Permissions(PermissionEffectAst::May { .. }) | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { .. })]
            ),
            "standalone optional cast lost optionality: {cast_effects:#?}"
        );

        let tokens = lex_line(
            "If it's a land card, you may put it onto the battlefield under your control. Otherwise, you may cast it without paying its mana cost.",
            0,
        )
        .expect("conditional cast line should lex");
        let effects = parse_effect_sentences_lexed(&tokens)
            .expect("conditional optional cast line should parse");
        let [
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                if_true, if_false, ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one conditional, got {effects:#?}");
        };

        assert!(
            matches!(
                if_true.as_slice(),
                [EffectAst::Permissions(PermissionEffectAst::May { .. }) | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { .. })]
            ),
            "true arm lost optionality: {effects:#?}"
        );
        assert!(
            matches!(
                if_false.as_slice(),
                [EffectAst::Permissions(PermissionEffectAst::May { .. }) | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { .. })]
            ),
            "false arm lost optionality: {effects:#?}"
        );
    }

    fn empty_mana_pool_player(effect: &EffectAst) -> Option<PlayerAst> {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && matches!(subject_verb.action, SubjectVerbActionAst::Mana(ManaActionAst::EmptyManaPool))
        {
            return Some(subject_verb.subject.player);
        }
        let mut found = None;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            if found.is_none() {
                found = nested.iter().find_map(empty_mana_pool_player);
            }
        });
        found
    }

    #[test]
    fn where_x_ability_count_preserves_authored_ability_noun() {
        let value = Value::Count(
            crate::filter::ObjectFilter::default()
                .in_zone(crate::zone::Zone::Graveyard)
                .owned_by(PlayerFilter::You)
                .with_ability_marker("cycling"),
        );
        let explicit_tokens = lex_line(
            "where X is the number of cards with a cycling ability in your graveyard",
            0,
        )
        .expect("lex explicit ability noun");
        let compact_tokens = lex_line(
            "where X is the number of cards with cycling in your graveyard",
            0,
        )
        .expect("lex compact ability marker");

        let explicit = super::with_where_x_surface_hints(value.clone(), &explicit_tokens);
        let compact = super::with_where_x_surface_hints(value, &compact_tokens);
        assert!(explicit.has_surface_hint(ironsmith_core::ValueSurfaceHint::ExplicitAbilityNoun));
        assert!(!compact.has_surface_hint(ironsmith_core::ValueSurfaceHint::ExplicitAbilityNoun));
    }

    #[test]
    fn leading_duration_coordinated_chain_bypasses_gain_fast_path() {
        let tokens = lex_line(
            "Until end of turn, double target creature's power and it gains first strike.",
            0,
        )
        .expect("coordinated duration sentence should lex");
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("coordinated duration sentence should parse");
        let [EffectAst::ControlFlow(control)] = effects.as_slice() else {
            panic!("expected one coordinated program, got {effects:#?}");
        };
        let crate::model::control_flow::ControlFlowNodeAst::Duration {
            duration: crate::model::control_flow::CompilerDurationAst::UntilEndOfTurn,
            program,
        } = &control.node
        else {
            panic!("expected an end-of-turn control-flow scope: {control:#?}");
        };
        let [EffectAst::Coordination(coordination)] = control
            .program(*program)
            .expect("duration program")
            .effects
            .as_slice()
        else {
            panic!("expected typed coordination inside duration: {control:#?}");
        };
        let coordinated = coordination.effects().cloned().collect::<Vec<_>>();
        assert!(
            matches!(
                coordinated.as_slice(),
                [
                    EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { .. }),
                        ..
                    }),
                    EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { .. }),
                        ..
                    }),
                ]
            ),
            "{coordinated:#?}"
        );
    }

    #[test]
    fn graveyard_play_permission_stays_whole_before_independent_replacement_sentence() {
        let tokens = lex_line(
            "Until end of turn, you may play lands and cast spells from your graveyard. \
             If a card would be put into your graveyard from anywhere this turn, exile that card instead.",
            0,
        )
        .expect("permission and replacement sentences should lex");

        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("permission conjunction must not split into `play lands`");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("PlayFromGraveyardUntilEot"), "{debug}");
        assert!(debug.contains("ExileInsteadOfGraveyardThisTurn"), "{debug}");
    }

    #[test]
    fn leading_duration_fast_paths_do_not_consume_a_following_sentence() {
        let tokens = lex_line(
            "Until your next turn, your life total can't change and you gain protection from everything. \
             All permanents you control phase out.",
            0,
        )
        .expect("duration and phase-out sentences should lex");

        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("both independent sentences should parse");
        let debug = format!("{effects:#?}");

        assert!(effects.len() >= 2, "{debug}");
        assert!(debug.contains("ChangeLifeTotal"), "{debug}");
        assert!(debug.contains("BeTargetedPlayer"), "{debug}");
        assert!(debug.contains("PreventAllDamageToTarget"), "{debug}");
        assert!(debug.contains("PhaseOutAll"), "{debug}");
    }

    #[test]
    fn public_sentence_loop_preserves_optional_looked_entry_with_counter() {
        let tokens = lex_line(
            "Look at the top seven cards of your library. You may put a permanent card with mana value 3 or less from among them onto the battlefield with a shield counter on it. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("optional looked-card procedure should lex");
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("the public dispatcher should keep the exact optional procedure");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("May"), "{debug}");
        assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
        assert!(debug.contains("Shield"), "{debug}");
        assert!(
            debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
            "{debug}"
        );
    }

    #[test]
    fn public_sentence_loop_preserves_hidden_partition_permission() {
        let tokens = lex_line(
            "Look at the top three cards of your library. Exile one face down and put the rest on the bottom of your library in any order. For as long as it remains exiled, you may cast it if it's a creature spell.",
            0,
        )
        .expect("hidden looked-card procedure should lex");
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("the exact procedure must preempt broad target parsing");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("Exile"), "{debug}");
        assert!(
            debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
            "{debug}"
        );
        assert!(
            debug.contains("GrantPlayTaggedForAsLongAsExiled"),
            "{debug}"
        );
        assert!(debug.contains("Creature"), "{debug}");
    }

    #[test]
    fn paid_label_condition_owns_its_complete_effects_in_the_public_sentence_family() {
        let tokens = lex_line(
            "Create four 2/2 blue Bird creature tokens with flying. \
             If the gift was promised, all permanents you control phase out, and until your next turn, your life total can't change and you gain protection from everything.",
            0,
        )
        .expect("multi-sentence paid-label fixture should lex");

        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("public effect-family entrypoint should preserve the typed condition");
        let [
            _,
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ThisSpellPaidLabel(label),
                if_true,
                if_false,
            }),
        ] = effects.as_slice()
        else {
            panic!("expected creation followed by one paid-label conditional: {effects:#?}");
        };
        assert!(label.display_label().eq_ignore_ascii_case("Gift"));
        assert!(if_false.is_empty());
        let debug = format!("{if_true:#?}");
        assert!(debug.contains("PhaseOutAll"), "{debug}");
        assert!(debug.contains("ChangeLifeTotal"), "{debug}");
        assert!(debug.contains("BeTargetedPlayer"), "{debug}");
        assert!(debug.contains("PreventAllDamageToTarget"), "{debug}");
        assert!(debug.matches("YourNextTurn").count() >= 3, "{debug}");
    }

    #[test]
    fn paid_label_preemption_unwraps_only_one_transparent_conditional() {
        let conditional = EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ThisSpellPaidLabel("Gift".into()),
            if_true: vec![EffectAst::SolveCase],
            if_false: Vec::new(),
        });
        let transparent = vec![EffectAst::Sequence {
            effects: vec![EffectAst::Coordinated {
                effects: vec![conditional],
                leading_duration: false,
                result_conjunction: false,
            }],
        }];

        assert!(matches!(
            super::into_exact_single_conditional(transparent),
            Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ThisSpellPaidLabel(_),
                ..
            }))
        ));

        let scoped_coordination = vec![EffectAst::Coordinated {
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ThisSpellPaidLabel("Gift".into()),
                if_true: vec![EffectAst::SolveCase],
                if_false: Vec::new(),
            })],
            leading_duration: true,
            result_conjunction: false,
        }];
        assert!(matches!(
            super::into_exact_single_conditional(scoped_coordination),
            Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ThisSpellPaidLabel(_),
                ..
            }))
        ));
        let multiple = vec![EffectAst::Coordinated {
            effects: vec![
                EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: PredicateAst::ThisSpellPaidLabel("Gift".into()),
                    if_true: vec![EffectAst::SolveCase],
                    if_false: Vec::new(),
                }),
                EffectAst::SolveCase,
            ],
            leading_duration: false,
            result_conjunction: false,
        }];
        assert!(
            super::into_exact_single_conditional(multiple).is_none(),
            "a wrapper with an unrelated sibling must fall through to ordinary dispatch"
        );
        assert!(
            super::into_exact_single_conditional(vec![EffectAst::SolveCase]).is_none(),
            "a typed paid-label prefix must not claim an unrelated returned effect"
        );
    }

    #[test]
    fn keyword_bundle_pump_survives_the_multi_sentence_entrypoint() {
        let tokens = lex_line(
            "Until end of turn, each other creature you control gets +1/+1 if it has flying, +1/+1 if it has first strike, and so on for double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, vigilance, and partner.",
            0,
        )
        .expect("keyword-bundle trigger body should lex");

        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("complete keyword bundle should parse before comma probing");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 14, "{debug}");
        assert!(debug.contains("Flying"), "{debug}");
        assert!(debug.contains("Partner"), "{debug}");
        assert!(
            effects.iter().all(|effect| matches!(
                effect,
                EffectAst::SubjectVerb(subject)
                    if matches!(
                        &subject.action,
                        SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                            set_quantifier_surface:
                                Some(ironsmith_core::SetQuantifierSurface::Each),
                            duration: crate::effect::Until::EndOfTurn,
                            ..
                        })
                    )
            )),
            "{debug}"
        );
        assert!(!debug.contains("IteratedPlayer"), "{debug}");
    }

    #[test]
    fn ordinary_leading_duration_pump_does_not_acquire_keyword_bundle_arms() {
        let tokens = lex_line(
            "Until end of turn, each other creature you control gets +1/+1.",
            0,
        )
        .expect("ordinary leading-duration pump should lex");

        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("ordinary leading-duration pump should retain its normal route");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 1, "{debug}");
        assert!(!debug.contains("Flying"), "{debug}");
        assert!(!debug.contains("Partner"), "{debug}");
    }

    #[test]
    fn inline_search_where_x_keeps_the_local_count_filter_surface() {
        let tokens = lex_line(
            "Search your library for up to X basic land cards, where X is the number of tapped creatures you control, put those cards onto the battlefield tapped, then shuffle.",
            0,
        )
        .expect("dynamic search should lex");
        let effects =
            super::parse_effect_sentences_lexed(&tokens).expect("dynamic search should parse");
        let [
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                        count_value: Some(count_value),
                        ..
                    }),
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one typed search effect, got {effects:#?}");
        };
        let Value::Count(filter) = count_value.unhinted() else {
            panic!("expected a filtered search count, got {count_value:#?}");
        };

        assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
        assert_eq!(filter.zone, Some(crate::zone::Zone::Battlefield));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(filter.tapped);
        assert!(!filter.has_explicit_card_noun(), "{filter:#?}");
        assert!(count_value.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs));
    }

    #[test]
    fn inline_mana_symbol_where_x_binds_through_comma_then() {
        let tokens = lex_line(
            "Scry X, where X is the amount of {S} spent to cast this spell, then draw three cards.",
            0,
        )
        .expect("mana-symbol where-X sentence should lex");
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("mana-symbol where-X sentence should parse");
        let [EffectAst::Coordination(coordination)] = effects.as_slice() else {
            panic!("expected a comma-then sequence, got {effects:#?}");
        };
        assert_eq!(
            coordination.kind,
            crate::model::coordination::CoordinationKindAst::Carry
        );
        assert!(matches!(
            coordination.boundaries.as_slice(),
            [crate::model::coordination::CoordinationBoundaryAst {
                operator: crate::model::coordination::CoordinationOperatorAst::CommaThen,
                ordering: crate::model::coordination::EffectOrderingAst::Ordered,
                ..
            }]
        ));
        let effects = coordination.effects().cloned().collect::<Vec<_>>();
        let [
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::KeywordActions(KeywordActionAst::Scry { count }),
                ..
            }),
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: draw_count }),
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected typed scry-then-draw effects, got {effects:#?}");
        };

        assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs));
        assert!(matches!(
            count.unhinted(),
            Value::ManaSymbolSpentToCastThisSpell {
                symbol: crate::mana::ManaSymbol::Snow,
                reference: ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
            }
        ));
        assert_eq!(draw_count, &Value::Fixed(3));
    }

    #[test]
    fn create_x_keeps_the_static_abilities_among_aggregate() {
        let tokens = lex_line(
            "Create X Blood tokens, where X is the number of abilities from among flying, first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, trample, and vigilance found among creatures you control.",
            0,
        )
        .expect("ability aggregate creation should lex");
        let binding =
            crate::grammar::effects::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(
                &tokens,
            )
            .expect("create count should expose its where-X binding");
        let direct = crate::keyword_static::parse_where_x_is_number_of_filter_value(
            crate::util::trim_edge_punctuation_tokens(binding.binding_tokens),
        )
        .expect("typed number-of binding should parse");
        assert!(
            matches!(direct.unhinted(), Value::StaticAbilitiesAmong { .. }),
            "typed binding was reduced before effect parsing: {direct:#?}; tokens: {:?}",
            crate::lexer::token_word_refs(binding.binding_tokens)
        );
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("ability aggregate creation should parse");
        let [
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { count, .. }),
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one typed token creation, got {effects:#?}");
        };
        let Value::StaticAbilitiesAmong { filter, abilities } = count.unhinted() else {
            panic!("expected the static-ability aggregate, got {count:#?}");
        };

        assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(abilities.len(), 12);
        assert!(abilities.contains(&crate::static_abilities::StaticAbilityId::Flying));
        assert!(abilities.contains(&crate::static_abilities::StaticAbilityId::Vigilance));
    }

    #[test]
    fn counter_unless_payment_decline_binds_nonpayment_branch_and_player() {
        let tokens = lex_line(
            "Counter target spell unless its controller pays {X}. If that player doesn't, they tap all lands with mana abilities they control and lose all unspent mana.",
            0,
        )
        .expect("lex counter-unless nonpayment followup");
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("parse counter-unless nonpayment followup");
        let debug = format!("{effects:#?}");

        let [
            _,
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate,
                effects: followups,
            }),
        ] = effects.as_slice()
        else {
            panic!("expected counter producer followed by a result branch\n{debug}");
        };
        assert_eq!(
            *predicate,
            crate::cards::builders::IfResultPredicate::Did,
            "declining an unless payment makes its consequence happen"
        );
        assert_eq!(
            followups.iter().find_map(empty_mana_pool_player),
            Some(PlayerAst::That),
            "the coordinated implicit life-resource action must retain the payer"
        );
    }

    #[test]
    fn delayed_coin_flip_keeps_its_outcome_inside_the_delayed_scope() {
        let tokens = lex_line(
            "Flip a coin at the beginning of the next end step. If you lose the flip, sacrifice that creature.",
            0,
        )
        .expect("lex delayed coin flip");
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("parse delayed coin flip and outcome");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 1, "{debug}");
        assert!(debug.contains("DelayedUntilNextEndStep"), "{debug}");
        assert!(debug.contains("FlipCoin"), "{debug}");
        assert!(debug.contains("IfResult"), "{debug}");
        assert!(debug.contains("DidNot"), "{debug}");
    }

    #[test]
    fn direct_coin_flip_outcomes_keep_the_flip_as_their_producer() {
        let tokens = lex_line(
            "Choose target spell, then flip a coin. If you win the flip, gain control of that spell. If you lose the flip, counter that spell.",
            0,
        )
        .expect("lex direct coin flip and outcomes");
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("parse direct coin flip and outcomes");
        let debug = format!("{effects:#?}");

        let [EffectAst::CommaThen { effects: producer }, win, loss] = effects.as_slice() else {
            panic!("expected one comma-then producer and two outcomes\n{debug}");
        };
        assert!(
            producer.last().is_some_and(super::is_direct_coin_flip),
            "{debug}"
        );
        assert!(matches!(
            win,
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                ..
            })
        ));
        assert!(matches!(
            loss,
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                ..
            })
        ));
    }

    #[test]
    fn delayed_definite_creature_sacrifice_keeps_the_prior_object_reference() {
        let tokens = lex_line(
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
            0,
        )
        .expect("lex creature insertion and delayed sacrifice");
        let effects = super::parse_effect_sentences_lexed(&tokens)
            .expect("parse creature insertion and delayed sacrifice");
        let debug = format!("{effects:#?}");

        let Some(crate::cards::builders::EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep {
            effects: delayed,
            ..
        })) = effects.last()
        else {
            panic!("expected delayed sacrifice as the final effect\n{debug}");
        };
        let [
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { filter, .. }),
                    ..
                },
            ),
        ] = delayed.as_slice()
        else {
            panic!("expected one delayed sacrifice effect\n{debug}");
        };
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str()
                    == crate::tag::CompilerReferenceTag::It.as_str()),
            "the definite creature should reference the established object\n{debug}"
        );
    }

    #[test]
    fn choose_then_tagged_damage_this_turn_keeps_delayed_trigger_scope() {
        let tokens = lex_line(
            "Choose target creature. Whenever that creature is dealt damage this turn, it deals that much damage to each other creature and each player.",
            0,
        )
        .expect("lex target declaration and delayed damage trigger");
        let (effects, trace) = crate::parse_trace::capture(|| {
            super::parse_effect_sentences_lexed(&tokens)
                .expect("parse target declaration and delayed damage trigger")
        });
        let debug = format!("{effects:#?}");

        assert!(
            effects.iter().any(|effect| matches!(
                effect,
                crate::cards::builders::EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { .. })
            )),
            "delayed trigger wrapper was lost\n{debug}\ntrace:\n{}",
            trace.render()
        );
    }

    #[test]
    fn target_player_draw_then_matching_spell_reduction_stays_a_player_effect() {
        let tokens = lex_line(
            "Target player draws two cards. Until your next turn, instant, sorcery, and planeswalker spells that player casts cost {2} less to cast.",
            0,
        )
        .expect("lex draw and matching-spell reduction");
        let (effects, trace) = crate::parse_trace::capture(|| {
            super::parse_effect_sentences_lexed(&tokens)
                .expect("parse draw and matching-spell reduction")
        });
        let debug = format!("{effects:#?}");

        assert!(debug.contains("action: Draw("), "{debug}");
        assert!(
            debug.contains("ReduceMatchingSpellCostThisTurn") && debug.contains("YourNextTurn"),
            "matching-spell reduction was misclassified\n{debug}\ntrace:\n{}",
            trace.render()
        );
        assert!(
            !debug.contains("GrantAbilitiesToTarget"),
            "matching-spell reduction became a hand-card ability grant\n{debug}"
        );
    }

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
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
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
                        crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                            preserve_other_types,
                            type_retention_surface,
                            ..
                        }),
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
    fn disturbed_slumber_keeps_leading_duration_pt_and_land_surfaces() {
        let tokens = lex_line(
            "Until end of turn, target land you control becomes a 4/4 Dinosaur creature with reach and haste. It's still a land. It must be blocked this turn if able.",
            0,
        )
        .expect("Disturbed Slumber should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("Disturbed Slumber should parse through generic effect rules");
        let mut found = false;
        let inspect = |effects: &[crate::cards::builders::EffectAst], found: &mut bool| {
            for effect in effects {
                if let crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action:
                            crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                                animation_pt_surface,
                                animation_duration_surface,
                                type_retention_surface,
                                duration,
                                ..
                            }),
                        ..
                    },
                ) = effect
                    && *animation_pt_surface
                        == Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness)
                    && *animation_duration_surface
                        == Some(ironsmith_core::AnimationDurationSurface::Leading)
                    && *type_retention_surface
                        == Some(ironsmith_core::TypeRetentionSurface::StillALand)
                    && *duration == crate::effect::Until::EndOfTurn
                {
                    *found = true;
                }
            }
        };
        inspect(&parsed, &mut found);
        for effect in &parsed {
            super::for_each_nested_effects(effect, true, |nested| inspect(nested, &mut found));
        }

        assert!(
            found,
            "Disturbed Slumber's animation surfaces must survive its follow-ups: {parsed:#?}"
        );
    }

    #[test]
    fn trailing_animation_duration_is_not_reclassified_as_leading() {
        let tokens = lex_line(
            "Target artifact you control becomes an artifact creature with base power and toughness 5/5 for as long as this creature remains on the battlefield.",
            0,
        )
        .expect("trailing-duration animation should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("trailing-duration animation should parse");
        let (duration_surface, duration) = parsed
            .iter()
            .find_map(|effect| match effect {
                crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action:
                            crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                                animation_duration_surface,
                                duration,
                                ..
                            }),
                        ..
                    },
                ) => Some((animation_duration_surface, duration)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected typed animation, got {parsed:#?}"));

        assert_eq!(*duration_surface, None);
        assert_eq!(*duration, crate::effect::Until::ThisLeavesTheBattlefield);
    }

    #[test]
    fn majestic_metamorphosis_keeps_leading_duration_and_pt_surfaces() {
        let tokens = lex_line(
            "Until end of turn, target artifact or creature becomes a 4/4 Angel artifact creature and gains flying. Draw a card.",
            0,
        )
        .expect("Majestic Metamorphosis should lex");

        let sentences = super::split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 2, "expected animation and draw sentences");
        let first_sentence = super::super::gain_ability::parse_gain_ability_sentence(sentences[0])
            .expect("the shared-subject animation should be valid gain grammar")
            .expect("the gain grammar should own the shared-subject animation");
        assert!(
            matches!(
                first_sentence.as_slice(),
                [crate::cards::builders::EffectAst::ControlFlow(_)]
            ),
            "single-sentence dispatch lost the leading-duration chain: {first_sentence:#?}"
        );

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("Majestic Metamorphosis should parse through generic effect rules");
        let (coordinated, control_duration) = parsed
            .iter()
            .find_map(|effect| match effect {
                crate::cards::builders::EffectAst::ControlFlow(control) => {
                    let crate::model::control_flow::ControlFlowNodeAst::Duration {
                        duration,
                        program,
                    } = &control.node
                    else {
                        return None;
                    };
                    let [crate::cards::builders::EffectAst::Coordination(coordination)] =
                        control.program(*program)?.effects.as_slice()
                    else {
                        return None;
                    };
                    Some((coordination, duration))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected leading-duration coordination: {parsed:#?}"));
        let (pt_surface, duration_surface, duration) = coordinated
            .effects()
            .find_map(|effect| match effect {
                crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action:
                            crate::cards::builders::SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                                animation_pt_surface,
                                animation_duration_surface,
                                duration,
                                ..
                            }),
                        ..
                    },
                ) => Some((animation_pt_surface, animation_duration_surface, duration)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected coordinated animation: {coordinated:#?}"));

        assert_eq!(
            control_duration,
            &crate::model::control_flow::CompilerDurationAst::UntilEndOfTurn
        );

        assert_eq!(
            *pt_surface,
            Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness)
        );
        assert_eq!(*duration_surface, None);
        assert_eq!(*duration, crate::effect::Until::Forever);
        assert!(coordinated.effects().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { .. }),
                    ..
                }
            )
        )));
        assert!(
            parsed.iter().any(|effect| matches!(
                effect,
                crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action: crate::cards::builders::SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { .. }),
                        ..
                    }
                )
            )),
            "draw follow-up was lost: {parsed:#?}"
        );
    }

    #[test]
    fn full_dispatch_keeps_leading_become_lose_gain_as_one_coordination() {
        let tokens = lex_line(
            "Until end of turn, target creature you control becomes a blue Dragon Illusion with base power and toughness 4/4, loses all abilities, and gains flying.",
            0,
        )
        .expect("coordinated animation should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("coordinated animation should parse through full dispatch");
        let [crate::cards::builders::EffectAst::ControlFlow(control)] = parsed.as_slice() else {
            panic!("expected one leading-duration coordination, got {parsed:#?}");
        };
        let crate::model::control_flow::ControlFlowNodeAst::Duration {
            duration: crate::model::control_flow::CompilerDurationAst::UntilEndOfTurn,
            program,
        } = &control.node
        else {
            panic!("expected end-of-turn duration ownership: {control:#?}");
        };
        let [crate::cards::builders::EffectAst::Coordination(coordination)] = control
            .program(*program)
            .expect("duration program")
            .effects
            .as_slice()
        else {
            panic!("expected one canonical coordination: {control:#?}");
        };
        let debug = format!("{coordination:#?}");
        assert!(debug.contains("BecomeBasePtCreature"), "{debug}");
        assert!(debug.contains("RemoveAbilitiesFromTarget"), "{debug}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
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
    fn serial_same_name_fanout_keeps_the_third_set_linked() {
        let tokens = lex_line(
            "Exile target nonland card from a graveyard, all other cards from graveyards with the same name as that card, and all permanents with that name.",
            0,
        )
        .expect("serial same-name fanout should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("serial same-name fanout should parse");
        let debug = format!("{parsed:#?}");
        assert!(debug.matches("SameNameAsTagged").count() >= 2, "{debug}");
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
                    action: crate::cards::builders::SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { .. }),
                },
            ))
        ));
        // Now composed from reusable primitives; rest->graveyard is a per-card split.
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::ForEach(ForEachEffectAst::ForEachTagged { .. })
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
                    action: crate::cards::builders::SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { .. }),
                },
            ))
        ));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                            order: crate::cards::builders::LibraryBottomOrderAst::ChooserChooses,
                            ..
                        }),
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
        let crate::cards::builders::SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll {
            tap_filter,
            untap_filter,
        }) = &subject_verb.action
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
                crate::cards::builders::EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { .. }),
                crate::cards::builders::EffectAst::ForEach(ForEachEffectAst::ForEachTagged { .. }),
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
        let exile_effect = parse_exile_top_library_clause(&exile_tokens, Some(subject), false);
        assert!(exile_effect.is_some(), "expected exile clause to parse");
        assert!(
            format!("{exile_effect:#?}").contains("LibraryOwnerAsActor"),
            "an authored library-owner subject must retain its actor placement"
        );

        let imperative_tokens =
            lex_line("the top two cards of target opponent's library", 0).unwrap();
        let imperative = parse_exile_top_library_clause(&imperative_tokens, None, false)
            .expect("imperative parses");
        assert!(
            !format!("{imperative:#?}").contains("LibraryOwnerAsActor"),
            "an imperative exile instruction must not acquire an owner-actor surface"
        );

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

        // One public reveal producer owns the collection tag. Per-card-type
        // conditional choices, the move, and the complement all reuse it.
        assert!(matches!(
            parsed.first(),
            Some(crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                        reveal: true,
                        ..
                    }),
                    ..
                }
            ))
        ));
        assert!(!parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { .. }),
                    ..
                }
            )
        )));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::ValueComparison { .. },
                ..
            })
        )));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary { .. }),
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

        // One public reveal producer owns the collection tag. Per-card-type
        // choices, the move, and the complement all reuse it.
        assert!(matches!(
            parsed.first(),
            Some(crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                        reveal: true,
                        ..
                    }),
                    ..
                }
            ))
        ));
        assert!(!parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { .. }),
                    ..
                }
            )
        )));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { .. })
        )));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { filter, .. })
                if filter.card_types == [crate::types::CardType::Kindred]
        )));
        assert!(parsed.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary { .. }),
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

        let Some(crate::cards::builders::EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { filter, effects })) =
            parsed.iter().find(|effect| {
                matches!(
                    effect,
                    crate::cards::builders::EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { .. })
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
                    action: crate::cards::builders::SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary { .. }),
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
                    action: crate::cards::builders::SubjectVerbActionAst::KeywordActions(KeywordActionAst::Clash { .. }),
                    ..
                }
            ))
        ));
        let Some(crate::cards::builders::EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::WonClash,
            effects,
        })) = parsed.last()
        else {
            panic!("expected a typed result branch, got {parsed:#?}");
        };
        let [crate::cards::builders::EffectAst::Coordination(coordination)] = effects.as_slice()
        else {
            panic!("expected the conjoined rewards to retain coordination: {effects:#?}");
        };
        assert_eq!(
            coordination.kind,
            crate::model::coordination::CoordinationKindAst::SharedSubject
        );
        let effects = coordination.effects().cloned().collect::<Vec<_>>();
        assert_eq!(
            effects.len(),
            2,
            "both rewards must stay gated: {effects:#?}"
        );
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { .. }),
                    ..
                }
            )
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                            ..
                        }),
                    ..
                }
            )
        )));

        let lowered = crate::compile_support::compile_statement_effects(&parsed)
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
    fn hoarders_greed_types_if_you_win_from_a_wrapped_terminal_clash() {
        let tokens = lex_line(
            "You lose 2 life and draw two cards, then clash with an opponent. If you win, repeat this process.",
            0,
        )
        .expect("Hoarder's Greed should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("Hoarder's Greed should parse as a typed clash loop");
        let [antecedent, EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, .. })] = parsed.as_slice() else {
            panic!("expected a wrapped antecedent and result follow-up: {parsed:#?}");
        };
        assert_eq!(
            terminal_result_producer(antecedent),
            Some(TerminalResultProducer::Clash),
            "the authored sequence should expose its terminal clash producer"
        );
        assert_eq!(
            predicate,
            &IfResultPredicate::WonClash,
            "`if you win` must retain clash-value semantics through the wrapper"
        );
    }

    #[test]
    fn optional_quoted_source_restriction_keeps_vigilance_result_semantics() {
        let tokens = lex_line(
            "You may have this creature gain \"this can't attack\" until end of combat. If you do, attacking doesn't cause creatures you control to tap this combat if this is untapped.",
            0,
        )
        .expect("Johan-style combat choice should lex");

        let parsed = super::parse_effect_sentences_lexed(&tokens)
            .expect("Johan-style combat choice should parse");
        let [
            EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player: PlayerAst::You,
                effects: optional,
            }),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::Did,
                effects: result,
            }),
        ] = parsed.as_slice()
        else {
            panic!("expected an optional restriction and gated result: {parsed:#?}");
        };

        let [
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Cant {
                        restriction: crate::effect::Restriction::Attack(source),
                        duration: crate::effect::Until::EndOfCombat,
                        ..
                    },
                ..
            }),
        ] = optional.as_slice()
        else {
            panic!("expected a source attack restriction: {optional:#?}");
        };
        assert!(
            source.source,
            "restriction should retain source identity: {source:#?}"
        );

        let [
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                        filter,
                        abilities,
                        duration: crate::effect::Until::EndOfCombat,
                        condition: Some(PredicateAst::Source(SourcePredicateAst::SourceIsUntapped)),
                        lock_filter_at_resolution: false,
                        ..
                    }),
                ..
            }),
        ] = result.as_slice()
        else {
            panic!("expected a source-conditioned vigilance grant: {result:#?}");
        };
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(abilities.iter().any(|ability| matches!(
            ability,
            crate::cards::builders::GrantedAbilityAst::KeywordAction(action)
                if action.as_ref() == &crate::KeywordAction::Vigilance
        )));
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
        let Some(crate::cards::builders::EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects,
        })) = parsed.last()
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
        let [crate::cards::builders::EffectAst::Permissions(PermissionEffectAst::MayByPlayer { .. }), gated] = parsed.as_slice()
        else {
            panic!("expected an optional antecedent followed by one gated sequence: {parsed:#?}");
        };
        let crate::cards::builders::EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects,
        }) = gated
        else {
            panic!("expected the consult procedure to remain under `If you do`: {gated:#?}");
        };
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary { .. }),
                    ..
                }
            )
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                        zone: crate::zone::Zone::Battlefield,
                        ..
                    }),
                    ..
                }
            )
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::ForEach(ForEachEffectAst::ForEachTagged { .. })
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
            crate::cards::builders::PredicateAst::TurnEvents(TurnEventPredicateAst::CreatureDiedThisTurn)
        ));
        assert_eq!(if_true.len(), 1, "{if_true:#?}");
        assert_eq!(if_false.len(), 1, "{if_false:#?}");
        assert!(!attach_to_previous_ability);

        let lowered = crate::compile_support::compile_statement_effects_with_imports(
            &parsed,
            &crate::model::reference_state::ReferenceImports::default(),
        )
        .expect("lower typed prior-token replacement");
        let debug = format!("{lowered:#?}");
        assert!(debug.contains("self_replacements"), "{debug}");
        assert!(debug.contains("CreatureDiedThisTurn"), "{debug}");
    }

    #[test]
    fn composable_fight_program_retains_the_correlated_followup() {
        let tokens = lex_line(
            "Choose two target creatures that share no creature types. Those creatures fight each other.",
            0,
        )
        .expect("lex correlated fight program");
        let effects =
            super::parse_effect_sentences_lexed(&tokens).expect("parse correlated fight program");

        let [
            crate::cards::builders::EffectAst::SourceSentence {
                effects: target_effects,
                ..
            },
            crate::cards::builders::EffectAst::SourceSentence {
                effects: fight_effects,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected two source-sentence effects: {effects:#?}");
        };
        assert_eq!(target_effects.len(), 1, "{target_effects:#?}");
        assert!(
            matches!(
                fight_effects.as_slice(),
                [crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action: crate::cards::builders::SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { .. }),
                        ..
                    }
                )]
            ),
            "{fight_effects:#?}"
        );

        let lowered = crate::compile_support::compile_statement_effects_with_imports(
            &effects,
            &crate::model::reference_state::ReferenceImports::default(),
        )
        .expect("lower correlated fight program");
        assert_eq!(
            lowered.effects.flattened_default_effects().len(),
            2,
            "{:#?}",
            lowered.effects
        );
        assert_eq!(lowered.effects.segments.len(), 2, "{:#?}", lowered.effects);
    }

    #[test]
    fn quantified_opponent_mill_retains_the_participant_scope() {
        let tokens =
            lex_line("Each opponent mills seven cards.", 0).expect("lex quantified opponent mill");
        let effects =
            super::parse_effect_sentences_lexed(&tokens).expect("parse quantified opponent mill");

        assert!(
            matches!(
                effects.as_slice(),
                [crate::cards::builders::EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects: nested })]
                    if matches!(
                        nested.as_slice(),
                        [crate::cards::builders::EffectAst::SubjectVerb(
                            crate::cards::builders::SubjectVerbEffectAst {
                                action: crate::cards::builders::SubjectVerbActionAst::Library(LibraryActionAst::Mill { .. }),
                                ..
                            }
                        )]
                    )
            ),
            "{effects:#?}"
        );
    }

    #[test]
    fn composable_leading_then_pump_and_fight_keeps_both_typed_actions() {
        let tokens = lex_line(
            "Put a +1/+1 counter on target creature you control if its power is 4 or greater. Then that creature gets +1/+1 until end of turn and fights target creature you don't control.",
            0,
        )
        .expect("lex pump-and-fight program");
        let effects =
            super::parse_effect_sentences_lexed(&tokens).expect("parse pump-and-fight program");

        let [
            _,
            crate::cards::builders::EffectAst::SourceSentence {
                effects: followup,
                leading_then: true,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected a leading-then source sentence: {effects:#?}");
        };
        assert!(
            matches!(
                followup.as_slice(),
                [
                    crate::cards::builders::EffectAst::SubjectVerb(
                        crate::cards::builders::SubjectVerbEffectAst {
                            action: crate::cards::builders::SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { .. }),
                            ..
                        }
                    ),
                    crate::cards::builders::EffectAst::SubjectVerb(
                        crate::cards::builders::SubjectVerbEffectAst {
                            action: crate::cards::builders::SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { .. }),
                            ..
                        }
                    )
                ]
            ),
            "pump or fight action was lost: {followup:#?}"
        );
    }

    #[test]
    fn labeled_fateful_hour_prior_token_followup_keeps_the_typed_replacement() {
        let tokens = lex_line(
            "Create two 1/1 white Human creature tokens. Fateful hour — If you have 5 or less life, create five of those tokens instead.",
            0,
        )
        .expect("lex labeled prior-token replacement");
        let sentences = crate::lexer::split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 2);
        assert!(
            !crate::activation_and_restrictions::is_generic_token_reminder_sentence(sentences[1]),
            "a labeled conditional replacement is not token reminder text"
        );
        let (parsed, trace) =
            crate::parse_trace::capture(|| super::parse_effect_sentences_lexed(&tokens));
        let parsed = parsed.unwrap_or_else(|error| {
            panic!(
                "parse labeled typed prior-token replacement: {error}\n{}",
                trace.render()
            )
        });
        let [
            crate::cards::builders::EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                ..
            },
        ] = parsed.as_slice()
        else {
            panic!("expected one typed self-replacement, got {parsed:#?}");
        };
        assert!(matches!(
            predicate,
            crate::cards::builders::PredicateAst::ValueComparison {
                left: crate::Value::LifeTotal(crate::filter::PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: crate::Value::Fixed(5),
            }
        ));
        assert_eq!(if_true.len(), 1, "{if_true:#?}");
        assert_eq!(if_false.len(), 1, "{if_false:#?}");

        let changed_reference = lex_line(
            "Create two 1/1 white Human creature tokens. Fateful hour — If you have 5 or less life, create five of those cards instead.",
            0,
        )
        .expect("lex changed-reference near miss");
        assert!(
            super::parse_effect_sentences_lexed(&changed_reference).is_err(),
            "a changed antecedent noun must not bind to the prior token creation"
        );
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
                            crate::cards::builders::SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                                target,
                                ..
                            }),
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

            let lowered = crate::compile_support::compile_statement_effects(&parsed)
                .unwrap_or_else(|error| panic!("lower fight replacement for {text:?}: {error}"));
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
                crate::cards::builders::EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate,
                    if_true,
                    if_false,
                }),
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
                assert_eq!(tag.as_str(), crate::tag::CompilerReferenceTag::It.as_str());
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
                                crate::cards::builders::SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                                    from_zone: Some(crate::zone::Zone::Battlefield),
                                    to_zone: Some(crate::zone::Zone::Graveyard),
                                    replacement_zone: crate::zone::Zone::Exile,
                                    duration:
                                        crate::cards::builders::ZoneReplacementDurationAst::UntilEndOfTurn,
                                    ..
                                }),
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
                Some(crate::cards::builders::EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    player: crate::cards::builders::PlayerAst::You,
                    ..
                }))
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
        let vote_option = match parsed.get(1) {
            Some(crate::cards::builders::EffectAst::Votes(VoteEffectAst::VoteOption { .. })) => parsed.get(1),
            Some(crate::cards::builders::EffectAst::Coordinated { effects, .. }) => {
                effects.iter().find(|effect| {
                    matches!(effect, crate::cards::builders::EffectAst::Votes(VoteEffectAst::VoteOption { .. }))
                })
            }
            _ => None,
        };
        let Some(crate::cards::builders::EffectAst::Votes(VoteEffectAst::VoteOption { effects, .. })) = vote_option
        else {
            panic!("expected a typed vote option, got {parsed:#?}");
        };
        fn find_choice_filter(
            effect: &crate::cards::builders::EffectAst,
        ) -> Option<&crate::filter::ObjectFilter> {
            match effect {
                crate::cards::builders::EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, .. }) => Some(filter),
                crate::cards::builders::EffectAst::Coordination(coordination) => {
                    coordination.effects().find_map(find_choice_filter)
                }
                crate::cards::builders::EffectAst::Coordinated { effects, .. }
                | crate::cards::builders::EffectAst::Votes(VoteEffectAst::VoteOption { effects, .. }) => {
                    effects.iter().find_map(find_choice_filter)
                }
                _ => None,
            }
        }
        let Some(filter) = effects.iter().find_map(find_choice_filter) else {
            panic!("expected the vote option to start with an object choice: {effects:#?}");
        };

        assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
    }

    #[test]
    fn unapplied_plural_token_haste_followup_keeps_they_surface() {
        let surface = |text: &str| {
            let lexed = lex_line(text, 0).expect("follow-up should lex");
            let sentences = split_lexed_sentences(&lexed);
            let tokens = sentences
                .first()
                .copied()
                .expect("follow-up should contain one sentence");
            let followup = super::parse_token_copy_followup_sentence_lexed(tokens)
                .expect("token haste follow-up should be recognized");
            let effects =
                super::apply_unapplied_token_copy_followup(tokens, tokens, followup, false)
                    .expect("token haste follow-up should lower");
            let [
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                            set_quantifier_surface,
                            ..
                        }),
                    ..
                }),
            ] = effects.as_slice()
            else {
                panic!("expected one targeted haste grant, got {effects:#?}");
            };
            *set_quantifier_surface
        };

        assert_eq!(
            surface("They gain haste until end of turn."),
            Some(ironsmith_core::SetQuantifierSurface::They)
        );
        assert_eq!(surface("It gains haste until end of turn."), None);
    }

    #[test]
    fn cant_be_regenerated_followup_applies_to_every_choice_mode() {
        let mut effects = vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
            modes: vec![
                crate::cards::builders::ChooseOneModeAst {
                    description: String::new(),
                    effects: vec![EffectAst::subject_verb_destroy_all(
                        crate::target::ObjectFilter::default()
                            .with_type(crate::types::CardType::Land),
                    )],
                },
                crate::cards::builders::ChooseOneModeAst {
                    description: String::new(),
                    effects: vec![EffectAst::subject_verb_destroy_all(
                        crate::target::ObjectFilter::creature(),
                    )],
                },
            ],
        })];

        assert!(super::apply_cant_be_regenerated_to_last_destroy_effect(
            &mut effects
        ));
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { modes })] = effects.as_slice() else {
            panic!("expected modal destroy");
        };
        assert!(modes.iter().all(|mode| {
            matches!(
                mode.effects.as_slice(),
                [EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll {
                            no_regeneration: true,
                            ..
                        }),
                        ..
                    }
                )]
            )
        }));
    }

    #[test]
    fn destroyed_this_way_surface_reaches_destroy_inside_unless_pays() {
        let mut effects = vec![EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_destroy_no_regeneration(
                crate::cards::builders::TargetAst::Object(
                    crate::target::ObjectFilter::creature(),
                    None,
                    None,
                ),
            )],
            player: PlayerAst::ItsController,
            cost: ironsmith_core::TotalCost::from_costs(Vec::new()),
            before_delayed_step: false,
        })];

        assert!(
            super::mark_last_destroy_creature_destroyed_this_way_surface(&mut effects),
            "the authored follow-up must reach the wrapped destroy"
        );
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { effects, .. })]
                if matches!(
                    effects.as_slice(),
                    [EffectAst::SubjectVerb(
                        crate::cards::builders::SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                                no_regeneration: true,
                                creature_destroyed_this_way_surface: true,
                                ..
                            }),
                            ..
                        }
                    )]
                )
        ));
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

pub fn replace_it_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_damage_target(effect, target);
    }
}

fn replace_definite_prior_damage_recipient_in_effects(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(subject_verb) => {
                let SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. }) = &mut subject_verb.action
                else {
                    continue;
                };
                let TargetAst::ObjectOrPlayer(filter, PlayerFilter::Any, None) = target else {
                    continue;
                };
                let mut permanent_or_player_object = ObjectFilter::permanent_card();
                permanent_or_player_object.zone = Some(Zone::Battlefield);
                if filter == &permanent_or_player_object {
                    *target =
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Damaged0.bind(), None);
                }
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                replace_definite_prior_damage_recipient_in_effects(nested);
            }),
        }
    }
}

pub fn replace_it_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_target(effect, target);
    }
}

pub fn is_placeholder_damage_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
    )
}

pub fn replace_placeholder_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_placeholder_damage_target(effect, target);
    }
}

pub fn replace_placeholder_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                target: damage_target,
                ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                target: damage_target,
                ..
            }) => {
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

pub fn replace_unbound_x_in_damage_effects(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_damage_effect(effect, replacement, clause)?;
    }
    Ok(())
}

pub fn replace_unbound_x_in_damage_effect(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, .. }) => {
                if value_contains_unbound_x(amount) {
                    *amount = replace_unbound_x_with_value(amount.clone(), replacement, clause)?;
                } else if amount.unhinted() == replacement.unhinted()
                    && replacement.has_surface_hint(ValueSurfaceHint::WhereXIs)
                    && !amount.has_surface_hint(ValueSurfaceHint::WhereXIs)
                {
                    // The damage parser can already have lowered the exact
                    // typed value named by the trailing where-X clause. In
                    // that case there is no literal X left to replace, but
                    // the authored `X ... where X is` surface still belongs
                    // to that same value. Preserve only the surface hints
                    // after proving semantic equality.
                    *amount = amount
                        .clone()
                        .with_surface_hints(replacement.surface_hints().iter().copied());
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

pub fn replace_unbound_x_in_effects_anywhere(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_effect_anywhere(effect, replacement, clause)?;
    }
    Ok(())
}

pub fn replace_unbound_x_in_effect_anywhere(
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
            // The effect-local parser may have retained a more precise object
            // surface than the sentence-wide where-X scan.  The latter can
            // include later prose (for example, "put those cards") and must
            // not turn "tapped creatures you control" into "tapped creature
            // cards you control".  Equal unhinted values need only inherit
            // the authored value-level hints.
            *value = value
                .clone()
                .with_surface_hints(replacement.surface_hints().iter().copied());
        }
        Ok(())
    }

    fn replace_values_in_cost_component(
        component: &mut crate::model::CompilerCost,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        match component {
            crate::model::CompilerCost::Mana(mana) if mana.has_x() => {
                *component = crate::model::CompilerCost::DynamicMana(
                    ironsmith_core::DynamicManaCost::from_x(mana.clone(), replacement.clone()),
                );
            }
            crate::model::CompilerCost::DynamicMana(dynamic) => {
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
            crate::model::CompilerCost::Life(value) => replace_value(value, replacement, clause)?,
            crate::model::CompilerCost::Effect(effect)
            | crate::model::CompilerCost::ValidatedEffect(effect) => {
                replace_unbound_x_in_effect_anywhere(effect, replacement, clause)?
            }
            _ => {}
        }
        Ok(())
    }

    fn replace_values_in_total_cost(
        cost: &mut ironsmith_core::TotalCost<crate::model::CompilerCost>,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        match cost.kind() {
            ironsmith_core::TotalCostKind::All(_) => {
                let mut components = cost.costs().to_vec();
                for component in &mut components {
                    replace_values_in_cost_component(component, replacement, clause)?;
                }
                *cost = ironsmith_core::TotalCost::from_costs(components);
            }
            ironsmith_core::TotalCostKind::OneOf(branches) => {
                let mut branches = branches.to_vec();
                for branch in &mut branches {
                    replace_values_in_total_cost(branch, replacement, clause)?;
                }
                *cost = ironsmith_core::TotalCost::one_of(branches);
            }
        }
        Ok(())
    }

    fn replace_values_in_granted_abilities(
        abilities: &mut [GrantedAbilityAst],
        replacement: &Value,
        clause: &str,
        rebase_it_to_ability_source: bool,
    ) -> Result<(), CardTextError> {
        fn rebase_it_reference(value: &mut Value) {
            match value {
                Value::SurfaceHinted { value, .. }
                | Value::Scaled(value, _)
                | Value::DividedRoundedDown(value, _)
                | Value::HalfRoundedDown(value) => rebase_it_reference(value),
                Value::Add(left, right) | Value::Min(left, right) => {
                    rebase_it_reference(left);
                    rebase_it_reference(right);
                }
                Value::PowerOf(spec)
                | Value::ToughnessOf(spec)
                | Value::ManaValueOf(spec)
                | Value::CountersOn(spec, _) => {
                    if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
                    {
                        *spec = Box::new(ChooseSpec::Source.with_surface_hint(
                            ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
                                SourceReferenceSurface::ThisPermanentType("it".to_string()),
                            ),
                        ));
                    }
                }
                _ => {}
            }
        }

        fn replace_static_ability_value(
            ability: &mut crate::model::CompilerStaticAbilityCore,
            replacement: &Value,
            clause: &str,
            rebase_it_to_ability_source: bool,
        ) -> Result<(), CardTextError> {
            let count = match &mut ability.payload {
                ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { count, .. }
                | ironsmith_core::StaticAbilityPayload::EntersWithCountersIfCondition {
                    count,
                    ..
                }
                | ironsmith_core::StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
                    count,
                    ..
                } => Some(count),
                _ => None,
            };
            if let Some(count) = count {
                replace_value(count, replacement, clause)?;
                if rebase_it_to_ability_source {
                    rebase_it_reference(count);
                }
            }
            Ok(())
        }

        for ability in abilities {
            match ability {
                GrantedAbilityAst::StaticAbility(ability) => {
                    if let crate::cards::builders::StaticAbilityAst::Static(ability) =
                        ability.as_mut()
                    {
                        replace_static_ability_value(
                            ability,
                            replacement,
                            clause,
                            rebase_it_to_ability_source,
                        )?;
                    }
                }
                GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
                    if let crate::model::CompilerAbilityKindCore::Static(static_ability) =
                        ability.kind_mut()
                    {
                        replace_static_ability_value(
                            static_ability,
                            replacement,
                            clause,
                            rebase_it_to_ability_source,
                        )?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    match effect {
        EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { effects, cost, .. }) => {
            let consequence_references_it =
                crate::tag_support::effects_reference_it_tag(effects);
            replace_values_in_total_cost(cost, replacement, clause)?;
            replace_unbound_x_in_effects_anywhere(effects, replacement, clause)?;
            if consequence_references_it {
                super::rewrite_unless_cost_source_values_to_it_tag(effect);
            }
        }
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count,
            count_value,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
            filter,
            count,
            count_value,
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone {
            filter,
            count,
            count_value,
            ..
        }) => {
            replace_in_filter(filter, replacement, clause)?;
            if let Some(value) = count_value.as_mut() {
                replace_value(value, replacement, clause)?;
            } else if count.dynamic_x {
                *count_value = Some(replacement.clone());
            }
        }
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: amount })
            | SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary { count: amount, .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount })
            | SubjectVerbActionAst::Library(LibraryActionAst::Mill { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Scry { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Surveil { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Proliferate { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Investigate { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Amass { amount, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Monstrosity { amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Discover { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fateseal { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Populate { count: amount, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Connive { count: amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage { amount, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageEach { amount, .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CopySpell { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCountersAll { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters { amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveCountersAll { amount, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PoisonCounters { count: amount })
            | SubjectVerbActionAst::Counters(CounterActionAst::EnergyCounters { count: amount })
            | SubjectVerbActionAst::Counters(CounterActionAst::ExperienceCounters { count: amount })
            | SubjectVerbActionAst::Counters(CounterActionAst::TicketCounters { count: amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayEnergy { amount })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetLifeTotal { amount })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaScaled { amount, .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyColor { amount, .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyOneColor { amount })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaChosenColor { amount, .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaFromLandCouldProduce { amount, .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaCommanderIdentity { amount })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget { amount, .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { count: amount, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop {
                position: amount, ..
            })
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalLandPlays { count: amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::HealDamage {
                amount: Some(amount),
                ..
            }) => {
                replace_value(amount, replacement, clause)?;
            }
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Incubate { amount, count }) => {
                replace_value(amount, replacement, clause)?;
                replace_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { cost, .. }) => {
                replace_values_in_total_cost(cost, replacement, clause)?;
            }
            SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
                cost,
                x_value,
                x_maximum,
            }) => {
                if cost.has_x() && x_value.is_none() && x_maximum.is_none() {
                    *x_value = Some(replacement.clone());
                } else {
                    if let Some(x_value) = x_value.as_mut() {
                        replace_value(x_value, replacement, clause)?;
                    }
                    if let Some(x_maximum) = x_maximum.as_mut() {
                        replace_value(x_maximum, replacement, clause)?;
                    }
                }
            }
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                amount: Some(amount),
                ..
            }) => {
                replace_value(amount, replacement, clause)?;
            }
            SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters {
                put_count,
                remove_count,
                ..
            }) => {
                replace_value(put_count, replacement, clause)?;
                replace_value(remove_count, replacement, clause)?;
            }
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                power, toughness, ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                power, toughness, ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                power, toughness, ..
            })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                power, toughness, ..
            }) => {
                replace_value(power, replacement, clause)?;
                replace_value(toughness, replacement, clause)?;
            }
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { power, .. }) => {
                replace_value(power, replacement, clause)?;
            }
            SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn { reduction, .. }) => {
                replace_value(reduction, replacement, clause)?;
            }
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { count, .. }) => {
                replace_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                filter,
                stop_rule,
                max_exposed,
                ..
            }) => {
                replace_in_filter(filter, replacement, clause)?;
                if let LibraryConsultStopRuleAst::MatchCount(count) = stop_rule {
                    replace_value(count, replacement, clause)?;
                }
                if let Some(max_exposed) = max_exposed {
                    replace_value(max_exposed, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { target, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { target }) => {
                replace_in_target(target, replacement, clause)?;
            }
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::ReplaceNextDamageToTarget {
                target,
                replacement_effects,
                ..
            }) => {
                replace_in_target(target, replacement, clause)?;
                replace_unbound_x_in_effects_anywhere(replacement_effects, replacement, clause)?;
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { filter })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. }) => {
                replace_in_filter(filter, replacement, clause)?;
            }
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealCardsFromHand {
                count,
                count_value,
                ..
            }) => {
                if count.dynamic_x {
                    if let Some(value) = count_value {
                        replace_value(value, replacement, clause)?;
                    } else {
                        *count_value = Some(replacement.clone());
                    }
                }
            }
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::DrawForEachTaggedMatching { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealHand)
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtObjects { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTarget { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::EmitKeywordAction { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Bolster { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Support { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Adapt { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Explore { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Endure { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Exploit)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ConniveIterated)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::OpenAttraction { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ManifestTopCardOfLibrary)
            | SubjectVerbActionAst::Library(LibraryActionAst::CloakTopCardOfLibrary)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ManifestCardFromHand)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ManifestDread)
            | SubjectVerbActionAst::Damage(DamageActionAst::HealDamage { amount: None, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Earthbend { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Behold { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::FightIterated { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Clash { .. })
            | SubjectVerbActionAst::Random(RandomActionAst::FlipCoin)
            | SubjectVerbActionAst::Random(RandomActionAst::FlipCoinFaceOnly)
            | SubjectVerbActionAst::Random(RandomActionAst::RollDie { .. })
            | SubjectVerbActionAst::Random(RandomActionAst::RollDiceChooseResult { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleHandAndGraveyardIntoLibrary)
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary)
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleGraveyardIntoLibrary { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ReorderGraveyard)
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseColor)
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardType { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNamedOption { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseLandType { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardName { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChoosePlayer { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::NoteLifeTotal)
            | SubjectVerbActionAst::Mana(ManaActionAst::AddMana { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeLifeTotals { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeTextBoxes { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeZones { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::PutRestOnBottomOfLibrary)
            | SubjectVerbActionAst::Mana(ManaActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn)
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeValues { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileInsteadOfGraveyardThisTurn)
            | SubjectVerbActionAst::Control(ControlActionAst::ControlCombatChoicesThisTurn { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::GainControl { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaColorsAmong { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddOneManaAnyColorAmong { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaImprintedColors)
            | SubjectVerbActionAst::Mana(ManaActionAst::DoubleManaPool)
            | SubjectVerbActionAst::Mana(ManaActionAst::EmptyManaPool)
            | SubjectVerbActionAst::Game(GameActionAst::EndTurn)
            | SubjectVerbActionAst::Game(GameActionAst::EndCombatPhase)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhases)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipNextCombatPhaseThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipMainPhasesThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhasesThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipDrawStep)
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PlayFromGraveyardUntilEot)
            | SubjectVerbActionAst::Control(ControlActionAst::ControlPlayer { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::ReduceNextSpellCostThisTurn { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RingTemptsYou)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::VentureIntoDungeon { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeMonarch)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::TakeInitiative)
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateEmblem { .. })
            | SubjectVerbActionAst::Game(GameActionAst::LoseGame)
            | SubjectVerbActionAst::Game(GameActionAst::WinGame)
            | SubjectVerbActionAst::ReorderTopPlanarDeck { .. }
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnSourceTransformedFromExile)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Reconfigure { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::CumulativeUpkeep { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Casualty { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyEnergy { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DiscardHand)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Detain { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Suspect { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ClearSuspected { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Flip { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RegenerateAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseIn { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Convert { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::Counter { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnEach { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::MoveOneCounter { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfChosenKind { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTop)
            | SubjectVerbActionAst::Game(GameActionAst::ExtraTurnAfterTurn { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ReorderTopOfLibrary { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary { .. })
            | SubjectVerbActionAst::PutSticker { .. }
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::ScaleXValue { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantProtectionChoice { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamage { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::AssignNoCombatDamage { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSource { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToPlayers { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToYou { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventNextTimeDamage { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageFromSourceFilter { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters { amount: None, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Meld { .. })
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenChoice { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrarySlotsToHand { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilityToSource { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControl { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControlHeterogeneous { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::Attach { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::Unattach { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileWhenSourceLeaves { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeSourceWhenLeaves { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterFutureZoneReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDrawReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterManaReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamagedBySourceZoneReplacement { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::Enchant { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseSpellCastHistory { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CastTagged { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { .. })
            | SubjectVerbActionAst::TargetOnly { .. }
            | SubjectVerbActionAst::TagMatchingObjects { .. }
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCreatureSubtypes { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeSaddledUntilEndOfTurn { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantBySpec { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget { .. })
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalPhases { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary) => {}
            SubjectVerbActionAst::Cant { .. } => {}
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                filter,
                count,
                count_value,
                library_position_from_top,
                ..
            }) => {
                replace_in_filter(filter, replacement, clause)?;
                if let Some(count_value) = count_value.as_mut() {
                    replace_value(count_value, replacement, clause)?;
                } else if count.dynamic_x {
                    *count_value = Some(replacement.clone());
                }
                if let Some(position) = library_position_from_top.as_mut() {
                    replace_value(position, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target,
                attached_to,
                ..
            }) => {
                replace_in_target(target, replacement, clause)?;
                if let Some(attached_to) = attached_to {
                    replace_in_target(attached_to, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { count, .. })
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { count, .. }) => {
                replace_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                count,
                dynamic_power_toughness,
                ..
            }) => {
                replace_value(count, replacement, clause)?;
                if let Some((power, toughness)) = dynamic_power_toughness {
                    replace_value(power, replacement, clause)?;
                    replace_value(toughness, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                granted_abilities: abilities,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                granted_abilities: abilities,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { abilities, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll { abilities, .. }) => {
                replace_values_in_granted_abilities(abilities, replacement, clause, false)?;
            }
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                target, abilities, ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                target, abilities, ..
            }) => {
                let rebase_it_to_ability_source =
                    matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str());
                replace_values_in_granted_abilities(
                    abilities,
                    replacement,
                    clause,
                    rebase_it_to_ability_source,
                )?;
            }
            SubjectVerbActionAst::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn { ability, .. }) => {
                replace_values_in_granted_abilities(
                    std::slice::from_mut(ability.as_mut()),
                    replacement,
                    clause,
                    false,
                )?;
            }
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterNextBatchEnterWithCounters { count, .. }) => {
                replace_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Learn)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::UnlockRoomDoor)
            | SubjectVerbActionAst::Game(GameActionAst::ReverseTurnOrder)
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterUnderControlReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterTappedReplacement { .. }) => {}
        },
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_effects_anywhere(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub fn parse_exact_where_x_value_expression(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation(tokens);
    if let Some(value) =
        crate::grammar::shared_util::value_semantics::parse_mana_symbol_spent_to_cast_value(&tokens)
    {
        return Some(value);
    }
    if matches!(
        crate::grammar::effects::sentence_predicate_shapes::parse_where_x_value_shape_tokens(
            &tokens,
            false,
        ),
        Some(
            crate::grammar::effects::sentence_predicate_shapes::WhereXValueShape::CardTypesInYourGraveyard
        )
    ) {
        return Some(Value::CardTypesInGraveyard(PlayerFilter::You));
    }
    let word_view = crate::grammar::primitives::TokenWordView::new(tokens.as_slice());
    let words = word_view.word_refs();
    if !crate::word_primitives::parse_sequence_prefix(&words, &["where", "x", "is"]) {
        return None;
    }
    let body = words.get(3..)?;
    let (value, used) = crate::grammar::shared_util::value_expr::parse_value_expr_words(body)?;
    (used == body.len()).then_some(value)
}

pub fn apply_where_x_to_damage_amounts(
    tokens: &[OwnedLexToken],
    effects: &mut [EffectAst],
) -> Result<(), CardTextError> {
    let Some(shape) =
        effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(tokens)
    else {
        return Ok(());
    };
    let binding_tokens = crate::util::trim_edge_punctuation_tokens(shape.binding_tokens);
    let Some(where_value) =
        crate::keyword_static::parse_where_x_is_aggregate_filter_value(binding_tokens)
            .or_else(|| {
                crate::grammar::shared_util::value_semantics::parse_turn_history_value_binding(
                    binding_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_number_of_filter_value(binding_tokens)
            })
            .or_else(|| parse_exact_where_x_value_expression(binding_tokens))
            .or_else(|| parse_value_binding_clause(binding_tokens))
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

pub fn replace_it_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => {
            if let SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                target: damage_target,
                ..
            }) = &mut subject_verb.action
                && target_references_it(damage_target)
            {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_damage_target_in_effects(nested, target);
        }),
    }
}

pub fn target_has_authored_it_qualification(target: &TargetAst) -> bool {
    match target {
        // A count around an anaphor is itself an authored selection from the
        // antecedent set ("one of those cards").
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_references_it(inner)
        }
        TargetAst::Object(filter, _, _) if target_references_it(target) => {
            let mut residual = filter.clone();
            residual.tagged_constraints.retain(|constraint| {
                constraint.tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
            });
            residual.union_surface = Default::default();
            residual != ObjectFilter::default()
        }
        _ => false,
    }
}

pub fn replace_it_target(effect: &mut EffectAst, target: &TargetAst) {
    fn rebind_qualified_it_reference(effect_target: &mut TargetAst, tag: &TagKey) -> bool {
        match effect_target {
            TargetAst::Tagged(reference, _)
                if reference.as_str() == crate::tag::CompilerReferenceTag::It.as_str() =>
            {
                *reference = crate::tag::TagRef::of(tag.clone());
                true
            }
            TargetAst::Object(filter, _, _) => {
                let mut rebound = false;
                for constraint in &mut filter.tagged_constraints {
                    if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                        constraint.tag = tag.clone();
                        rebound = true;
                    }
                }
                rebound
            }
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
                rebind_qualified_it_reference(inner, tag)
            }
            _ => false,
        }
    }

    fn should_replace_self_replacement_target(
        effect_target: &mut TargetAst,
        target: &TargetAst,
    ) -> bool {
        // A quantified anaphor (for example, "one of those cards") is a new
        // authored selection from the antecedent set. Keep its count and any
        // narrowing predicates; reference preparation will bind its `it` tag
        // to the preceding tagged effect. Only bare anaphors repeat the
        // antecedent target wholesale.
        if target_has_authored_it_qualification(effect_target)
            && let TargetAst::Tagged(tag, _) = target
            && tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
        {
            rebind_qualified_it_reference(effect_target, tag);
            return false;
        }
        target_references_it(effect_target)
            || matches!(
                effect_target,
                TargetAst::Tagged(_, _) | TargetAst::Source(_)
            )
    }

    match effect {
        EffectAst::SubjectVerb(subject_verb) => {
            if let SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnEach {
                counter_type,
                filter,
            }) = &subject_verb.action
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                })
            {
                subject_verb.action = SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget {
                    counter_type: *counter_type,
                    target: target.clone(),
                });
                return;
            }
            match &mut subject_verb.action {
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap {
                    target: effect_target,
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap {
                    target: effect_target,
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand {
                    target: effect_target,
                })
                | SubjectVerbActionAst::Stack(StackActionAst::Counter {
                    target: effect_target,
                })
                | SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Detain {
                    target: effect_target,
                })
                | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Suspect {
                    target: effect_target,
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat {
                    target: effect_target,
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Flip {
                    target: effect_target,
                })
                | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap {
                    target: effect_target,
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseIn {
                    target: effect_target,
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform {
                    target: effect_target,
                })
                | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Convert {
                    target: effect_target,
                })
                | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Explore {
                    target: effect_target,
                })
                | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Endure {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Control(ControlActionAst::GainControl {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                    source: effect_target,
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::TargetOnly {
                    target: effect_target,
                    ..
                }
                | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Connive {
                    target: effect_target,
                    ..
                }) => {
                    if should_replace_self_replacement_target(effect_target, target) {
                        *effect_target = target.clone();
                    }
                }
                SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                    protected_target,
                    destination_target,
                    ..
                }) => {
                    for effect_target in protected_target
                        .iter_mut()
                        .chain(destination_target.iter_mut())
                    {
                        if should_replace_self_replacement_target(effect_target, target) {
                            *effect_target = target.clone();
                        }
                    }
                }
                SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect {
                    target: effect_target,
                    ..
                }) => {
                    if should_replace_self_replacement_target(effect_target, target) {
                        *effect_target = target.clone();
                    }
                }
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    target: effect_target,
                    attached_to,
                    ..
                }) => {
                    if should_replace_self_replacement_target(effect_target, target) {
                        *effect_target = target.clone();
                    }
                    if let Some(effect_target) = attached_to
                        && should_replace_self_replacement_target(effect_target, target)
                    {
                        *effect_target = target.clone();
                    }
                }
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    target: effect_target,
                    ..
                }) => {
                    if should_replace_self_replacement_target(effect_target, target) {
                        *effect_target = target.clone();
                    }
                }
                SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget {
                    target: effect_target,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                    target: effect_target,
                    ..
                }) => {
                    if target_references_it(effect_target) {
                        *effect_target = target.clone();
                    }
                }
                _ => {}
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_target_in_effects(nested, target);
        }),
    }
}

pub fn target_references_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str(),
        TargetAst::Object(filter, _, _) => filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        }),
        TargetAst::WithCount(inner, _) => target_references_it(inner),
        _ => false,
    }
}

pub fn is_that_turn_end_step_sentence(tokens: &[OwnedLexToken]) -> bool {
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

pub fn most_recent_extra_turn_player(effects: &[EffectAst]) -> Option<PlayerAst> {
    effects.iter().rev().find_map(|effect| {
        let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            subject: crate::cards::builders::SubjectVerbSubjectAst { player, .. },
            action: SubjectVerbActionAst::Game(GameActionAst::ExtraTurnAfterTurn { .. }),
        }) = effect
        else {
            return None;
        };
        Some(*player)
    })
}

pub fn rewrite_when_one_or_more_this_way_clause_prefix(
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
        // Keep the authored active result predicate intact. The typed modal
        // grammar retains its action, object filter, actor, and one-or-more
        // cardinality, which are needed both for LKI matching and for exact
        // reflexive-trigger rendering. Collapsing this to `When you do`
        // discards those facts and makes repeated `that many` references
        // vulnerable to binding to an intervening effect instead.
        return tokens.to_vec();
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

pub fn strip_otherwise_sentence_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != OTHERWISE_WORD)
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

pub fn rewrite_otherwise_referential_subject(tokens: Vec<OwnedLexToken>) -> Vec<OwnedLexToken> {
    if !effect_grammar::dispatch_entry_shapes::has_otherwise_referential_subject_tokens(&tokens) {
        return tokens;
    }

    let mut rewritten = tokens;
    if let Some(first) = rewritten.get_mut(0) {
        first.replace_word("target");
    }
    rewritten
}

pub fn is_nonsemantic_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    is_activate_only_restriction_sentence(tokens)
        || is_trigger_only_restriction_sentence(tokens)
        || effect_grammar::dispatch_entry_shapes::is_x_cant_be_zero_tokens(tokens)
}

fn token_copy_followup_container_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::ResolvedWhenResult { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTaggedWithControllerAtLastBlockedBy { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDoesNot { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDoesNot { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDid { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDid { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::RepeatProcess { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextCleanupStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUntapStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUpkeep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextDrawStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextMainPhase { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextFirstMainPhase { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat { effects })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectLeavesBattlefield { effects, .. })
        | EffectAst::Votes(VoteEffectAst::VoteOption { effects, .. }) => Some(effects),
        _ => None,
    }
}

pub fn parse_token_copy_followup_sentence(tokens: &[OwnedLexToken]) -> Option<TokenCopyFollowup> {
    let tokens = if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "create")
    {
        &tokens[1..]
    } else {
        tokens
    };
    let filtered = crate::util::non_article_token_word_refs(tokens);
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
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep(
            super::token_copy_action_reference_surface(tokens, "sacrifice")?,
        ));
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
                .then(|| super::token_copy_action_reference_surface(tokens, "exile"))
                .flatten()
                .map(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            is_sacrifice_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

pub fn parse_token_copy_followup_sentence_lexed(
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
    let filtered = crate::util::non_article_token_word_refs(tokens);
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
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep(
            super::token_copy_action_reference_surface(tokens, "sacrifice")?,
        ));
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
                .then(|| super::token_copy_action_reference_surface(tokens, "exile"))
                .flatten()
                .map(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            super::is_sacrifice_that_token_at_end_of_combat_lexed(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

pub fn parse_token_granted_ability_followup_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let Some(ability_tokens) =
        effect_grammar::dispatch_entry_shapes::parse_token_granted_ability_tokens(tokens)
    else {
        return Ok(None);
    };
    let clause_words = crate::lexer::token_word_refs(tokens);
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
            let surface = SourceReferenceSurface::ThisPermanentType("it".to_string());
            if bind_leading_it_to_source {
                return TargetAst::Object(
                    ObjectFilter::source_with_surface(surface),
                    None,
                    it_span,
                );
            }
            let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
            filter.source_surface = Some(surface);
            return TargetAst::Object(filter, None, span);
        }
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), span)
    };
    let effects = match followup {
        TokenCopyFollowup::HasHaste(surface) => {
            vec![
                EffectAst::subject_verb_grant_abilities_to_target(
                    fallback_target(),
                    vec![KeywordAction::Haste.into()],
                    Until::Forever,
                )
                .with_set_quantifier_surface(
                    (surface == crate::effect::TokenCopyReferenceSurface::They)
                        .then_some(ironsmith_core::SetQuantifierSurface::They),
                ),
            ]
        }
        TokenCopyFollowup::GainHasteUntilEndOfTurn(surface) => {
            vec![
                EffectAst::subject_verb_grant_abilities_to_target(
                    fallback_target(),
                    vec![KeywordAction::Haste.into()],
                    Until::EndOfTurn,
                )
                .with_set_quantifier_surface(
                    (surface == crate::effect::TokenCopyReferenceSurface::They)
                        .then_some(ironsmith_core::SetQuantifierSurface::They),
                ),
            ]
        }
        TokenCopyFollowup::EnterTappedAndAttacking
        | TokenCopyFollowup::EnterTappedAndAttackingThatPlayer => {
            return Err(CardTextError::ParseError(
                "standalone 'enters tapped and attacking' follow-up requires a preceding token-copy, populate, or meld effect".to_string(),
            ));
        }
        TokenCopyFollowup::SacrificeAtNextEndStep(_) => {
            vec![EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep {
                player: PlayerFilter::Any,
                effects: vec![EffectAst::subject_verb_sacrifice(
                    PlayerAst::Implicit,
                    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    1,
                    None,
                )],
            })]
        }
        TokenCopyFollowup::SacrificeAtNextUpkeep => vec![EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUpkeep {
            player: PlayerAst::Any,
            effects: vec![EffectAst::subject_verb_sacrifice(
                PlayerAst::Implicit,
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                1,
                None,
            )],
        })],
        TokenCopyFollowup::ExileAtNextEndStep(_) => vec![EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::subject_verb_exile(
                TargetAst::Object(
                    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    span,
                    None,
                ),
                false,
            )],
        })],
        TokenCopyFollowup::ExileAtEndOfCombat(_) => vec![EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::subject_verb_exile(
                TargetAst::Object(
                    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    span,
                    None,
                ),
                false,
            )],
        })],
        TokenCopyFollowup::SacrificeAtEndOfCombat => vec![EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::subject_verb_sacrifice(
                PlayerAst::Implicit,
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                1,
                None,
            )],
        })],
    };
    Ok(effects)
}

pub fn try_apply_token_granted_ability_followup(
    effects: &mut [EffectAst],
    abilities: &[GrantedAbilityAst],
    presentation: ironsmith_core::TokenAbilityPresentation,
) -> Result<bool, CardTextError> {
    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    match last {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                    definition,
                    granted_abilities,
                    ability_presentation,
                    ..
                }),
            ..
        }) => {
            let combine_separate_sentence =
                !definition.has_intrinsic_abilities() && granted_abilities.is_empty();
            // The creation sentence carried the token's keywords inline ("… Bat
            // creature token with flying.") and grouped nothing itself, so this
            // followup is an ADDITIONAL sentence for the ability it introduces —
            // not the place those keywords belong. `SeparateSentence` claims the
            // trailing sentence owns every grouped ability, which dragged the
            // keywords back out into their own "It has flying." sentence.
            // A standalone tail leaves `grouped_presentation()` empty, so the
            // keywords keep their " with " clause.
            let keywords_authored_inline = ability_presentation.is_none()
                && definition.has_intrinsic_abilities()
                && granted_abilities.is_empty();
            granted_abilities.extend(abilities.iter().cloned());
            *ability_presentation = Some(if keywords_authored_inline {
                ironsmith_core::TokenAbilityPresentation::with_added_standalone_tail(None)
            } else if combine_separate_sentence {
                presentation.combined_separate_sentence()
            } else {
                presentation
            });
            Ok(true)
        }
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        })
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            if try_apply_token_granted_ability_followup(
                if_true.as_mut_slice(),
                abilities,
                presentation,
            )? {
                return Ok(true);
            }
            if try_apply_token_granted_ability_followup(
                if_false.as_mut_slice(),
                abilities,
                presentation,
            )? {
                return Ok(true);
            }
            Ok(false)
        }
        EffectAst::TagAffected { effect, .. } => try_apply_token_granted_ability_followup(
            std::slice::from_mut(effect.as_mut()),
            abilities,
            presentation,
        ),
        _ => {
            let Some(nested_effects) = token_copy_followup_container_effects_mut(last) else {
                return Ok(false);
            };
            if nested_effects.is_empty() {
                return Ok(false);
            }
            try_apply_token_granted_ability_followup(
                nested_effects.as_mut_slice(),
                abilities,
                presentation,
            )
        }
    }
}

pub fn try_apply_token_copy_followup(
    effects: &mut [EffectAst],
    followup: TokenCopyFollowup,
) -> Result<bool, CardTextError> {
    // Lowering a source sentence or loop may append bookkeeping effects after
    // the authored token creation. Search backward so a follow-up still binds
    // to the most recent structurally reachable token action instead of
    // requiring that action to be the wrapper's literal final child.
    for effect in effects.iter_mut().rev() {
        let applied = match effect {
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::KeywordActions(KeywordActionAst::Populate {
                    has_haste,
                    enters_tapped,
                    enters_attacking,
                    exile_at_end_of_combat,
                    sacrifice_at_next_end_step,
                    exile_at_next_end_step,
                    ..
                }) => match followup {
                    TokenCopyFollowup::HasHaste(_) => {
                        *has_haste = true;
                        true
                    }
                    TokenCopyFollowup::EnterTappedAndAttacking => {
                        *enters_tapped = true;
                        *enters_attacking = true;
                        true
                    }
                    TokenCopyFollowup::SacrificeAtNextEndStep(_) => {
                        *sacrifice_at_next_end_step = true;
                        true
                    }
                    TokenCopyFollowup::ExileAtNextEndStep(_) => {
                        *exile_at_next_end_step = true;
                        true
                    }
                    TokenCopyFollowup::ExileAtEndOfCombat(_) => {
                        *exile_at_end_of_combat = true;
                        true
                    }
                    TokenCopyFollowup::EnterTappedAndAttackingThatPlayer
                    | TokenCopyFollowup::GainHasteUntilEndOfTurn(_)
                    | TokenCopyFollowup::SacrificeAtNextUpkeep
                    | TokenCopyFollowup::SacrificeAtEndOfCombat => return Ok(false),
                },
                SubjectVerbActionAst::KeywordActions(KeywordActionAst::Meld {
                    enters_tapped,
                    enters_attacking,
                    ..
                }) => match followup {
                    TokenCopyFollowup::EnterTappedAndAttacking => {
                        *enters_tapped = true;
                        *enters_attacking = true;
                        true
                    }
                    _ => return Ok(false),
                },
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy {
                    has_haste,
                    enters_tapped,
                    enters_attacking,
                    entry_tapped_attacking_followup,
                    attack_target_player_or_planeswalker_controlled_by,
                    attack_target_player_only,
                    exile_at_end_of_combat,
                    exile_at_end_of_combat_reference_surface,
                    sacrifice_at_next_end_step,
                    sacrifice_at_next_end_step_reference_surface,
                    exile_at_next_end_step,
                    exile_at_next_end_step_reference_surface,
                    haste_followup_reference_surface,
                    ..
                })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                    has_haste,
                    enters_tapped,
                    enters_attacking,
                    entry_tapped_attacking_followup,
                    attack_target_player_or_planeswalker_controlled_by,
                    attack_target_player_only,
                    exile_at_end_of_combat,
                    exile_at_end_of_combat_reference_surface,
                    sacrifice_at_next_end_step,
                    sacrifice_at_next_end_step_reference_surface,
                    exile_at_next_end_step,
                    exile_at_next_end_step_reference_surface,
                    haste_followup_reference_surface,
                    ..
                }) => match followup {
                    TokenCopyFollowup::HasHaste(surface) => {
                        *has_haste = true;
                        *haste_followup_reference_surface = Some(surface);
                        true
                    }
                    TokenCopyFollowup::EnterTappedAndAttacking => {
                        *enters_tapped = true;
                        *enters_attacking = true;
                        *entry_tapped_attacking_followup = true;
                        true
                    }
                    TokenCopyFollowup::EnterTappedAndAttackingThatPlayer => {
                        *enters_tapped = true;
                        *enters_attacking = true;
                        *entry_tapped_attacking_followup = true;
                        *attack_target_player_or_planeswalker_controlled_by = Some(PlayerAst::That);
                        *attack_target_player_only = true;
                        true
                    }
                    TokenCopyFollowup::SacrificeAtNextEndStep(surface) => {
                        *sacrifice_at_next_end_step = true;
                        *sacrifice_at_next_end_step_reference_surface = Some(surface);
                        true
                    }
                    TokenCopyFollowup::ExileAtNextEndStep(surface) => {
                        *exile_at_next_end_step = true;
                        *exile_at_next_end_step_reference_surface = Some(surface);
                        true
                    }
                    TokenCopyFollowup::ExileAtEndOfCombat(surface) => {
                        *exile_at_end_of_combat = true;
                        *exile_at_end_of_combat_reference_surface = Some(surface);
                        true
                    }
                    TokenCopyFollowup::GainHasteUntilEndOfTurn(_)
                    | TokenCopyFollowup::SacrificeAtNextUpkeep
                    | TokenCopyFollowup::SacrificeAtEndOfCombat => return Ok(false),
                },
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                    tapped,
                    attacking,
                    exile_at_end_of_combat,
                    sacrifice_at_end_of_combat,
                    ..
                }) => match followup {
                    TokenCopyFollowup::ExileAtEndOfCombat(_) => {
                        *exile_at_end_of_combat = true;
                        true
                    }
                    TokenCopyFollowup::SacrificeAtEndOfCombat => {
                        *sacrifice_at_end_of_combat = true;
                        true
                    }
                    TokenCopyFollowup::EnterTappedAndAttacking => {
                        *tapped = true;
                        *attacking = true;
                        true
                    }
                    TokenCopyFollowup::HasHaste(_)
                    | TokenCopyFollowup::EnterTappedAndAttackingThatPlayer
                    | TokenCopyFollowup::GainHasteUntilEndOfTurn(_)
                    | TokenCopyFollowup::SacrificeAtNextEndStep(_)
                    | TokenCopyFollowup::SacrificeAtNextUpkeep
                    | TokenCopyFollowup::ExileAtNextEndStep(_) => return Ok(false),
                },
                _ => false,
            },
            _ => {
                let mut applied = false;
                try_for_each_nested_effects_mut(effect, false, |nested_effects| {
                    if !applied && !nested_effects.is_empty() {
                        applied = try_apply_token_copy_followup(nested_effects, followup)?;
                    }
                    Ok::<(), CardTextError>(())
                })?;
                applied
            }
        };
        if applied {
            return Ok(true);
        }
    }
    Ok(false)
}

// AST queries and edits live beside the AST they operate on, so lowering can
// use them without importing recognition.
pub use crate::model::ast::{
    apply_cant_be_regenerated_to_effect, apply_cant_be_regenerated_to_effects_tail,
    apply_cant_be_regenerated_to_last_target_effect, primary_damage_target_from_effect,
    primary_target_from_effect,
};
