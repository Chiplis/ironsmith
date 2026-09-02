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
    CardTextError, EffectAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan,
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
            action: SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayLife { .. },
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
                EffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::Implicit,
                    tag: crate::tag::CompilerReferenceTag::It.key(),
                }
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
        chain_grammar::RestActionShape::Exile => EffectAst::subject_verb_exile_all(filter, false),
        chain_grammar::RestActionShape::Sacrifice => {
            EffectAst::subject_verb_sacrifice_all(player, filter)
        }
    }
}

fn try_apply_rest_action_followup(
    effects: &mut Vec<EffectAst>,
    action: chain_grammar::RestActionShape,
) -> bool {
    if let Some(EffectAst::ChooseObjects {
        filter,
        tag,
        player,
        ..
    }) = effects.last()
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
        EffectAst::ForEachPlayer {
            effects: inner_effects,
        }
        | EffectAst::ForEachOpponent {
            effects: inner_effects,
        } => {
            let Some(EffectAst::ChooseObjects {
                filter,
                tag,
                player,
                ..
            }) = inner_effects.last()
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
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.key())
            .in_zone(Zone::Exile);
    source_exiled.owner = Some(PlayerFilter::IteratedPlayer);
    source_exiled.source_surface = Some(SourceReferenceSurface::ThisPermanentType(
        source_surface.to_string(),
    ));
    let reveal = EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::Actor,
        PlayerAst::That,
        SubjectVerbActionAst::TurnFaceUp {
            target: TargetAst::Object(source_exiled.clone(), None, None),
        },
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
    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![reveal, put_onto_battlefield],
    }])
}

pub fn parse_effect_chain_lexed(tokens: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    // Chain parsing recursively re-enters the sentence dispatcher for
    // nested clauses and quoted/conditional payloads.  The public chain
    if let Some(effects) = super::parse_complete_create_statement(tokens)? {
        return Ok(effects);
    }
    parse_effect_chain_lexed_inner(tokens)
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
            action: SubjectVerbActionAst::CreateTokenWithMods {
                dynamic_power_toughness: Some(_),
                ..
            },
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
            action: SubjectVerbActionAst::CreateTokenCopy { .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
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

    let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key());
    filter.card_types.push(CardType::Creature);
    filter.set_explicit_card_type_noun(Some(CardType::Creature));
    Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst {
            role: SubjectVerbRoleAst::Actor,
            player: PlayerAst::You,
        },
        action: SubjectVerbActionAst::MoveToZone {
            target: TargetAst::Object(filter, None, None),
            source_top_only: false,
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
        },
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
        || !segments
            .iter()
            .all(|segment| parse_leading_player_may_lexed(segment).is_some())
    {
        return Ok(None);
    }

    // Repeating the complete "<player> may" subject after the conjunction
    // starts a new choice. Parsing the whole line through the broad leading-
    // may path would instead wrap every later choice inside the first May,
    // so declining the first action would incorrectly suppress the rest.
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

fn parse_effect_chain_lexed_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(stripped) = grammar::strip_lexed_prefix_phrase(tokens, &["you", "may"]) {
        let player = PlayerAst::You;
        let stripped = crate::util::trim_edge_punctuation_tokens(stripped);
        let placement_tokens =
            grammar::strip_lexed_suffix_phrase(stripped, &["instead"]).unwrap_or(stripped);
        if let Some(effect) = parse_simple_that_creature_owner_library_placement(placement_tokens) {
            return Ok(vec![EffectAst::MayByPlayer {
                player,
                effects: vec![effect],
            }]);
        }
    }

    // Action-first delayed-step sentences expose an ordinary resource verb
    // before their schedule. Claim the complete typed schedule/payment shape
    // before broad resource dispatch tries to consume the timing suffix as
    // part of the life-loss operand.
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_delayed_next_step_unless_pays(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }
    if parse_leading_player_may_lexed(tokens).is_none()
        && let Some(effects) =
            super::subject_verb_primitives::parse_sentence_shuffle_object_into_library(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
    {
        return Ok(effects);
    }

    // A joint object subject owns every conjunction in its shared action
    // tail (`this creature and that creature each get ... and gain ...`).
    // Claim the grammar-proven subject before generic chain splitting can
    // mistake the subject's first `and` for an effect boundary.
    if let Some(effects) =
        super::subject_verb_primitives::parse_source_and_tagged_object_each_actions_sentence(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }

    if let Some(effects) = parse_repeated_counter_placement_coordination(tokens)? {
        return Ok(effects);
    }

    // A dynamic counter amount can itself be a coordinated object domain:
    // `for each suspended card ... and each other permanent ...`. Once the
    // count grammar consumes that entire suffix, its conjunction is data,
    // not an effect boundary. Materialize the one typed counter action before
    // generic sentence coordination can expose either count arm as a target.
    if is_atomic_put_counter_for_each_sentence(tokens) {
        return Ok(vec![super::zone_counter_helpers::parse_put_counters(
            tokens,
        )?]);
    }

    // A copy-token exception is part of one creation action even when its
    // characteristic bundle contains `and` (`except it's 1/1 and it's a
    // Nightmare ...`). Let the typed creation grammar prove and materialize
    // the complete shape before effect coordination can expose a modifier as
    // a second create clause.
    if let Some(effect) = parse_atomic_token_copy_exception(tokens)? {
        return Ok(vec![effect]);
    }

    // A target-player subject can govern several coordinated resource
    // actions (`loses life, gets a poison counter, then mills`). The typed
    // coordination recognizer proves the multi-member shape; route it to the
    // chain materializer before whole-sentence primitive registries can treat
    // the leading `target` as an object-selection verb and try to parse the
    // remaining player action as an object filter.
    if has_target_player_resource_coordination(tokens) {
        return parse_effect_chain_inner_lexed(tokens);
    }

    // A conditional entry-counter list is one atomic subject/verb sentence:
    // every `and an additional ... if it's ...` arm is a sibling action on
    // the same returned set. Claim it before generic conjunction splitting,
    // which otherwise treats the second counter descriptor as a continuation
    // of the first condition and nests the two predicates.
    if let Some(effects) =
        super::subject_verb_primitives::parse_tagged_conditional_entry_counters_sentence(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }
    // A distributed-target range may contain authored commas of its own
    // (`among one, two, or three target creatures`).  Claim the complete
    // typed sentence before generic comma-chain splitting can mistake those
    // list separators for executable boundaries and truncate the target
    // phrase at `one`.
    if let Some(effects) = super::subject_verb_primitives::parse_sentence_distribute_counters(
        SubjectVerbPrimitiveClause::new(tokens),
    )? {
        return Ok(effects);
    }
    let venture_view = TokenWordView::new(tokens);
    if let Some(venture_word) =
        venture_view.parse_phrase_start(&["and", "venture", "into", "the", "dungeon"])
        && let Some(venture_start) = venture_view.map_word_to_token_start(venture_word)
        && let Some(venture_end) = venture_view.token_index_after_words(venture_word + 5)
        && tokens[venture_end..]
            .iter()
            .all(|token| token.as_word().is_none())
    {
        let mut effects = parse_effect_chain_lexed(&tokens[..venture_start])?;
        effects.push(EffectAst::subject_verb_venture_into_dungeon(
            PlayerAst::You,
            false,
        ));
        let coordination = crate::grammar::effects::coordination::coordination_from_effects(
            crate::model::CoordinationKindAst::Conjunction,
            crate::model::CoordinationOperatorAst::And,
            crate::model::EffectOrderingAst::Unordered,
            effects,
        )
        .expect("venture conjunction contains at least two effects");
        return Ok(vec![EffectAst::Coordination(coordination)]);
    }
    if let Some(effect) = parse_each_prior_affected_object_controller_mana_value_life(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some((leading_tokens, where_value)) = parse_terminal_where_x_binding(tokens) {
        let mut effects = parse_effect_chain_lexed(leading_tokens)?;
        replace_unbound_x_in_effects_anywhere(
            &mut effects,
            &where_value,
            &token_word_refs(tokens).join(" "),
        )?;
        ensure_explicit_target_player_subject_declarations(&mut effects, leading_tokens);
        dedupe_shared_target_player_draw_lose_x(&mut effects, tokens);
        preserve_independent_target_player_coordination(&mut effects, leading_tokens);
        return Ok(effects);
    }
    if let Some(effect) = super::clause_primitives::parse_until_duration_triggered_clause(tokens)? {
        // `Until ..., whenever ...` is one delayed-trigger clause. Keep that
        // typed outer scope intact before general conjunction/duration chain
        // recognition can expose words in the trigger event as direct effect
        // heads (for example, `deals combat damage` or `becomes tapped`).
        return Ok(vec![effect]);
    }
    if super::verb_handlers::damage_clause_has_terminal_unpreventable_rider(tokens) {
        // The final prevention rider belongs to the damage action. Preserve
        // object-or-player recipient unions before generic coordination can
        // split their `or` arm into a standalone restriction clause.
        let damage_tokens =
            super::lex_chain_helpers::strip_leading_instead_prefix_lexed(tokens).unwrap_or(tokens);
        return Ok(vec![parse_effect_clause_lexed(damage_tokens)?]);
    }
    // A spell-copy characteristic exception is one typed action even though
    // its authored comma resembles a coordination boundary. Claim only a
    // leading copy clause with a grammar-proven `except` tail so delayed
    // trigger bodies such as "copy it, except the copy isn't legendary"
    // cannot strand the exception as a verb-less second action.
    let copy_shape =
        super::super::grammar::effects::clause_pattern_shapes::parse_copy_clause_shape_tokens(
            tokens,
        );
    if copy_shape.is_some_and(|shape| shape.copy_word == 0 && shape.tail.exception_split.is_some())
        && let Some(copy) = super::clause_pattern_helpers::parse_copy_spell_clause(tokens)?
    {
        return Ok(vec![copy]);
    }
    // A hand/graveyard-into-library shuffle owns its complete clause. The
    // chain splitter would otherwise sever the coordinated zone list ("their
    // hand and graveyard into their library") and hand the shuffle verb a
    // bare "their hand" fragment.
    if let Some(effects) =
        super::search_library::parse_shuffle_graveyard_into_library_sentence(tokens)?
    {
        return Ok(effects);
    }
    // A tagged cast/play permission owns its complete sentence, including a
    // coordinated "and you may spend mana as though ..." payment suffix. The
    // chain splitter would otherwise sever that suffix into a verb-less arm
    // that no clause grammar accepts.
    {
        let words = crate::lexer::parser_token_word_refs(tokens);
        if crate::word_primitives::sequence_occurs(&words, &["for", "as", "long", "as"])
            && crate::word_primitives::sequence_occurs(
                &words,
                &["may", "spend", "mana", "as", "though"],
            )
            && let Some(permission) =
                crate::permission_helpers::parse_cast_or_play_tagged_clause(tokens)?
        {
            return Ok(vec![permission]);
        }
    }
    // A paid-label condition owns the complete consequence, including every
    // authored conjunction inside it. A consequence such as "phase out, and
    // until ..., can't change and gain protection" contains several
    // independently executable heads, but the typed leading condition makes
    // them members of one conditional consequence rather than top-level
    // coordination.
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

    let leading_duration_shape = chain_grammar::parse_carry_duration_prefix_tokens(tokens);
    let pair_tokens = leading_duration_shape
        .as_ref()
        .map_or(tokens, |shape| shape.rest);
    if let Some(mut effect) = super::clause_dispatch::parse_conditional_become_pair(pair_tokens)? {
        if let Some(shape) = leading_duration_shape {
            super::dispatch_entry::apply_leading_duration_to_become_effect(
                &mut effect,
                &shape.duration,
            );
        }
        return Ok(vec![effect]);
    }
    // Conditional consequence parsing can call the chain entrypoint directly
    // before the ordinary uncoordinated dispatcher gets a chance to inspect
    // the complete `for each ..., effect` shape. Keep that grammar atomic at
    // this boundary so the iterator's subject is not sent to the generic
    // subject/verb parser as an orphaned clause.
    if let Some(effects) = parse_for_each_object_effect_chain_shape(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_independent_explicit_may_coordination(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = super::fanout_family::parse_compound_damage_fanout_sentence(tokens)? {
        // Conditional consequence parsing enters through the chain boundary.
        // Preserve a repeated damage head as one typed fanout before the
        // generic conjunction splitter leaves the second amount without its
        // shared source/verb.
        return Ok(effects);
    }
    if let Some(effects) =
        super::fanout_family::parse_remove_counters_then_shared_damage_fanout(tokens)?
    {
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]);
    }
    if let Some(effect) = parse_leading_action_then_shared_damage_fanout(tokens)? {
        return Ok(vec![effect]);
    }
    // In `A, then B unless C`, the payment gates only the final authored
    // action. A complete trailing-unless primitive is also a valid parse of
    // the unsplit token stream, so establish the ordered sentence boundary
    // before that broad primitive can wrap both A and B.
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

fn parse_terminal_where_x_binding(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], Value)> {
    let shape =
        super::super::grammar::effects::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(
            tokens,
        )?;
    let view = TokenWordView::new(tokens);
    let where_word = view.parse_phrase_start(&["where", "x", "is"])?;
    let where_index = view.map_word_to_token_start(where_word)?;
    if has_explicit_comma_then_boundary_lexed(&tokens[where_index..]) {
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
    Some((
        leading_tokens,
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
        return Ok(Some(vec![EffectAst::RepeatEffects {
            count: count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
            effects,
        }]));
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
    Ok(Some(vec![EffectAst::ForEachObject { filter, effects }]))
}

fn parse_effect_chain_uncoordinated_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn immediate_tagged_permission_spec(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
        Ok(matches!(
            parse_permission_clause_spec_lexed(tokens)?,
            Some(PermissionClauseSpec::Tagged {
                lifetime: PermissionLifetime::Immediate,
                ..
            })
        ))
    }

    if let Some(effects) = parse_sentence_each_player_may_reveal_selected_cards_in_their_hand(
        SubjectVerbPrimitiveClause::new(tokens),
    )? {
        return Ok(effects);
    }

    let named_token_appositive = tokens.first().is_some_and(|token| token.is_word("create"))
        && crate::slice_primitives::select_position(tokens, OwnedLexToken::is_comma).is_some_and(
            |comma| {
                !tokens[..comma].iter().any(|token| token.is_word("token"))
                    && tokens[comma + 1..]
                        .iter()
                        .any(|token| token.is_word("token"))
            },
        );
    if named_token_appositive {
        return Ok(vec![super::creation_handlers::parse_create(tokens, None)?]);
    }

    if clause_may_contain_cast_or_play_permission_lexed(tokens)
        && let Some(effect) = parse_cast_or_play_tagged_clause(tokens)?
    {
        // A complete tagged permission may include its own coordinated
        // any-color mana rider. Preserve that atomic grammar before the
        // generic leading-`may` path removes the first modal subject and
        // leaves the rider as a verb-less `spend mana ...` clause.
        if immediate_tagged_permission_spec(tokens)?
            && let Some(player) = parse_leading_player_may_lexed(tokens)
        {
            return Ok(vec![EffectAst::MayByPlayer {
                player,
                effects: vec![effect],
            }]);
        }
        return Ok(vec![effect]);
    }

    let comma_then_segments = split_segments_on_comma_then_lexed(vec![tokens]);
    if let Some(effects) = parse_inline_looked_card_partition_chain(tokens) {
        return Ok(effects);
    }
    // A hand/graveyard-into-library shuffle owns its complete clause even
    // when a participant loop already stripped the subject; coordination
    // would otherwise sever the zone list from the shuffle verb. An optional
    // shuffle keeps its `may` scope through the may-aware routes.
    if parse_leading_player_may_lexed(tokens).is_none()
        && !chain_grammar::starts_with_may_tokens(tokens)
        && super::super::grammar::effects::parse_shuffle_graveyard_shape_lexed(tokens)
            .is_some_and(|shape| shape.has_hand_clause)
        && let Some(effects) =
            super::search_library::parse_shuffle_graveyard_into_library_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let [mill_tokens, followup_tokens] = comma_then_segments.as_slice() {
        let inline_sentences = [
            SentenceInput::from_lexed(mill_tokens),
            SentenceInput::from_lexed(followup_tokens),
        ];
        if let Some(effects) =
            super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_mill_then_may_put_from_among_into_hand(
                &inline_sentences,
                0,
            )?
        {
            return Ok(effects);
        }
    }
    if let [leading_tokens, shuffle_tokens] = comma_then_segments.as_slice() {
        let leading_segments = split_segments_on_comma_effect_head_lexed(vec![leading_tokens]);
        if let [look_tokens, deployment_tokens] = leading_segments.as_slice() {
            let inline_sentences = [
                SentenceInput::from_lexed(look_tokens),
                SentenceInput::from_lexed(deployment_tokens),
                SentenceInput::from_lexed(shuffle_tokens),
            ];
            if let Some(effects) =
                super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_put_matching_onto_battlefield_then_shuffle(
                    &inline_sentences,
                    0,
                )?
            {
                return Ok(effects);
            }
        }
    }
    if let Some(effects) = parse_reveal_source_exiled_permanents_sentence_lexed(tokens) {
        return Ok(effects);
    }
    // Once the conservative typed splitter has proved a real `, then`
    // boundary, route the complete chain before any prefix-tolerant
    // specialist can claim only its final verb. The inner chain pass retains
    // carried players, result values, tags, and authored ordering across the
    // separately parsed arms.
    if comma_then_segments.len() > 1 {
        return parse_effect_chain_inner_lexed(tokens);
    }

    // An immediate "you may cast/play" instruction is an optional action,
    // not a persistent permission. Claim it before the generic leading-may
    // path strips `may` while probing broader cast-permission surfaces.
    if let Some(spec) = parse_may_cast_it_sentence(tokens) {
        return Ok(vec![build_may_cast_tagged_effect(&spec)]);
    }

    if let Some(effects) = parse_for_each_exiled_this_way_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) = parse_for_each_object_effect_chain_shape(tokens)? {
        return Ok(effects);
    }

    if let Some(shape) = super::super::grammar::effects::sentence_predicate_shapes::
        parse_attacking_doesnt_tap_if_source_untapped_tokens(tokens)
    {
        let filter = parse_object_filter(shape.affected_tokens, false)?;
        return Ok(vec![
            EffectAst::subject_verb_grant_abilities_all_dynamically_with_condition(
                filter,
                vec![crate::cards::builders::GrantedAbilityAst::KeywordAction(
                    Box::new(crate::payload::KeywordAction::Vigilance),
                )],
                Until::EndOfCombat,
                PredicateAst::SourceIsUntapped,
            ),
        ]);
    }

    // A phase-insertion clause has no ordinary subject/verb head ("there is
    // an additional combat phase").  Conditional and labeled effect bodies
    // enter through the chain parser, so route the already-typed phase shape
    // before generic verb discovery as well as at the sentence entrypoint.
    if let Some(shape) = parse_additional_phases_shape(tokens) {
        return Ok(vec![EffectAst::subject_verb_additional_phases(
            shape.phases,
        )]);
    }

    // Preserve coordinated object operands as one tap result so a subsequent
    // "them" refers to the entire affected set, not only the first operand.
    if let Some(shape) = parse_tap_object_union_then_tokens(tokens) {
        let first = parse_target_phrase(shape.first_target_tokens)?;
        let first_filter = match first {
            TargetAst::Source(_) => ObjectFilter::source(),
            TargetAst::Object(filter, None, _) => filter,
            TargetAst::Tagged(tag, _) => ObjectFilter::tagged(tag),
            _ => {
                return Err(CardTextError::ParseError(
                    "coordinated tap operand must be a non-target object reference".to_string(),
                ));
            }
        };
        let all_filter = parse_object_filter(shape.all_filter_tokens, false)?;
        let mut union = ObjectFilter::default();
        union.any_of = vec![first_filter, all_filter];
        let mut effects = vec![EffectAst::subject_verb_tap_all(union)];
        effects.extend(parse_effect_chain_lexed(shape.followup_tokens)?);
        return Ok(effects);
    }

    if let Some(effect) = parse_may_have_any_number_tagged_phase_out_lexed(tokens) {
        return Ok(vec![effect]);
    }

    if let Some(effects) = parse_destroy_then_temporary_cant_attack_block_chain_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_exile_library_then_shuffle_graveyard_chain_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(shape) =
        sacrifice_discard_grammar::parse_each_player_may_discard_hand_and_draw_tokens(tokens)
    {
        let optional_effects = vec![
            EffectAst::subject_verb_discard_hand(PlayerAst::That),
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::That,
                SubjectVerbActionAst::Draw {
                    count: shape.draw_count,
                },
            ),
        ];
        return Ok(vec![EffectAst::ForEachPlayer {
            effects: vec![EffectAst::May {
                effects: optional_effects,
            }],
        }]);
    }
    if let Some(effects) =
        super::dispatch_inner::parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(tokens)
    {
        return Ok(effects);
    }

    if let Some(result_tokens) = chain_grammar::parse_meld_them_into_tokens(tokens) {
        let result_words = token_word_refs(result_tokens);
        if result_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing meld result name (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }
        return Ok(vec![EffectAst::subject_verb_meld(
            result_words.join(" "),
            false,
            false,
        )]);
    }

    let leading_scope = chain_grammar::parse_leading_chain_scope_tokens(tokens);
    let starts_with_each_opponent =
        leading_scope == Some(chain_grammar::ChainPlayerScope::EachOpponent);
    let starts_with_each_player =
        leading_scope == Some(chain_grammar::ChainPlayerScope::EachPlayer);

    // "Any player may pay ..." is a turn-order offer that ends when one
    // player accepts, rather than a single optional action performed by an
    // arbitrary player. Keep the payer and payer-relative dynamic values
    // inside the existing sequential AnyPlayerMay scope.
    if let Some(player) = parse_leading_player_may_lexed(tokens)
        && matches!(player, PlayerAst::Any | PlayerAst::Opponent)
    {
        let stripped = remove_through_first_word(tokens);
        let stripped = crate::util::trim_edge_punctuation_tokens(&stripped);
        if stripped.first().is_some_and(|token| token.is_word("pay")) {
            let payment = super::zone_handlers::parse_pay(
                crate::util::trim_edge_punctuation_tokens(&stripped[1..]),
                Some(crate::cards::builders::SubjectAst::Player(PlayerAst::That)),
            )?;
            return Ok(vec![EffectAst::AnyPlayerMay {
                players: if player == PlayerAst::Opponent {
                    PlayerFilter::Opponent
                } else {
                    PlayerFilter::Any
                },
                effects: vec![payment],
            }]);
        }
    }

    if let Some(shape) = parse_any_player_may_sacrifice_shape(tokens) {
        let sacrifice = super::zone_handlers::parse_sacrifice(
            shape.action_tokens,
            Some(crate::cards::builders::SubjectAst::Player(PlayerAst::That)),
            None,
        )?;
        return Ok(vec![EffectAst::AnyPlayerMay {
            players: shape.players,
            effects: vec![sacrifice],
        }]);
    }

    // Claim the complete causative damage offer before the broad leading-may
    // handler strips its participant and lowers only the inner damage. The
    // specialist distinguishes sequential "any player/opponent" offers from
    // a single targeted player's choice.
    if let Some(effects) =
        super::dispatch_inner::parse_any_player_may_have_source_deal_damage(tokens)?
    {
        return Ok(effects);
    }

    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) {
        if let Some(player) = parse_leading_player_may_lexed(trailing_if.leading_tokens) {
            let mut stripped = remove_through_first_word(trailing_if.leading_tokens);
            if let Some(rest) = chain_grammar::strip_leading_choose_to_tokens(&stripped) {
                stripped = rest.to_vec();
            }
            if let Some(rest) = chain_grammar::strip_leading_have_tokens(&stripped) {
                stripped = rest.to_vec();
            }
            let mut effects = parse_effect_chain_lexed(&stripped)?;
            for effect in &mut effects {
                bind_implicit_player_context(effect, player);
            }
            return Ok(vec![EffectAst::TrailingIf {
                predicate: trailing_if.predicate,
                effects: vec![EffectAst::MayByPlayer { player, effects }],
            }]);
        }

        if chain_grammar::starts_with_may_tokens(trailing_if.leading_tokens)
            && !starts_with_each_opponent
            && !starts_with_each_player
        {
            let stripped = remove_first_word(trailing_if.leading_tokens);
            let effects = parse_effect_chain_lexed(&stripped)?;
            return Ok(vec![EffectAst::TrailingIf {
                predicate: trailing_if.predicate,
                effects: vec![EffectAst::May { effects }],
            }]);
        }
    }

    if let Some(player) = parse_leading_player_may_lexed(tokens) {
        let mut stripped = remove_through_first_word(tokens);
        if let Some(rest) = chain_grammar::strip_leading_choose_to_tokens(&stripped) {
            stripped = rest.to_vec();
        }
        if let Some(rest) = chain_grammar::strip_leading_have_tokens(&stripped) {
            stripped = rest.to_vec();
        }
        if let Some(mut permission) = parse_additional_land_plays_clause_lexed(&stripped)? {
            bind_implicit_player_context(&mut permission, player);
            return Ok(vec![permission]);
        }
        let stripped_words = crate::lexer::parser_token_word_refs(&stripped);
        let has_copy_exception =
            crate::slice_primitives::select_last_position(&stripped_words, |word| {
                matches!(*word, "become" | "becomes")
            })
            .is_some_and(|become_word_idx| {
                let view = TokenWordView::new(&stripped);
                let body_start = view
                    .map_word_or_end_to_token_boundary(become_word_idx + 1)
                    .unwrap_or(stripped.len());
                super::super::grammar::effects::become_shapes::parse_become_rest_shape(
                    &stripped[body_start..],
                )
                .copy_exception
                .is_some()
            });
        let mut effects = if has_copy_exception {
            super::parse_effect_sentence_lexed(&stripped)?
        } else {
            parse_effect_chain_lexed(&stripped)?
        };
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        if leading_may_is_permission_clause_lexed(&stripped)? {
            if immediate_tagged_permission_spec(&stripped)? {
                return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
            }
            return Ok(effects);
        }
        if has_any_number_of_times_suffix(&stripped) && is_repeatable_optional_payment(&effects) {
            return Ok(vec![EffectAst::RepeatProcess {
                effects: vec![EffectAst::MayByPlayer { player, effects }],
                continue_effect_index: 0,
                continue_predicate: crate::cards::builders::IfResultPredicate::Did,
            }]);
        }
        return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
    }

    if chain_grammar::starts_with_may_tokens(tokens)
        && !starts_with_each_opponent
        && !starts_with_each_player
    {
        let stripped = remove_first_word(tokens);
        if let Some(permission) = parse_additional_land_plays_clause_lexed(&stripped)? {
            return Ok(vec![permission]);
        }
        let effects = parse_effect_chain_lexed(&stripped)?;
        if leading_may_is_permission_clause_lexed(&stripped)? {
            if immediate_tagged_permission_spec(&stripped)? {
                return Ok(vec![EffectAst::May { effects }]);
            }
            return Ok(effects);
        }
        if has_any_number_of_times_suffix(&stripped) && is_repeatable_optional_payment(&effects) {
            return Ok(vec![EffectAst::RepeatProcess {
                effects: vec![EffectAst::May { effects }],
                continue_effect_index: 0,
                continue_predicate: crate::cards::builders::IfResultPredicate::Did,
            }]);
        }
        return Ok(vec![EffectAst::May { effects }]);
    }

    // The broad consult recognizer intentionally accepts a traversal prefix.
    // Claim the complete inline consult/disposition program first so a result-
    // prefixed clause does not silently lose its battlefield move and library
    // remainder after the traversal.
    if split_leading_result_prefix_lexed(tokens).is_none()
        && let Some(effects) =
        super::dispatch_inner::parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(
            tokens,
        )?
    {
        return Ok(effects);
    }

    // A consult traversal can continue after its stop condition in the same
    // sentence. Preserve that complete procedure before the bare traversal
    // fallback intentionally returns only the consult action.
    if let Some(effects) =
        super::consult_family::parse_consult_traversal_with_inline_followup(tokens)?
    {
        return Ok(effects);
    }

    // Consult traversal has a `reveal` verb, but its `until` stop rule is
    // what gives the sentence its semantics. Claim the complete traversal
    // before the ordinary subject/verb registry lowers only the leading
    // reveal as a plain top-of-library effect.
    if let Some(parts) = super::consult_family::parse_consult_traversal_sentence(tokens)? {
        return Ok(parts.effects);
    }

    if chain_grammar::parse_tap_or_untap_all_choice_tokens(tokens) {
        let action_tokens = remove_first_word(tokens);
        return Ok(vec![super::zone_handlers::parse_tap(&action_tokens)?]);
    }

    // An `or` inside a grammar-proven trailing payment is cost structure,
    // not effect coordination. Route the complete clause through the typed
    // trailing-unless builder before the generic chain splitter sees either
    // alternative as a sibling action.
    if has_unless_payment_choice(tokens)? {
        return Ok(vec![parse_effect_clause_lexed(tokens)?]);
    }

    if let Some(unless_action) = parse_or_action_clause_lexed(tokens)? {
        return Ok(vec![unless_action]);
    }

    if clause_may_contain_cast_or_play_permission_lexed(tokens)
        && let Some(effect) = parse_cast_or_play_tagged_clause(tokens)?
    {
        if immediate_tagged_permission_spec(tokens)?
            && let Some(player) = parse_leading_player_may_lexed(tokens)
        {
            return Ok(vec![EffectAst::MayByPlayer {
                player,
                effects: vec![effect],
            }]);
        }
        return Ok(vec![effect]);
    }

    // Some specialized subject/verb parsers accept a valid leading clause
    // without requiring end-of-input. Split a genuine top-level conjunction
    // before entering that registry, otherwise a first arm such as `copy that
    // spell` or `deals damage` can silently consume the whole sentence and
    // drop the following action.
    if has_explicit_comma_then_boundary_lexed(tokens) {
        return parse_effect_chain_inner_lexed(tokens);
    }
    let split_segments = split_effect_chain_on_and_lexed(tokens);
    let executable_heads = split_segments
        .iter()
        .filter(|segment| super::lex_chain_helpers::segment_has_effect_head_lexed(segment))
        .count();
    let has_expandable_shared_verb_operand = split_segments
        .iter()
        .zip(split_segments.iter().skip(1))
        .any(|(left, right)| expand_missing_verb_segment_lexed(left, right).is_some());
    if split_leading_result_prefix_lexed(tokens).is_none()
        && split_segments.len() > 1
        && (executable_heads > 1 || has_expandable_shared_verb_operand)
    {
        return parse_effect_chain_inner_lexed(tokens);
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
        (LeadingResultPrefixKind::If, [EffectAst::IfResult { predicate, effects }])
            if predicate == &prefix.predicate =>
        {
            effects
        }
        (LeadingResultPrefixKind::When, [EffectAst::WhenResult { predicate, effects }])
            if predicate == &prefix.predicate =>
        {
            effects
        }
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

        return Ok(Some(EffectAst::UnlessAction {
            effects: first_effects,
            alternative: second_effects,
            player: PlayerAst::Implicit,
        }));
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
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: body,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: body,
            },
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
                action: SubjectVerbActionAst::Discard { .. } | SubjectVerbActionAst::DiscardHand,
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

pub fn parse_effect_chain_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_inner_lexed_unstacked(tokens, true)
}

fn parse_effect_chain_inner_lexed_unstacked(
    tokens: &[OwnedLexToken],
    recognize_control_flow: bool,
) -> Result<Vec<EffectAst>, CardTextError> {
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
    if (!recognize_control_flow || split_trailing_if_clause_lexed(tokens).is_none())
        && let Some(effects) =
            super::subject_verb_primitives::parse_sentence_sacrifice_it_next_end_step(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
    {
        return Ok(effects);
    }
    // Nested consequence parsing enters this inner materializer directly.
    // Preserve the same typed copy-token exception ownership as the public
    // chain entrypoint before coordination exposes an `and` inside the
    // characteristic bundle as another create clause.
    if let Some(effect) = parse_atomic_token_copy_exception(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) = parse_inline_looked_card_partition_chain(tokens) {
        return Ok(effects);
    }
    // A hand/graveyard-into-library shuffle owns its complete clause even
    // when a participant loop already stripped the subject; coordination
    // would otherwise sever the zone list from the shuffle verb. An optional
    // shuffle keeps its `may` scope through the may-aware routes.
    if parse_leading_player_may_lexed(tokens).is_none()
        && !chain_grammar::starts_with_may_tokens(tokens)
        && super::super::grammar::effects::parse_shuffle_graveyard_shape_lexed(tokens)
            .is_some_and(|shape| shape.has_hand_clause)
        && let Some(effects) =
            super::search_library::parse_shuffle_graveyard_into_library_sentence(tokens)?
    {
        return Ok(effects);
    }
    // A gain/get compound carries one shared target and can also carry a
    // leading duration. Its typed parser must see the intact sentence before
    // generic duration/control-flow wrapping splits the coordinated actions;
    // otherwise a per-card modifier is reduced to a generic dynamic pump.
    if recognize_control_flow
        && super::super::grammar::effects::gain_ability_shapes::parse_gain_then_get_shape(tokens)
            .is_some()
        && let Some(effects) = super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(effects);
    }
    if recognize_control_flow {
        match super::super::grammar::effects::control_flow::recognize_control_flow(tokens) {
            crate::recognition::ParseOutcome::Match(matched) => {
                let plan = matched.value;
                let mut effects = if plan.parse_original_with_legacy {
                    parse_effect_chain_inner_lexed_unstacked(tokens, false)?
                } else {
                    parse_effect_chain_inner_lexed(plan.body_tokens)?
                };
                let body_words = crate::lexer::token_word_refs(plan.body_tokens);
                if crate::word_primitives::parse_any_sequence_prefix(
                    &body_words,
                    &[&["discard"], &["then", "discard"]],
                ) {
                    for effect in &mut effects {
                        bind_implicit_player_context(effect, PlayerAst::You);
                    }
                }
                if let Some(control) = plan.into_ast(effects.clone()) {
                    return Ok(vec![EffectAst::ControlFlow(Box::new(control))]);
                }
                return Ok(effects);
            }
            crate::recognition::ParseOutcome::NoMatch => {}
            crate::recognition::ParseOutcome::Error(diagnostic) => {
                return Err(diagnostic.into_card_text_error());
            }
        }
    }
    // `Venture into the dungeon` is a complete subjectless mechanic action.
    // When it leads a coordinated clause, the general subject/verb splitter
    // can treat it as context for the later explicit-player arm and retain
    // only that arm. Prove the exact mechanic phrase and conjunction before
    // lowering both executable actions.
    if tokens.len() > 5
        && tokens[0].is_word("venture")
        && tokens[1].is_word("into")
        && tokens[2].is_word("the")
        && tokens[3].is_word("dungeon")
        && tokens[4].is_word("and")
    {
        let mut effects = vec![EffectAst::subject_verb_venture_into_dungeon(
            PlayerAst::You,
            false,
        )];
        effects.extend(parse_effect_chain_inner_lexed(&tokens[5..])?);
        let coordination = crate::grammar::effects::coordination::coordination_from_effects(
            crate::model::CoordinationKindAst::Conjunction,
            crate::model::CoordinationOperatorAst::And,
            crate::model::EffectOrderingAst::Unordered,
            effects,
        )
        .expect("leading venture conjunction contains at least two effects");
        return Ok(vec![EffectAst::Coordination(coordination)]);
    }
    // A keyword mechanic at the end of a coordinated chain must not consume
    // the earlier action as part of its target phrase (for example, "you
    // lose 1 life and this creature endures 1"). Let the semantic chain
    // splitter isolate those arms before probing the bare-keyword parser.
    if split_effect_chain_on_and_lexed(tokens).len() <= 1
        && !has_explicit_comma_then_boundary_lexed(tokens)
        && let Some(effect) = parse_keyword_mechanic_without_terminal_punctuation(tokens)?
    {
        return Ok(vec![effect]);
    }
    // Preserve coordinated conditional animations before the generic `and`
    // splitter turns the second branch into an orphaned follow-up clause.
    let leading_duration_shape = chain_grammar::parse_carry_duration_prefix_tokens(tokens);
    let pair_tokens = leading_duration_shape
        .as_ref()
        .map_or(tokens, |shape| shape.rest);
    if let Some(mut effect) = super::clause_dispatch::parse_conditional_become_pair(pair_tokens)? {
        if let Some(shape) = leading_duration_shape {
            super::dispatch_entry::apply_leading_duration_to_become_effect(
                &mut effect,
                &shape.duration,
            );
        }
        return Ok(vec![effect]);
    }

    // Keep the duration attached while recognizing a base-P/T clause. The
    // ordinary chain path carries a leading duration separately, but doing so
    // before verb dispatch leaves `creatures ... have base power ...` without
    // the temporal evidence that distinguishes a temporary effect from a
    // static characteristic-setting sentence. A surrounding where-X sentence
    // may also have already removed its binding tail; its typed binding pass
    // will replace the X values after this clause has been lowered.
    if let Some(effect) = super::for_each_helpers::parse_has_base_power_toughness_clause(tokens)? {
        return Ok(vec![effect]);
    }

    // This mechanic is a single authored process even though the generic
    // conjunction splitter sees `lose ... and draw ...` and would otherwise
    // send the draw/clash tail through the ordinary draw parser. Claim the
    // complete shape before splitting coordinated verbs.
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_lose_draw_clash_repeat_process(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }

    if chain_grammar::starts_with_unless_tokens(tokens)
        && let Some(effects) = parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }

    if let Some(effects) =
        super::dispatch_inner::parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(
            tokens,
        )?
    {
        return Ok(effects);
    }

    if let Some(effects) =
        super::consult_family::parse_consult_traversal_with_inline_followup(tokens)?
    {
        return Ok(effects);
    }

    if let Some(parts) = super::consult_family::parse_consult_traversal_sentence(tokens)? {
        return Ok(parts.effects);
    }

    if let Some(effects) = parse_search_library_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    let source_exiled_bottom_random = {
        let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
        crate::word_primitives::sequence_occurs(&words, &["exiled", "with", "this"])
            && crate::word_primitives::sequence_occurs(&words, &["on", "the", "bottom"])
            && crate::word_primitives::sequence_occurs(&words, &["in", "a", "random"])
    };
    if source_exiled_bottom_random {
        let action_tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
            &tokens[1..]
        } else {
            tokens
        };
        if let Some(surface) =
            super::verb_handlers::parse_exiled_with_source_move_surface(action_tokens)
        {
            let verb_index = crate::slice_primitives::select_position(action_tokens, |token| {
                token.is_word("put") || token.is_word("puts")
            })
            .unwrap_or(0);
            let effect =
                super::verb_handlers::parse_put_into_hand(&action_tokens[verb_index..], None)?
                    .with_exiled_with_source_surface(Some(surface));
            return Ok(vec![effect]);
        }
    }
    if let Some(effects) = parse_player_chooses_source_excluded_permanent_then_exiles(tokens) {
        return Ok(effects);
    }
    if let Some(effects) = parse_tap_those_then_unattach_equipment_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_return_it_then_loses_all_abilities_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(clauses) =
        super::player_subject_sequences::split_explicit_player_subject_clauses(tokens)
    {
        let mut effects = Vec::new();
        for clause in clauses {
            effects.extend(parse_effect_chain_inner_lexed(clause)?);
        }
        return Ok(effects);
    }
    if let Some(effect) = parse_quantified_participant_subject_effect(tokens)? {
        return Ok(vec![effect]);
    }

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
    let mut coordination_plan =
        match super::super::grammar::effects::coordination::recognize_coordination(
            effect_chain_tokens,
        ) {
            crate::recognition::ParseOutcome::Match(matched) => Some(matched.value),
            crate::recognition::ParseOutcome::NoMatch => None,
            crate::recognition::ParseOutcome::Error(diagnostic) => {
                return Err(diagnostic.into_card_text_error());
            }
        };
    let planned_segments = coordination_plan
        .as_ref()
        .and_then(|plan| plan.materialized_segments());
    if coordination_plan.is_some() && planned_segments.is_none() {
        coordination_plan = None;
    }
    let mut segments: Vec<Vec<OwnedLexToken>> = if let Some(planned_segments) = planned_segments {
        planned_segments
    } else {
        let raw_segments = split_effect_chain_on_and_lexed(effect_chain_tokens);
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
    segments = expand_segments_with_comma_action_clauses_lexed(segments);
    segments = expand_segments_with_multi_create_clauses_lexed(segments);
    segments = merge_for_each_counter_group_segments_lexed(segments);
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
                LeadingResultPrefixKind::If => EffectAst::IfResult {
                    predicate: prefix.predicate,
                    effects: parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
                },
                LeadingResultPrefixKind::When => EffectAst::WhenResult {
                    predicate: prefix.predicate,
                    effects: parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
                },
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
            let effect = EffectAst::ForEachObject {
                filter,
                effects: nested_effects,
            };
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
        let primitive_segment_effects = if let Some(effects) = run_subject_verb_primitives_lexed(
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
                    action: SubjectVerbActionAst::Pump { .. },
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
    let SubjectVerbActionAst::DestroyAll {
        filter,
        no_regeneration: true,
        ..
    } = action
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
    let SubjectVerbActionAst::GainLife { amount } = action else {
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
                    ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key())
                        .with_surface_hints(spec.surface_hints().iter().cloned()),
                )))
            }
            _ => None,
        }
    }
    let Some(amount) = retag_mana_value(amount, tag) else {
        return;
    };

    effects[gain_index] = EffectAst::ForEachTagged {
        tag: tag.clone(),
        effects: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::ItsController,
            SubjectVerbActionAst::GainLife { amount },
        )],
    };
}

fn tap_then_next_untap_actions(effects: &[EffectAst]) -> bool {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tap { .. },
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
            SubjectVerbActionAst::Destroy { .. }
        ) | (
            chain_grammar::CoordinatedTargetActionKind::Exile,
            SubjectVerbActionAst::Exile { .. }
        ) | (
            chain_grammar::CoordinatedTargetActionKind::Return,
            SubjectVerbActionAst::ReturnToHand { .. }
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
    target_is_generic_token_filter,
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
