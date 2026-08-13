use winnow::Parser;
use winnow::combinator::{alt, repeat};
use winnow::error::{ContextError, ErrMode};

use super::super::compile_support::{
    effects_have_cross_arm_tag_dependency, effects_reference_it_tag,
};
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
use super::super::span_from_tokens;
use crate::recognition::RuleId;
use crate::registry::{HeadDiscriminator, RegistryRuleMetadata};
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
    SubjectVerbPrimitiveClause, has_unless_sacrifice_or_pay_choice,
    parse_cant_effect_sentence_lexed, parse_effect_clause_lexed,
    parse_search_library_sentence_lexed,
    parse_sentence_each_player_may_reveal_selected_cards_in_their_hand,
    parse_sentence_exile_source_with_counters_lexed,
    parse_sentence_put_onto_battlefield_with_counters_on_it_lexed,
    parse_sentence_return_with_counters_on_it_lexed, parse_sentence_unless_pays,
    parse_simple_gain_ability_clause_lexed, parse_simple_lose_ability_clause_lexed,
    parse_token_copy_followup_sentence_lexed, token_copy_action_reference_surface,
    try_apply_token_copy_followup,
};
use crate::runtime_backend::grammar::shared_util::value_semantics::{
    parse_number_prefix_lexed, parse_value_prefix_lexed,
};

const ENCHANTED_TAG_NAME: &str = "enchanted";
const SENTENCE_HELPER_REVEALED_TAG_PREFIX: &str = "__sentence_helper_revealed";
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey,
    TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, SourceReferenceSurface, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

fn has_any_number_of_times_suffix(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
    words.ends_with(&["any", "number", "of", "times"])
}

fn parse_player_chooses_source_excluded_permanent_then_exiles(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
    if words.as_slice()
        != [
            "an",
            "opponent",
            "chooses",
            "a",
            "permanent",
            "you",
            "control",
            "other",
            "than",
            "this",
            "creature",
            "and",
            "exiles",
            "it",
        ]
    {
        return None;
    }
    let tag = crate::runtime_backend::util::helper_tag_for_tokens(tokens, "chosen");
    let mut filter = ObjectFilter::permanent().you_control();
    filter.other = true;
    filter.source_surface = Some(SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));
    Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false),
    ])
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
    // generic fanout path strips the participant subject and reparses the
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
                    tag: TagKey::from(crate::cards::builders::IT_TAG),
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
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_effect_chain_lexed(view.tokens).map(Some)
}

pub(super) const FALLBACK_POST_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 1] =
    [LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("effect-chain"),
            HeadDiscriminator::words(&[]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Legacy(parse_effect_chain_rule_lexed),
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

pub(crate) fn looks_like_multi_create_chain_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches!(find_verb_lexed(tokens), Some((Verb::Create, _)))
        && chain_grammar::count_token_mentions(tokens) >= 2
}

pub(crate) fn parse_reveal_source_exiled_permanents_sentence_lexed(
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
        ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile);
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

pub(crate) fn parse_effect_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    // Chain parsing recursively re-enters the sentence dispatcher for
    // nested clauses and quoted/conditional payloads.  The public chain
    // entrypoint is also used directly by compiler tests and lower-level
    // callers, so it needs the same stack growth protection as the sentence
    // entrypoint.
    stacker::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        parse_effect_chain_lexed_inner(tokens)
    })
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
    if let Some(venture_start) = tokens.windows(5).position(|window| {
        window[0].is_word("and")
            && window[1].is_word("venture")
            && window[2].is_word("into")
            && window[3].is_word("the")
            && window[4].is_word("dungeon")
    }) && tokens[venture_start + 5..]
        .iter()
        .all(|token| token.as_word().is_none())
    {
        let mut effects = parse_effect_chain_lexed(&tokens[..venture_start])?;
        effects.push(EffectAst::subject_verb_venture_into_dungeon(
            PlayerAst::You,
            false,
        ));
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]);
    }
    if let Some(effect) = parse_each_prior_affected_object_controller_mana_value_life(tokens) {
        return Ok(vec![effect]);
    }
    // A paid-label condition owns the complete consequence, including every
    // authored conjunction inside it.  This must run before the ordinary
    // chain splitter: a consequence such as "phase out, and until ..., can't
    // change and gain protection" contains several independently executable
    // heads, so the splitter can otherwise lower those heads and discard the
    // leading condition before the subject/verb priority route below is ever
    // reached.
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
    if let Some(effects) =
        super::fanout_family::parse_remove_counters_then_shared_damage_fanout(tokens)?
    {
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]);
    }
    let effects = parse_effect_chain_uncoordinated_lexed(tokens)?;
    if effects.len() > 1 && has_authored_comma_then_surface_lexed(tokens) {
        return Ok(vec![EffectAst::CommaThen { effects }]);
    }
    Ok(preserve_coordinated_effect_chain_surface(tokens, effects))
}

/// Parse the demonstrative per-object reward
/// `the controller of each of those <objects> gains life equal to its mana
/// value`. The unresolved `__it__` collection is intentionally retained here:
/// sentence-sequence reference resolution binds it to the immediately prior
/// affected-object tag, then runtime iteration evaluates each object's LKI
/// controller and mana value independently.
pub(crate) fn parse_each_prior_affected_object_controller_mana_value_life(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let words = token_word_refs(tokens);
    const PREFIX: &[&str] = &["the", "controller", "of", "each", "of", "those"];
    const SUFFIX: &[&str] = &["gains", "life", "equal", "to", "its", "mana", "value"];
    if !words.starts_with(PREFIX)
        || !words.ends_with(SUFFIX)
        || words.len() <= PREFIX.len() + SUFFIX.len()
    {
        return None;
    }
    let noun_words = &words[PREFIX.len()..words.len() - SUFFIX.len()];
    let noun_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(noun_words);
    let noun_filter = parse_object_filter(&noun_tokens, false).ok()?;
    if noun_filter.card_types.is_empty()
        && noun_filter.subtypes.is_empty()
        && noun_filter.any_of.is_empty()
    {
        return None;
    }

    let it = TagKey::from(IT_TAG);
    Some(EffectAst::ForEachTagged {
        tag: it.clone(),
        effects: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::ItsController,
            SubjectVerbActionAst::GainLife {
                amount: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(it)))
                    .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
            },
        )],
    })
}

fn nested_comma_then_candidate_count(effect: &EffectAst) -> usize {
    if matches!(effect, EffectAst::CommaThen { .. }) {
        return 0;
    }

    let mut count = 0usize;
    for_each_nested_effects(effect, true, |nested| {
        if nested.len() > 1 {
            count += 1;
        } else if let [child] = nested {
            count += nested_comma_then_candidate_count(child);
        }
    });
    count
}

fn wrap_first_nested_comma_then_candidate(effect: &mut EffectAst) -> bool {
    if matches!(effect, EffectAst::CommaThen { .. }) {
        return false;
    }

    let mut wrapped = false;
    for_each_nested_effect_vec_mut(effect, true, |nested| {
        if wrapped {
            return;
        }
        if nested.len() > 1 {
            let effects = std::mem::take(nested);
            nested.push(EffectAst::CommaThen { effects });
            wrapped = true;
        } else if let [child] = nested.as_mut_slice() {
            wrapped = wrap_first_nested_comma_then_candidate(child);
        }
    });
    wrapped
}

fn preserve_unique_nested_comma_then_surface(effects: &mut [EffectAst]) {
    let [effect] = effects else {
        return;
    };
    if nested_comma_then_candidate_count(effect) == 1 {
        let _ = wrap_first_nested_comma_then_candidate(effect);
    }
}

pub(crate) fn preserve_coordinated_effect_chain_surface(
    tokens: &[OwnedLexToken],
    mut effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    // Whole-line parsing can reach this surface-preservation pass with the
    // semantic actions already flattened, without going through
    // `parse_effect_chain_lexed`. Preserve an authored same-sentence
    // comma-then boundary here as well so every parser entrypoint produces
    // the same typed sequence.
    if effects.len() > 1 && has_authored_comma_then_surface_lexed(tokens) {
        return vec![EffectAst::CommaThen { effects }];
    }
    if has_authored_comma_then_surface_lexed(tokens) {
        preserve_unique_nested_comma_then_surface(&mut effects);
    }

    let Some(leading_duration) = chain_grammar::coordinated_effect_chain_leading_duration(tokens)
    else {
        return effects;
    };

    // A shared-subject tail can already be coordinated by its specialist
    // parser even though the top-level grammar proves that an earlier action
    // belongs to the same authored conjunction. Flatten only an ordinary
    // nested conjunction so the complete source clause keeps one typed
    // boundary. Result conjunctions and duration-leading conjunctions carry
    // additional semantics and must remain nested.
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::Coordinated {
                leading_duration: true,
                ..
            } | EffectAst::Coordinated {
                result_conjunction: true,
                ..
            }
        )
    }) {
        return effects;
    }
    if effects.len() > 1
        && effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::Coordinated { .. }))
    {
        effects = effects
            .into_iter()
            .flat_map(|effect| match effect {
                EffectAst::Coordinated {
                    effects,
                    leading_duration: false,
                    result_conjunction: false,
                } => effects,
                effect => vec![effect],
            })
            .collect();
    }

    // The grammar above proves this was a top-level source conjunction and
    // rejects card-type lists, quoted text, shared subjects, and every clause
    // containing an explicit "then". Keep that authored relationship as
    // typed surface metadata for every semantic action family. The sequence
    // still executes its children in order, so reference flow between arms is
    // preserved without asking the renderer to infer coordination later.
    if effects.len() < 2 {
        return effects;
    }

    // In `gains flying and loses trample until end of turn`, the trailing
    // duration scopes both coordinated continuous-effect arms. Only carry it
    // backward across an exact two-arm pair with the same semantic target,
    // and only when the first arm has no authored duration of its own.
    if !leading_duration
        && let Some(duration) = shared_trailing_continuous_effect_duration(&effects)
    {
        apply_carried_effect_duration(&mut effects[0], &duration);
    }

    vec![EffectAst::Coordinated {
        effects,
        leading_duration,
        result_conjunction: false,
    }]
}

enum ContinuousEffectScope<'a> {
    Target(&'a TargetAst),
    Filter(&'a ObjectFilter),
}

fn target_is_source(target: &TargetAst) -> bool {
    matches!(target, TargetAst::Source(_))
        || matches!(target, TargetAst::Object(filter, _, _) if filter.source)
}

fn same_target_ignoring_surface_spans(left: &TargetAst, right: &TargetAst) -> bool {
    if target_is_source(left) && target_is_source(right) {
        return true;
    }
    match (left, right) {
        (TargetAst::AnyTarget(_), TargetAst::AnyTarget(_))
        | (TargetAst::AnyOtherTarget(_), TargetAst::AnyOtherTarget(_))
        | (
            TargetAst::AttackedPlayerOrPlaneswalker(_),
            TargetAst::AttackedPlayerOrPlaneswalker(_),
        )
        | (TargetAst::Spell(_), TargetAst::Spell(_)) => true,
        (
            TargetAst::ObjectOrPlayer(left_object, left_player, _),
            TargetAst::ObjectOrPlayer(right_object, right_player, _),
        ) => left_object == right_object && left_player == right_player,
        (TargetAst::PlayerOrPlaneswalker(left, _), TargetAst::PlayerOrPlaneswalker(right, _))
        | (TargetAst::Player(left, _), TargetAst::Player(right, _)) => left == right,
        (TargetAst::Object(left, _, _), TargetAst::Object(right, _, _)) => left == right,
        (TargetAst::Tagged(left, _), TargetAst::Tagged(right, _)) => left == right,
        (
            TargetAst::WithCount(left_target, left_count),
            TargetAst::WithCount(right_target, right_count),
        ) => {
            left_count == right_count
                && same_target_ignoring_surface_spans(left_target, right_target)
        }
        (
            TargetAst::WithCountValue(left_target, left_count, left_value),
            TargetAst::WithCountValue(right_target, right_count, right_value),
        ) => {
            left_count == right_count
                && left_value == right_value
                && same_target_ignoring_surface_spans(left_target, right_target)
        }
        _ => false,
    }
}

fn continuous_effect_scope_and_duration(
    effect: &EffectAst,
) -> Option<(ContinuousEffectScope<'_>, &Until)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return None;
    };
    match action {
        SubjectVerbActionAst::GainControl {
            target, duration, ..
        }
        | SubjectVerbActionAst::Pump {
            target, duration, ..
        }
        | SubjectVerbActionAst::PumpForEach {
            target, duration, ..
        }
        | SubjectVerbActionAst::PumpByLastEffect {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetBasePowerToughness {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetBasePower {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeBasePtCreature {
            target, duration, ..
        }
        | SubjectVerbActionAst::AddCardTypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetCardTypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::RemoveCardTypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::AddSubtypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::RemoveSubtypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetCreatureSubtypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::AddColors {
            target, duration, ..
        }
        | SubjectVerbActionAst::AddAllSubtypesOfFamily {
            target, duration, ..
        }
        | SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeAuraEnchantment {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeBasicLandType {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetColors {
            target, duration, ..
        }
        | SubjectVerbActionAst::MakeColorless {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeBasicLandTypeChoice {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeCreatureTypeChoice {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeColorChoice {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeCopy {
            target, duration, ..
        }
        | SubjectVerbActionAst::GrantAbilitiesToTarget {
            target, duration, ..
        }
        | SubjectVerbActionAst::RemoveAbilitiesFromTarget {
            target, duration, ..
        }
        | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
            target, duration, ..
        } => Some((ContinuousEffectScope::Target(target), duration)),
        SubjectVerbActionAst::PumpAll {
            filter, duration, ..
        }
        | SubjectVerbActionAst::GrantAbilitiesAll {
            filter, duration, ..
        }
        | SubjectVerbActionAst::RemoveAbilitiesAll {
            filter, duration, ..
        }
        | SubjectVerbActionAst::GrantAbilitiesChoiceAll {
            filter, duration, ..
        } => Some((ContinuousEffectScope::Filter(filter), duration)),
        _ => None,
    }
}

fn same_continuous_effect_scope(
    left: ContinuousEffectScope<'_>,
    right: ContinuousEffectScope<'_>,
) -> bool {
    match (left, right) {
        (ContinuousEffectScope::Target(left), ContinuousEffectScope::Target(right)) => {
            same_target_ignoring_surface_spans(left, right)
        }
        (ContinuousEffectScope::Filter(left), ContinuousEffectScope::Filter(right)) => {
            left == right
        }
        _ => false,
    }
}

fn shared_trailing_continuous_effect_duration(effects: &[EffectAst]) -> Option<Until> {
    let [first, second] = effects else {
        return None;
    };
    let (first_scope, first_duration) = continuous_effect_scope_and_duration(first)?;
    let (second_scope, second_duration) = continuous_effect_scope_and_duration(second)?;
    (matches!(first_duration, Until::Forever)
        && !matches!(second_duration, Until::Forever)
        && same_continuous_effect_scope(first_scope, second_scope))
    .then(|| second_duration.clone())
}

fn parse_for_each_object_effect_chain_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_object_effect_shape(tokens) else {
        return Ok(None);
    };

    let mut count_words = vec!["for", "each"];
    count_words.extend(crate::runtime_backend::token_word_refs(shape.filter_tokens));
    let effect_words = crate::runtime_backend::token_word_refs(shape.effect_tokens);
    let has_that_player_payload = effect_words
        .windows(2)
        .any(|window| window == ["that", "player"]);
    if let Some((count, used)) =
        crate::runtime_backend::util::parse_for_each_count_value_words(&count_words)
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

    let comma_then_segments = split_segments_on_comma_then_lexed(vec![tokens]);
    if let [look_tokens, partition_tokens] = comma_then_segments.as_slice()
        && let Some(effects) =
            super::sequence_rules::generic_subject_verb_sequences::pairs::parse_inline_look_at_top_then_singleton_hand_partition(
                look_tokens,
                partition_tokens,
            )
    {
        return Ok(effects);
    }
    if let [mill_tokens, followup_tokens] = comma_then_segments.as_slice() {
        let inline_sentences = [
            SentenceInput::from_lexed(mill_tokens),
            SentenceInput::from_lexed(followup_tokens),
        ];
        if let Some(effects) =
            super::sequence_rules::generic_subject_verb_sequences::pairs::parse_mill_then_may_put_from_among_into_hand(
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
                super::sequence_rules::generic_subject_verb_sequences::triples::parse_look_at_top_put_matching_onto_battlefield_then_shuffle(
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
                vec![crate::cards::builders::GrantedAbilityAst::StaticAbility(
                    crate::static_abilities::StaticAbility::vigilance(),
                )],
                Until::EndOfCombat,
                crate::ConditionExpr::SourceIsUntapped,
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
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
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
        let mut effects = parse_effect_chain_lexed(&stripped)?;
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
        return parse_effect_chain_with_subject_verb_primitives_lexed(tokens);
    }

    if let Some(unless_action) = parse_or_action_clause_lexed(tokens)? {
        return Ok(vec![unless_action]);
    }

    if clause_may_contain_cast_or_play_permission_lexed(tokens)
        && let Some(effect) = parse_cast_or_play_tagged_clause(tokens)?
    {
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
        .windows(2)
        .any(|pair| expand_missing_verb_segment_lexed(pair[0], pair[1]).is_some());
    if split_leading_result_prefix_lexed(tokens).is_none()
        && split_segments.len() > 1
        && (executable_heads > 1 || has_expandable_shared_verb_operand)
    {
        return parse_effect_chain_inner_lexed(tokens);
    }

    parse_effect_chain_with_subject_verb_primitives_lexed(tokens)
}

pub(crate) fn preserve_result_conjunction_body_lexed(
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

pub(crate) fn preserve_leading_result_coordination_lexed(
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

pub(crate) fn parse_destroy_then_temporary_cant_attack_block_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    for split in chain_grammar::parse_destroy_restriction_splits_tokens(tokens) {
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

pub(crate) fn parse_or_action_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if chain_grammar::parse_tap_or_untap_all_choice_tokens(tokens) {
        return Ok(None);
    }
    if has_unless_sacrifice_or_pay_choice(tokens)? {
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

pub(crate) fn parse_effect_chain_with_subject_verb_primitives_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    stacker::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        parse_effect_chain_with_subject_verb_primitives_lexed_unstacked(tokens)
    })
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

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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

pub(crate) fn leading_condition_is_paid_label(tokens: &[OwnedLexToken]) -> bool {
    let Some(if_idx) = tokens.iter().position(|token| token.is_word("if")) else {
        return false;
    };
    if crate::runtime_backend::token_word_refs(&tokens[..if_idx])
        .iter()
        .any(|word| !matches!(*word, "then"))
    {
        return false;
    }
    let Some(comma_idx) = tokens[if_idx + 1..]
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
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

pub(crate) fn append_missing_coordinated_return_discard_tail(
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

pub(crate) fn parse_effect_chain_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    stacker::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        parse_effect_chain_inner_lexed_unstacked(tokens, true)
    })
}

fn parse_effect_chain_inner_lexed_unstacked(
    tokens: &[OwnedLexToken],
    recognize_control_flow: bool,
) -> Result<Vec<EffectAst>, CardTextError> {
    if recognize_control_flow {
        match super::super::grammar::effects::control_flow::recognize_control_flow(tokens) {
            crate::recognition::ParseOutcome::Match(matched) => {
                let plan = matched.value;
                let effects = if plan.parse_original_with_legacy {
                    parse_effect_chain_inner_lexed_unstacked(tokens, false)?
                } else {
                    parse_effect_chain_inner_lexed(plan.body_tokens)?
                };
                if let Some(control) = plan.into_ast(effects.clone()) {
                    return Ok(vec![EffectAst::ControlFlow(Box::new(control))]);
                }
                return Ok(effects);
            }
            crate::recognition::ParseOutcome::NoMatch => {}
            crate::recognition::ParseOutcome::Error(diagnostic) => {
                return Err(diagnostic.into_legacy_error());
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
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]);
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
        words
            .windows(3)
            .any(|window| window == ["exiled", "with", "this"])
            && words
                .windows(3)
                .any(|window| window == ["on", "the", "bottom"])
            && words
                .windows(3)
                .any(|window| window == ["in", "a", "random"])
    };
    if source_exiled_bottom_random {
        let action_tokens = tokens
            .first()
            .is_some_and(|token| token.is_word("then"))
            .then_some(&tokens[1..])
            .unwrap_or(tokens);
        if super::verb_handlers::parse_exiled_with_source_move_surface(action_tokens).is_some() {
            return Ok(vec![super::verb_handlers::parse_put_into_hand(
                action_tokens,
                None,
            )?]);
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
                return Err(diagnostic.into_legacy_error());
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
        if let Some(shape) = chain_grammar::parse_carry_duration_prefix_tokens(&segment) {
            carried_duration = Some(shape.duration.clone());
            carried_leading_duration = true;
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
        // feed those through the same typed gain parser with an implicit
        // `it gains` subject instead of treating them as effect clauses.
        if find_verb_lexed(&segment).is_none() {
            let mut gain_tokens = vec![synthetic_lexed_word("it"), synthetic_lexed_word("gains")];
            gain_tokens.extend(segment.iter().cloned());
            if let Some(mut effect) = parse_simple_gain_ability_clause_lexed(&gain_tokens)? {
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
    bind_adjacent_life_stat_pronouns(
        &mut effects,
        coordination_reference_facts.life_stat_pronoun,
    );
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
    if let Some(plan) = coordination_plan
        && let Some(coordination) = plan.into_ast(effects.clone())
    {
        return Ok(vec![EffectAst::Coordination(coordination)]);
    }
    Ok(effects)
}

/// Bind an authored per-object controller reward to each object affected by
/// the immediately preceding tagged sweep. A scalar gain by `you` cannot
/// represent "the controller of each of those artifacts" when the destroyed
/// set can have several different controllers.
fn bind_each_prior_affected_object_controller_life_gain(
    effects: &mut Vec<EffectAst>,
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
    if filter.card_types != [crate::CardType::Artifact] {
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
                    ChooseSpec::Tagged(TagKey::from(IT_TAG))
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

fn target_ast_is_source(target: &TargetAst) -> bool {
    match target {
        TargetAst::Source(_) => true,
        TargetAst::Object(filter, _, _) => filter.source,
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_ast_is_source(inner)
        }
        _ => false,
    }
}

fn source_damage_then_gain_ability_actions(effects: &[EffectAst]) -> bool {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEqualToPower { source, .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. },
            ..
        }),
    ] = effects
    else {
        return false;
    };
    target_ast_is_source(source) && target_ast_is_source(target)
}

fn bind_adjacent_discard_count_draws(effects: &mut [EffectAst]) {
    fn is_discard(effect: &EffectAst) -> bool {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Discard { .. },
                ..
            })
        )
    }

    fn bind_draw(effect: &mut EffectAst) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }) = effect
        else {
            return;
        };
        let hints = count.surface_hints().to_vec();
        let bound = match count.unhinted() {
            Value::EventValue(crate::effect::EventValueSpec::Amount) => {
                Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                }
            }
            Value::EventValueOffset(crate::effect::EventValueSpec::Amount, offset) => {
                Value::PendingEffectMetricOffset {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                    offset: *offset,
                }
            }
            _ => return,
        };
        *count = bound.with_surface_hints(hints);
    }

    for index in 0..effects.len().saturating_sub(1) {
        if is_discard(&effects[index]) {
            bind_draw(&mut effects[index + 1]);
        }
    }
}

fn bind_adjacent_implicit_draw_discard_subjects(
    effects: &mut [EffectAst],
    recognized_shared_actor: bool,
) {
    if !recognized_shared_actor {
        return;
    }
    for index in 0..effects.len().saturating_sub(1) {
        let draw_is_implicit = matches!(
            &effects[index],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action: SubjectVerbActionAst::Draw { .. },
            }) if subject.player == PlayerAst::Implicit
        );
        if !draw_is_implicit {
            continue;
        }
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::Discard { .. },
        }) = &mut effects[index + 1]
            && subject.player == PlayerAst::Implicit
        {
            subject.player = PlayerAst::You;
        }
    }
}

fn bind_adjacent_life_stat_pronouns(
    effects: &mut [EffectAst],
    recognized_reference: bool,
) {
    if !recognized_reference {
        return;
    }

    fn tagged_stat_reference(value: &Value) -> Option<crate::target::ChooseSpec> {
        let spec = match value.unhinted() {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) => spec.as_ref(),
            _ => return None,
        };
        matches!(spec.unhinted(), crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG)
            .then(|| spec.clone())
    }

    fn life_amount(effect: &EffectAst) -> Option<&Value> {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return None;
        };
        match action {
            SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::PayLife { amount } => Some(amount),
            _ => None,
        }
    }

    fn retarget_source_stat(value: &mut Value, antecedent: &crate::target::ChooseSpec) {
        match value {
            Value::SurfaceHinted { value, .. } => retarget_source_stat(value, antecedent),
            Value::SourcePower => {
                *value = Value::PowerOf(Box::new(antecedent.clone()));
            }
            Value::SourceToughness => {
                *value = Value::ToughnessOf(Box::new(antecedent.clone()));
            }
            Value::PowerOf(spec)
                if matches!(spec.unhinted(), crate::target::ChooseSpec::Source) =>
            {
                *spec = Box::new(antecedent.clone());
            }
            Value::ToughnessOf(spec)
                if matches!(spec.unhinted(), crate::target::ChooseSpec::Source) =>
            {
                *spec = Box::new(antecedent.clone());
            }
            _ => {}
        }
    }

    for index in 0..effects.len().saturating_sub(1) {
        let Some(antecedent) = life_amount(&effects[index]).and_then(tagged_stat_reference) else {
            continue;
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = &mut effects[index + 1]
        else {
            continue;
        };
        let amount = match action {
            SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::PayLife { amount } => amount,
            _ => continue,
        };
        retarget_source_stat(amount, &antecedent);
    }
}

/// Repair an X-valued life follow-up that the isolated clause parser lowered
/// to its historical source-stat fallback before the sentence-wide where-X
/// binder ran. The authored X uses and typed tagged stat value together prove
/// that both adjacent life actions share one value; copying the complete value
/// preserves the same LKI object identity and presentation hints.
pub(crate) fn bind_adjacent_shared_x_life_stat_values(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) {
    let words = token_word_refs(tokens);
    let Some(where_x_index) = words
        .windows(3)
        .position(|window| window == ["where", "x", "is"])
    else {
        return;
    };
    if words[..where_x_index]
        .iter()
        .filter(|word| **word == "x")
        .count()
        < 2
        || !matches!(
            words.get(where_x_index + 3..where_x_index + 5),
            Some(["its", "power"] | ["its", "toughness"])
        )
    {
        return;
    }

    fn life_amount(effect: &EffectAst) -> Option<&Value> {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return None;
        };
        match action {
            SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::PayLife { amount } => Some(amount),
            _ => None,
        }
    }

    fn life_amount_mut(effect: &mut EffectAst) -> Option<&mut Value> {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return None;
        };
        match action {
            SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::PayLife { amount } => Some(amount),
            _ => None,
        }
    }

    fn tagged_where_x_stat(value: &Value) -> bool {
        if !value.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs) {
            return false;
        }
        let spec = match value.unhinted() {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) => spec,
            _ => return false,
        };
        matches!(spec.unhinted(), ChooseSpec::Tagged(_))
    }

    fn bind_in_list(effects: &mut [EffectAst]) {
        for index in 0..effects.len().saturating_sub(1) {
            let (leading, trailing) = effects.split_at_mut(index + 1);
            let Some(shared_value) = life_amount(&leading[index])
                .filter(|value| tagged_where_x_stat(value))
                .cloned()
            else {
                continue;
            };
            let Some(follow_up) = life_amount_mut(&mut trailing[0]) else {
                continue;
            };
            if !follow_up.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
                && matches!(
                    follow_up.unhinted(),
                    Value::SourcePower | Value::SourceToughness
                )
            {
                *follow_up = shared_value;
            }
        }
    }

    bind_in_list(effects);
    for effect in effects {
        for_each_nested_effects_mut(effect, true, |nested| bind_in_list(nested));
    }
}

/// Keep one authored target declaration for a coordinated draw/life-loss X
/// clause whose shared basis names a single target player's zone. Isolated
/// value parsing can synthesize the same TargetOnly prelude once per X use;
/// the lexical one-target proof distinguishes that from two independently
/// authored target slots.
pub(crate) fn dedupe_shared_target_player_draw_lose_x(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let words = token_word_refs(tokens);
    if words.iter().filter(|word| **word == "target").count() != 1
        || words.iter().filter(|word| **word == "x").count() < 3
        || !words
            .windows(3)
            .any(|window| window == ["where", "x", "is"])
    {
        return;
    }

    let mut target: Option<&TargetAst> = None;
    let mut target_indices = Vec::new();
    let mut draw_value: Option<&Value> = None;
    let mut lose_value: Option<&Value> = None;
    for (index, effect) in effects.iter().enumerate() {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return;
        };
        match &subject_verb.action {
            SubjectVerbActionAst::TargetOnly {
                target: candidate,
                explicit_declaration: false,
            } => {
                if let Some(existing) = target {
                    if existing != candidate {
                        return;
                    }
                } else {
                    target = Some(candidate);
                }
                target_indices.push(index);
            }
            SubjectVerbActionAst::Draw { count }
                if matches!(
                    subject_verb.subject.player,
                    crate::cards::builders::PlayerAst::You
                        | crate::cards::builders::PlayerAst::Implicit
                ) =>
            {
                if draw_value.replace(count).is_some() {
                    return;
                }
            }
            SubjectVerbActionAst::LoseLife { amount }
                if matches!(
                    subject_verb.subject.player,
                    crate::cards::builders::PlayerAst::You
                        | crate::cards::builders::PlayerAst::Implicit
                ) =>
            {
                if lose_value.replace(amount).is_some() {
                    return;
                }
            }
            _ => return,
        }
    }
    if target_indices.len() < 2
        || draw_value.is_none()
        || draw_value.map(Value::unhinted) != lose_value.map(Value::unhinted)
    {
        return;
    }
    for index in target_indices.into_iter().skip(1).rev() {
        effects.remove(index);
    }
}

fn bind_source_exiled_effect(effect: EffectAst, bind: bool) -> EffectAst {
    if bind {
        EffectAst::TagAffected {
            effect: Box::new(effect),
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        }
    } else {
        effect
    }
}

fn parse_tap_those_then_unattach_equipment_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !chain_grammar::parse_tap_then_unattach_tokens(tokens) {
        return Ok(None);
    }

    let mut tapped_filter = ObjectFilter::creature();
    tapped_filter.zone = Some(Zone::Battlefield);
    tapped_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut equipment_filter = ObjectFilter::permanent();
    equipment_filter.card_types.push(CardType::Artifact);
    equipment_filter.subtypes.push(Subtype::Equipment);
    equipment_filter.zone = Some(Zone::Battlefield);
    equipment_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::AttachedToTaggedObject,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_tap(TargetAst::Object(tapped_filter, None, None)),
        EffectAst::subject_verb_unattach(TargetAst::WithCount(
            Box::new(TargetAst::Object(equipment_filter, None, None)),
            ChoiceCount::any_number(),
        )),
    ]))
}

pub(crate) fn parse_may_have_any_number_tagged_phase_out_lexed(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if token_word_refs(tokens).as_slice()
        != [
            "you", "may", "have", "any", "number", "of", "them", "phase", "out",
        ]
    {
        return None;
    }

    let chosen_tag = TagKey::from("phase_out_selection");
    let mut available = ObjectFilter::default().in_zone(Zone::Battlefield);
    available
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let mut phase_out_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    phase_out_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: chosen_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Some(EffectAst::MayByPlayer {
        player: PlayerAst::You,
        effects: vec![
            EffectAst::ChooseObjects {
                filter: available,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag,
            },
            EffectAst::subject_verb_phase_out_all(phase_out_filter),
        ],
    })
}

pub(crate) fn parse_return_it_then_loses_all_abilities_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(return_tokens) = chain_grammar::split_return_then_loses_tokens(tokens) else {
        return Ok(None);
    };
    let mut effects = parse_effect_chain_inner_lexed(return_tokens)?;
    effects.push(EffectAst::subject_verb_remove_abilities_from_target(
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        Vec::new(),
        Until::Forever,
    ));
    Ok(Some(effects))
}

fn is_orphan_rounded_up_where_x_tail(
    segment: &[OwnedLexToken],
    previous: Option<&[OwnedLexToken]>,
    previous_effect: Option<&EffectAst>,
) -> bool {
    if !chain_grammar::is_rounded_up_segment_tokens(segment) {
        return false;
    }
    if previous.is_none() && previous_effect.is_none() {
        return true;
    }
    previous.is_some_and(chain_grammar::has_where_x_is_half_tokens)
        || previous_effect.is_some_and(effect_uses_half_life_total_value)
}

fn effect_uses_half_life_total_value(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenWithMods {
                    dynamic_power_toughness,
                    ..
                },
            ..
        }) => dynamic_power_toughness
            .as_ref()
            .is_some_and(|(power, toughness)| {
                value_is_half_life_total(power) || value_is_half_life_total(toughness)
            }),
        EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachTaggedWithControllerAtLastBlockedBy { effects, .. }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::AnyPlayerMay { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::WhenResult { effects, .. }
        | EffectAst::ManaRestricted { effects, .. } => {
            effects.iter().any(effect_uses_half_life_total_value)
        }
        _ => false,
    }
}

fn value_is_half_life_total(value: &Value) -> bool {
    matches!(value.unhinted(), Value::HalfLifeTotalRoundedUp(_))
}

fn effect_duration_for_gain_followup_carry(effect: &EffectAst) -> Option<Until> {
    let duration = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GainControl { duration, .. }
                | SubjectVerbActionAst::Pump { duration, .. }
                | SubjectVerbActionAst::PumpForEach { duration, .. }
                | SubjectVerbActionAst::PumpAll { duration, .. }
                | SubjectVerbActionAst::PumpByLastEffect { duration, .. }
                | SubjectVerbActionAst::SetBasePowerToughness { duration, .. }
                | SubjectVerbActionAst::SetBasePower { duration, .. }
                | SubjectVerbActionAst::BecomeBasePtCreature { duration, .. }
                | SubjectVerbActionAst::AddCardTypes { duration, .. }
                | SubjectVerbActionAst::SetCardTypes { duration, .. }
                | SubjectVerbActionAst::RemoveCardTypes { duration, .. }
                | SubjectVerbActionAst::AddSubtypes { duration, .. }
                | SubjectVerbActionAst::RemoveSubtypes { duration, .. }
                | SubjectVerbActionAst::SetCreatureSubtypes { duration, .. }
                | SubjectVerbActionAst::AddColors { duration, .. }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily { duration, .. }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { duration, .. }
                | SubjectVerbActionAst::BecomeAuraEnchantment { duration, .. }
                | SubjectVerbActionAst::SetColors { duration, .. }
                | SubjectVerbActionAst::MakeColorless { duration, .. }
                | SubjectVerbActionAst::BecomeBasicLandType { duration, .. }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice { duration, .. }
                | SubjectVerbActionAst::BecomeColorChoice { duration, .. }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice { duration, .. }
                | SubjectVerbActionAst::BecomeCopy { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesToTarget { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesAll { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { duration, .. }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget { duration, .. }
                | SubjectVerbActionAst::RemoveAbilitiesAll { duration, .. }
                | SubjectVerbActionAst::Cant { duration, .. },
            ..
        }) => duration,
        _ => return None,
    };

    if matches!(duration, Until::Forever) {
        None
    } else {
        Some(duration.clone())
    }
}

fn apply_carried_effect_duration(effect: &mut EffectAst, duration: &Until) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GainControl {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::Pump {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PumpForEach {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PumpAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PumpByLastEffect {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePowerToughness {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePower {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasePtCreature {
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
                | SubjectVerbActionAst::RemoveCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetCreatureSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeAuraEnchantment {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::MakeColorless {
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
                | SubjectVerbActionAst::BecomeColorChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCopy {
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
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PreventAllDamageToTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PreventAllDamageToTargetFromSourceFilter {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PreventAllDamageFromSourceFilter {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::Cant {
                    duration: effect_duration,
                    ..
                },
            ..
        }) if matches!(effect_duration, Until::Forever) => {
            *effect_duration = duration.clone();
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            for nested in if_true.iter_mut().chain(if_false.iter_mut()) {
                apply_carried_effect_duration(nested, duration);
            }
        }
        // A compound continuous clause ("has base power and toughness 4/4
        // and gains flying") parses into a coordinated wrapper; the carried
        // duration scopes every member. The Forever guard above keeps
        // authored member durations intact.
        EffectAst::Sequence { effects } | EffectAst::Coordinated { effects, .. } => {
            for nested in effects.iter_mut() {
                apply_carried_effect_duration(nested, duration);
            }
        }
        _ => {}
    }
}

fn parse_carried_cant_effects(
    tokens: &[OwnedLexToken],
    duration: &Until,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(restrictions) =
        super::super::activation_and_restrictions::parse_cant_restrictions(tokens)?
    else {
        return Ok(None);
    };

    let mut target = None;
    let mut effects = Vec::with_capacity(restrictions.len() + 1);
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target
                && existing != &parsed_target
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported mixed carried restriction targets (clause: '{}')",
                    token_word_refs(tokens).join(" ")
                )));
            }
            target = Some(parsed_target);
        }
        effects.push(EffectAst::subject_verb_cant(
            parsed.restriction,
            duration.clone(),
            None,
        ));
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::subject_verb_target_only(target));
    }
    Ok(Some(effects))
}

pub(crate) fn collapse_for_each_player_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::ForEachPlayer { .. },
                EffectAst::ForEachPlayer {
                    effects: followup_effects,
                },
            ) => effects_reference_it_tag(followup_effects),
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (
                EffectAst::ForEachPlayer {
                    effects: first_effects,
                },
                EffectAst::ForEachPlayer {
                    effects: mut followup_effects,
                },
            ) => {
                first_effects.append(&mut followup_effects);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

pub(crate) fn collapse_for_each_object_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (EffectAst::ForEachObject { filter, .. }, followup) => {
                effects_reference_it_tag(std::slice::from_ref(followup))
                    || (for_each_revealed_this_way_filter(filter)
                        && is_revealed_this_way_scalar_reward(followup))
            }
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (EffectAst::ForEachObject { effects: inner, .. }, followup) => {
                inner.push(followup);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

fn for_each_revealed_this_way_filter(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && (constraint.tag.as_str() == IT_TAG
                || sentence_helper_revealed_tag(constraint.tag.as_str()))
    })
}

fn sentence_helper_revealed_tag(tag: &str) -> bool {
    tag.starts_with(SENTENCE_HELPER_REVEALED_TAG_PREFIX)
}

fn is_revealed_this_way_scalar_reward(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. },
            ..
        })
    )
}

pub(crate) fn parse_effect_clause_with_trailing_if_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    // Some clauses contain an authored trailing condition inside a larger
    // typed procedure, such as a face-down return followed by turning the
    // returned permanent face up. Let the clause parser preserve that
    // multi-effect structure before this generic splitter treats everything
    // after the first `if` as predicate text.
    let may_have_embedded_followup = tokens.iter().any(|token| token.is_word("if"))
        && tokens
            .windows(2)
            .any(|pair| pair[0].is_comma() && pair[1].is_word("then"))
        && tokens.iter().any(|token| token.is_word("turn"))
        && tokens
            .windows(2)
            .any(|pair| pair[0].is_word("face") && pair[1].is_word("up"));
    if may_have_embedded_followup
        && let Ok(effect) = parse_effect_clause_lexed(tokens)
        && matches!(
            &effect,
            EffectAst::TrailingIf { effects, .. } if effects.len() > 1
        )
    {
        return Ok(effect);
    }

    let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) else {
        return parse_effect_clause_lexed(tokens);
    };
    let mut predicate = trailing_if.predicate;
    if !trailing_if_predicate_supported(&predicate) {
        return parse_effect_clause_lexed(tokens);
    }

    // Equality is executable independently of its authored wording. Retain
    // the exact-comparison surface on the numeric operand only when the
    // predicate itself (after the trailing `if`) contains `exactly`; a count
    // in the leading effect must not leak into the condition's presentation.
    let exact_predicate_surface = tokens
        .iter()
        .rposition(|token| token.is_word("if"))
        .is_some_and(|if_index| {
            tokens[if_index + 1..]
                .iter()
                .any(|token| token.is_word("exactly"))
        });
    if exact_predicate_surface
        && let PredicateAst::ValueComparison {
            operator: ironsmith_core::ValueComparisonOperator::Equal,
            right,
            ..
        } = &mut predicate
    {
        *right = right
            .clone()
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ExactComparison);
    }

    let base_effect = if let Ok(effect) = parse_effect_clause_lexed(trailing_if.leading_tokens) {
        effect
    } else {
        if let Some(effect) = parse_simple_lose_ability_clause_lexed(trailing_if.leading_tokens)? {
            effect
        } else if let Some(effect) =
            parse_simple_gain_ability_clause_lexed(trailing_if.leading_tokens)?
        {
            effect
        } else {
            return parse_effect_clause_lexed(tokens);
        }
    };

    let predicate = bind_trailing_it_predicate_to_explicit_effect_target(predicate, &base_effect);
    Ok(EffectAst::TrailingIf {
        predicate,
        effects: vec![base_effect],
    })
}

fn explicit_tagged_target(target: &TargetAst) -> Option<TagKey> {
    match target {
        TargetAst::Tagged(tag, _) if tag.as_str() != IT_TAG => Some(tag.clone()),
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .find(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() != IT_TAG
            })
            .map(|constraint| constraint.tag.clone()),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            explicit_tagged_target(inner)
        }
        _ => None,
    }
}

fn explicit_effect_object_tag(effect: &EffectAst) -> Option<TagKey> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => match action {
            SubjectVerbActionAst::MoveToZone { target, .. }
            | SubjectVerbActionAst::MayMoveToZone { target, .. }
            | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
            | SubjectVerbActionAst::PutOntoBattlefield { target, .. }
            | SubjectVerbActionAst::TurnFaceUp { target }
            | SubjectVerbActionAst::ReturnToHand { target, .. } => explicit_tagged_target(target),
            _ => None,
        },
        EffectAst::May { effects } | EffectAst::MayByPlayer { effects, .. }
            if effects.len() == 1 =>
        {
            explicit_effect_object_tag(&effects[0])
        }
        EffectAst::TagAffected { tag, .. } if tag.as_str() != IT_TAG => Some(tag.clone()),
        _ => None,
    }
}

fn explicit_target_choose_spec(target: &TargetAst) -> Option<ChooseSpec> {
    match target {
        TargetAst::Object(filter, Some(_), _) => Some(ChooseSpec::Target(Box::new(
            ChooseSpec::Object(filter.clone()),
        ))),
        TargetAst::WithCount(inner, count) if count.is_single() => {
            explicit_target_choose_spec(inner)
        }
        TargetAst::WithCountValue(inner, count, _) if count.is_single() => {
            explicit_target_choose_spec(inner)
        }
        _ => None,
    }
}

fn explicit_effect_object_target(effect: &EffectAst) -> Option<ChooseSpec> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => match action {
            SubjectVerbActionAst::MoveToZone { target, .. }
            | SubjectVerbActionAst::MayMoveToZone { target, .. }
            | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
            | SubjectVerbActionAst::PutOntoBattlefield { target, .. }
            | SubjectVerbActionAst::TurnFaceUp { target }
            | SubjectVerbActionAst::ReturnToHand { target, .. } => {
                explicit_target_choose_spec(target)
            }
            _ => None,
        },
        EffectAst::May { effects } | EffectAst::MayByPlayer { effects, .. }
            if effects.len() == 1 =>
        {
            explicit_effect_object_target(&effects[0])
        }
        _ => None,
    }
}

fn bind_it_metric_to_explicit_target(value: Value, target: &ChooseSpec) -> Value {
    match value {
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_it_metric_to_explicit_target(*value, target)),
            hints,
        },
        Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) => {
            Value::PowerOf(Box::new(
                target
                    .clone()
                    .with_surface_hints(spec.surface_hints().iter().cloned()),
            ))
        }
        Value::ToughnessOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) => {
            Value::ToughnessOf(Box::new(
                target
                    .clone()
                    .with_surface_hints(spec.surface_hints().iter().cloned()),
            ))
        }
        Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) => {
            Value::ManaValueOf(Box::new(
                target
                    .clone()
                    .with_surface_hints(spec.surface_hints().iter().cloned()),
            ))
        }
        other => other,
    }
}

fn bind_trailing_it_predicate_to_explicit_effect_target(
    predicate: PredicateAst,
    effect: &EffectAst,
) -> PredicateAst {
    match predicate {
        PredicateAst::ItMatches(filter) => {
            let explicit_target = explicit_effect_object_target(effect);
            let demonstrative_land = filter.demonstrative_antecedent_surface()
                == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land);
            let explicit_target_is_land = explicit_target.as_ref().is_some_and(|target| {
                matches!(
                    target.base(),
                    ChooseSpec::Object(target_filter)
                        if target_filter.card_types.contains(&crate::CardType::Land)
                            || target_filter
                                .subtypes
                                .iter()
                                .any(crate::Subtype::is_basic_land_type)
                )
            });
            // A typed demonstrative can deliberately skip over the target in
            // this replacement clause. In Emeria Shepherd, “that land” still
            // means the landfall event object, not the nonland graveyard card
            // the optional return action targets.
            if demonstrative_land && !explicit_target_is_land {
                return PredicateAst::ItMatches(filter);
            }
            if let Some(tag) = explicit_effect_object_tag(effect) {
                PredicateAst::TaggedMatches(tag, filter)
            } else if explicit_target.is_some() {
                PredicateAst::TargetMatches(filter)
            } else {
                PredicateAst::ItMatches(filter)
            }
        }
        PredicateAst::ValueComparison {
            left,
            operator,
            right,
        } if explicit_effect_object_target(effect).is_some() => {
            let target = explicit_effect_object_target(effect)
                .expect("guarded explicit effect target should remain available");
            PredicateAst::ValueComparison {
                left: bind_it_metric_to_explicit_target(left, &target),
                operator,
                right: bind_it_metric_to_explicit_target(right, &target),
            }
        }
        other => other,
    }
}

fn trailing_if_predicate_supported(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
            | PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::ItMatches(_)
            | PredicateAst::ItMatchedLastKnown(_)
            | PredicateAst::TargetMatches(_)
            | PredicateAst::PlayerControlsMoreThanYou { .. }
            | PredicateAst::PlayerControls { .. }
            | PredicateAst::PlayerHasAtLeast { .. }
            | PredicateAst::PlayerControlsExactly { .. }
            | PredicateAst::PlayerHasAtLeastWithDifferentPowers { .. }
            | PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { .. }
            | PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { .. }
            | PredicateAst::PlayerHasMoreLifeThanYou { .. }
            | PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { .. }
            | PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { .. }
            | PredicateAst::PlayerIsMonarch { .. }
            | PredicateAst::PlayerHasInitiative { .. }
            | PredicateAst::PlayerHasCitysBlessing { .. }
            | PredicateAst::PlayerHasMoreCardsInHandThanYou { .. }
            | PredicateAst::PlayerHasCardTypesInGraveyardOrMore { .. }
            | PredicateAst::YouControlMoreCreaturesThanTargetSpellController
            | PredicateAst::ValueComparison { .. }
    ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if tag.as_str() == ENCHANTED_TAG_NAME)
}

pub(crate) fn target_is_generic_token_filter(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.token
        && filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.tagged_constraints.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    let Some(chain_grammar::DelayedCopyTiming::EndStep { player_is_you }) = facts.timing else {
        return;
    };
    if !facts.has_exile || !facts.has_token {
        return;
    }
    let next_end_step_player = if player_is_you {
        PlayerFilter::You
    } else {
        PlayerFilter::Any
    };

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                    ..
                }),
                EffectAst::SubjectVerb(subject_verb),
            ) => match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                }
                | SubjectVerbActionAst::Exile { target, .. } => {
                    target_is_generic_token_filter(target)
                }
                _ => false,
            },
            _ => false,
        };

        if !mark_next_end_step_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        exile_at_next_end_step,
                        exile_at_next_end_step_reference_surface,
                        next_end_step_player: effect_next_end_step_player,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        exile_at_next_end_step,
                        exile_at_next_end_step_reference_surface,
                        next_end_step_player: effect_next_end_step_player,
                        ..
                    },
                ..
            }) => {
                *exile_at_next_end_step = true;
                *exile_at_next_end_step_reference_surface =
                    token_copy_action_reference_surface(tokens, "exile");
                *effect_next_end_step_player = next_end_step_player.clone();
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_next_end_step_sacrifice_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    if !facts.has_sacrifice || !facts.has_token {
        return;
    }
    let (is_next_upkeep, upkeep_player_is_you, next_end_step_player) = match facts.timing {
        Some(chain_grammar::DelayedCopyTiming::EndStep { player_is_you }) => (
            false,
            false,
            if player_is_you {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            },
        ),
        Some(chain_grammar::DelayedCopyTiming::Upkeep { player_is_you }) => {
            (true, player_is_you, PlayerFilter::Any)
        }
        _ => return,
    };

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_sacrifice = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Sacrifice { filter, count, .. },
                    ..
                }),
            ) => *count == 1 && filter.token,
            _ => false,
        };

        if !mark_next_end_step_sacrifice {
            idx += 1;
            continue;
        }

        if is_next_upkeep {
            let sacrifice = effects.remove(idx + 1);
            effects.insert(
                idx + 1,
                EffectAst::DelayedUntilNextUpkeep {
                    player: if upkeep_player_is_you {
                        PlayerAst::You
                    } else {
                        PlayerAst::Any
                    },
                    effects: vec![sacrifice],
                },
            );
            idx += 2;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        sacrifice_at_next_end_step,
                        sacrifice_at_next_end_step_reference_surface,
                        next_end_step_player: effect_next_end_step_player,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        sacrifice_at_next_end_step,
                        sacrifice_at_next_end_step_reference_surface,
                        next_end_step_player: effect_next_end_step_player,
                        ..
                    },
                ..
            }) => {
                *sacrifice_at_next_end_step = true;
                *sacrifice_at_next_end_step_reference_surface =
                    token_copy_action_reference_surface(tokens, "sacrifice");
                *effect_next_end_step_player = next_end_step_player.clone();
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    if !facts.has_exile
        || !facts.has_token
        || facts.timing != Some(chain_grammar::DelayedCopyTiming::EndOfCombat)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_end_of_combat_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. }
                        | SubjectVerbActionAst::CreateTokenWithMods { .. },
                    ..
                }),
                EffectAst::SubjectVerb(subject_verb),
            ) => match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                }
                | SubjectVerbActionAst::Exile { target, .. } => {
                    target_is_generic_token_filter(target)
                }
                _ => false,
            },
            _ => false,
        };

        if !mark_end_of_combat_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        exile_at_end_of_combat,
                        exile_at_end_of_combat_reference_surface,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        exile_at_end_of_combat,
                        exile_at_end_of_combat_reference_surface,
                        ..
                    },
                ..
            }) => {
                *exile_at_end_of_combat = true;
                *exile_at_end_of_combat_reference_surface =
                    token_copy_action_reference_surface(tokens, "exile");
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenWithMods {
                        exile_at_end_of_combat,
                        ..
                    },
                ..
            }) => {
                *exile_at_end_of_combat = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

fn split_on_comma_or_semicolon_lexed(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_quotes = false;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes || !matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon) {
            continue;
        }
        let current = trim_lexed_commas(&tokens[start..idx]);
        if !current.is_empty() {
            segments.push(current.to_vec());
        }
        start = idx + 1;
    }
    let tail = trim_lexed_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail.to_vec());
    }
    segments
}

pub(crate) fn expand_segments_with_comma_action_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let looks_like_sac_discard_chain = (grammar::contains_word(&segment, "sacrifice")
            || grammar::contains_word(&segment, "sacrifices"))
            && (grammar::contains_word(&segment, "discard")
                || grammar::contains_word(&segment, "discards"));
        if !looks_like_sac_discard_chain {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        let mut valid_split = true;

        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while let Some(rest) = chain_grammar::strip_leading_and_tokens(&part) {
                part = rest.to_vec();
            }
            if part.is_empty() {
                continue;
            }

            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }

            valid_split = false;
            break;
        }

        if valid_split && local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_segments_with_multi_create_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let Some((Verb::Create, _)) = find_verb_lexed(&segment) else {
            expanded.push(segment);
            continue;
        };
        let has_token_rules_tail = chain_grammar::has_token_rules_tail_tokens(&segment);
        if has_token_rules_tail {
            expanded.push(segment);
            continue;
        }
        let token_mentions = chain_grammar::count_token_mentions(&segment);
        if token_mentions < 2 {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while let Some(rest) = chain_grammar::strip_leading_and_tokens(&part) {
                part = rest.to_vec();
            }
            if part.is_empty() {
                continue;
            }
            if let Some(previous) = local_parts.last()
                && is_token_creation_context(previous)
                && starts_with_inline_token_rules_tail(&part)
            {
                if let Some(last) = local_parts.last_mut() {
                    last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                    last.extend(part);
                }
                continue;
            }
            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }
            if let Some(last) = local_parts.last_mut() {
                last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                last.extend(part);
            } else {
                local_parts.push(part);
            }
        }

        if local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_missing_verb_segment_lexed(
    previous: &[OwnedLexToken],
    segment: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let (verb, verb_idx) = find_verb_lexed(previous)?;
    match verb {
        Verb::Deal => {
            if parse_value_prefix_lexed(segment).is_none()
                || !grammar::contains_word(segment, "damage")
            {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Sacrifice => {
            let segment_words = token_word_refs(segment);
            let starts_like_object_phrase = matches!(
                segment_words.first().copied(),
                Some("a" | "an" | "another" | "target")
            ) || parse_number_prefix_lexed(segment).is_some();
            if !starts_like_object_phrase {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Exile => {
            let segment_words = token_word_refs(segment);
            let starts_like_object_reference = matches!(
                segment_words.first().copied(),
                Some(
                    "a" | "an"
                        | "another"
                        | "target"
                        | "it"
                        | "them"
                        | "this"
                        | "that"
                        | "these"
                        | "those"
                )
            ) || parse_number_prefix_lexed(segment).is_some();
            if !starts_like_object_reference {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Create => {
            if !starts_like_create_fragment_lexed(segment) {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarryContext {
    Player(PlayerAst),
    ForEachPlayer,
    ForEachTargetPlayers(ChoiceCount),
    ForEachOpponent,
}

pub(crate) fn player_ast_from_filter_for_carry(filter: &PlayerFilter) -> Option<PlayerAst> {
    match filter {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(inner) => {
            if matches!(inner.as_ref(), PlayerFilter::Opponent) {
                Some(PlayerAst::TargetOpponent)
            } else {
                Some(PlayerAst::Target)
            }
        }
        PlayerFilter::AliasedTarget(_) => Some(PlayerAst::That),
        _ => None,
    }
}

pub(crate) fn player_owner_filter_from_target_for_carry(target: &TargetAst) -> Option<PlayerAst> {
    match target {
        TargetAst::Player(filter, _) => player_ast_from_filter_for_carry(filter),
        TargetAst::Object(filter, _, _) => {
            if !matches!(
                filter.zone,
                Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
            ) {
                return None;
            }
            filter
                .owner
                .as_ref()
                .and_then(player_ast_from_filter_for_carry)
        }
        TargetAst::WithCount(inner, _) => player_owner_filter_from_target_for_carry(inner),
        _ => None,
    }
}

fn player_target_carry_context(target: &TargetAst) -> Option<CarryContext> {
    match target {
        TargetAst::Player(filter, _) => {
            player_ast_from_filter_for_carry(filter).map(CarryContext::Player)
        }
        TargetAst::WithCount(inner, count) => {
            let inner_context = player_target_carry_context(inner.as_ref())?;
            if count.min > 1 && count.max == Some(count.min) {
                Some(CarryContext::ForEachTargetPlayers(*count))
            } else {
                Some(inner_context)
            }
        }
        _ => None,
    }
}

pub(crate) fn explicit_player_for_carry(effect: &EffectAst) -> Option<CarryContext> {
    if matches!(effect, EffectAst::ForEachPlayer { .. }) {
        return Some(CarryContext::ForEachPlayer);
    }
    if let EffectAst::ForEachTargetPlayers { count, .. } = effect {
        return Some(CarryContext::ForEachTargetPlayers(*count));
    }
    if matches!(effect, EffectAst::ForEachOpponent { .. }) {
        return Some(CarryContext::ForEachOpponent);
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::TargetOnly { target, .. } = &subject_verb.action
        && let Some(context) = player_target_carry_context(target)
    {
        return Some(context);
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::Exile { target, .. } = &subject_verb.action
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. },
        ..
    }) = effect
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileAll { filter, .. },
        ..
    }) = effect
        && let Some(owner) = filter.owner.as_ref()
        && let Some(player) = player_ast_from_filter_for_carry(owner)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::TapAll { filter },
        ..
    }) = effect
        && let Some(controller) = filter.controller.as_ref()
        && let Some(player) = player_ast_from_filter_for_carry(controller)
    {
        // In a clause such as "they tap all lands they control and lose all
        // unspent mana", the explicit player is represented by the tapped
        // objects' controller rather than the SubjectVerb subject.  Retain it
        // for the coordinated implicit player action that follows.
        return Some(CarryContext::Player(player));
    }
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ChoosePlayer { .. },
            ..
        })
    ) {
        return Some(CarryContext::Player(PlayerAst::That));
    }

    let player = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => {
            if !matches!(player, PlayerAst::Implicit) {
                *player
            } else if !matches!(chooser, PlayerAst::Implicit) {
                *chooser
            } else {
                return None;
            }
        }
        EffectAst::SubjectVerb(_) => subject_verb_player_action_player(effect)?,
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => *player,
        _ => return None,
    };

    if matches!(player, PlayerAst::Implicit) {
        None
    } else {
        Some(CarryContext::Player(player))
    }
}

pub(crate) fn effect_uses_implicit_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => matches!(*chooser, PlayerAst::Implicit) || matches!(*player, PlayerAst::Implicit),
        EffectAst::SubjectVerb(_) => {
            matches!(
                subject_verb_player_action_player(effect),
                Some(PlayerAst::Implicit)
            )
        }
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
            matches!(*player, PlayerAst::Implicit)
        }
        _ => false,
    }
}

fn effect_uses_that_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => matches!(*chooser, PlayerAst::That) || matches!(*player, PlayerAst::That),
        EffectAst::SubjectVerb(_) => {
            matches!(
                subject_verb_player_action_player(effect),
                Some(PlayerAst::That)
            )
        }
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
            matches!(*player, PlayerAst::That)
        }
        _ => false,
    }
}

fn subject_verb_player_action_player_mut(effect: &mut EffectAst) -> Option<&mut PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { player, .. },
            ..
        }) => Some(player),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::Draw { .. }
                | SubjectVerbActionAst::LoseLife { .. }
                | SubjectVerbActionAst::PayLife { .. }
                | SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::RevealHand
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::Scry { .. }
                | SubjectVerbActionAst::Surveil { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::DiscardHand
                | SubjectVerbActionAst::PoisonCounters { .. }
                | SubjectVerbActionAst::EnergyCounters { .. }
                | SubjectVerbActionAst::ExperienceCounters { .. }
                | SubjectVerbActionAst::TicketCounters { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayAnyLife { .. }
                | SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::DoubleManaPool
                | SubjectVerbActionAst::EmptyManaPool
                | SubjectVerbActionAst::SetLifeTotal { .. }
                | SubjectVerbActionAst::SkipTurn
                | SubjectVerbActionAst::EndTurn
                | SubjectVerbActionAst::SkipCombatPhases
                | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
                | SubjectVerbActionAst::SkipMainPhasesThisTurn
                | SubjectVerbActionAst::SkipCombatPhasesThisTurn
                | SubjectVerbActionAst::SkipDrawStep
                | SubjectVerbActionAst::RingTemptsYou
                | SubjectVerbActionAst::VentureIntoDungeon { .. }
                | SubjectVerbActionAst::BecomeMonarch
                | SubjectVerbActionAst::TakeInitiative
                | SubjectVerbActionAst::CreateEmblem { .. }
                | SubjectVerbActionAst::LoseGame
                | SubjectVerbActionAst::WinGame
                | SubjectVerbActionAst::FlipCoin
                | SubjectVerbActionAst::FlipCoinFaceOnly
                | SubjectVerbActionAst::RollDie { .. }
                | SubjectVerbActionAst::RollDiceChooseResult { .. }
                | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
                | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary { .. }
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
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::ReturnToBattlefield { .. }
                | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
                | SubjectVerbActionAst::ReturnToHand { .. }
                | SubjectVerbActionAst::ReturnAllToHand { .. }
                | SubjectVerbActionAst::MoveToZone { .. }
                | SubjectVerbActionAst::AdditionalLandPlays { .. }
                | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::Attach { .. }
                | SubjectVerbActionAst::ShuffleLibrary,
        }) => Some(player),
        _ => None,
    }
}

fn subject_verb_player_action_player(effect: &EffectAst) -> Option<PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { player, .. },
            ..
        }) => Some(*player),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::Draw { .. }
                | SubjectVerbActionAst::LoseLife { .. }
                | SubjectVerbActionAst::PayLife { .. }
                | SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::RevealHand
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::Scry { .. }
                | SubjectVerbActionAst::Surveil { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::DiscardHand
                | SubjectVerbActionAst::PoisonCounters { .. }
                | SubjectVerbActionAst::EnergyCounters { .. }
                | SubjectVerbActionAst::ExperienceCounters { .. }
                | SubjectVerbActionAst::TicketCounters { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayAnyLife { .. }
                | SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::DoubleManaPool
                | SubjectVerbActionAst::EmptyManaPool
                | SubjectVerbActionAst::SetLifeTotal { .. }
                | SubjectVerbActionAst::SkipTurn
                | SubjectVerbActionAst::EndTurn
                | SubjectVerbActionAst::SkipCombatPhases
                | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
                | SubjectVerbActionAst::SkipMainPhasesThisTurn
                | SubjectVerbActionAst::SkipCombatPhasesThisTurn
                | SubjectVerbActionAst::SkipDrawStep
                | SubjectVerbActionAst::RingTemptsYou
                | SubjectVerbActionAst::VentureIntoDungeon { .. }
                | SubjectVerbActionAst::BecomeMonarch
                | SubjectVerbActionAst::TakeInitiative
                | SubjectVerbActionAst::CreateEmblem { .. }
                | SubjectVerbActionAst::LoseGame
                | SubjectVerbActionAst::WinGame
                | SubjectVerbActionAst::FlipCoin
                | SubjectVerbActionAst::FlipCoinFaceOnly
                | SubjectVerbActionAst::RollDie { .. }
                | SubjectVerbActionAst::RollDiceChooseResult { .. }
                | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
                | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary { .. }
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
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::ReturnToBattlefield { .. }
                | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
                | SubjectVerbActionAst::ReturnToHand { .. }
                | SubjectVerbActionAst::ReturnAllToHand { .. }
                | SubjectVerbActionAst::MoveToZone { .. }
                | SubjectVerbActionAst::AdditionalLandPlays { .. }
                | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::Attach { .. }
                | SubjectVerbActionAst::ShuffleLibrary,
        }) => Some(*player),
        _ => None,
    }
}

pub(crate) fn maybe_apply_carried_player(effect: &mut EffectAst, carried_context: CarryContext) {
    match carried_context {
        CarryContext::Player(carried_player) => {
            // When carrying an explicit target player/opponent into an implicit clause,
            // bind to the previously selected target ("that player") instead of creating
            // a fresh explicit target. This preserves shared-target semantics for chains
            // like "Target player mills..., draws..., and loses...".
            let carried_player = match carried_player {
                PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
                other => other,
            };
            match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::SearchLibrary { player, .. },
                    ..
                }) => {
                    // A bare `search` is imperative: its omitted actor is the
                    // spell or ability's controller. A target introduced by
                    // "target player's library" is the library owner, not a
                    // grammatical subject to carry into the chooser slot.
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                EffectAst::SubjectVerb(_) => {
                    if let Some(player) = subject_verb_player_action_player_mut(effect)
                        && *player == PlayerAst::Implicit
                    {
                        *player = carried_player;
                    }
                }
                EffectAst::ChooseObjects { player, .. }
                | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                _ => {}
            }
        }
        CarryContext::ForEachPlayer => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachPlayer {
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachTargetPlayers(count) => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachTargetPlayers {
                    count,
                    filter: PlayerFilter::Any,
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachOpponent => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachOpponent {
                    effects: vec![wrapped],
                };
            }
        }
    }
}

pub(crate) fn maybe_apply_carried_player_with_clause_lexed(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    let facts = super::super::grammar::effects::coordination::recognize_coordination_clause_facts(
        clause_tokens,
    );
    maybe_apply_carried_player_with_clause_facts(effect, carried_context, facts);
}

fn maybe_apply_carried_player_with_clause_facts(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    facts: super::super::grammar::effects::coordination::CoordinationClauseFacts,
) {
    let imperative_collection_move = facts.imperative_collection_move
        && matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone { .. },
                ..
            })
        );
    let imperative_return = facts.imperative_return
        && matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: SubjectVerbSubjectAst {
                    player: PlayerAst::Implicit,
                    ..
                },
                action: SubjectVerbActionAst::ReturnToBattlefield { .. }
                    | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
                    | SubjectVerbActionAst::ReturnToHand { .. }
                    | SubjectVerbActionAst::ReturnAllToHand { .. },
            })
        );
    if facts.head == chain_grammar::CarryClauseHead::Choose
        && normalize_imperative_choose_player(effect)
    {
        return;
    }
    if facts.head == chain_grammar::CarryClauseHead::Create
        && normalize_imperative_create_player(effect)
    {
        return;
    }
    let should_skip = match carried_context {
        CarryContext::Player(_) => {
            imperative_return
                || (matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: SubjectVerbSubjectAst {
                            player: PlayerAst::Implicit,
                            ..
                        },
                        action: SubjectVerbActionAst::Draw { .. },
                    })
                ) && facts.head == chain_grammar::CarryClauseHead::Draw)
                    && !facts.explicitly_conjugated_player_action
                || (matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: SubjectVerbSubjectAst {
                            player: PlayerAst::Implicit,
                            ..
                        },
                        action: SubjectVerbActionAst::Scry { .. }
                            | SubjectVerbActionAst::Surveil { .. },
                    })
                ) && matches!(
                    facts.head,
                    chain_grammar::CarryClauseHead::Scry | chain_grammar::CarryClauseHead::Surveil
                ) && !facts.explicitly_conjugated_player_action)
        }
        CarryContext::ForEachPlayer
        | CarryContext::ForEachTargetPlayers(_)
        | CarryContext::ForEachOpponent => {
            let is_implicit_vision_effect = matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::Implicit,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw { .. }
                        | SubjectVerbActionAst::Scry { .. }
                        | SubjectVerbActionAst::Surveil { .. },
                })
            );
            imperative_collection_move
                || (is_implicit_vision_effect
                    && matches!(
                        facts.head,
                        chain_grammar::CarryClauseHead::Draw
                            | chain_grammar::CarryClauseHead::Scry
                            | chain_grammar::CarryClauseHead::Surveil
                    )
                    && !facts.explicitly_conjugated_player_action)
        }
    };
    if should_skip {
        return;
    }
    maybe_apply_carried_player(effect, carried_context);
}

fn normalize_imperative_choose_player(effect: &mut EffectAst) -> bool {
    let player = match effect {
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. }
        | EffectAst::ChooseObjectsAcrossZones { player, .. } => player,
        _ => return false,
    };

    if matches!(
        player,
        PlayerAst::Implicit | PlayerAst::Target | PlayerAst::TargetOpponent | PlayerAst::That
    ) {
        *player = PlayerAst::You;
        return true;
    }
    false
}

fn normalize_imperative_create_player(effect: &mut EffectAst) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::CreateTokenWithMods { player, .. },
        ..
    }) = effect
    else {
        return false;
    };

    if matches!(
        player,
        PlayerAst::Implicit | PlayerAst::Target | PlayerAst::TargetOpponent | PlayerAst::That
    ) {
        *player = PlayerAst::You;
        return true;
    }
    false
}

pub(crate) fn bind_implicit_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::RetargetStackObject { .. },
        }) => {
            if matches!(subject.player, PlayerAst::Implicit) {
                subject.player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CopySpell {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::CopySpellForEachTarget {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::CastTagged {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player: effect_player,
                    ..
                },
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    player: effect_player,
                    chooser,
                    ..
                },
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
            if matches!(*chooser, PlayerAst::Implicit) {
                *chooser = player;
            }
        }
        EffectAst::SubjectVerb(_) => {
            if let Some(effect_player) = subject_verb_player_action_player_mut(effect)
                && matches!(*effect_player, PlayerAst::Implicit)
            {
                *effect_player = player;
            }
        }
        EffectAst::ChooseObjects {
            player: effect_player,
            ..
        }
        | EffectAst::ChooseObjectsWithAggregateConstraint {
            player: effect_player,
            ..
        }
        | EffectAst::ChooseObjectsAcrossZones {
            player: effect_player,
            ..
        } => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                bind_implicit_player_context(nested_effect, player);
            }
        }),
    }
}

fn parse_leading_player_may_words(words: &[&str]) -> Option<PlayerAst> {
    type WordInput<'a> = grammar::WordSliceInput<'a>;
    use grammar::word_slice_exact as word_eq;

    fn player_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((word_eq("player"), word_eq("players"))).void()
    }

    fn opponent_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((word_eq("opponent"), word_eq("opponents"))).void()
    }

    fn controller_subject_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((
            word_eq("creatures"),
            word_eq("lands"),
            word_eq("permanents"),
            word_eq("planeswalkers"),
            word_eq("sources"),
            word_eq("spells"),
        ))
        .void()
    }

    fn controller_or_owner_subject_word<'a>()
    -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((
            word_eq("creatures"),
            word_eq("lands"),
            word_eq("permanents"),
            word_eq("sources"),
            word_eq("spells"),
        ))
        .void()
    }

    fn leading_conjunctions<'a>(input: &mut WordInput<'a>) -> Result<(), ErrMode<ContextError>> {
        repeat::<_, _, (), _, _>(0.., alt((word_eq("then"), word_eq("and")))).parse_next(input)
    }

    fn parse_player_may_prefix<'a>(
        input: &mut WordInput<'a>,
    ) -> Result<PlayerAst, ErrMode<ContextError>> {
        (
            leading_conjunctions,
            alt((
                alt((
                    (word_eq("you"), word_eq("may")).value(PlayerAst::You),
                    (word_eq("any"), player_word(), word_eq("may")).value(PlayerAst::Any),
                    (word_eq("any"), opponent_word(), word_eq("may")).value(PlayerAst::Opponent),
                )),
                alt((
                    (word_eq("target"), opponent_word(), word_eq("may"))
                        .value(PlayerAst::TargetOpponent),
                    (word_eq("target"), player_word(), word_eq("may")).value(PlayerAst::Target),
                    (word_eq("that"), player_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("that"), opponent_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("they"), word_eq("may")).value(PlayerAst::That),
                    (
                        word_eq("that"),
                        word_eq("player"),
                        word_eq("or"),
                        word_eq("that"),
                        controller_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ThatPlayerOrTargetController),
                    (
                        word_eq("that"),
                        controller_or_owner_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (
                        word_eq("that"),
                        controller_or_owner_subject_word(),
                        word_eq("owner"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsOwner),
                )),
                alt((
                    (word_eq("the"), player_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("defending"), word_eq("player"), word_eq("may"))
                        .value(PlayerAst::Defending),
                    alt((
                        (word_eq("attacking"), word_eq("player"), word_eq("may"))
                            .value(PlayerAst::Attacking),
                        (
                            word_eq("that"),
                            word_eq("attacking"),
                            word_eq("player"),
                            word_eq("may"),
                        )
                            .value(PlayerAst::Attacking),
                        (
                            word_eq("the"),
                            word_eq("attacking"),
                            word_eq("player"),
                            word_eq("may"),
                        )
                            .value(PlayerAst::Attacking),
                    )),
                    (
                        alt((word_eq("its"), word_eq("their"))),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (
                        alt((word_eq("its"), word_eq("their"))),
                        word_eq("owner"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsOwner),
                    alt((
                        (opponent_word(), word_eq("may")).value(PlayerAst::Opponent),
                        (word_eq("an"), word_eq("opponent"), word_eq("may"))
                            .value(PlayerAst::Opponent),
                    )),
                )),
            )),
        )
            .map(|(_, player)| player)
            .parse_next(input)
    }

    let mut input = words;
    parse_player_may_prefix(&mut input).ok()
}

pub(crate) fn parse_leading_player_may_lexed(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    parse_leading_player_may_words(&words)
}

pub(crate) fn find_verb(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    find_verb_lexed(tokens)
}

pub(crate) fn parse_effect_chain(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_lexed(tokens)
}

pub(crate) fn parse_effect_chain_with_subject_verb_primitives(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_with_subject_verb_primitives_lexed(tokens)
}

pub(crate) fn parse_effect_chain_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_inner_lexed(tokens)
}

pub(crate) fn parse_effect_clause_with_trailing_if(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_clause_with_trailing_if_lexed(tokens)
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_next_end_step_exile_followup_lexed(effects, tokens);
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_end_of_combat_exile_followup_lexed(effects, tokens);
}

pub(crate) fn maybe_apply_carried_player_with_clause(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    maybe_apply_carried_player_with_clause_lexed(effect, carried_context, clause_tokens);
}

pub(crate) fn remove_first_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    remove_first_may_word_tokens(tokens)
}

pub(crate) fn remove_through_first_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    remove_through_first_may_word_tokens(tokens)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
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
