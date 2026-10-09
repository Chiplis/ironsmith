use crate::cards::builders::DamagePreventionActionAst;
use crate::cards::builders::ForEachEffectAst;
use winnow::Parser;
use winnow::combinator::{alt, repeat};
use winnow::error::{ContextError, ErrMode};

use super::super::effect_ast_traversal::{
    for_each_nested_effect_vec_mut, for_each_nested_effects, for_each_nested_effects_mut,
};
use super::super::grammar::effects::{
    SourceLinkedExileReferenceKind, chain_carry as chain_grammar, for_each_shapes,
    parse_additional_phases_shape, parse_any_player_may_sacrifice_shape,
    parse_choose_then_exile_reference_shape, parse_conditional_sentence_family_lexed,
    parse_exile_reference_action_shape, parse_reveal_source_exiled_permanents_tokens,
    parse_tap_object_union_then_tokens, sacrifice_discard_shapes as sacrifice_discard_grammar,
};
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, parse_predicate_with_grammar_entrypoint_lexed,
    split_leading_result_prefix_lexed, split_trailing_if_clause_lexed,
};
use super::super::lexer::{OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    PermissionClauseSpec, PermissionLifetime, parse_additional_land_plays_clause_lexed,
    parse_cast_or_play_tagged_clause, parse_permission_clause_spec_lexed,
    parse_unsupported_play_cast_permission_clause_lexed,
};
use super::super::rule_engine::{LexClauseView, LexRuleDef, LexRuleHandler, LexRuleIndex};
use super::super::tag_support::{effects_have_cross_arm_tag_dependency, effects_reference_it_tag};
#[cfg(test)]
use super::super::token_primitives::str_contains as string_contains;
use super::super::util::{
    parse_target_phrase, remove_first_may_word as remove_first_may_word_tokens,
    remove_through_first_may_word as remove_through_first_may_word_tokens,
};
use super::clause_pattern_helpers::{parse_copy_spell_clause, parse_keyword_mechanic_clause};
use super::dispatch_entry::SentenceInput;
use super::dispatch_inner::parse_subject_verb_extension_sentence;
use super::lex_chain_helpers::{
    find_verb_lexed, has_authored_comma_then_surface_lexed, has_effect_head_without_verb_lexed,
    has_explicit_comma_then_boundary_lexed, segment_has_effect_head_lexed,
    split_effect_chain_on_and_lexed, split_segments_on_comma_effect_head_lexed,
    split_segments_on_comma_then_lexed,
};
use super::search_library::parse_for_each_exiled_this_way_sentence;
use super::sentence_helpers::*;
use super::{
    SubjectVerbPrimitiveClause, has_unless_payment_choice, parse_cant_effect_sentence_lexed,
    parse_effect_clause_lexed, parse_search_library_sentence_lexed,
    parse_sentence_each_player_may_reveal_selected_cards_in_their_hand,
    parse_sentence_exile_source_with_counters_lexed,
    parse_sentence_put_onto_battlefield_with_counters_on_it_lexed,
    parse_sentence_return_with_counters_on_it_lexed,
    parse_sentence_target_player_reveals_random_card_from_hand, parse_sentence_unless_pays,
    parse_simple_gain_ability_clause_lexed, parse_simple_lose_ability_clause_lexed,
    parse_token_copy_followup_sentence_lexed, token_copy_action_reference_surface,
    try_apply_token_copy_followup,
};
use crate::grammar::shared_util::value_semantics::{
    parse_number_prefix_lexed, parse_value_prefix_lexed,
};
use crate::recognition::{ParseOutcome, RuleId};
use crate::registry::{HeadDiscriminator, RegistryRuleMetadata};
use crate::util::span_from_tokens;

use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, DelayedEffectAst, EffectAst, GrantActionAst,
    LifeResourceActionAst, ManaActionAst, ObjectChoiceEffectAst, PermanentStateActionAst,
    PlayerAst, PredicateAst, ReturnControllerAst, SourcePredicateAst, StatChangeActionAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey,
    TargetAst, TextSpan, TokenActionAst, ZoneMoveActionAst,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, SourceReferenceSurface, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

fn has_any_number_of_times_suffix(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
    crate::word_primitives::parse_sequence_suffix(&words, &["any", "number", "of", "times"])
}

fn is_repeatable_optional_payment(effects: &[EffectAst]) -> bool {
    matches!(
        effects,
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mana(ManaActionAst::PayMana { .. })
                | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayEnergy { .. })
                | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { .. }),
            ..
        })]
    )
}

fn synthetic_lexed_word(word: &str) -> OwnedLexToken {
    OwnedLexToken::word(word, TextSpan::synthetic())
}

fn parse_keyword_mechanic_without_terminal_punctuation(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Chain segments are split before the sentence terminator is retained,
    // while the keyword-mechanic grammar intentionally owns the complete
    // sentence shape. Reattach a synthetic terminator so executable keyword
    // clauses such as `Bolster 2`, `Adapt 2`, and `Harness this` use the same
    // typed parser as standalone text instead of becoming implicit grants.
    let mut terminated = tokens.to_vec();
    if !terminated.last().is_some_and(|token| token.is_period()) {
        terminated.push(OwnedLexToken::period(TextSpan::synthetic()));
    }
    parse_keyword_mechanic_clause(&terminated)
}

fn parse_quantified_participant_subject_effect(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Some quantified-player clauses describe one coordinated choice across
    // multiple zones.  Preserve those full-clause specialists before the
    // generic fanout path strips the participant subject and recognizes the
    // remainder as one object filter (which would intersect the zone arms).
    if let Some(effect) =
        super::zone_handlers::parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(
            tokens,
        )
    {
        return Ok(Some(effect));
    }

    let Some(shape) = for_each_shapes::parse_participant_clause_shape(tokens) else {
        return Ok(None);
    };
    if !shape.participant_is_actor {
        return Ok(None);
    }
    if let Some(effect) = super::parse_for_each_opponent_clause(tokens)? {
        return Ok(Some(effect));
    }
    super::parse_for_each_player_clause(tokens)
}

fn parse_choose_land_of_each_basic_land_type_segment(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    if !chain_grammar::parse_choose_each_basic_land_type_tokens(tokens) {
        return None;
    }

    let basic_land_types = [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ];
    Some(
        basic_land_types
            .into_iter()
            .map(|subtype| {
                let mut filter = ObjectFilter::land().with_subtype(subtype);
                filter.controller = Some(PlayerFilter::Any);
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::Implicit,
                    tag: crate::tag::CompilerReferenceTag::It.bind(),
                })
            })
            .collect(),
    )
}

fn rest_action_effect(
    action: chain_grammar::RestActionShape,
    filter: ObjectFilter,
    player: PlayerAst,
) -> EffectAst {
    match action {
        chain_grammar::RestActionShape::Destroy => EffectAst::subject_verb_destroy_all(filter),
        // CR 701.26a: tapping the rest of the matching permanents.
        chain_grammar::RestActionShape::Tap => EffectAst::subject_verb_tap_all(filter),
        chain_grammar::RestActionShape::Exile => EffectAst::subject_verb_exile_all(filter, false),
        chain_grammar::RestActionShape::Sacrifice => {
            EffectAst::subject_verb_sacrifice_all(player, filter)
        }
        // Every other card the chooser holds that matches the chosen set's
        // description (CR 701.9a discards from that player's hand).
        chain_grammar::RestActionShape::Discard => EffectAst::subject_verb_discard(
            player,
            Value::Count(filter.clone()),
            false,
            false,
            Some(filter),
            None,
        ),
    }
}

fn try_apply_rest_action_followup(
    effects: &mut Vec<EffectAst>,
    action: chain_grammar::RestActionShape,
) -> bool {
    if let Some(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter,
        tag,
        player,
        ..
    })) = effects.last()
    {
        let rest_filter = filter.clone().not_tagged(tag.clone());
        let player = *player;
        effects.push(rest_action_effect(action, rest_filter, player));
        return true;
    }

    let Some(last) = effects.last_mut() else {
        return false;
    };
    match last {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: inner_effects,
        })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: inner_effects,
        }) => {
            let Some(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter,
                tag,
                player,
                ..
            })) = inner_effects.last()
            else {
                return false;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            let player = *player;
            inner_effects.push(rest_action_effect(action, rest_filter, player));
            true
        }
        _ => false,
    }
}

fn starts_like_create_fragment_lexed(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::parse_create_fragment_tokens(tokens)
}

fn is_for_each_counter_group_removed_this_way_prefix_lexed(tokens: &[OwnedLexToken]) -> bool {
    super::super::grammar::effects::clause_dispatch_shapes::parse_counter_group_removed_shape(
        tokens,
    )
    .is_some_and(|shape| shape.effect_tokens.is_empty())
}

fn merge_for_each_counter_group_segments_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut merged = Vec::new();
    let mut iter = segments.into_iter().peekable();
    while let Some(mut segment) = iter.next() {
        if is_for_each_counter_group_removed_this_way_prefix_lexed(&segment)
            && let Some(next) = iter.next()
        {
            segment.extend(next);
        }
        merged.push(segment);
    }
    merged
}

pub(super) fn parse_effect_chain_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    match parse_effect_chain_lexed(view.tokens) {
        Ok(effects) => ParseOutcome::matched(effects, crate::rule_engine::lex_clause_span(view)),
        Err(error) => {
            ParseOutcome::Error(crate::recognition::ParseDiagnostic::from_card_text_error(
                crate::recognition::RuleId::new("effect-chain"),
                crate::rule_engine::lex_clause_span(view),
                error,
            ))
        }
    }
}

pub(super) const FALLBACK_POST_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 1] =
    [LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("effect-chain"),
            HeadDiscriminator::words(&[]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_effect_chain_rule_lexed),
    }];

pub(super) const FALLBACK_POST_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&FALLBACK_POST_DIAGNOSTIC_RULES_LEXED);

fn parse_exile_library_then_shuffle_graveyard_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(spec) = chain_grammar::parse_exile_library_shuffle_tokens(tokens) else {
        return Ok(None);
    };
    let (owner_filter, owner_player) = match spec.owner {
        chain_grammar::ChainOwner::You => (PlayerFilter::You, PlayerAst::You),
        chain_grammar::ChainOwner::TargetPlayer => {
            (PlayerFilter::target_player(), PlayerAst::Target)
        }
        chain_grammar::ChainOwner::TargetOpponent => {
            (PlayerFilter::target_opponent(), PlayerAst::TargetOpponent)
        }
    };

    let mut filter = crate::target::ObjectFilter::default().in_zone(Zone::Library);
    filter.owner = Some(owner_filter);
    Ok(Some(vec![
        EffectAst::subject_verb_exile_all(filter, true),
        EffectAst::subject_verb_shuffle_graveyard_into_library(owner_player),
    ]))
}

pub fn looks_like_multi_create_chain_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches!(find_verb_lexed(tokens), Some((Verb::Create, _)))
        && chain_grammar::count_token_mentions(tokens) >= 2
}

pub fn parse_reveal_source_exiled_permanents_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = parse_reveal_source_exiled_permanents_tokens(tokens)?;
    let source_surface = match shape.source_kind {
        SourceLinkedExileReferenceKind::Permanent => "this permanent",
        SourceLinkedExileReferenceKind::CardType(CardType::Artifact) => "this artifact",
        SourceLinkedExileReferenceKind::CardType(CardType::Creature) => "this creature",
        SourceLinkedExileReferenceKind::CardType(CardType::Enchantment) => "this enchantment",
        SourceLinkedExileReferenceKind::CardType(CardType::Land) => "this land",
        SourceLinkedExileReferenceKind::CardType(CardType::Planeswalker) => "this planeswalker",
        SourceLinkedExileReferenceKind::CardType(CardType::Battle) => "this battle",
        SourceLinkedExileReferenceKind::CardType(_) => return None,
    };
    let mut source_exiled =
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind())
            .in_zone(Zone::Exile);
    source_exiled.owner = Some(PlayerFilter::IteratedPlayer);
    source_exiled.source_surface = Some(SourceReferenceSurface::ThisPermanentType(
        source_surface.to_string(),
    ));
    let reveal = EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::Actor,
        PlayerAst::That,
        SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp {
            target: TargetAst::Object(source_exiled.clone(), None, None),
        }),
    );

    let mut permanents = source_exiled;
    permanents.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    let put_onto_battlefield = EffectAst::subject_verb_put_all_onto_battlefield(
        permanents,
        false,
        false,
        ReturnControllerAst::Owner,
    );
    Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
        effects: vec![reveal, put_onto_battlefield],
    })])
}

pub fn parse_effect_chain_lexed(tokens: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    crate::grammar::shared_util::value_expr::validate_result_quantity_bindings(tokens)?;
    super::pair_procedure::validate_discard_replacements(tokens)?;
    super::local_self_replacement::validate(tokens)?;
    // Chain parsing recursively re-enters the sentence dispatcher for
    // nested clauses and quoted/conditional payloads.  The public chain
    if let Some(effects) = super::parse_complete_create_statement(tokens)? {
        return Ok(effects);
    }
    let effects = parse_effect_chain_lexed_inner(tokens)?;
    Ok(group_distributed_exile_instruction(tokens, effects))
}

/// "Target opponent exiles the top card of their library, a card at random
/// from their graveyard, and a card at random from their hand"
/// (Crabomination): one exile verb distributed over an object list is ONE
/// instruction that lowered into several exile actions. Keep them together
/// as one coordinated clause so "cards exiled this way" names all of them;
/// separate sentences (each with its own verb) never merge.
fn group_distributed_exile_instruction(
    tokens: &[OwnedLexToken],
    effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    if effects.len() < 2 {
        return effects;
    }
    let exile_verbs = tokens
        .iter()
        .filter(|token| token.is_any_word(&["exile", "exiles"]))
        .count();
    let one_clause = !tokens.iter().any(|token| {
        token.is_period() || token.is_any_word(&["then", "if", "unless", "where"])
    });
    fn exiles(effect: &EffectAst) -> bool {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => matches!(
                action,
                SubjectVerbActionAst::ZoneMoves(
                    ZoneMoveActionAst::Exile { .. } | ZoneMoveActionAst::ExileAll { .. }
                ) | SubjectVerbActionAst::Library(
                    crate::cards::builders::LibraryActionAst::ExileTopOfLibrary { .. }
                )
            ),
            EffectAst::TagAffected { effect, .. } | EffectAst::TagReferenced { effect, .. } => {
                exiles(effect)
            }
            EffectAst::Sequence { effects } => effects.iter().any(exiles),
            _ => false,
        }
    }
    // Only a list whose members are themselves several exiles: other
    // "exile ... and <action>" chains keep their flat shape for the
    // specialist bundle readers that match it.
    if exile_verbs != 1 || !one_clause || effects.iter().filter(|effect| exiles(effect)).count() < 2
    {
        return effects;
    }
    vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
        result_conjunction: false,
    }]
}

/// Parse the typed producer chain `put a counter ..., then create an X/Y
/// token, where X ...` without entering the aggregate effect dispatcher.
/// The dynamic token action retains the created-object identity used by
/// lowering to emit its base-power/toughness follow-up.
pub fn parse_counter_then_dynamic_token_creation_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("put")) {
        return Ok(None);
    }
    let segments = split_segments_on_comma_then_lexed(vec![tokens]);
    let [counter_tokens, create_tokens] = segments.as_slice() else {
        return Ok(None);
    };
    if !create_tokens
        .first()
        .is_some_and(|token| token.is_word("create"))
    {
        return Ok(None);
    }
    let create = super::creation_handlers::parse_create(create_tokens, None)?;
    if !matches!(
        &create,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                dynamic_power_toughness: Some(_),
                ..
            }),
            ..
        })
    ) {
        return Ok(None);
    }
    let counter = super::zone_counter_helpers::parse_put_counters(counter_tokens)?;
    Ok(Some(vec![EffectAst::CommaThen {
        effects: vec![counter, create],
    }]))
}

pub(super) fn is_atomic_put_counter_for_each_sentence(tokens: &[OwnedLexToken]) -> bool {
    super::super::grammar::effects::zone_counter_shapes::parse_atomic_put_counter_for_each_shape(
        tokens,
    )
}

/// Expand two peer counter-placement clauses when the second carries the
/// shared leading `put` implicitly (`put A counter on each X and B counter on
/// each Y`). Object-filter parsing must not absorb the second descriptor as a
/// union arm of the first target.
pub(super) fn parse_repeated_counter_placement_coordination(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        super::super::grammar::effects::zone_counter_shapes::parse_repeated_counter_placement_shape(
            tokens,
        )
    else {
        return Ok(None);
    };
    let mut second = shape.second_tokens.to_vec();
    if !second.first().is_some_and(|token| token.is_word("put")) {
        second.insert(0, synthetic_lexed_word("put"));
    }
    let effects = vec![
        super::zone_counter_helpers::parse_put_counters(shape.first_tokens)?,
        super::zone_counter_helpers::parse_put_counters(&second)?,
    ];
    let coordination = crate::grammar::effects::coordination::coordination_from_effects(
        crate::model::CoordinationKindAst::SharedSubject,
        crate::model::CoordinationOperatorAst::And,
        crate::model::EffectOrderingAst::Unordered,
        effects,
    )
    .expect("repeated counter placement contains two effects");
    Ok(Some(vec![EffectAst::Coordination(coordination)]))
}

fn parse_atomic_token_copy_exception(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if !super::super::grammar::effects::parse_atomic_token_copy_exception_shape(tokens) {
        return Ok(None);
    }

    let effect = super::creation_handlers::parse_create(tokens, None)?;
    Ok(matches!(
        &effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { .. })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { .. }),
            ..
        })
    )
    .then_some(effect))
}

pub(crate) fn parse_simple_that_creature_owner_library_placement(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    use super::super::grammar::effects::control_copy_attach_shapes::{
        LibraryPlacementShape, parse_library_placement_destination_shape,
    };

    let shape = parse_library_placement_destination_shape(tokens)?;
    let target_words = token_word_refs(shape.target_tokens);
    let destination_words = token_word_refs(shape.destination_tokens);
    let owner_library = matches!(
        destination_words.as_slice(),
        ["its", "owner", "library"] | ["its", "owner's", "library"]
    );
    if shape.order.is_some()
        || target_words.as_slice() != ["put", "that", "creature"]
        || !owner_library
    {
        return None;
    }

    let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
    filter.card_types.push(CardType::Creature);
    filter.set_explicit_card_type_noun(Some(CardType::Creature));
    Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst {
            role: SubjectVerbRoleAst::Actor,
            player: PlayerAst::You,
        },
        action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
            target: TargetAst::Object(filter, None, None),
            source_top_only: false,
            tagged_destinations: Vec::new(),
            zone: Zone::Library,
            to_top: shape.placement == LibraryPlacementShape::Top,
            library_order: None,
            library_order_chooser: PlayerAst::Implicit,
            verb_surface: ironsmith_core::MoveToZoneVerbSurface::Put,
            target_plural_surface: false,
            target_reference_surface: None,
            destination_player_surface: None,
            destination_player_reference_surface: None,
            exiled_with_source_surface: None,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            battlefield_attacking: false,
            battlefield_attack_target_player_or_planeswalker_controlled_by: None,
            battlefield_face_down: false,
            battlefield_transformed: false,
            attached_to: None,
            all: false,
        }),
    }))
}

pub(super) fn has_target_player_resource_coordination(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let starts_with_target_player = words.get(..2).is_some_and(|prefix| {
        prefix[0].eq_ignore_ascii_case("target")
            && (prefix[1].eq_ignore_ascii_case("player")
                || prefix[1].eq_ignore_ascii_case("opponent"))
    }) && find_verb_lexed(tokens)
        .is_some_and(|(_, verb_index)| verb_index == 2);
    starts_with_target_player
        && (has_explicit_comma_then_boundary_lexed(tokens)
            || split_effect_chain_on_and_lexed(tokens).len() > 1)
}

fn parse_independent_explicit_may_coordination(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_effect_chain_on_and_lexed(tokens);
    if segments.len() < 2
        || parse_leading_player_may_lexed(segments[0]).is_none()
        || !segments.iter().skip(1).all(|segment| {
            parse_leading_player_may_lexed(segment).is_some()
                || chain_grammar::parse_leading_chain_scope_tokens(segment).is_some()
        })
    {
        return Ok(None);
    }

    // A repeated modal subject or explicit each-player subject starts an
    // independent instruction. Keep its mandatory/optional scope separate:
    // declining the first action must not suppress the later instruction.
    // A bare shared-subject verb still belongs inside the original May.
    let mut effects = Vec::with_capacity(segments.len());
    for segment in segments {
        let parsed = parse_effect_chain_lexed(segment)?;
        let [effect] = parsed.as_slice() else {
            return Ok(None);
        };
        effects.push(effect.clone());
    }
    Ok(Some(vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
        result_conjunction: false,
    }]))
}

#[path = "chain_carry/chain_entry_readings.rs"]
mod chain_entry_readings;

fn parse_effect_chain_lexed_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effect) = super::duration_source_prevention::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = super::temporary_attack_requirement::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = super::attacked_turn_permission::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = super::graveyard_self_cast::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) = super::temporary_mana_clause::parse(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = super::conditional_protection_list::parse(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = super::repeated_doubling::parse(tokens)? {
        return Ok(effects);
    }
    if let Some(effect) = super::loyalty_activation_allowance::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = matching_spell_cost_modifier_chain(tokens) {
        return Ok(vec![effect]);
    }
    let input = chain_entry_readings::ChainEntry {
        tokens,
        read_by_cache: Default::default(),
    };
    match chain_entry_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(matched.value.value),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    let comma_then_segments = split_segments_on_comma_then_lexed(vec![tokens]);
    if comma_then_segments.len() > 1
        && comma_then_segments
            .last()
            .is_some_and(|segment| segment.iter().any(|token| token.is_word("unless")))
    {
        return parse_effect_chain_inner_lexed_unstacked(tokens, false);
    }
    let effects = parse_effect_chain_uncoordinated_lexed(tokens)?;
    if effects.len() > 1 && has_authored_comma_then_surface_lexed(tokens) {
        return Ok(vec![EffectAst::CommaThen { effects }]);
    }
    Ok(preserve_coordinated_effect_chain_surface(tokens, effects))
}

fn ensure_explicit_target_player_subject_declarations(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let authored_targets = words
        .iter()
        .zip(words.iter().skip(1))
        .filter(|(left, right)| **left == "target" && **right == "player")
        .count();
    if authored_targets == 0 {
        return;
    }
    // "you may have target player mill X cards, where X is ..." (Cloudhoof
    // Kirin): the optional instruction declares its own target player as it
    // lowers, exactly as the X-free sentence does. A second, unconditional
    // declaration ahead of it would ask for another target.
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::Permissions(
                crate::cards::builders::PermissionEffectAst::May { .. }
                    | crate::cards::builders::PermissionEffectAst::MayByPlayer { .. }
            )
        )
    }) {
        return;
    }

    let mut target_subjects = 0usize;
    let mut declarations = 0usize;
    let mut inspect = |nested: &[EffectAst]| {
        for effect in nested {
            let EffectAst::SubjectVerb(subject_verb) = effect else {
                continue;
            };
            if subject_verb.subject.player == PlayerAst::Target {
                target_subjects += 1;
            }
            if matches!(subject_verb.action, SubjectVerbActionAst::TargetOnly { .. }) {
                declarations += 1;
            }
        }
    };
    inspect(effects);
    for effect in effects.iter() {
        for_each_nested_effects(effect, true, &mut inspect);
    }
    drop(inspect);
    if target_subjects < authored_targets || declarations >= authored_targets {
        return;
    }

    for _ in declarations..authored_targets {
        effects.insert(
            0,
            EffectAst::subject_verb_explicit_target_only(TargetAst::Player(
                PlayerFilter::Any,
                span_from_tokens(tokens),
            )),
        );
    }
}

pub(super) fn preserve_independent_target_player_coordination(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let authored_targets =
        super::super::grammar::effects::chain_carry::explicit_target_player_count(tokens);
    if authored_targets < 2
        || effects.len() < 2
        || matches!(effects.as_slice(), [EffectAst::Coordination(_)])
    {
        return;
    }

    let members = std::mem::take(effects);
    if let Some(coordination) = crate::grammar::effects::coordination::coordination_from_effects(
        crate::model::CoordinationKindAst::Conjunction,
        crate::model::CoordinationOperatorAst::And,
        crate::model::EffectOrderingAst::Unordered,
        members,
    ) {
        effects.push(EffectAst::Coordination(coordination));
    }
}

fn parse_leading_action_then_shared_damage_fanout(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens
        .first()
        .is_some_and(|token| token.is_word("if") || token.is_word("unless"))
    {
        return Ok(None);
    }
    for (and_index, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let leading = trim_lexed_commas(&tokens[..and_index]);
        let trailing = trim_lexed_commas(&tokens[and_index + 1..]);
        if leading.is_empty() || !trailing.first().is_some_and(|token| token.is_word("it")) {
            continue;
        }
        let Some(mut damage) =
            super::fanout_family::parse_compound_damage_fanout_sentence(trailing)?
        else {
            continue;
        };
        let mut effects = parse_effect_chain_lexed(leading)?;
        let [
            EffectAst::Coordinated {
                effects: damage_effects,
                ..
            },
        ] = damage.as_mut_slice()
        else {
            continue;
        };
        effects.append(damage_effects);
        return Ok(Some(EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }));
    }
    Ok(None)
}

fn parse_terminal_where_x_binding(tokens: &[OwnedLexToken]) -> Option<(Vec<OwnedLexToken>, Value)> {
    let shape =
        super::super::grammar::effects::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(
            tokens,
        )?;
    let view = TokenWordView::new(tokens);
    let where_word = view.parse_phrase_start(&["where", "x", "is"])?;
    let where_index = view.map_word_to_token_start(where_word)?;
    // `it gains "..., draw X cards, where X is ..."` (Cosima): a where-X
    // inside a quoted ability defines that ability's X; the quoted text is
    // parsed (with its definition) as the granted ability.
    if tokens[..where_index]
        .iter()
        .filter(|token| token.kind == TokenKind::Quote)
        .count()
        % 2
        == 1
    {
        return None;
    }
    if shape.followup_tokens.is_none()
        && has_explicit_comma_then_boundary_lexed(&tokens[where_index..])
    {
        return None;
    }
    let leading_tokens = trim_lexed_commas(&tokens[..where_index]);
    if leading_tokens.is_empty() {
        return None;
    }
    let binding_tokens = crate::util::trim_edge_punctuation_tokens(shape.binding_tokens);
    let value = crate::keyword_static::parse_where_x_is_aggregate_filter_value(binding_tokens)
        .or_else(|| {
            crate::grammar::shared_util::value_semantics::parse_turn_history_value_binding(
                binding_tokens,
            )
        })
        .or_else(|| crate::keyword_static::parse_where_x_is_number_of_filter_value(binding_tokens))
        .or_else(|| super::dispatch_entry::parse_exact_where_x_value_expression(binding_tokens))
        .or_else(|| {
            super::super::grammar::effects::sentence_predicate_shapes::
                parse_where_x_value_shape_tokens(binding_tokens, false)
                .and_then(super::dispatch_inner::lower_where_x_shape)
                .map(|(_, value)| value)
        })
        .or_else(|| crate::keyword_static::parse_value_binding_clause(binding_tokens))?;
    let mut action_tokens = leading_tokens.to_vec();
    if let Some(followup) = shape.followup_tokens {
        action_tokens.extend_from_slice(followup);
    }
    Some((
        action_tokens,
        super::dispatch_entry::with_where_x_surface_hints(value, tokens),
    ))
}

/// Parse the demonstrative per-object reward
/// `the controller of each of those <objects> gains life equal to its mana
/// value`. The unresolved `__it__` collection is intentionally retained here:
/// sentence-sequence reference resolution binds it to the immediately prior
/// affected-object tag, then runtime iteration evaluates each object's LKI
/// controller and mana value independently.
#[path = "chain_carry/surface_preservation.rs"]
mod surface_preservation;
use surface_preservation::shared_trailing_continuous_effect_duration;
pub use surface_preservation::{
    parse_each_prior_affected_object_controller_mana_value_life,
    preserve_coordinated_effect_chain_surface,
};

fn parse_for_each_object_effect_chain_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if crate::grammar::effects::counter_marker_shapes::parse_for_each_counter_kind_tokens(tokens).is_some() {
        return Ok(None);
    }
    if let Some(effects) = super::search_library::parse_for_each_revealed_this_way_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    let Some(shape) = for_each_shapes::parse_for_each_object_effect_shape(tokens) else {
        return Ok(None);
    };

    let mut count_words = vec!["for", "each"];
    count_words.extend(crate::lexer::token_word_refs(shape.filter_tokens));
    let effect_words = crate::lexer::token_word_refs(shape.effect_tokens);
    let has_that_player_payload =
        crate::word_primitives::sequence_occurs(&effect_words, &["that", "player"]);
    if let Some((count, used)) = crate::util::parse_for_each_count_value_words(&count_words)
        && used == count_words.len()
        && !matches!(count.unhinted(), Value::Count(_))
        // A body referring to the quantified object needs an object binding,
        // not repeated execution against the entire prior result set.
        && !(matches!(count.unhinted(), Value::PendingPriorEffectMetric(_))
            && effect_words.iter().any(|word| matches!(*word, "it" | "its")))
        && !(has_that_player_payload
            && matches!(
                count.unhinted(),
                Value::PendingPriorEffectMetric(query)
                    if query.action == Some(ironsmith_core::PriorEffectAction::Tapped)
            ))
    {
        let effects = if shape
            .effect_tokens
            .iter()
            .any(|token| token.is_word("unless"))
        {
            match parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(shape.effect_tokens))?
            {
                Some(effects) => effects,
                None => parse_effect_chain_lexed(shape.effect_tokens)?,
            }
        } else {
            parse_effect_chain_lexed(shape.effect_tokens)?
        };
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each scalar sentence missing effect payload".to_string(),
            ));
        }
        return Ok(Some(vec![EffectAst::ForEach(
            ForEachEffectAst::RepeatEffects {
                count: count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
                effects,
            },
        )]));
    }

    let filter = super::for_each_helpers::parse_for_each_object_filter(shape.filter_tokens)?;
    let effects = if shape
        .effect_tokens
        .iter()
        .any(|token| token.is_word("unless"))
    {
        match parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(shape.effect_tokens))? {
            Some(effects) => effects,
            None => parse_effect_chain_lexed(shape.effect_tokens)?,
        }
    } else {
        parse_effect_chain_lexed(shape.effect_tokens)?
    };
    if effects.is_empty() {
        return Err(CardTextError::ParseError(
            "for-each object sentence missing effect payload".to_string(),
        ));
    }
    Ok(Some(vec![EffectAst::ForEach(
        ForEachEffectAst::ForEachObject { filter, effects },
    )]))
}

#[path = "chain_carry/chain_readings.rs"]
mod chain_readings;

fn parse_effect_chain_uncoordinated_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let input = chain_readings::Chain {
        tokens,
        read_by_cache: Default::default(),
        claims: Default::default(),
    };
    match chain_readings::read_chain(&input) {
        ParseOutcome::Match(matched) => return Ok(matched.value.value),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    parse_effect_chain_with_subject_verb_primitives_lexed(tokens)
}

pub fn preserve_result_conjunction_body_lexed(
    trailing_tokens: &[OwnedLexToken],
    effects: &mut Vec<EffectAst>,
) {
    let Some(grammar_leading_duration) =
        chain_grammar::coordinated_effect_chain_leading_duration(trailing_tokens)
    else {
        return;
    };

    if let [
        EffectAst::Coordinated {
            effects: coordinated,
            result_conjunction: _,
            ..
        },
    ] = effects.as_slice()
    {
        if effects_have_cross_arm_tag_dependency(coordinated) {
            // A coordination boundary must not hide a semantic pipeline from
            // the ordinary specialist lowerers. Those specialists preserve
            // the authored relationship from the typed tag dependency.
            let Some(EffectAst::Coordinated {
                effects: nested, ..
            }) = effects.pop()
            else {
                unreachable!("matched one coordinated effect above")
            };
            *effects = nested;
            return;
        }

        let [
            EffectAst::Coordinated {
                leading_duration,
                result_conjunction,
                ..
            },
        ] = effects.as_mut_slice()
        else {
            unreachable!("matched one coordinated effect above")
        };
        *leading_duration |= grammar_leading_duration;
        *result_conjunction = true;
        return;
    }

    if effects.len() > 1 && !effects_have_cross_arm_tag_dependency(effects) {
        let coordinated = std::mem::take(effects);
        effects.push(EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: grammar_leading_duration,
            result_conjunction: true,
        });
    }
}

pub fn preserve_leading_result_coordination_lexed(
    tokens: &[OwnedLexToken],
    effects: &mut Vec<EffectAst>,
) {
    let Some(prefix) = split_leading_result_prefix_lexed(tokens) else {
        return;
    };

    let nested = match (prefix.kind, effects.as_mut_slice()) {
        (
            LeadingResultPrefixKind::If,
            [EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, effects })],
        ) if predicate == &prefix.predicate => effects,
        (
            LeadingResultPrefixKind::When,
            [EffectAst::Conditionals(ConditionalEffectAst::WhenResult { predicate, effects })],
        ) if predicate == &prefix.predicate => effects,
        _ => return,
    };

    preserve_result_conjunction_body_lexed(prefix.trailing_tokens, nested);
}

pub fn parse_destroy_then_temporary_cant_attack_block_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(split) = chain_grammar::parse_destroy_restriction_splits_tokens(tokens)
        .into_iter()
        .next()
    {
        let mut effects = vec![parse_effect_clause_lexed(split.destroy_tokens)?];
        let Some(tail_effects) = parse_cant_effect_sentence_lexed(split.restriction_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported destroy plus attack/block restriction tail (clause: '{}')",
                token_word_refs(split.restriction_tokens).join(" ")
            )));
        };
        effects.extend(tail_effects);
        return Ok(Some(effects));
    }
    Ok(None)
}

fn clause_may_contain_cast_or_play_permission_lexed(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| {
            matches!(
                word,
                "may" | "cast" | "casts" | "casting" | "play" | "plays" | "playing" | "played"
            )
        })
}

fn leading_may_is_permission_clause_lexed(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    Ok(parse_additional_land_plays_clause_lexed(tokens)?.is_some()
        || parse_permission_clause_spec_lexed(tokens)?.is_some()
        || parse_unsupported_play_cast_permission_clause_lexed(tokens)?.is_some())
}

fn starts_with_until_end_of_turn_trigger_clause(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::parse_until_end_of_turn_trigger_tokens(tokens)
}

fn is_would_enter_replacement_clause(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::parse_would_enter_replacement_tokens(tokens)
}

pub fn parse_or_action_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if chain_grammar::parse_tap_or_untap_all_choice_tokens(tokens) {
        return Ok(None);
    }
    if has_unless_payment_choice(tokens)? {
        return Ok(None);
    }

    for split in chain_grammar::parse_or_action_splits_tokens(tokens) {
        let first = split.first_tokens;
        let second = split.second_tokens;

        let first_starts_effect = find_verb_lexed(first).is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb_lexed(first);
        let second_gain_effect = parse_simple_gain_ability_clause_lexed(second)?;
        let second_starts_effect = find_verb_lexed(second)
            .is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb_lexed(second)
            || second_gain_effect.is_some();
        if !first_starts_effect || !second_starts_effect {
            continue;
        }

        let first_effects = match parse_effect_chain_with_subject_verb_primitives_lexed(first) {
            Ok(effects) if !effects.is_empty() => effects,
            _ => continue,
        };
        let mut second_effects = match second_gain_effect {
            Some(effect) => vec![effect],
            None => match parse_effect_chain_with_subject_verb_primitives_lexed(second) {
                Ok(effects) if !effects.is_empty() => effects,
                _ => continue,
            },
        };
        if effects_reference_it_tag(&second_effects)
            && let Some(primary_target) = first_effects
                .iter()
                .find_map(super::primary_target_from_effect)
        {
            // An explicit target declared before the outer action choice is
            // shared by every branch. In "put a counter on target creature or
            // that creature gains ...", leaving the demonstrative as ambient
            // `it` can bind it to an activation-cost object instead, and a
            // result tag from the primary branch would not exist when the
            // alternative is chosen. Reuse the actual target declaration so
            // legality and execution both have one target slot.
            super::replace_it_target_in_effects(&mut second_effects, &primary_target);
        }

        return Ok(Some(EffectAst::ObjectChoices(
            ObjectChoiceEffectAst::ChooseOneOf {
                chooser: crate::target::PlayerFilter::You,
                modes: vec![
                    crate::cards::builders::ChooseOneModeAst {
                        description: String::new(),
                        effects: first_effects,
                    },
                    crate::cards::builders::ChooseOneModeAst {
                        description: String::new(),
                        effects: second_effects,
                    },
                ],
            },
        )));
    }

    Ok(None)
}

#[cfg(test)]
#[path = "chain_carry/tests.rs"]
mod tests;

pub fn parse_effect_chain_with_subject_verb_primitives_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_with_subject_verb_primitives_lexed_unstacked(tokens)
}

fn parse_effect_chain_with_subject_verb_primitives_lexed_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(rest) = chain_grammar::strip_leading_and_tokens(tokens) {
        return parse_effect_chain_with_subject_verb_primitives_lexed(rest);
    }

    if let Some(effects) =
        super::player_subject_sequences::parse_each_player_exile_sacrifice_return_exiled(tokens)?
    {
        return Ok(effects);
    }
    // A complete win-game clause has a player subject, but `you` is not a
    // generic controller action head. Claim the typed terminal action before
    // the subject/verb registry so it can also serve as the consequence of a
    // value-comparison conditional.
    if let Some(effect) = super::clause_pattern_helpers::parse_win_the_game_clause(tokens)? {
        return Ok(vec![effect]);
    }

    let clause_words = crate::lexer::token_word_refs(tokens);
    if starts_with_until_end_of_turn_trigger_clause(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-end-of-turn permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_would_enter_replacement_clause(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported would-enter replacement clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effects) = parse_return_it_then_loses_all_abilities_lexed(tokens)? {
        return Ok(effects);
    }

    // Copular animation clauses are complete state changes, not ordinary
    // subject/verb ability grants.  The pre-conditional primitive registry
    // also recognizes `are`/`get`, so claim the typed animation shape before
    // that broad registry can reinterpret its P/T and type words.
    if super::super::grammar::effects::clause_dispatch_shapes::parse_copular_animation_shape(tokens)
        .is_some()
    {
        return Ok(vec![parse_effect_clause_lexed(tokens)?]);
    }

    // Result-prefixed clauses own the entire comma-separated body.  Route
    // them through the conditional family before the broad pre-conditional
    // subject/verb registry, whose `draw ... and gain ...` matcher can consume
    // only the first arm and leave the second arm outside the When/If result.
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        let body = if let Some(copy_effect) = parse_copy_spell_clause(prefix.trailing_tokens)? {
            // The copy specialist owns coordinated stack-object sets such as
            // "copy all spells ..., then copy all other abilities ...".
            // Splitting its authored `then` first loses the ordinary
            // coordination surface even though both actions survive.
            vec![copy_effect]
        } else {
            match parse_effect_chain_lexed(prefix.trailing_tokens) {
                Ok(effects) => effects,
                // Restriction bodies ("target creature you control can't be
                // blocked this turn" — Evie Frye) parse as a single clause,
                // not an effect chain.
                Err(chain_error) => match parse_effect_clause_lexed(prefix.trailing_tokens) {
                    Ok(effect) => vec![effect],
                    Err(_) => return Err(chain_error),
                },
            }
        };
        let mut effects = vec![match prefix.kind {
            LeadingResultPrefixKind::If => {
                EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    predicate: prefix.predicate,
                    effects: body,
                })
            }
            LeadingResultPrefixKind::When => {
                EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                    predicate: prefix.predicate,
                    effects: body,
                })
            }
        }];
        preserve_leading_result_coordination_lexed(tokens, &mut effects);
        return Ok(effects);
    }

    // A broad ability-grant primitive can find a later `gain` in a complete
    // paid-label conditional and consume the whole clause as an unconditional
    // action. Route exact optional-cost predicates through the conditional
    // grammar first so the runtime keeps the payment/promise gate. Restricting
    // this priority exception to the typed predicate (including its negation)
    // avoids changing the established routing of unrelated `if` sentences.
    if leading_condition_is_paid_label(tokens) {
        let Some(mut effects) =
            parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)?
        else {
            return Err(CardTextError::ParseError(
                "paid-label condition did not parse as a conditional sentence".to_string(),
            ));
        };
        preserve_leading_result_coordination_lexed(tokens, &mut effects);
        return Ok(effects);
    }

    let pre_conditional_effects = run_subject_verb_primitives_lexed(
        tokens,
        PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )?;
    if let Some(effects) = pre_conditional_effects {
        return Ok(effects);
    }
    let extension_effects = parse_subject_verb_extension_sentence(tokens)?;
    if let Some(effects) = extension_effects {
        return Ok(effects);
    }
    if chain_grammar::starts_with_unless_tokens(tokens)
        && let Some(effects) = parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }
    let conditional_effects =
        parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)?;
    if let Some(mut effects) = conditional_effects {
        preserve_leading_result_coordination_lexed(tokens, &mut effects);
        return Ok(effects);
    }
    if let Some(effects) = run_subject_verb_primitives_lexed(
        tokens,
        POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )? {
        let mut effects = effects;
        append_missing_coordinated_return_discard_tail(tokens, &mut effects)?;
        return Ok(effects);
    }
    parse_effect_chain_inner_lexed(tokens)
}

pub fn leading_condition_is_paid_label(tokens: &[OwnedLexToken]) -> bool {
    let Some(if_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("if"))
    else {
        return false;
    };
    if crate::lexer::token_word_refs(&tokens[..if_idx])
        .iter()
        .any(|word| !matches!(*word, "then"))
    {
        return false;
    }
    let Some(comma_idx) =
        crate::slice_primitives::select_position(&tokens[if_idx + 1..], |token| {
            token.kind == TokenKind::Comma
        })
        .map(|offset| if_idx + 1 + offset)
    else {
        return false;
    };
    let predicate_tokens = &tokens[if_idx + 1..comma_idx];
    match parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
        Ok(crate::cards::builders::PredicateAst::ThisSpellPaidLabel(_)) => true,
        Ok(crate::cards::builders::PredicateAst::Not(inner)) => matches!(
            *inner,
            crate::cards::builders::PredicateAst::ThisSpellPaidLabel(_)
        ),
        _ => false,
    }
}

pub fn append_missing_coordinated_return_discard_tail(
    tokens: &[OwnedLexToken],
    effects: &mut Vec<EffectAst>,
) -> Result<(), CardTextError> {
    if !matches!(
        chain_grammar::coordinated_target_action_kind(tokens),
        Some(chain_grammar::CoordinatedTargetActionKind::Return)
    ) || effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard { .. })
                    | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DiscardHand),
                ..
            })
        )
    }) {
        return Ok(());
    }
    if let Some(discard_tokens) = chain_grammar::trailing_then_discard_tokens(tokens) {
        let mut discard_effects = parse_effect_chain_lexed(discard_tokens)?;
        if discard_tokens
            .first()
            .is_some_and(|token| token.is_word("discard"))
        {
            for effect in &mut discard_effects {
                bind_implicit_player_context(effect, PlayerAst::You);
            }
        }
        effects.extend(discard_effects);
    }
    Ok(())
}

/// A whole chain that is one "spells <player> cast(s) this turn cost {N}
/// less/more to cast" sentence is a resolving, turn-scoped spell-cost
/// modifier. Result consequences ("If they do, spells they cast this turn
/// cost {2} more to cast") reach the chain parser directly, where the clause
/// splitter would otherwise read `cost` as a static ability of the source.
fn matching_spell_cost_modifier_chain(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    // A coordinated chain ("..., then spells you cast ...") has another
    // action the single-sentence modifier reading would swallow.
    if tokens.iter().any(|token| token.is_word("then")) {
        return None;
    }
    super::dispatch_inner::lower_matching_spell_cost_reduction_sentence(
        crate::util::trim_edge_punctuation_tokens(tokens),
    )
}

pub fn parse_effect_chain_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_inner_lexed_unstacked(tokens, true)
}

#[path = "chain_carry/inner_chain_readings.rs"]
mod inner_chain_readings;

/// "you gain life and draw cards equal to its power" (Lifeblood Hydra),
/// "each player loses life and discards cards equal to ..." (Blim): a bare
/// life gain/loss coordinated with a later instruction shares that
/// instruction's terminal "equal to" amount. Restate the amount on the life
/// clause so each coordinated action reads its own complete quantity.
fn expand_shared_life_equal_to_amount(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let and_idx = tokens.iter().position(|token| token.is_word("and"))?;
    let head = &tokens[..and_idx];
    let (life, before_life) = head.split_last()?;
    if !life.is_word("life")
        || !before_life
            .last()
            .is_some_and(|verb| verb.is_any_word(&["gain", "gains", "lose", "loses"]))
    {
        return None;
    }
    let tail = &tokens[and_idx + 1..];
    let equal_idx = tail
        .windows(2)
        .rposition(|pair| pair[0].is_word("equal") && pair[1].is_word("to"))?;
    let equal_tail = crate::util::trim_edge_punctuation_tokens(&tail[equal_idx..]);
    // Only a single terminal amount phrase is shared; a further coordinated
    // or sequenced action after it would make the scope ambiguous.
    if equal_tail.len() <= 2
        || equal_tail
            .iter()
            .any(|token| token.is_any_word(&["and", "then"]) || token.is_comma())
    {
        return None;
    }
    let mut expanded = head.to_vec();
    expanded.extend_from_slice(equal_tail);
    expanded.extend_from_slice(&tokens[and_idx..]);
    Some(expanded)
}

fn parse_effect_chain_inner_lexed_unstacked(
    tokens: &[OwnedLexToken],
    recognize_control_flow: bool,
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(expanded) = expand_shared_life_equal_to_amount(tokens) {
        return parse_effect_chain_inner_lexed_unstacked(&expanded, recognize_control_flow);
    }
    if let Some(effect) = super::duration_source_prevention::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = super::temporary_attack_requirement::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = super::attacked_turn_permission::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = super::graveyard_self_cast::parse(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) = super::temporary_mana_clause::parse(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = super::conditional_protection_list::parse(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = super::repeated_doubling::parse(tokens)? {
        return Ok(effects);
    }
    if let Some(effect) = super::loyalty_activation_allowance::parse(tokens)? {
        return Ok(vec![effect]);
    }
    // Conditional sentence readers enter here directly. A value definition
    // between coordinated actions still belongs to the complete chain; the
    // outer binding reader removes that definition before recursing here.
    if parse_terminal_where_x_binding(tokens).is_some() {
        return parse_effect_chain_lexed(tokens);
    }
    if recognize_control_flow {
        let comma_then_segments = split_segments_on_comma_then_lexed(vec![tokens]);
        if comma_then_segments.len() > 1
            && comma_then_segments
                .last()
                .is_some_and(|segment| segment.iter().any(|token| token.is_word("unless")))
        {
            return parse_effect_chain_inner_lexed_unstacked(tokens, false);
        }
    }
    if let Some(effect) = matching_spell_cost_modifier_chain(tokens) {
        return Ok(vec![effect]);
    }
    let input = inner_chain_readings::InnerChain {
        tokens,
        recognize_control_flow,
        read_by_cache: Default::default(),
    };
    match inner_chain_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(matched.value.value),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    let leading_duration_shape = chain_grammar::parse_carry_duration_prefix_tokens(tokens);
    let choose_then_exile_reference = parse_choose_then_exile_reference_shape(tokens).is_some();
    let (effect_chain_tokens, leading_duration) = match leading_duration_shape.as_ref() {
        Some(shape) => (shape.rest, Some(shape.duration.clone())),
        None => (tokens, None),
    };
    let coordination_reference_facts =
        super::super::grammar::effects::coordination::recognize_coordination_reference_facts(
            effect_chain_tokens,
        );
    let mut effects = Vec::new();
    // "Add two mana in any combination of {U}, {B}, and/or {R}." (Relic of
    // Sauron) lists mana symbols; its conjunctions never coordinate actions.
    let lists_mana_combination = effect_chain_tokens.windows(3).any(|window| {
        window[0].is_word("any") && window[1].is_word("combination") && window[2].is_word("of")
    });
    // "sacrifices a Plains or a white permanent" (Omen of Fire) names one
    // sacrificed object chosen from a union; the bare noun phrase after
    // "or" has no verb of its own, so it never coordinates a second action.
    let sacrifices_object_union = {
        let words = crate::lexer::token_word_refs(effect_chain_tokens);
        let or_positions = effect_chain_tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| token.is_word("or"))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if words
            .iter()
            .any(|word| matches!(*word, "sacrifice" | "sacrifices"))
            && !words
                .iter()
                .any(|word| matches!(*word, "and" | "and/or" | "then" | "unless" | "if"))
            && !effect_chain_tokens.iter().any(OwnedLexToken::is_comma)
            && let [or_index] = or_positions.as_slice()
            && effect_chain_tokens
                .get(or_index + 1)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            && !super::lex_chain_helpers::segment_has_effect_head_lexed(
                &effect_chain_tokens[or_index + 1..],
            )
        {
            true
        } else {
            false
        }
    };
    // "target creature gains your choice of flying, vigilance, deathtouch, or
    // lifelink until end of turn" (Atraxa's Skitterfang, Éowyn): the commas
    // and "or" after "your choice of" list the options of one choice; they
    // never coordinate separate actions. Only claimed when nothing before the
    // list and no conjunction elsewhere could coordinate a real action.
    let lists_explicit_choice = effect_chain_tokens
        .windows(3)
        .position(|window| {
            window[0].is_word("your") && window[1].is_word("choice") && window[2].is_word("of")
        })
        .is_some_and(|choice_idx| {
            !effect_chain_tokens[..choice_idx].iter().any(OwnedLexToken::is_comma)
                && !effect_chain_tokens
                    .iter()
                    .any(|token| token.is_any_word(&["and", "then", "unless", "if"]))
        });
    let mut coordination_plan = if lists_mana_combination
        || sacrifices_object_union
        || lists_explicit_choice
    {
        None
    } else {
        match super::super::grammar::effects::coordination::recognize_coordination(
            effect_chain_tokens,
        ) {
            crate::recognition::ParseOutcome::Match(matched) => Some(matched.value),
            crate::recognition::ParseOutcome::NoMatch => None,
            crate::recognition::ParseOutcome::Error(diagnostic) => {
                return Err(diagnostic.into_card_text_error());
            }
        }
    };
    let has_verb_less_attack_permission = effect_chain_tokens
        .windows(2)
        .any(|pair| pair[0].is_word("can") && pair[1].is_word("attack"));
    if has_verb_less_attack_permission
        && split_segments_on_comma_effect_head_lexed(vec![effect_chain_tokens]).len() > 1
    {
        // Preserve punctuation while the ordinary chain parser expands
        // comma siblings nested inside a conjunction arm.
        coordination_plan = None;
    }
    let planned_segments = coordination_plan
        .as_ref()
        .and_then(|plan| plan.materialized_segments());
    if coordination_plan.is_some() && planned_segments.is_none() {
        coordination_plan = None;
    }
    let mut segments: Vec<Vec<OwnedLexToken>> = if let Some(planned_segments) = planned_segments {
        planned_segments
    } else {
        let raw_segments = if lists_mana_combination {
            vec![effect_chain_tokens]
        } else {
            split_effect_chain_on_and_lexed(effect_chain_tokens)
        };
        let mut lexed_segments = Vec::new();
        for segment in raw_segments {
            if segment.is_empty() {
                continue;
            }
            lexed_segments.push(segment);
        }

        let mut merged_lexed_segments: Vec<Vec<OwnedLexToken>> = Vec::new();
        for lexed_segment in lexed_segments {
            let segment = lexed_segment.to_vec();
            if merged_lexed_segments.is_empty() {
                merged_lexed_segments.push(segment);
                continue;
            }
            if !super::lex_chain_helpers::segment_has_effect_head_lexed(&segment) {
                if let Some(previous) = merged_lexed_segments.last()
                    && let Some(previous_tail) =
                        split_segments_on_comma_then_lexed(vec![previous.as_slice()]).last()
                    && previous_tail.len() < previous.len()
                    && let Some(expanded) =
                        expand_missing_verb_segment_lexed(previous_tail, &segment)
                {
                    merged_lexed_segments.push(expanded);
                    continue;
                }
                if let Some(previous) = merged_lexed_segments.last()
                    && let Some(expanded) = expand_missing_verb_segment_lexed(previous, &segment)
                {
                    merged_lexed_segments.push(expanded);
                    continue;
                }
                let last = merged_lexed_segments
                    .last_mut()
                    .expect("non-empty segments");
                last.push(synthetic_lexed_word("and"));
                last.extend(segment);
                continue;
            }
            merged_lexed_segments.push(segment);
        }
        while merged_lexed_segments.len() > 1
            && !super::lex_chain_helpers::segment_has_effect_head_lexed(&merged_lexed_segments[0])
        {
            let mut first = merged_lexed_segments.remove(0);
            first.push(synthetic_lexed_word("and"));
            let mut next = merged_lexed_segments.remove(0);
            first.append(&mut next);
            merged_lexed_segments.insert(0, first);
        }
        let merged_segment_slices = merged_lexed_segments
            .iter()
            .map(Vec::as_slice)
            .collect::<Vec<_>>();
        split_segments_on_comma_effect_head_lexed(split_segments_on_comma_then_lexed(
            merged_segment_slices,
        ))
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect()
    };
    // A verb-less attack permission can share a comma arm with a grant.
    // Keep the ordinary typed coordination tree for all other effect chains.
    if has_verb_less_attack_permission {
        let sibling_segments =
            split_segments_on_comma_effect_head_lexed(segments.iter().map(Vec::as_slice).collect())
                .into_iter()
                .map(<[OwnedLexToken]>::to_vec)
                .collect::<Vec<_>>();
        if sibling_segments.len() != segments.len() {
            coordination_plan = None;
            segments = sibling_segments;
        }
    }
    segments = expand_bare_tap_state_verb_shared_object_lexed(segments);
    segments = expand_segments_with_comma_action_clauses_lexed(segments);
    segments = expand_segments_with_multi_create_clauses_lexed(segments);
    segments = merge_for_each_counter_group_segments_lexed(segments);
    // "loses first strike or swampwalk" is a choice between two removals
    // (Urborg); only an "and" list removes both at once.
    let disjunctive_chain = coordination_plan.as_ref().is_some_and(|plan| {
        plan.kind == crate::model::CoordinationKindAst::Disjunction
            || plan
                .boundaries
                .iter()
                .any(|boundary| boundary.operator == crate::model::CoordinationOperatorAst::Or)
    });
    let mut carried_context: Option<CarryContext> = None;
    let mut carried_duration: Option<Until> = leading_duration.clone();
    let mut carried_leading_duration = leading_duration.is_some();
    let mut previous_segment: Option<Vec<OwnedLexToken>> = None;
    for segment in segments {
        let mut segment = segment;
        let segment_carry_facts =
            super::super::grammar::effects::coordination::recognize_coordination_clause_facts(
                &segment,
            );
        let bind_source_exiled =
            choose_then_exile_reference && parse_exile_reference_action_shape(&segment).is_some();
        // A trailing where-X clause binds values in the enclosing sentence;
        // it is not another executable member of the coordination.  The
        // sentence dispatcher applies the typed value after this chain has
        // been lowered, so keep the action list free of a synthetic
        // subject/verb parse for the binding text itself.
        if is_standalone_where_x_binding_segment(&segment) {
            continue;
        }
        // "destroy that creature and it can't be regenerated" (Consuming
        // Ferocity): the rider sets the destroy's no-regeneration flag.
        if bind_no_regeneration_rider(&mut effects, &segment) {
            previous_segment = Some(segment);
            continue;
        }
        if append_shared_damage_player_operand(&mut effects, &segment) {
            previous_segment = Some(segment);
            continue;
        }
        if is_orphan_rounded_up_where_x_tail(&segment, previous_segment.as_deref(), effects.last())
        {
            continue;
        }
        if coordination_plan.is_none()
            && let Some(previous) = &previous_segment
            && let Some(expanded) =
                super::super::grammar::effects::coordination::materialize_shared_subject_followup(
                    previous, &segment,
                )
        {
            segment = expanded;
        }

        // A leading duration can begin inside a larger coordinated chain:
        // "[action], and until your next turn, [restriction] and [grant]."
        // Once that exact prefix appears, it scopes the remaining arms of
        // this same conjunction just as a whole-chain leading duration does.
        if let Some((duration, scoped_clause)) =
            chain_grammar::parse_carry_duration_prefix_tokens(&segment)
                .map(|shape| (shape.duration.clone(), shape.rest.to_vec()))
        {
            carried_duration = Some(duration.clone());
            carried_leading_duration = true;
            // The prefix is grammar for the remaining coordination members,
            // not part of this member's restriction subject. Dispatch the
            // complete scoped clause recursively before a broad restriction
            // leaf can consume only its first action and discard a following
            // grant. The typed duration then applies to every returned arm.
            let scoped_clause = trim_lexed_commas(&scoped_clause);
            if !scoped_clause.is_empty() {
                // Retain the duration prefix while asking the complete
                // sentence dispatcher to lower this member.  Removing the
                // prefix and re-entering the chain fallback makes a lone
                // `life total can't change` arm look like an ability grant;
                // the sentence grammar uses the temporal prefix to select
                // the typed global restriction before that fallback.
                let mut scoped_effects = super::parse_effect_sentence_lexed(&segment)?;
                for effect in &mut scoped_effects {
                    if let Some(context) = carried_context {
                        maybe_apply_carried_player_with_clause_facts(
                            effect,
                            context,
                            segment_carry_facts,
                        );
                    }
                    apply_carried_effect_duration(effect, &duration);
                }
                effects.extend(
                    scoped_effects
                        .into_iter()
                        .map(|effect| bind_source_exiled_effect(effect, bind_source_exiled)),
                );
                previous_segment = Some(segment);
                continue;
            }
        }

        // A comma/"then" chain is split into individual executable arms
        // before this loop. Give a bare keyword action in any arm the same
        // typed lowering as a standalone sentence before the no-verb fallback
        // can reinterpret it as an ability granted to the previous object.
        if let Some(effect) = parse_keyword_mechanic_without_terminal_punctuation(&segment)? {
            effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            previous_segment = Some(segment);
            continue;
        }

        let carry_gain_duration = find_verb_lexed(&segment).is_some_and(|(verb, verb_idx)| {
            verb_idx == 0 && matches!(verb, Verb::Gain | Verb::Lose)
        });
        let carry_leading_duration = carried_leading_duration;
        let segment_effects = if let Some(effect) =
            parse_quantified_participant_subject_effect(&segment)?
        {
            Some(vec![effect])
        } else if let Some(effects) = parse_sentence_return_with_counters_on_it_lexed(&segment)? {
            Some(effects)
        } else if let Some(effects) =
            parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(&segment)?
        {
            Some(effects)
        } else if let Some(prefix) = split_leading_result_prefix_lexed(&segment) {
            Some(vec![match prefix.kind {
                LeadingResultPrefixKind::If => {
                    EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                        predicate: prefix.predicate,
                        effects: parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
                    })
                }
                LeadingResultPrefixKind::When => {
                    EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                        predicate: prefix.predicate,
                        effects: parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
                    })
                }
            }])
        } else {
            parse_sentence_exile_source_with_counters_lexed(&segment)?
        };
        if let Some(segment_effects) = segment_effects {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_facts(
                        &mut effect,
                        context,
                        segment_carry_facts,
                    );
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            continue;
        }
        if let Some(segment_effects) = parse_search_library_sentence_lexed(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_facts(
                        &mut effect,
                        context,
                        segment_carry_facts,
                    );
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            continue;
        }
        if let Some(segment_effects) = parse_cant_effect_sentence_lexed(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_facts(
                        &mut effect,
                        context,
                        segment_carry_facts,
                    );
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            continue;
        }
        if carry_leading_duration
            && let Some(duration) = &carried_duration
            && let Some(segment_effects) = parse_carried_cant_effects(&segment, duration)?
        {
            effects.extend(
                segment_effects
                    .into_iter()
                    .map(|effect| bind_source_exiled_effect(effect, bind_source_exiled)),
            );
            previous_segment = Some(segment);
            continue;
        }
        if let Some(shape) = for_each_shapes::parse_for_each_object_effect_shape(&segment) {
            let filter =
                super::for_each_helpers::parse_for_each_object_filter(shape.filter_tokens)?;
            let nested_effects = parse_effect_chain_lexed(shape.effect_tokens)?;
            if nested_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "for-each object sentence missing effect payload".to_string(),
                ));
            }
            let effect = EffectAst::ForEach(ForEachEffectAst::ForEachObject {
                filter,
                effects: nested_effects,
            });
            effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            previous_segment = Some(segment);
            continue;
        }
        if let Some(segment_effects) =
            super::subject_verb_special_recognizers::parse_scaled_target_power_sentence(&segment)?
        {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_facts(
                        &mut effect,
                        context,
                        segment_carry_facts,
                    );
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            previous_segment = Some(segment);
            continue;
        }
        if let Some(segment_effects) = parse_subject_verb_extension_sentence(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_facts(
                        &mut effect,
                        context,
                        segment_carry_facts,
                    );
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            previous_segment = Some(segment);
            continue;
        }
        // The outer chain splitter isolates an anaphoric combat-assignment
        // clause such as "you choose how those creatures block" before the
        // sentence-level subject/verb dispatcher runs. Preserve the reusable
        // combat-choice capability here instead of letting the broad `choose`
        // primitive turn the pronoun into an unrelated object selection.
        if let Some(effect) =
            super::dispatch_inner::parse_generic_control_combat_choices_subject_verb(&segment)?
        {
            effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            previous_segment = Some(segment);
            continue;
        }
        // A later comma/then arm can introduce its own complete optional
        // action (`..., then you may pay ...`). The top-level leading-may
        // handler cannot see that nested arm after segmentation, while the
        // bare subject/verb primitive dispatcher does not own `may`. Re-enter
        // the full chain parser for this strictly smaller segment so the
        // optional action remains typed instead of being folded into the
        // preceding verb.
        if parse_leading_player_may_lexed(&segment).is_some()
            || chain_grammar::starts_with_may_tokens(&segment)
        {
            let segment_effects = parse_effect_chain_lexed(&segment)?;
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_facts(
                        &mut effect,
                        context,
                        segment_carry_facts,
                    );
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            previous_segment = Some(segment);
            continue;
        }
        // Coordination has already materialized an omitted player subject on
        // a followup such as `then reveals a card at random from their hand`.
        // Preserve that exact random-selection program before the generic
        // complete subject/verb fast path reduces it to an ordinary reveal.
        // The specialist proves the player, hand ownership, single-card
        // count, and authored random qualifier; unrelated reveal clauses keep
        // using the ordinary fast path below.
        if let Some(segment_effects) = parse_sentence_target_player_reveals_random_card_from_hand(
            SubjectVerbPrimitiveClause::new(&segment),
        )? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_facts(
                        &mut effect,
                        context,
                        segment_carry_facts,
                    );
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            previous_segment = Some(segment);
            continue;
        }
        // Typed coordination materializes an omitted player subject onto each
        // member before this loop. A complete resource-action member such as
        // `Target opponent loses 2 life` already has an unambiguous ordinary
        // clause parse. Give that complete parse priority over indexed
        // ability-modifier primitives: the latter see the leading `target`
        // and can otherwise treat the entire `opponent loses ...` tail as an
        // object selector before validating the `loses` action.
        if super::super::grammar::effects::clause_dispatch_shapes::parse_clause_subject_verb_shape(
            &segment,
        )
        .is_some()
            // A remaining `then` tail is a compound instruction, so the
            // single-clause fast path cannot claim it as complete.
            && !segment.iter().any(|token| token.is_word("then"))
            && let Ok(mut effect) = parse_effect_clause_lexed(&segment)
        {
            if let Some(context) = carried_context {
                maybe_apply_carried_player_with_clause_facts(
                    &mut effect,
                    context,
                    segment_carry_facts,
                );
            }
            if (carry_gain_duration || carry_leading_duration)
                && let Some(duration) = &carried_duration
            {
                apply_carried_effect_duration(&mut effect, duration);
            }
            if let Some(context) = explicit_player_for_carry(&effect) {
                carried_context = Some(context);
            }
            if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                carried_duration = Some(duration);
            }
            effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            previous_segment = Some(segment);
            continue;
        }
        // A transform/convert segment may retain its own `then` tail after
        // the outer comma split. Preserve that tail before a single-verb
        // clause parser consumes only the transform target.
        let primitive_segment_effects = if let Some(effects) =
            super::subject_verb_primitives::parse_sentence_transform_with_followup(
                super::SubjectVerbPrimitiveClause::new(&segment),
            )? {
            Some(effects)
        } else if let Some(effects) = run_subject_verb_primitives_lexed(
            &segment,
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            Some(effects)
        } else if let Some(effects) =
            parse_conditional_sentence_family_lexed(&segment, parse_effect_chain_lexed)?
        {
            Some(effects)
        } else {
            run_subject_verb_primitives_lexed(
                &segment,
                POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
                &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
            )?
        };
        if let Some(segment_effects) = primitive_segment_effects {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_facts(
                        &mut effect,
                        context,
                        segment_carry_facts,
                    );
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            previous_segment = Some(segment);
            continue;
        }
        if let Some(followup) = parse_token_copy_followup_sentence_lexed(&segment)
            && try_apply_token_copy_followup(&mut effects, followup)?
        {
            continue;
        }
        if let Some(segment_effects) = parse_choose_land_of_each_basic_land_type_segment(&segment) {
            effects.extend(segment_effects);
            previous_segment = Some(segment);
            continue;
        }
        if let Some(action) = chain_grammar::parse_rest_action_tokens(&segment)
            && try_apply_rest_action_followup(&mut effects, action)
        {
            previous_segment = Some(segment);
            continue;
        }
        if let Some(gain_tail) = chain_grammar::split_all_abilities_and_gain_tokens(&segment) {
            let mut gain_tokens = Vec::new();
            gain_tokens.push(synthetic_lexed_word("it"));
            gain_tokens.extend(gain_tail.iter().cloned());
            if let Some(mut effect) = parse_simple_gain_ability_clause_lexed(&gain_tokens)? {
                if let Some(duration) = &carried_duration {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
                previous_segment = Some(segment);
                continue;
            }
        }
        // Coordinated ability lists commonly omit the repeated "gains":
        // `gains flying, double strike, and vigilance until end of turn`.
        // The comma splitter leaves the later arms as bare keyword phrases;
        // feed those through the same typed modifier parser with an implicit
        // subject instead of treating them as effect clauses. Preserve a
        // preceding `loses` head as well: `loses first strike or swampwalk`
        // is a choice between two removals, not a removal and a grant.
        if find_verb_lexed(&segment).is_none() {
            let losing = previous_segment.as_deref().is_some_and(|previous| {
                find_verb_lexed(previous).is_some_and(|(verb, _)| verb == Verb::Lose)
            });
            let modifier = if losing { "loses" } else { "gains" };
            let mut modifier_tokens =
                vec![synthetic_lexed_word("it"), synthetic_lexed_word(modifier)];
            modifier_tokens.extend(segment.iter().cloned());
            let parsed = if losing {
                parse_simple_lose_ability_clause_lexed(&modifier_tokens)?
            } else {
                parse_simple_gain_ability_clause_lexed(&modifier_tokens)?
            };
            if let Some(mut effect) = parsed {
                if let Some(duration) = &carried_duration {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                // "Target creature loses first strike or swampwalk" (Urborg):
                // the alternatives are one choice over the same announced
                // target (CR 700.2), so the bare arm acts on the preceding
                // arm's subject instead of an unbound synthetic "it".
                if disjunctive_chain
                    && let Some(previous) = effects.last()
                    && let Some(subject) = shared_ability_modifier_target(previous)
                {
                    set_ability_modifier_target(&mut effect, subject);
                }
                // "<subject> lose A and B until end of turn": the bare arm
                // shares the preceding subject and its trailing duration.
                if losing
                    && !disjunctive_chain
                    && effects.last_mut().is_some_and(|previous| {
                        chain_carry_ability_programs::merge_shared_subject_lose_followup(
                            previous, &effect,
                        )
                    })
                {
                    previous_segment = Some(segment);
                    continue;
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
                previous_segment = Some(segment);
                continue;
            }
        }
        let mut effect = parse_effect_clause_with_trailing_if_lexed(&segment)?;
        if let Some(context) = carried_context {
            maybe_apply_carried_player_with_clause_facts(&mut effect, context, segment_carry_facts);
        }
        if (carry_gain_duration || carry_leading_duration)
            && let Some(duration) = &carried_duration
        {
            apply_carried_effect_duration(&mut effect, duration);
        }
        if let Some(context) = explicit_player_for_carry(&effect) {
            carried_context = Some(context);
        }
        if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
            carried_duration = Some(duration);
        }
        effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
        previous_segment = Some(segment);
    }
    collapse_for_each_player_it_tag_followups(&mut effects);
    collapse_for_each_object_it_tag_followups(&mut effects);
    collapse_token_copy_next_end_step_exile_followup_lexed(&mut effects, tokens);
    collapse_token_copy_next_end_step_sacrifice_followup_lexed(&mut effects, tokens);
    collapse_token_copy_end_of_combat_exile_followup_lexed(&mut effects, tokens);
    append_missing_coordinated_return_discard_tail(tokens, &mut effects)?;
    bind_adjacent_discard_count_draws(&mut effects);
    bind_adjacent_implicit_draw_discard_subjects(
        &mut effects,
        coordination_reference_facts.implicit_draw_discard_actor,
    );
    bind_adjacent_life_stat_pronouns(&mut effects, coordination_reference_facts.life_stat_pronoun);
    bind_each_prior_affected_object_controller_life_gain(
        &mut effects,
        coordination_reference_facts.affected_object_controller_reward,
    );
    if let Some(kind) = chain_grammar::coordinated_target_action_kind(tokens) {
        wrap_leading_coordinated_target_actions(&mut effects, kind);
    }
    if chain_grammar::coordinated_tap_then_next_untap(tokens)
        && tap_then_next_untap_actions(&effects)
    {
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]);
    }
    if chain_grammar::coordinated_source_damage_then_gain(tokens)
        && source_damage_then_gain_ability_actions(&effects)
    {
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]);
    }
    if let Some(leading_duration) =
        chain_grammar::coordinated_target_stat_modifier_leading_duration(tokens)
        && effects.len() >= 2
        && effects.iter().all(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { .. }),
                    ..
                })
            )
        })
    {
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration,
            result_conjunction: false,
        }]);
    }
    // The typed coordination path bypasses the legacy surface wrapper below,
    // so carry an exact trailing duration across the first arm before the
    // flat effects are moved into `CoordinationAst`. The shared-target check
    // prevents a duration authored for the second action from leaking onto an
    // independent object or player.
    if coordination_plan.is_some()
        && let Some(duration) = shared_trailing_continuous_effect_duration(&effects)
    {
        apply_carried_effect_duration(&mut effects[0], &duration);
    }
    if let Some(plan) = coordination_plan
        && let Some(coordination) = plan.into_ast(effects.clone())
    {
        return Ok(vec![EffectAst::Coordination(coordination)]);
    }
    Ok(effects)
}

/// "Untap and goad that creature" (Besmirch): a bare tap-state verb shares
/// the object of the following coordinated action. Materialize the shared
/// operand on the bare arm ("untap that creature") so each arm owns the same
/// antecedent reference instead of an untargeted untap.
fn expand_bare_tap_state_verb_shared_object_lexed(
    mut segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    for idx in 0..segments.len().saturating_sub(1) {
        let [verb] = segments[idx].as_slice() else {
            continue;
        };
        if !verb.is_any_word(&["tap", "untap"]) {
            continue;
        }
        let next = &segments[idx + 1];
        if !next.first().is_some_and(|token| token.as_word().is_some())
            || !matches!(find_verb_lexed(next), Some((_, 0)))
        {
            continue;
        }
        let object = &next[1..];
        if !object
            .first()
            .is_some_and(|token| token.is_any_word(&["that", "it", "them", "those", "target"]))
        {
            continue;
        }
        let mut expanded = segments[idx].clone();
        expanded.extend(object.iter().cloned());
        segments[idx] = expanded;
    }
    segments
}

pub(super) fn parse_inline_looked_card_partition_chain(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let comma_then_segments = split_segments_on_comma_then_lexed(vec![tokens]);
    let effects = if let [look_tokens, partition_tokens] = comma_then_segments.as_slice()
        && let Some(effects) =
            super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_inline_look_at_top_then_singleton_hand_partition(
            look_tokens,
            partition_tokens,
        )
    {
        effects
    } else {
        // Some prepared CST views retain the typed `then` connective but
        // discard its comma. The full compositional pattern still proves the
        // same look/selection/remainder ownership in that representation.
        super::dispatch_inner::parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(
            tokens,
        )?
    };
    Some(vec![EffectAst::CommaThen { effects }])
}

fn parse_required_inline_looked_card_partition_chain(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_inline_looked_card_partition_chain(tokens).ok_or_else(|| {
        CardTextError::InvariantViolation(
            "grammar-proven conditional looked-card partition did not materialize".to_string(),
        )
    })
}

pub(super) fn parse_conditional_inline_looked_card_partition(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("if")) {
        return Ok(None);
    }
    let Some(comma) = crate::slice_primitives::select_position(tokens, OwnedLexToken::is_comma)
    else {
        return Ok(None);
    };
    if parse_inline_looked_card_partition_chain(crate::util::trim_edge_punctuation_tokens(
        &tokens[comma + 1..],
    ))
    .is_none()
    {
        return Ok(None);
    }
    crate::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed(
        tokens,
        parse_required_inline_looked_card_partition_chain,
    )
    .map(Some)
}

/// Bind an authored per-object controller reward to each object affected by
/// the immediately preceding tagged sweep. A scalar gain by `you` cannot
/// represent "the controller of each of those artifacts" when the destroyed
/// set can have several different controllers.
fn bind_each_prior_affected_object_controller_life_gain(
    effects: &mut [EffectAst],
    recognized_reference: bool,
) {
    if !recognized_reference || effects.len() < 2 {
        return;
    }
    let preceding_index = effects.len() - 2;
    let gain_index = effects.len() - 1;
    let EffectAst::TagAffected {
        effect: destroyed,
        tag,
    } = &effects[preceding_index]
    else {
        return;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = destroyed.as_ref() else {
        return;
    };
    let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll {
        filter,
        no_regeneration: true,
        ..
    }) = action
    else {
        return;
    };
    if !matches!(filter.card_types.as_slice(), [crate::CardType::Artifact]) {
        return;
    }

    let EffectAst::SubjectVerb(SubjectVerbEffectAst { subject, action }) = &effects[gain_index]
    else {
        return;
    };
    if subject.role != SubjectVerbRoleAst::AffectedPlayer
        || !matches!(subject.player, PlayerAst::You | PlayerAst::Implicit)
    {
        return;
    }
    let SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }) = action
    else {
        return;
    };

    fn retag_mana_value(value: &Value, prior_tag: &TagKey) -> Option<Value> {
        match value {
            Value::SurfaceHinted { value, hints } => Some(Value::SurfaceHinted {
                value: Box::new(retag_mana_value(value, prior_tag)?),
                hints: hints.clone(),
            }),
            Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == prior_tag) => {
                Some(Value::ManaValueOf(Box::new(
                    ChooseSpec::Tagged((crate::tag::CompilerReferenceTag::It.bind()).into())
                        .with_surface_hints(spec.surface_hints().iter().cloned()),
                )))
            }
            _ => None,
        }
    }
    let Some(amount) = retag_mana_value(amount, tag) else {
        return;
    };

    effects[gain_index] = EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: tag.clone(),
        effects: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::ItsController,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }),
        )],
    });
}

fn tap_then_next_untap_actions(effects: &[EffectAst]) -> bool {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { .. }),
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::Untap(_),
                    duration: Until::ControllersNextUntapStep,
                    condition: None,
                    ..
                },
            ..
        }),
    ] = effects
    else {
        return false;
    };
    true
}

fn coordinated_target_action_matches(
    effect: &EffectAst,
    kind: chain_grammar::CoordinatedTargetActionKind,
) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return false;
    };
    matches!(
        (kind, action),
        (
            chain_grammar::CoordinatedTargetActionKind::Destroy,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { .. })
        ) | (
            chain_grammar::CoordinatedTargetActionKind::Exile,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { .. })
        ) | (
            chain_grammar::CoordinatedTargetActionKind::Return,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { .. })
        )
    )
}

fn wrap_leading_coordinated_target_actions(
    effects: &mut Vec<EffectAst>,
    kind: chain_grammar::CoordinatedTargetActionKind,
) {
    let coordinated_len = effects
        .iter()
        .take_while(|effect| coordinated_target_action_matches(effect, kind))
        .count();
    if coordinated_len < 2 {
        return;
    }
    let remainder = effects.split_off(coordinated_len);
    let coordinated = std::mem::take(effects);
    effects.push(EffectAst::Coordinated {
        effects: coordinated,
        leading_duration: false,
        result_conjunction: false,
    });
    effects.extend(remainder);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarryContext {
    Player(PlayerAst),
    ForEachPlayer,
    ForEachTargetPlayers(ChoiceCount),
    ForEachOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Add,
    Move,
    Deal,
    Draw,
    Counter,
    Destroy,
    Exile,
    Untap,
    Unlock,
    Scry,
    Discard,
    Transform,
    Convert,
    Flip,
    Roll,
    Regenerate,
    Heal,
    Mill,
    Get,
    Reveal,
    Look,
    Lose,
    Gain,
    Put,
    Sacrifice,
    Create,
    Investigate,
    Proliferate,
    Tap,
    Attach,
    Unattach,
    Remove,
    Return,
    Exchange,
    Become,
    Switch,
    Skip,
    Surveil,
    Incubate,
    Shuffle,
    Reorder,
    Reverse,
    Pay,
    Take,
    Detain,
    Assign,
    Goad,
    Suspect,
    Note,
    End,
}

#[path = "chain_carry/chain_carry_zone.rs"]
mod chain_carry_zone_programs;
pub use chain_carry_zone_programs::{
    parse_return_it_then_loses_all_abilities_lexed, remove_first_word, remove_through_first_word,
};
#[path = "chain_carry/chain_carry_reference.rs"]
mod chain_carry_reference_programs;
pub use chain_carry_reference_programs::{
    bind_implicit_player_context, collapse_for_each_object_it_tag_followups,
    collapse_for_each_player_it_tag_followups, dedupe_shared_target_player_draw_lose_x,
    effect_uses_implicit_player, explicit_player_for_carry, maybe_apply_carried_player,
    maybe_apply_carried_player_with_clause, maybe_apply_carried_player_with_clause_lexed,
    normalize_source_references_with_context, parse_effect_chain_with_subject_verb_primitives,
    parse_leading_player_may_lexed, parse_may_have_any_number_tagged_phase_out_lexed,
    player_ast_from_filter_for_carry, player_owner_filter_from_target_for_carry,
    release_offered_leading_discard_actor, target_is_generic_token_filter,
};
use chain_carry_reference_programs::{
    bind_it_metric_to_explicit_target, bind_source_exiled_effect,
    bind_trailing_it_predicate_to_explicit_effect_target, effect_uses_that_player,
    explicit_effect_object_tag, explicit_effect_object_target, explicit_tagged_target,
    maybe_apply_carried_player_with_clause_facts, normalize_imperative_create_player,
    parse_leading_player_may_words, player_target_carry_context, subject_verb_player_action_player,
    subject_verb_player_action_player_mut, target_ast_is_source,
};
#[path = "chain_carry/chain_carry_combat.rs"]
mod chain_carry_combat_programs;
use chain_carry_combat_programs::{
    append_shared_damage_player_operand, source_damage_then_gain_ability_actions,
};
pub use chain_carry_combat_programs::{
    collapse_token_copy_end_of_combat_exile_followup,
    collapse_token_copy_end_of_combat_exile_followup_lexed,
};
#[path = "chain_carry/chain_carry_object_action.rs"]
mod chain_carry_object_action_programs;
use chain_carry_object_action_programs::parse_tap_those_then_unattach_equipment_lexed;
pub use chain_carry_object_action_programs::{
    collapse_token_copy_next_end_step_exile_followup,
    collapse_token_copy_next_end_step_exile_followup_lexed,
    expand_segments_with_multi_create_clauses_lexed,
};
#[path = "chain_carry/chain_carry_condition.rs"]
mod chain_carry_condition_programs;
use chain_carry_condition_programs::{parse_carried_cant_effects, trailing_if_predicate_supported};
pub use chain_carry_condition_programs::{
    parse_effect_clause_with_trailing_if, parse_effect_clause_with_trailing_if_lexed,
};
#[path = "chain_carry/chain_carry_core.rs"]
mod chain_carry_core_programs;
use chain_carry_core_programs::{
    apply_carried_effect_duration, is_orphan_rounded_up_where_x_tail,
    is_standalone_where_x_binding_segment, split_on_comma_or_semicolon_lexed,
};
pub use chain_carry_core_programs::{
    expand_missing_verb_segment_lexed, expand_segments_with_comma_action_clauses_lexed, find_verb,
    parse_effect_chain, parse_effect_chain_inner, parse_effect_chain_lexed_with_context,
};
#[path = "chain_carry/chain_carry_choice.rs"]
mod chain_carry_choice_programs;
use chain_carry_choice_programs::{
    explicit_target_choose_spec, normalize_imperative_choose_player,
    parse_player_chooses_source_excluded_permanent_then_exiles,
};
#[path = "chain_carry/chain_carry_resource.rs"]
mod chain_carry_resource_programs;
use chain_carry_resource_programs::{
    bind_adjacent_life_stat_pronouns, effect_uses_half_life_total_value, value_is_half_life_total,
};
pub use chain_carry_resource_programs::{
    bind_adjacent_shared_x_life_stat_values,
    collapse_token_copy_next_end_step_sacrifice_followup_lexed,
};
#[path = "chain_carry/chain_carry_library.rs"]
mod chain_carry_library_programs;
use chain_carry_library_programs::{
    bind_adjacent_discard_count_draws, bind_adjacent_implicit_draw_discard_subjects,
    for_each_revealed_this_way_filter, is_revealed_this_way_scalar_reward,
    sentence_helper_revealed_tag,
};
#[path = "chain_carry/chain_carry_ability.rs"]
mod chain_carry_ability_programs;
use chain_carry_ability_programs::effect_duration_for_gain_followup_carry;

/// "It can't be regenerated." / "They can't be regenerated." after a destroy
/// sentence: the pronoun is the destroyed objects, so the rider sets the
/// destroy's no-regeneration flag rather than standing as an effect of its
/// own. A singular pronoun binds only a single-target destroy. Returns false
/// when the sentence is not this rider or nothing destroys before it.
pub fn bind_no_regeneration_rider(
    effects: &mut [EffectAst],
    sentence: &[crate::lexer::OwnedLexToken],
) -> bool {
    let words = crate::lexer::token_word_refs(sentence);
    if !crate::word_primitives::last_is(&words, "regenerated")
        || !crate::word_primitives::first_is_any(&words, &["it", "they", "those", "that"])
        || !crate::slice_primitives::contains_any(&words, &["cant", "can't"])
    {
        return false;
    }
    // "That creature can't be regenerated." (Parallax Dementia) is singular.
    let singular = crate::word_primitives::first_is_any(&words, &["it", "that"]);
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. })) = effects.last_mut()
    else {
        return false;
    };
    match action {
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
            no_regeneration, ..
        }) => {
            *no_regeneration = true;
            true
        }
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll {
            no_regeneration, ..
        })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor {
            no_regeneration,
            ..
        }) if !singular => {
            *no_regeneration = true;
            true
        }
        _ => false,
    }
}

/// Whether "put a +0/+1 counter on it for each 1 damage prevented this way at
/// the beginning of the next end step" follows "if it's a creature".
fn is_delayed_creature_counters_followup(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    let expected_suffix = [
        "put",
        "a",
        "+0/+1",
        "counter",
        "on",
        "it",
        "for",
        "each",
        "1",
        "damage",
        "prevented",
        "this",
        "way",
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "end",
        "step",
    ];
    let prefix_len = if crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[
            &["if", "its", "a", "creature"],
            &["if", "it's", "a", "creature"],
        ],
    ) {
        4
    } else if crate::word_primitives::parse_sequence_prefix(
        &words,
        &["if", "it", "is", "a", "creature"],
    ) {
        5
    } else {
        return false;
    };
    crate::word_primitives::parse_sequence_complete(&words[prefix_len..], &expected_suffix)
}

/// "If damage from a <quality> source is prevented this way, you gain that
/// much life." — a prevention rider conditioned on the prevented damage's
/// source. Only the life-gain body is accepted: its amount is the prevented
/// event's amount.
/// The quality named by "damage from a <quality> source" in a prevention
/// rider ("a black source", "an artifact source").
fn prevention_source_quality_filter(quality_tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let words = token_word_refs(quality_tokens);
    if words.is_empty() || words.len() != quality_tokens.len() {
        return None;
    }
    // A bare color ("a black source") names only the source's color; don't
    // let the object-filter parser add a permanent noun it never saw.
    let color_only = words
        .iter()
        .map(|word| crate::util::parse_color(word))
        .collect::<Option<Vec<_>>>();
    let filter = if let Some(colors) = color_only {
        let mut filter = ObjectFilter::default();
        filter.colors = colors.into_iter().reduce(|left, right| left.union(right));
        filter
    } else {
        parse_object_filter(quality_tokens, false).ok()?
    };
    if filter.colors.is_none() && filter.card_types.is_empty() && filter.subtypes.is_empty() {
        return None;
    }
    Some(filter)
}

fn parse_colored_source_prevention_followup(sentence: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = token_word_refs(sentence);
    let ["if", "damage", "from", "a" | "an", rest @ ..] = words.as_slice() else {
        return None;
    };
    let source_idx = rest.iter().position(|word| *word == "source")?;
    if source_idx == 0
        || rest.get(source_idx + 1..source_idx + 5) != Some(&["is", "prevented", "this", "way"][..])
        || rest[source_idx + 5..] != ["you", "gain", "that", "much", "life"]
    {
        return None;
    }
    // The leading words carry no punctuation, so they align with tokens.
    let quality_tokens = sentence.get(4..4 + source_idx)?;
    if token_word_refs(quality_tokens).as_slice() != &rest[..source_idx] {
        return None;
    }
    let filter = prevention_source_quality_filter(quality_tokens)?;
    Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(
            crate::tag::CompilerReferenceTag::Triggering.bind(),
            filter,
        ),
        if_true: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                amount: Value::EventValue(crate::effect::EventValueSpec::Amount),
            }),
        )],
        if_false: Vec::new(),
    }))
}

/// Bind a sentence that says what happens to damage the preceding prevention
/// shield prevents: "You gain life equal to the damage prevented this way.",
/// "Exile cards from the top of your library equal to the damage prevented
/// this way.", "For each 1 damage prevented this way, put a +1/+1 counter on
/// that creature.", "If damage is prevented this way, ~ deals that much damage
/// to any target.", and the creature-only counters at the next end step. The
/// sentence names the prevention event's amount, which only the shield knows;
/// parsed on its own it loses that context, so it is a rider on the shield.
/// Returns false when the last effect is not such a shield or the sentence is
/// none of these.
/// "If <condition>, this effect doesn't affect combat damage that would be
/// dealt by <colored creatures>." (Undergrowth): when the condition holds the
/// Fog prevents only combat damage from creatures outside that color
/// (CR 615.1); otherwise it prevents all combat damage.
fn conditional_combat_prevention_exception(
    last: &EffectAst,
    sentence: &[OwnedLexToken],
) -> Option<EffectAst> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamage {
                duration,
            }),
        ..
    }) = last
    else {
        return None;
    };
    let clean = crate::util::trim_edge_punctuation(sentence);
    let (_, rest) = grammar::parse_prefix(&clean, grammar::kw("if"))?;
    let (comma_idx, _, after_comma) = grammar::find_prefix(rest, grammar::comma)?;
    let condition_tokens = &rest[..comma_idx];
    const EXCEPTION_HEADS: [&[&str]; 2] = [
        &["this", "effect", "doesn't", "affect", "combat", "damage", "that", "would", "be", "dealt", "by"],
        &["this", "effect", "doesnt", "affect", "combat", "damage", "that", "would", "be", "dealt", "by"],
    ];
    let source_tokens = EXCEPTION_HEADS.iter().find_map(|head| {
        grammar::parse_prefix(after_comma, grammar::phrase(*head)).map(|((), tail)| tail)
    })?;
    let condition = parse_predicate_with_grammar_entrypoint_lexed(condition_tokens).ok()?;
    let excepted = parse_object_filter(source_tokens, false).ok()?;
    let colors = excepted.colors?;
    if excepted.card_types != [crate::types::CardType::Creature] {
        return None;
    }
    let mut prevented = excepted.clone();
    prevented.colors = None;
    prevented.excluded_colors = colors;
    Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: condition,
        if_true: vec![EffectAst::subject_verb_prevent_all_combat_damage_from_source_filter(
            prevented,
            duration.clone(),
        )],
        if_false: vec![last.clone()],
    }))
}

pub fn bind_prevention_followup(effects: &mut Vec<EffectAst>, sentence: &[OwnedLexToken]) -> bool {
    use crate::grammar::effects::generic_sequence_shapes as sequence_grammar;
    if let Some(last) = effects.last()
        && let Some(replacement) = conditional_combat_prevention_exception(last, sentence)
    {
        *effects.last_mut().expect("checked") = replacement;
        return true;
    }
    // "If this spell was kicked, prevent the next 6 damage this way instead."
    // (Pollen Remedy): the divided total depends on the announced kicker.
    if let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage {
                amount,
                divided: true,
                ..
            }),
        ..
    })) = effects.last_mut()
        && let Value::Fixed(base) = *amount.unhinted()
        && let Some(kicked) = super::prevention_source_riders::parse_kicked_amount_override(sentence)
    {
        *amount = Value::Add(
            Box::new(Value::Fixed(base)),
            Box::new(Value::Scaled(Box::new(Value::WasKicked), kicked - base)),
        );
        return true;
    }
    // CR 615.5: the additional part reads the prevented damage's source
    // (Channel Harm, Comeuppance, Judgment of Alexander, Samite Ministration).
    if let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. })) = effects.last_mut() {
        let slot = match action {
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                    of_chosen_color: false,
                    follow_up_effects,
                    ..
                },
            ) => Some(follow_up_effects),
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTarget {
                    source_of_your_choice: true,
                    source_choice_shares_activation_mana_color: false,
                    source_target: None,
                    target: TargetAst::Player(crate::target::PlayerFilter::You, _),
                    follow_up_effects,
                    ..
                },
            ) => Some(follow_up_effects),
            _ => None,
        };
        if let Some(slot) = slot
            && let Ok(Some(rider)) = super::prevention_source_riders::parse(sentence)
        {
            match rider {
                super::prevention_source_riders::PreventionRider::Inline(rider) => {
                    slot.push(rider);
                }
                // The delayed trigger is registered right after the shield it
                // links to (CR 603.7).
                super::prevention_source_riders::PreventionRider::Delayed(delayed) => {
                    effects.push(delayed);
                }
            }
            return true;
        }
    }
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. })) = effects.last_mut()
    else {
        return false;
    };
    match action {
        SubjectVerbActionAst::DamagePrevention(
            DamagePreventionActionAst::PreventAllDamageToTarget {
                source_target: Some(_),
                follow_up_effects,
                ..
            },
        ) if follow_up_effects.is_empty()
            && sequence_grammar::parse_prevention_gain_life_followup_shape(sentence) =>
        {
            follow_up_effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                    amount: Value::EventValue(crate::effect::EventValueSpec::Amount),
                }),
            ));
            true
        }
        SubjectVerbActionAst::DamagePrevention(
            DamagePreventionActionAst::PreventAllDamageFromSourceFilter {
                of_chosen_color: false,
                follow_up_effects,
                ..
            },
        ) if follow_up_effects.is_empty()
            && sequence_grammar::parse_prevention_gain_life_followup_shape(sentence) =>
        {
            follow_up_effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                    amount: Value::EventValue(crate::effect::EventValueSpec::Amount),
                }),
            ));
            true
        }

        SubjectVerbActionAst::DamagePrevention(
            DamagePreventionActionAst::PreventNextTimeDamage {
                source,
                reflect_damage_to_source_controller,
                reflect_source_filter,
                follow_up_effects,
                ..
            },
        ) if follow_up_effects.is_empty() => {
            // "If/When damage is prevented this way, ~ deals that much damage
            // to that source's controller": the source is the one this shield
            // chose, which only the shield knows when the damage is prevented.
            // "If damage from a red source is prevented this way, ..."
            // (Honorable Passage) gates the reflection on the source's
            // quality when the damage is prevented.
            if !*reflect_damage_to_source_controller
                && !matches!(
                    source,
                    crate::cards::builders::PreventNextTimeDamageSourceAst::Filter(_)
                )
                && let Some((source_quality, tail)) =
                    sequence_grammar::parse_prevention_source_controller_reflect_followup_shape(
                        sentence,
                    )
                && let Some(quality_filter) = match source_quality {
                    None => Some(None),
                    Some(quality_tokens) => {
                        prevention_source_quality_filter(quality_tokens).map(Some)
                    }
                }
            {
                *reflect_damage_to_source_controller = true;
                *reflect_source_filter = quality_filter;
                let amount = Value::EventValue(crate::effect::EventValueSpec::Amount);
                match tail {
                    sequence_grammar::PreventionSourceControllerReflectTail::None => {}
                    sequence_grammar::PreventionSourceControllerReflectTail::DrawThatMany => {
                        follow_up_effects.push(EffectAst::subject_verb(
                            SubjectVerbRoleAst::AffectedPlayer,
                            PlayerAst::You,
                            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                                count: amount,
                            }),
                        ));
                    }
                    sequence_grammar::PreventionSourceControllerReflectTail::GainThatMuchLife => {
                        follow_up_effects.push(EffectAst::subject_verb(
                            SubjectVerbRoleAst::AffectedPlayer,
                            PlayerAst::You,
                            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                                amount,
                            }),
                        ));
                    }
                }
                return true;
            }
            if sequence_grammar::parse_prevention_gain_life_followup_shape(sentence) {
                follow_up_effects.push(EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                        amount: Value::EventValue(crate::effect::EventValueSpec::Amount),
                    }),
                ));
                return true;
            }
            if sequence_grammar::parse_prevention_exile_top_followup_shape(sentence) {
                follow_up_effects.push(EffectAst::subject_verb_exile_top_of_library(
                    PlayerAst::You,
                    Value::EventValue(crate::effect::EventValueSpec::Amount),
                    Vec::new(),
                    Vec::new(),
                ));
                return true;
            }
            // "If damage from a black source is prevented this way, you gain
            // that much life." (Shadowbane): the rider runs as the shield
            // prevents the damage, gated on the damage source's quality at
            // that time. The prevented damage event's object is its source.
            if let Some(follow_up) = parse_colored_source_prevention_followup(sentence) {
                follow_up_effects.push(follow_up);
                return true;
            }
            false
        }
        SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage {
            amount,
            target,
            duration,
            combat_only,
            follow_up_effects,
            ..
        }) => {
            if follow_up_effects.is_empty()
                && sequence_grammar::parse_prevention_gain_life_followup_shape(sentence)
            {
                follow_up_effects.push(EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                        amount: Value::EventValue(crate::effect::EventValueSpec::Amount),
                    }),
                ));
                return true;
            }
            if !*combat_only && sequence_grammar::parse_prevention_counter_followup_shape(sentence) {
                let replacement = EffectAst::subject_verb_prevent_damage_to_target_put_counters(
                    Some(amount.clone()),
                    target.clone(),
                    duration.clone(),
                    crate::object::CounterType::PlusOnePlusOne,
                );
                *effects.last_mut().expect("checked") = replacement;
                return true;
            }
            if sequence_grammar::parse_prevention_reflect_followup_shape(sentence) {
                let follow_up = EffectAst::subject_verb_damage(
                    Value::EventValue(crate::effect::EventValueSpec::Amount),
                    TargetAst::AnyTarget(None),
                );
                follow_up_effects.push(follow_up);
                return true;
            }
            if matches!(target, TargetAst::AnyTarget(Some(_)))
                && is_delayed_creature_counters_followup(sentence)
            {
                let put = EffectAst::subject_verb_put_counters(
                    crate::object::CounterType::PlusZeroPlusOne,
                    Value::PendingPriorEffectMetric(
                        ironsmith_core::PriorEffectMetricQuery::new(
                            ironsmith_core::EffectMetricSource::Outcome,
                            ironsmith_core::EffectMetric::DamagePrevented,
                        )
                        .with_action(ironsmith_core::PriorEffectAction::Prevented),
                    ),
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::Targeted0.bind(), None),
                    None,
                    false,
                );
                effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: PredicateAst::TargetMatches(ObjectFilter::creature()),
                    if_true: vec![EffectAst::Delayed(
                        DelayedEffectAst::DelayedUntilNextEndStep {
                            player: crate::target::PlayerFilter::Any,
                            effects: vec![put],
                        },
                    )],
                    if_false: Vec::new(),
                }));
                return true;
            }
            false
        }
        SubjectVerbActionAst::DamagePrevention(
            DamagePreventionActionAst::PreventAllCombatDamageToYou {
                follow_up_effects, ..
            },
        ) if follow_up_effects.is_empty() => {
            // "For each 1 damage prevented this way, create ..." (Inkshield):
            // one created token per prevented damage, as it is prevented.
            let words = crate::lexer::token_word_refs(sentence);
            let prefix = ["for", "each", "1", "damage", "prevented", "this", "way"];
            if !crate::word_primitives::parse_sequence_prefix(&words, &prefix) {
                return false;
            }
            let Some(body_start) = sentence
                .iter()
                .position(|token| token.is_word("way"))
                .map(|idx| idx + 1)
            else {
                return false;
            };
            let body = crate::util::trim_edge_punctuation(&sentence[body_start..]);
            let Ok(mut body_effects) = super::parse_effect_sentence_lexed(&body) else {
                return false;
            };
            let [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                            count, ..
                        }),
                    ..
                }),
            ] = body_effects.as_mut_slice()
            else {
                return false;
            };
            if !matches!(count, Value::Fixed(1)) {
                return false;
            }
            *count = Value::EventValue(crate::effect::EventValueSpec::Amount);
            *follow_up_effects = body_effects;
            true
        }
        SubjectVerbActionAst::DamagePrevention(
            DamagePreventionActionAst::PreventAllDamageToTarget {
                target,
                duration,
                combat_only,
                source_target: None,
                protect_source_target: false,
                ..
            },
        ) => {
            if !*combat_only && sequence_grammar::parse_prevention_counter_followup_shape(sentence)
            {
                let replacement = EffectAst::subject_verb_prevent_damage_to_target_put_counters(
                    None,
                    target.clone(),
                    duration.clone(),
                    crate::object::CounterType::PlusOnePlusOne,
                );
                *effects.last_mut().expect("checked") = replacement;
                return true;
            }
            false
        }
        _ => false,
    }
}

/// "Each other artifact doesn't untap during its controller's untap step for
/// as long as this artifact remains tapped." after "Tap all other artifacts.":
/// the lock is bound to the tapped set.
pub fn bind_tap_lock(effects: &mut Vec<EffectAst>, sentence: &[OwnedLexToken]) -> bool {
    use crate::grammar::effects::generic_sequence_shapes as sequence_grammar;
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { filter }),
        ..
    })) = effects.last()
    else {
        return false;
    };
    if !sequence_grammar::parse_source_tapped_lock_shape(sentence) {
        return false;
    }
    let Some(Some((duration, clause_tokens))) =
        crate::grammar::primitives::probe_shape(super::parse_restriction_duration(sentence))
    else {
        return false;
    };
    if !sequence_grammar::parse_untap_clause_prefix_shape(&clause_tokens) {
        return false;
    }
    let filter = filter.clone();
    effects.push(EffectAst::subject_verb_cant(
        crate::effect::Restriction::untap(filter),
        duration,
        Some(PredicateAst::Source(SourcePredicateAst::SourceIsTapped)),
    ));
    true
}

/// "Then if this enchantment isn't a creature, this enchantment becomes a 3/3
/// Angel creature ..." after a life-gain sentence: the animation is of the
/// source, bound after the gain.
pub fn bind_self_animate_after_life_gain(
    effects: &mut Vec<EffectAst>,
    sentence: &[OwnedLexToken],
) -> bool {
    use super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::{
        contains_triggered_life_gain_effect, parse_self_animate_followup_effects,
        retarget_source_self_animate_effect,
    };
    if !effects.iter().any(contains_triggered_life_gain_effect) {
        return false;
    }
    let Some(Some(followups)) =
        crate::grammar::primitives::probe_shape(parse_self_animate_followup_effects(sentence))
    else {
        return false;
    };
    effects.extend(
        followups
            .into_iter()
            .map(retarget_source_self_animate_effect),
    );
    true
}

/// "Destroy any of them that are Walls." after "Up to three target creatures
/// can't block this turn.": the destroy bound to the restricted target set.
pub fn bind_destroy_typed_subset(effects: &mut Vec<EffectAst>, sentence: &[OwnedLexToken]) -> bool {
    use super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::{
        counted_target_object_filter, tagged_subset_destroy_words,
    };
    if !tagged_subset_destroy_words(sentence) {
        return false;
    }
    let len = effects.len();
    if len < 2 {
        return false;
    }
    let Some((target_effect, cant_effect)) = effects[len - 2..]
        .split_first_mut()
        .and_then(|(first, rest)| rest.first_mut().map(|second| (first, second)))
    else {
        return false;
    };
    let target_filter = match &*target_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    target,
                    explicit_declaration: false,
                },
            ..
        }) => counted_target_object_filter(target).cloned(),
        _ => None,
    };
    let Some(target_filter) = target_filter else {
        return false;
    };
    let restriction_filter = match cant_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::Block(filter),
                    duration: crate::effect::Until::EndOfTurn,
                    start: crate::effect::RestrictionStart::Immediate,
                    duration_surface: crate::effect::RestrictionDurationSurface::Default,
                    condition: None,
                },
            ..
        }) => filter,
        _ => return false,
    };
    let expected_it_constraint = crate::target::TaggedObjectConstraint {
        tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    };
    if !matches!(restriction_filter.tagged_constraints.as_slice(), [constraint] if *constraint == expected_it_constraint)
    {
        return false;
    }
    let mut restriction_base = restriction_filter.clone();
    restriction_base.tagged_constraints.clear();
    if restriction_base != target_filter {
        return false;
    }
    let Some(are_index) =
        crate::slice_primitives::select_position(sentence, |token| token.is_word("are"))
    else {
        return false;
    };
    let Some(Some(mut destroy_filter)) = crate::grammar::primitives::probe_shape(
        parse_object_filter(&sentence[are_index + 1..], false),
    )
    .map(Some) else {
        return false;
    };
    if !destroy_filter.tagged_constraints.is_empty() {
        return false;
    }
    let target_set_tag = crate::util::helper_tag_for_tokens(sentence, "restricted_target_set");
    restriction_filter.tagged_constraints[0].tag = target_set_tag.clone().into();
    destroy_filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: target_set_tag.clone().into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let original_target = target_effect.clone();
    *target_effect = EffectAst::TagAffected {
        effect: Box::new(original_target),
        tag: crate::tag::TagRef::of(target_set_tag),
    };
    effects.push(EffectAst::subject_verb_destroy(TargetAst::Object(
        destroy_filter,
        None,
        None,
    )));
    true
}

/// "Then put all cards exiled this way into their owners' hands." after an
/// exile (with any sentences between): the exiled cards, tagged on the exile,
/// returned.
pub fn bind_return_exiled_to_owners_hands(
    effects: &mut Vec<EffectAst>,
    sentence: &[OwnedLexToken],
) -> bool {
    use super::sequence_rules::generic_subject_verb_sequences::exiled_collections::{
        contains_word_phrase, has_owner_hands_destination, tag_first_exile_in_effects,
    };
    if !contains_word_phrase(sentence, &["all", "cards", "exiled", "this", "way"])
        || !has_owner_hands_destination(sentence)
    {
        return false;
    }
    let exiled_tag = crate::util::helper_tag_for_tokens(sentence, "exiled");
    if !tag_first_exile_in_effects(effects, &exiled_tag) {
        return false;
    }
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(exiled_tag), None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    true
}

/// Capture the objects that supply a counted draw before executing it, so a
/// later "those creatures" grant refers to that collection, even if a draw
/// replacement changes the battlefield in between.
pub fn bind_counted_object_grant_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if words.get(..2) != Some(&["those", "creatures"]) {
        return false;
    }
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::LifeResources(crate::cards::builders::LifeResourceActionAst::Draw {
                count,
            }),
        ..
    })) = effects.last()
    else {
        return false;
    };
    let Value::Count(filter) = count.unhinted() else {
        return false;
    };
    if !filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        return false;
    }
    let filter = filter.clone();
    let Ok(Some(mut followup)) = super::gain_ability::parse_gain_ability_sentence(tokens) else {
        return false;
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Grants(
                    crate::cards::builders::GrantActionAst::GrantAbilitiesToTarget {
                        target,
                        set_quantifier_surface: Some(ironsmith_core::SetQuantifierSurface::Those),
                        ..
                    },
                ),
            ..
        }),
    ] = followup.as_mut_slice()
    else {
        return false;
    };
    let tag = crate::util::helper_tag_for_tokens(tokens, "counted_objects");
    let mut referent = ObjectFilter::tagged(tag.clone());
    referent.source_surface = Some(crate::target::SourceReferenceSurface::ThisPermanentType(
        "those creatures".to_owned(),
    ));
    *target = TargetAst::Object(referent, None, span_from_tokens(tokens));
    effects.insert(
        effects.len() - 1,
        EffectAst::subject_verb_tag_matching_objects(
            filter,
            Vec::new(),
            crate::tag::TagRef::of(tag),
        ),
    );
    effects.extend(followup);
    true
}

/// A plural pronoun keeps the population affected by the preceding mass action.
pub fn bind_population_counter_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if words.first() != Some(&"put") || !words.windows(4).any(|w| w == ["on", "each", "of", "them"])
    {
        return false;
    }
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. })) = effects.last() else {
        return false;
    };
    let population = match action {
        SubjectVerbActionAst::StatChanges(
            crate::cards::builders::StatChangeActionAst::PumpAll { filter, .. },
        )
        | SubjectVerbActionAst::Grants(
            crate::cards::builders::GrantActionAst::GrantAbilitiesAll { filter, .. },
        ) => filter.clone(),
        _ => return false,
    };
    let Ok(mut followup) = super::zone_counter_helpers::parse_put_counters(tokens) else {
        return false;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Counters(crate::cards::builders::CounterActionAst::PutCountersAll {
                filter,
                ..
            }),
        ..
    }) = &mut followup
    else {
        return false;
    };
    let tag = crate::util::helper_tag_for_tokens(tokens, "affected_population");
    *filter = ObjectFilter::tagged(tag.clone());
    filter.source_surface = Some(crate::target::SourceReferenceSurface::ThisPermanentType(
        "them".to_owned(),
    ));
    effects.insert(
        effects.len() - 1,
        EffectAst::subject_verb_tag_matching_objects(
            population,
            vec![Zone::Battlefield],
            crate::tag::TagRef::of(tag),
        ),
    );
    effects.push(followup);
    true
}

/// Bind a same-clause characteristic pronoun before global reference fallback.
pub(super) fn bind_it_metric_to_declared_target(value: Value, target: &TargetAst) -> Value {
    let spec = match target {
        TargetAst::Spell(_) => Some(ChooseSpec::Target(Box::new(ChooseSpec::Object(
            ObjectFilter::spell(),
        )))),
        _ => explicit_target_choose_spec(target),
    };
    match spec {
        Some(spec) => bind_it_metric_to_explicit_target(value, &spec),
        None => value,
    }
}

/// The object subject of a single-target ability grant or removal.
fn shared_ability_modifier_target(effect: &EffectAst) -> Option<TargetAst> {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return None;
    };
    match &subject_verb.action {
        SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
            target,
            ..
        })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { target, .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
            target,
            ..
        }) => Some(target.clone()),
        _ => None,
    }
}

fn set_ability_modifier_target(effect: &mut EffectAst, subject: TargetAst) {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return;
    };
    if let SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
        target,
        ..
    })
    | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { target, .. })
    | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
        target,
        ..
    }) = &mut subject_verb.action
    {
        *target = subject;
    }
}

mod extremum_choice;
pub(crate) use extremum_choice::bind as bind_extremum_choice_followup;
