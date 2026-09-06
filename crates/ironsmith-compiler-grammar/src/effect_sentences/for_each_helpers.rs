use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IfResultPredicate, OwnedLexToken, PlayerAst,
    PredicateAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey,
    TargetAst, TokenActionAst, ObjectChoiceEffectAst, PermissionEffectAst, PlayerPredicateAst,
};
use crate::diagnostics::TextSpan;
use crate::effect::{Until, Value};
use crate::target::{ObjectFilter, ObjectRef, PlayerFilter};
use ironsmith_core::ValueSurfaceHint;

use super::super::effect_ast_traversal::{for_each_nested_effects, for_each_nested_effects_mut};
use super::super::grammar::effects::for_each_shapes::{
    self, ForEachParticipantScope, ManaClauseShape, ModifierTailAction, OpponentSpecialShape,
    RelativeControlClauseShape, WhoClauseShape,
};
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::LexedClause;
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    comparison_to_value_comparison_operator, parse_for_each_count_value_words, parse_target_phrase,
    replace_unbound_x_with_value, value_contains_unbound_x,
};
use super::chain_carry::bind_implicit_player_context;
use super::chain_carry::{parse_effect_chain, parse_effect_chain_inner, remove_first_word};
use super::conditionals::parse_for_each_doesnt_control_lose_game;
use super::dispatch_entry::replace_unbound_x_in_effects_anywhere;

fn has_independent_participant_continuation(tokens: &[OwnedLexToken]) -> bool {
    // The consequence after "who can't, ..." belongs to the quantified
    // failure clause even when it names a different actor explicitly.
    if for_each_shapes::parse_participant_clause_shape(tokens)
        .is_some_and(|shape| for_each_shapes::parse_who_clause_shape(shape.inner_tokens).is_some())
    { return false; }
    use crate::grammar::effects::{coordination, typed_clause_heads::ClauseActorHeadAst};
    matches!(coordination::recognize_coordination(tokens),
        crate::recognition::ParseOutcome::Match(plan)
            if plan.value.members.iter().skip(1).any(|member| member.head.is_some_and(|head|
                matches!(head.actor, ClauseActorHeadAst::Controller | ClauseActorHeadAst::Player)
                    && !member.tokens.first().is_some_and(|token| token.is_word("that"))
            ))
    )
}

fn prepend_that_player_life_total_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    if !for_each_shapes::starts_life_total_becomes(tokens) {
        return tokens.to_vec();
    }
    prepend_that_player_subject(tokens)
}

fn prepend_that_player_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut rewritten = Vec::with_capacity(tokens.len() + 2);
    rewritten.push(OwnedLexToken::word(
        "that".to_string(),
        TextSpan::synthetic(),
    ));
    rewritten.push(OwnedLexToken::word(
        "player".to_string(),
        TextSpan::synthetic(),
    ));
    rewritten.extend_from_slice(tokens);
    rewritten
}

pub fn parse_for_each_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_object_subject_shape(subject_tokens) else {
        return Ok(None);
    };
    Ok(Some(parse_for_each_object_filter(shape.filter_tokens)?))
}

pub fn parse_for_each_object_filter(
    filter_tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let mut filter = parse_object_filter(filter_tokens, false)?;
    let words = crate::lexer::token_word_refs(filter_tokens);
    // Quantified subjects use the older family-level object-filter parser,
    // so they do not pass through the grammar filter finalizer that normally
    // restores this exact coordinated Stack domain. Reassert only the
    // grammar-proven terminal noun phrase here; ordinary spell filters must
    // retain their mana-cost predicate and Spell-only domain.
    if crate::word_primitives::any_sequence_occurs(
        &words,
        &[
            &["spell", "and", "ability"],
            &["spell", "and", "abilities"],
            &["spells", "and", "ability"],
            &["spells", "and", "abilities"],
        ],
    ) {
        filter.zone = Some(crate::zone::Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        filter.has_mana_cost = false;
        filter.set_conjunctive_set_surface(true);
    }
    let owner_index = crate::word_primitives::parse_sequence_start(&words, &["you", "own"]);
    let zone_index = crate::slice_primitives::select_position(&words, |word| {
        matches!(
            *word,
            "battlefield" | "graveyard" | "hand" | "library" | "exile" | "command"
        )
    });
    if owner_index
        .zip(zone_index)
        .is_some_and(|(owner, zone)| owner < zone)
    {
        filter.set_owner_before_zone_surface(true);
    }
    let counter_index = crate::slice_primitives::select_last_position(&words, |word| {
        matches!(*word, "counter" | "counters")
    });
    if zone_index
        .zip(counter_index)
        .is_some_and(|(zone, counter)| zone < counter)
    {
        filter.set_counter_requirement_after_zone_surface(true);
    }
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("those"))
    {
        filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
    }
    Ok(filter)
}

pub fn parse_for_each_targeted_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_target_subject_shape(subject_tokens) else {
        return Ok(None);
    };
    let target = match parse_target_phrase(shape.target_tokens) {
        Ok(target) => target,
        Err(_) => return Ok(None),
    };
    let TargetAst::WithCount(inner, count) = target else {
        return Ok(None);
    };
    let TargetAst::Object(filter, _, _) = *inner else {
        return Ok(None);
    };
    Ok(Some((filter, count)))
}

pub fn is_target_player_dealt_damage_by_this_turn_subject(words: &[&str]) -> bool {
    for_each_shapes::is_target_player_damage_subject_words(words)
}

pub fn is_mana_replacement_clause_words(words: &[&str]) -> bool {
    for_each_shapes::parse_mana_clause_shape_words(words) == Some(ManaClauseShape::Replacement)
}

pub fn is_mana_trigger_additional_clause_words(words: &[&str]) -> bool {
    for_each_shapes::parse_mana_clause_shape_words(words)
        == Some(ManaClauseShape::AdditionalTrigger)
}

pub fn parse_has_base_power_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_base_power_clause_shape(tokens)? else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    Ok(Some(EffectAst::subject_verb_set_base_power(
        shape.power,
        target,
        shape.duration,
    )))
}

pub fn parse_has_base_power_toughness_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Copular animation clauses such as "Each of them is a 1/1 Spirit in
    // addition to its other types" carry type/subtype semantics beyond the
    // P/T assignment. Leave those for the complete animation parser instead
    // of consuming only their leading P/T fragment here.
    if super::super::grammar::effects::clause_dispatch_shapes::parse_copular_animation_shape(tokens)
        .is_some()
    {
        return Ok(None);
    }
    let Some(shape) = for_each_shapes::parse_base_power_toughness_clause_shape(tokens)? else {
        return Ok(None);
    };
    // "<subject> lose all abilities and have base power and toughness N/N"
    // — the target phrase must not eat the remove-abilities half.
    let mut target_tokens = shape.target_tokens;
    let mut lose_all_abilities = false;
    {
        let mut end = target_tokens.len();
        if end > 0 && target_tokens[end - 1].is_word("and") {
            end -= 1;
        }
        if end >= 3
            && (target_tokens[end - 3].is_word("lose") || target_tokens[end - 3].is_word("loses"))
            && target_tokens[end - 2].is_word("all")
            && target_tokens[end - 1].is_word("abilities")
        {
            lose_all_abilities = true;
            target_tokens = &target_tokens[..end - 3];
        }
    }
    let target = parse_target_phrase(target_tokens)?;
    if lose_all_abilities && shape.where_x_tokens.is_none() {
        let set_pt = EffectAst::subject_verb_set_base_power_toughness(
            shape.power.clone(),
            shape.toughness.clone(),
            target.clone(),
            shape.duration.clone(),
        );
        let remove = EffectAst::subject_verb_remove_abilities_from_target(
            target,
            Vec::new(),
            shape.duration,
        );
        return Ok(Some(EffectAst::Sequence {
            effects: vec![remove, set_pt],
        }));
    }
    let (mut power, mut toughness) = (shape.power, shape.toughness);
    if let Some(where_x_tokens) = shape.where_x_tokens {
        let clause = LexedClause::new(tokens).text();
        if !value_contains_unbound_x(&power) && !value_contains_unbound_x(&toughness) {
            return Err(CardTextError::ParseError(format!(
                "where-X base power/toughness clause missing X value (clause: '{}')",
                clause
            )));
        }
        let x_value = parse_value_binding_clause(where_x_tokens)
            .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-X base power/toughness clause (clause: '{}')",
                    clause
                ))
            })?;
        power = replace_unbound_x_with_value(power, &x_value, &clause)?;
        toughness = replace_unbound_x_with_value(toughness, &x_value, &clause)?;
    }
    Ok(Some(EffectAst::subject_verb_set_base_power_toughness(
        power,
        toughness,
        target,
        shape.duration,
    )))
}

pub fn parse_get_for_each_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_target_subject_shape(tokens) else {
        return Ok(None);
    };
    if let Some(value) =
        crate::grammar::shared_util::value_semantics::parse_turn_history_count_value(tokens)
    {
        return Ok(Some(value.with_surface_hint(ValueSurfaceHint::ForEach)));
    }
    // Parse the authored object phrase at the grammar boundary before the
    // broad value vocabulary is considered. This preserves punctuation and
    // subtype-list provenance without a downstream semantic repair pass.
    let authored_filter =
        crate::grammar::primitives::probe_shape(parse_object_filter(shape.target_tokens, false));
    let words = LexedClause::new(tokens).word_refs();
    let Some((value, _)) = parse_for_each_count_value_words(&words) else {
        return Err(CardTextError::ParseError(
            "missing filter after 'for each' in gets clause".to_string(),
        ));
    };
    let exact_surface_filter = |original: ObjectFilter| {
        let Some(authored) = authored_filter.clone() else {
            return original;
        };
        // The authored filter is only a surface enrichment. Preserve a
        // specialized semantic count (for example, the cast-time snapshot tag
        // for "modified creatures you controlled as you cast this spell")
        // whenever the ordinary object-filter parse denotes a different set.
        if authored == original {
            authored
        } else {
            original
        }
    };
    let value = match value {
        Value::Count(original) => Value::Count(exact_surface_filter(original)),
        Value::CountScaled(original, multiplier) => {
            Value::CountScaled(exact_surface_filter(original), multiplier)
        }
        other => other,
    };
    Ok(Some(value.with_surface_hint(ValueSurfaceHint::ForEach)))
}

pub fn parse_get_modifier_values_with_tail(
    modifier_tokens: &[OwnedLexToken],
    power: Value,
    toughness: Value,
) -> Result<(Value, Value, Until, Option<PredicateAst>), CardTextError> {
    let clause = LexedClause::new(modifier_tokens).text();
    let mut out_power = power;
    let mut out_toughness = toughness;
    let shape = for_each_shapes::parse_modifier_tail_shape(modifier_tokens);

    if let ModifierTailAction::DynamicForEach(count_tokens) = shape.action {
        let Some(count) = parse_get_for_each_count_value(count_tokens)? else {
            return Err(CardTextError::ParseError(
                "missing filter after 'for each' in gets clause".to_string(),
            ));
        };
        let scale_modifier = |modifier: Value| -> Result<Value, CardTextError> {
            match modifier {
                Value::Fixed(0) => Ok(Value::Fixed(0)),
                Value::Fixed(1) => Ok(count.clone()),
                Value::Fixed(multiplier) => Ok(Value::Scaled(Box::new(count.clone()), multiplier)),
                other if value_contains_unbound_x(&other) => {
                    replace_unbound_x_with_value(other, &count, &clause)
                }
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported dynamic gets-for-each clause (clause: '{}')",
                    clause
                ))),
            }
        };
        out_power = scale_modifier(out_power)?;
        out_toughness = scale_modifier(out_toughness)?;
        return Ok((out_power, out_toughness, shape.duration, shape.condition));
    }

    let ModifierTailAction::WhereX(binding_tokens) = shape.action else {
        return match shape.action {
            ModifierTailAction::Complete => {
                Ok((out_power, out_toughness, shape.duration, shape.condition))
            }
            ModifierTailAction::Unsupported => Err(CardTextError::ParseError(format!(
                "unsupported trailing gets clause (clause: '{}')",
                clause
            ))),
            ModifierTailAction::DynamicForEach(_) | ModifierTailAction::WhereX(_) => unreachable!(),
        };
    };
    if !value_contains_unbound_x(&out_power) && !value_contains_unbound_x(&out_toughness) {
        return Err(CardTextError::ParseError(format!(
            "where-X gets clause missing X modifier (clause: '{}')",
            clause
        )));
    }
    let x_value =
        crate::grammar::effects::sentence_predicate_shapes::parse_where_x_value_shape_tokens(
            binding_tokens,
            false,
        )
        .and_then(super::dispatch_inner::lower_where_x_shape)
        .map(|(_, value)| value)
        .or_else(|| parse_value_binding_clause(binding_tokens))
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-X gets clause (clause: '{}')",
                clause
            ))
        })?;
    out_power = replace_unbound_x_with_value(out_power, &x_value, &clause)?;
    out_toughness = replace_unbound_x_with_value(out_toughness, &x_value, &clause)?;
    Ok((out_power, out_toughness, shape.duration, shape.condition))
}

pub fn force_implicit_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { player, .. })
                    | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { player, .. })
                    | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { player, .. }),
                ..
            }) => {
                if matches!(*player, PlayerAst::Implicit) {
                    *player = PlayerAst::You;
                }
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                force_implicit_token_controller_you(nested);
            }),
        }
    }
}

fn bind_implicit_choose_chooser(effects: &mut [EffectAst], chooser: PlayerAst) {
    for effect in effects {
        match effect {
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { player, .. })
                if matches!(*player, PlayerAst::Implicit) =>
            {
                *player = chooser;
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                bind_implicit_choose_chooser(nested, chooser);
            }),
        }
    }
}

/// Give a participant's standalone choice a stable aggregate tag.
///
/// An implicit `__it__` choice is normally iteration-local: the runtime
/// replaces it for the next participant so an immediate nested consumer sees
/// only that participant's choice. When the participant clause consists only
/// of the choice, however, a later sentence can refer to the union ("chosen
/// this way"). Use a source-anchored explicit tag so those selections
/// accumulate across the participant loop without colliding with another
/// standalone participant choice in the same ability.
fn stabilize_standalone_participant_choice_tag(
    effects: &mut [EffectAst],
    source_tokens: &[OwnedLexToken],
) {
    let [effect] = effects else {
        return;
    };
    let tag = match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { tag, .. }) => tag,
        _ => return,
    };
    if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() {
        return;
    }
    let anchor = source_tokens
        .first()
        .map(|token| token.span)
        .unwrap_or_else(TextSpan::synthetic);
    *tag = crate::tag::CompilerProvenanceTag::ParticipantChoice {
        line: anchor.line,
        start: anchor.start,
    }
    .key();
}

fn tagged_predicate(filter_tokens: Option<&[OwnedLexToken]>) -> Option<PredicateAst> {
    let filter =
        crate::grammar::primitives::probe_shape(parse_object_filter(filter_tokens?, false))?;
    Some(PredicateAst::Player(PlayerPredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: crate::tag::CompilerReferenceTag::It.bind(),
        filter,
        mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
    }))
}

fn parse_maybe_effects(
    tokens: &[OwnedLexToken],
    parse_inner: bool,
    scope_may_to_that_player: bool,
) -> Result<Vec<EffectAst>, CardTextError> {
    let parse_body = |tokens: &[OwnedLexToken]| -> Result<Vec<EffectAst>, CardTextError> {
        // The inner chain dispatcher deliberately skips standalone front doors
        // to avoid rediscovering its outer quantified clause. Preserve the
        // complete typed create leaf explicitly before entering that dispatcher;
        // otherwise adjective conjunctions in token definitions can fall
        // through to a lossy generic subject/verb interpretation.
        if let Some(effects) = super::parse_complete_create_statement(tokens)? {
            return Ok(effects);
        }
        if parse_inner {
            parse_effect_chain_inner(tokens)
        } else {
            parse_effect_chain(tokens)
        }
    };

    if !for_each_shapes::contains_may(tokens) {
        return parse_body(tokens);
    }
    let stripped = remove_first_word(tokens);
    let mut effects = parse_body(&stripped)?;
    if scope_may_to_that_player {
        // The quantified wrapper supplies the actor. Keep the inner Oracle
        // imperative in its base-verb form (`copy`, `choose`, ...), then bind
        // otherwise implicit player roles to the iterated participant. Adding
        // a synthetic `that player` subject ahead of an uninflected verb
        // creates invalid prose such as `that player copy that spell` and
        // prevents the typed verb registry from claiming the action.
        for effect in &mut effects {
            bind_implicit_player_context(effect, PlayerAst::That);
        }
    }
    Ok(vec![EffectAst::Permissions(PermissionEffectAst::May { effects })])
}

/// Parse a coordinated action program whose actor is supplied by an outer
/// quantified-participant clause. The wrapper owns the player subject, so
/// each materialized member receives that subject exactly once and is parsed
/// through the inner dispatcher. This prevents a multi-action body from
/// rediscovering its outer `each player`/`each opponent` sentence.
fn parse_quantified_participant_actor_program(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // An `or` inside a trailing unless payment belongs to the payer, not to
    // the quantified participant's outer action program. Leave the complete
    // body to the unless parser so it can materialize `TotalCost::OneOf`
    // instead of repeating the second payment arm after the consequence.
    if super::has_unless_payment_choice(tokens)? {
        return Ok(None);
    }

    fn starts_object_domain_list_arm(words: &[&str]) -> bool {
        let words = if crate::word_primitives::first_is_any(words, &["and", "or", "and/or"]) {
            &words[1..]
        } else {
            words
        };
        let words = if crate::word_primitives::first_is_any(
            words,
            &["basic", "nonbasic", "token", "nontoken"],
        ) {
            &words[1..]
        } else {
            words
        };
        crate::word_primitives::first_is_any(
            words,
            &[
                "artifact",
                "artifacts",
                "battle",
                "battles",
                "creature",
                "creatures",
                "enchantment",
                "enchantments",
                "instant",
                "instants",
                "land",
                "lands",
                "planeswalker",
                "planeswalkers",
                "sorcery",
                "sorceries",
                "kindred",
            ],
        )
    }

    let plan = match super::super::grammar::effects::coordination::recognize_coordination(tokens) {
        crate::recognition::ParseOutcome::Match(matched) => matched.value,
        crate::recognition::ParseOutcome::NoMatch => return Ok(None),
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    };
    let raw_segments = plan.member_segments();
    if raw_segments.len() > 1
        && raw_segments.iter().skip(1).all(|segment| {
            let words = crate::lexer::parser_token_word_refs(segment);
            starts_object_domain_list_arm(&words)
        })
    {
        // A comma-separated object-domain union is one operand of the
        // quantified action, not a coordinated action program. Leave the
        // complete clause to the ordinary subject/verb parser so qualifiers
        // such as the terminal `nonbasic` in
        // `artifacts, enchantments, and nonbasic lands` stay on their arm.
        return Ok(None);
    }
    let segments = plan.materialized_segments().unwrap_or(raw_segments);
    if segments.len() < 2 {
        return Ok(None);
    }
    let member_count = segments.len();

    let mut effects = Vec::new();
    for segment in segments {
        let normalized = prepend_that_player_subject(&segment);
        let mut segment_effects = parse_maybe_effects(&normalized, true, true)?;
        if !segment.first().is_some_and(|token| token.is_word("you")) {
            bind_quantified_participant_actor(&mut segment_effects);
        }
        effects.extend(segment_effects);
    }
    if effects.len() == member_count
        && let Some(coordination) = plan.into_ast(effects.clone())
    {
        return Ok(Some(vec![EffectAst::Coordination(coordination)]));
    }
    Ok(Some(effects))
}

fn bind_quantified_participant_actor(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action:
                    SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { player, .. })
                    | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { player, .. })
                    | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { player, .. }),
            }) => {
                if subject.role == SubjectVerbRoleAst::Actor
                    && matches!(subject.player, PlayerAst::Implicit | PlayerAst::You)
                {
                    subject.player = PlayerAst::That;
                }
                if matches!(*player, PlayerAst::Implicit | PlayerAst::You) {
                    *player = PlayerAst::That;
                }
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst { subject, .. }) => {
                if subject.role == SubjectVerbRoleAst::Actor
                    && matches!(subject.player, PlayerAst::Implicit | PlayerAst::You)
                {
                    subject.player = PlayerAst::That;
                }
            }
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { player, .. })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { player, .. }) => {
                if matches!(*player, PlayerAst::Implicit | PlayerAst::You) {
                    *player = PlayerAst::That;
                }
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                bind_quantified_participant_actor(nested);
            }),
        }
    }
}

fn find_word_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> Option<usize> {
    super::super::grammar::primitives::find_prefix(tokens, || {
        super::super::grammar::primitives::phrase(phrase)
    })
    .map(|(idx, _, _)| idx)
}

/// Normalize Oracle's prose upper bound
/// "a number of <objects> less than or equal to the difference" to the typed
/// dynamic-choice surface understood by the ordinary search parser. The
/// caller subsequently binds X to the actual count difference, so runtime
/// semantics do not depend on the authored shorthand.
fn rewrite_difference_bounded_search(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    const COUNT_PREFIX: &[&str] = &["a", "number", "of"];
    const DIFFERENCE_SUFFIX: &[&str] = &["less", "than", "or", "equal", "to", "the", "difference"];

    let prefix = find_word_phrase(tokens, COUNT_PREFIX)?;
    let selector_start = prefix + COUNT_PREFIX.len();
    let suffix_relative = find_word_phrase(tokens.get(selector_start..)?, DIFFERENCE_SUFFIX)?;
    let suffix = selector_start + suffix_relative;
    if suffix == selector_start {
        return None;
    }

    let mut rewritten =
        Vec::with_capacity(tokens.len() + 3 - COUNT_PREFIX.len() - DIFFERENCE_SUFFIX.len());
    rewritten.extend_from_slice(&tokens[..prefix]);
    for word in ["up", "to", "x"] {
        rewritten.push(OwnedLexToken::word(word.to_string(), TextSpan::synthetic()));
    }
    rewritten.extend_from_slice(&tokens[selector_start..suffix]);
    rewritten.extend_from_slice(&tokens[suffix + DIFFERENCE_SUFFIX.len()..]);
    Some(rewritten)
}

/// Recover an authored where-X binding from the body of a participant-scoped
/// clause. These clauses are lowered before the ordinary sentence-level
/// where-X pass, so a binding that follows a complete search procedure (for
/// example, after its battlefield destination) would otherwise leave the
/// search's dynamic choice count unbound.
fn parse_participant_body_where_x_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let where_idx = find_word_phrase(tokens, &["where", "x", "is"])?;
    parse_value_binding_clause(&tokens[where_idx..])
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
}

#[cfg(test)]
#[path = "for_each_helpers_inline_dynamic_modifier_surface_tests.rs"]
mod dynamic_modifier_surface_tests;

#[cfg(test)]
#[path = "for_each_helpers_inline_participant_choice_ownership_tests_2.rs"]
mod participant_choice_ownership_tests;

#[path = "for_each_helpers/participant_scopes.rs"]
mod participant_scopes;
use participant_scopes::{
    opponent_filter, player_filter, reanchor_other_player_copy_filter, wrap_players,
};
pub use participant_scopes::{parse_for_each_player_clause, parse_for_each_target_players_clause};
#[path = "for_each_helpers/combat_history.rs"]
mod combat_history;
use combat_history::parse_combat_damage_history_participant;
#[path = "for_each_helpers/predicates.rs"]
mod predicates;
pub use predicates::parse_who_did_this_way_predicate;
#[path = "for_each_helpers/opponent_iteration.rs"]
mod opponent_iteration;
pub use opponent_iteration::parse_for_each_opponent_clause;
use opponent_iteration::wrap_opponents;
#[path = "for_each_helpers/relative_control.rs"]
mod relative_control;
use relative_control::parse_relative_control_conditional;
#[path = "for_each_helpers/triggering_stack_copy.rs"]
mod triggering_stack_copy;
use triggering_stack_copy::effect_copies_triggering_stack_object;
#[path = "for_each_helpers/participant_choices.rs"]
mod participant_choices;
use participant_choices::{
    parse_participant_choice_complement_effects, parse_participant_creature_type_choice,
};
