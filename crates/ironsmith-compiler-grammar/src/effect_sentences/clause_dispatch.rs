pub use self::become_clause::parse_become_clause;
use self::helpers::{parse_controller_or_owner_of_target_subject, render_lower_words};
use self::next_turn_cant::parse_next_turn_cant_clause;
use super::super::activation_and_restrictions::{
    build_may_cast_tagged_effect, find_negation_span, parse_cant_restrictions,
    parse_choose_card_type_phrase_words, parse_choose_color_phrase_words,
    parse_choose_creature_type_phrase_words, parse_choose_player_phrase_words,
    parse_may_cast_it_sentence, parse_single_word_keyword_action,
    parse_target_player_choose_objects_clause_with_count_value,
    parse_you_choose_objects_clause_with_count_value, parse_you_choose_player_clause,
    starts_with_target_indicator,
};
use super::super::grammar::choices::{
    ChoiceClauseActor, parse_choice_clause_head_tokens, parse_choice_land_type_phrase_words,
    parse_choice_subtype_family_phrase_words,
};
use super::super::grammar::effects as effect_grammar;
use super::super::grammar::effects::clause_dispatch_shapes as clause_grammar;
use super::super::grammar::effects::followup_shapes as followup_grammar;
use super::super::grammar::effects::parse_mana_replacement_clause_spec_lexed;
use super::super::grammar::primitives::TokenWordView;
use super::super::grammar::structure::{
    parse_predicate_with_grammar_entrypoint_lexed, split_trailing_if_clause_lexed,
};
use super::super::keyword_static::{parse_ability_line, parse_pt_modifier_values};
use super::super::lexer::{
    LexedClause, OwnedLexToken, contains_token_word, parser_token_word_positions, trim_lexed_commas,
};
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    parse_additional_land_plays_clause, parse_cast_or_play_tagged_clause,
};
use super::super::util::{
    parse_subject, parse_target_phrase, parse_value, parser_trace, parser_trace_stack,
    source_reference_surface_for_words, span_from_tokens, trim_commas,
};
use super::clause_primitives::run_clause_primitives;
use super::dispatch_inner::{
    parse_additional_phase_sentence, parse_prevent_damage_sentence, parse_take_extra_turn_sentence,
    trim_edge_punctuation,
};
use super::for_each_helpers::{
    is_mana_replacement_clause_words, is_mana_trigger_additional_clause_words,
    is_target_player_dealt_damage_by_this_turn_subject, parse_for_each_object_subject,
    parse_get_for_each_count_value, parse_get_modifier_values_with_tail,
    parse_has_base_power_clause, parse_has_base_power_toughness_clause,
};
use super::search_library::parse_restriction_duration;
use super::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, find_unquoted_token_word,
    parse_sentence_delayed_next_step_unless_pays, try_build_unless,
};
use super::verb_dispatch::parse_effect_with_verb;
use super::verb_handlers::{parse_control_duration, parse_deal_damage};
use super::zone_counter_helpers::parse_put_counters;
use super::zone_handlers::{
    collapse_leading_signed_pt_modifier_tokens,
    parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control, parse_return,
    parse_sacrifice,
};
use super::{
    Verb, bind_implicit_player_context, find_verb, parse_effect_chain_with_subject_verb_primitives,
    parse_simple_gain_ability_clause, parse_simple_lose_ability_clause,
};
use crate::TagKey;
use crate::cards::builders::{
    ABILITY_CONTROLLER_TARGET_CHOICE_TAG, CardTextError, ChooseOneModeAst, EffectAst,
    GrantedAbilityAst, IT_TAG, OPPONENT_TARGET_CHOICE_TAG, PlayerAst, ReturnControllerAst,
    SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TargetAst,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityId;
use crate::target::{ObjectFilter, PlayerFilter, SourceReferenceSurface};
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;

mod become_clause;
mod helpers;
mod next_turn_cant;

type ClauseDispatchCompatWords<'a> = TokenWordView<'a>;

const TARGET_WORD: &str = "target";

fn target_object_filter_mut(target: &mut TargetAst) -> Option<&mut ObjectFilter> {
    match target {
        TargetAst::Object(filter, ..) | TargetAst::ObjectOrPlayer(filter, ..) => Some(filter),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
            target_object_filter_mut(inner)
        }
        _ => None,
    }
}

/// Parse a participant-owned object choice followed by a return of exactly
/// that chosen set. The explicit choice actor and tagged return are both
/// executable; this is intentionally narrower than ordinary coordinated
/// prose so a later unrelated return cannot capture the choice.
fn parse_participant_choice_then_return_chosen_set(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(and_idx) = tokens.iter().enumerate().find_map(|(idx, token)| {
        (token.is_word("and")
            && tokens
                .get(idx + 1)
                .is_some_and(|next| next.is_word("return") || next.is_word("returns")))
        .then_some(idx)
    }) else {
        return Ok(None);
    };
    let tail_words = crate::lexer::token_word_refs(&tokens[and_idx + 1..]);
    if !crate::word_primitives::parse_choice_sequence_complete(
        &tail_words,
        &[
            &["return", "returns"],
            &["them"],
            &["to"],
            &["their"],
            &["owner", "owners", "owner's", "owners'"],
            &["hand", "hands"],
        ],
    ) {
        return Ok(None);
    }
    let choice_tokens = trim_commas(&tokens[..and_idx]);
    let Some((chooser, filter, count, count_value)) =
        parse_target_player_choose_objects_clause_with_count_value(&choice_tokens)?
    else {
        return Ok(None);
    };
    let chosen_tag = TagKey::from(IT_TAG);
    Ok(Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::ChooseObjects {
                filter,
                count,
                count_value,
                player: chooser,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_return_all_to_hand(ObjectFilter::tagged(chosen_tag)),
        ],
    }))
}

fn player_filter_mentions_source_object(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag))
        | PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag)) => {
            tag.as_str() == crate::tag::SOURCE_OBJECT_TAG
        }
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
            player_filter_mentions_source_object(inner)
        }
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
        | PlayerFilter::HasMoreLifeThanYou { base }
        | PlayerFilter::MaxSpeed { base, .. }
        | PlayerFilter::WasDealtDamageBySourceThisGame { base }
        | PlayerFilter::LostLifeThisTurn { base }
        | PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, .. } => {
            player_filter_mentions_source_object(base)
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_mentions_source_object(base)
                || player_filter_mentions_source_object(excluded)
        }
        _ => false,
    }
}

fn target_player_mentions_source_object(target: &TargetAst) -> bool {
    match target {
        TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) => {
            player_filter_mentions_source_object(filter)
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
            target_player_mentions_source_object(inner)
        }
        _ => false,
    }
}

/// Preserve an explicit grammatical object as the source of the damage it
/// deals.
///
/// The ordinary player/action dispatcher intentionally reduces most subjects
/// to `SubjectAst`, which has no object-target variant. That is correct for
/// "you/that player" actions, but it used to discard the target in sentences
/// such as "target enchantment deals ..." and the antecedent in "it deals ...
/// to each creature blocking it". Lower these clauses through the existing
/// explicit-source damage action so targeting, source-relative values,
/// controller references, and recipient relations all share one object.
fn bind_explicit_damage_subject_characteristics_to_source(value: &mut Value) {
    match value {
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => {
            bind_explicit_damage_subject_characteristics_to_source(value);
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            bind_explicit_damage_subject_characteristics_to_source(left);
            bind_explicit_damage_subject_characteristics_to_source(right);
        }
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::CountersOn(spec, _)
        | Value::ManaSymbolsInManaCostOf { spec, .. }
            if matches!(
                spec.base(),
                crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG
            ) =>
        {
            *spec = Box::new(crate::target::ChooseSpec::Source);
        }
        _ => {}
    }
}

fn parse_explicit_target_object_damage_source(
    subject_tokens: &[OwnedLexToken],
    action_tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let explicitly_targeted = subject_tokens
        .first()
        .is_some_and(|token| token.is_word(TARGET_WORD));
    let anaphoric_object = crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(subject_tokens),
        &["it"],
    );
    if !explicitly_targeted && !anaphoric_object {
        return Ok(None);
    }

    let parsed = parse_deal_damage(action_tokens)?;
    let EffectAst::SubjectVerb(parsed) = parsed else {
        return Ok(None);
    };
    let source = if explicitly_targeted {
        let source = parse_target_phrase(subject_tokens)?;
        if !matches!(
            &source,
            TargetAst::Object(_, _, _)
                | TargetAst::ObjectOrPlayer(_, _, _)
                | TargetAst::WithCount(_, _)
                | TargetAst::WithCountValue(_, _, _)
        ) {
            return Ok(None);
        }
        source
    } else {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens))
    };

    let (mut amount, target, unpreventable) = match parsed.action {
        SubjectVerbActionAst::DealDamage {
            amount,
            target,
            unpreventable,
        } if explicitly_targeted => (amount, target, unpreventable),
        SubjectVerbActionAst::DealDamageEach { amount, mut filter } => {
            // The ordinary each-target damage AST carries set semantics in
            // its variant. This explicit-source form uses a TargetAst, so
            // retain the same fact as presentation metadata for mass-damage
            // lowering without turning it into a target choice.
            filter.set_plural_object_noun_surface(true);
            (
                amount,
                TargetAst::Object(filter, None, span_from_tokens(action_tokens)),
                false,
            )
        }
        _ => return Ok(None),
    };
    // Within an explicit damage-source clause, "its" names the grammatical
    // subject, not an older object in reference memory. Preserve that
    // relationship as Source until lowering assigns the target a stable tag.
    bind_explicit_damage_subject_characteristics_to_source(&mut amount);

    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::DealDamageEqualToPower {
            source,
            amount,
            target,
            unpreventable,
        },
    )))
}

fn bind_gain_control_pronoun_to_source(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::GainControl {
            target,
            source_reference_surface,
            ..
        } = &mut subject_verb.action
        && let TargetAst::Tagged(tag, span) = target
        && tag.as_str() == IT_TAG
    {
        *target = TargetAst::Source(*span);
        source_reference_surface
            .get_or_insert_with(|| SourceReferenceSurface::ThisPermanentType("it".to_string()));
        return;
    }

    crate::model::visit::for_each_nested_effects_mut(effect, true, |effects| {
        for effect in effects {
            bind_gain_control_pronoun_to_source(effect);
        }
    });
}

fn target_choice_excluded_controller(
    chooser: clause_grammar::ChooseTargetChooserShape,
) -> Option<PlayerFilter> {
    match chooser {
        clause_grammar::ChooseTargetChooserShape::AbilityController => Some(PlayerFilter::You),
        clause_grammar::ChooseTargetChooserShape::ItsController
        | clause_grammar::ChooseTargetChooserShape::ThatOpponent => Some(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(TagKey::from(IT_TAG))),
        ),
        clause_grammar::ChooseTargetChooserShape::Unresolved => None,
    }
}

/// Keep an authored target declaration and its stable chooser-attribution tag
/// together. The explicit runtime tag lets later clauses distinguish "the
/// creature you chose" from "the creature your opponent chose" after another
/// target declaration has replaced the ordinary last-object reference.
fn explicit_target_choice(
    shape: clause_grammar::ChooseTargetShape<'_>,
    target: TargetAst,
) -> EffectAst {
    let (choice, alias) = match shape.chooser {
        clause_grammar::ChooseTargetChooserShape::AbilityController => (
            EffectAst::subject_verb_explicit_target_only(target),
            Some(ABILITY_CONTROLLER_TARGET_CHOICE_TAG),
        ),
        clause_grammar::ChooseTargetChooserShape::ItsController => (
            EffectAst::subject_verb_explicit_target_only_for_chooser(
                target,
                PlayerAst::ItsController,
            ),
            None,
        ),
        clause_grammar::ChooseTargetChooserShape::ThatOpponent => (
            EffectAst::subject_verb_explicit_target_only_for_chooser(
                target,
                PlayerAst::ItsController,
            ),
            Some(OPPONENT_TARGET_CHOICE_TAG),
        ),
        clause_grammar::ChooseTargetChooserShape::Unresolved => {
            (EffectAst::subject_verb_target_only(target), None)
        }
    };
    let Some(alias) = alias else {
        return choice;
    };
    EffectAst::TagAffected {
        effect: Box::new(choice),
        tag: TagKey::from(alias),
    }
}

fn preserve_target_choice_controller_exclusion(
    target: &mut TargetAst,
    chooser: clause_grammar::ChooseTargetChooserShape,
) {
    let Some(excluded) = target_choice_excluded_controller(chooser) else {
        return;
    };
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    filter.controller = Some(PlayerFilter::excluding(PlayerFilter::Any, excluded));
}

/// Parse both CR 701.69a authored surfaces:
///
/// - `Heal N damage already dealt to/from [permanent].`
/// - `All damage already dealt to [permanent] is healed.`
fn parse_heal_damage_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if words.len() < 4 {
        return Ok(None);
    }

    let (amount_start, mut damage_word, passive_end) = if matches!(words[0], "heal" | "heals") {
        let Some(damage_word) = crate::word_primitives::parse_sequence_start(&words, &["damage"])
        else {
            return Ok(None);
        };
        if damage_word <= 1 {
            return Ok(None);
        }
        (1, damage_word, words.len())
    } else if crate::word_primitives::parse_sequence_suffix(&words, &["is", "healed"]) {
        let Some(damage_word) =
            crate::word_primitives::parse_sequence_start(&words[..words.len() - 2], &["damage"])
        else {
            return Ok(None);
        };
        if damage_word == 0 {
            return Ok(None);
        }
        (0, damage_word, words.len() - 2)
    } else {
        return Ok(None);
    };

    let mut amount_end = damage_word;
    if amount_end > amount_start && words[amount_end - 1] == "other" {
        amount_end -= 1;
    }
    let amount = if crate::word_primitives::parse_sequence_complete(
        &words[amount_start..amount_end],
        &["all"],
    ) {
        None
    } else {
        let Some(amount_tokens) = view.token_span_for_words(amount_start, amount_end) else {
            return Ok(None);
        };
        let Some((value, used)) = parse_value(&tokens[amount_tokens.clone()]) else {
            return Ok(None);
        };
        if used != amount_tokens.len() {
            return Ok(None);
        }
        Some(value)
    };

    damage_word += 1;
    if words.get(damage_word..damage_word + 2).is_some_and(|tail| {
        crate::word_primitives::parse_sequence_complete(tail, &["already", "dealt"])
    }) {
        damage_word += 2;
    }
    if words
        .get(damage_word)
        .is_some_and(|word| matches!(*word, "to" | "from" | "on"))
    {
        damage_word += 1;
    }
    if damage_word >= passive_end {
        return Ok(None);
    }
    let Some(target_tokens) = view.token_span_for_words(damage_word, passive_end) else {
        return Ok(None);
    };
    let target = parse_target_phrase(&tokens[target_tokens])?;
    Ok(Some(EffectAst::subject_verb_heal_damage(target, amount)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommonPlayerActionPattern {
    Amount,
    ObjectSelection,
    ZoneMovement,
    Choice,
    Payment,
    StateChange,
}

#[derive(Debug, Clone, Copy)]
struct ComposedPlayerActionClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

fn parse_copular_base_pt_animation_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_copular_animation_shape(tokens) else {
        return Ok(None);
    };

    parse_become_clause(shape.subject_tokens, shape.animation_tokens).map(Some)
}

#[derive(Debug, Clone, Copy)]
enum CommonPlayerActionClause<'a> {
    Amount(ComposedPlayerActionClause<'a>),
    Object(ComposedPlayerActionClause<'a>),
    Zone(ComposedPlayerActionClause<'a>),
    Choice(ComposedPlayerActionClause<'a>),
    Payment(ComposedPlayerActionClause<'a>),
    State(ComposedPlayerActionClause<'a>),
}

impl<'a> ComposedPlayerActionClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

fn common_player_action_pattern_for(
    verb: Verb,
    action_tokens: &[OwnedLexToken],
) -> Option<CommonPlayerActionPattern> {
    let words = TokenWordView::new(action_tokens);
    if matches!(verb, Verb::Pay) {
        return Some(CommonPlayerActionPattern::Payment);
    }
    if matches!(verb, Verb::Scry | Verb::Surveil) {
        return Some(CommonPlayerActionPattern::Choice);
    }
    if matches!(
        verb,
        Verb::Sacrifice | Verb::Discard | Verb::Reveal | Verb::Look
    ) {
        return Some(CommonPlayerActionPattern::ObjectSelection);
    }
    if matches!(
        verb,
        Verb::Shuffle | Verb::Move | Verb::Put | Verb::Return | Verb::Exile
    ) || words.word_refs().iter().any(|word| {
        matches!(
            *word,
            "library" | "graveyard" | "hand" | "battlefield" | "exile"
        )
    }) {
        return Some(CommonPlayerActionPattern::ZoneMovement);
    }
    if matches!(
        verb,
        Verb::Draw | Verb::Lose | Verb::Gain | Verb::Mill | Verb::Get | Verb::Add
    ) {
        return Some(CommonPlayerActionPattern::Amount);
    }
    if matches!(verb, Verb::Skip | Verb::Take | Verb::Become | Verb::End) {
        return Some(CommonPlayerActionPattern::StateChange);
    }
    None
}

fn parse_control_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_control_player_shape(tokens) else {
        return Ok(None);
    };
    let TargetAst::Player(target_filter, _) = parse_target_phrase(shape.target_tokens)? else {
        return Ok(None);
    };
    let duration = parse_control_duration(shape.duration_tokens)?;
    Ok(Some(EffectAst::subject_verb_control_player(
        shape.player,
        PlayerFilter::Target(Box::new(target_filter)),
        duration,
    )))
}

fn is_pronoun_top_or_bottom_library_choice_put_tail(tokens: &[OwnedLexToken]) -> bool {
    clause_grammar::is_pronoun_library_choice_put_shape(tokens)
}

impl<'a> CommonPlayerActionClause<'a> {
    fn recognize(
        subject: SubjectAst,
        verb: Verb,
        action_tokens: &'a [OwnedLexToken],
    ) -> Option<Self> {
        if !matches!(subject, SubjectAst::Player(_)) {
            return None;
        }
        let pattern = common_player_action_pattern_for(verb, action_tokens)?;
        let clause = ComposedPlayerActionClause {
            subject,
            verb,
            action_tokens,
        };
        Some(match pattern {
            CommonPlayerActionPattern::Amount => Self::Amount(clause),
            CommonPlayerActionPattern::ObjectSelection => Self::Object(clause),
            CommonPlayerActionPattern::ZoneMovement => Self::Zone(clause),
            CommonPlayerActionPattern::Choice => Self::Choice(clause),
            CommonPlayerActionPattern::Payment => Self::Payment(clause),
            CommonPlayerActionPattern::StateChange => Self::State(clause),
        })
    }

    #[cfg(test)]
    fn pattern(&self) -> CommonPlayerActionPattern {
        match self {
            Self::Amount(_) => CommonPlayerActionPattern::Amount,
            Self::Object(_) => CommonPlayerActionPattern::ObjectSelection,
            Self::Zone(_) => CommonPlayerActionPattern::ZoneMovement,
            Self::Choice(_) => CommonPlayerActionPattern::Choice,
            Self::Payment(_) => CommonPlayerActionPattern::Payment,
            Self::State(_) => CommonPlayerActionPattern::StateChange,
        }
    }

    fn lower(self) -> Result<EffectAst, CardTextError> {
        match self {
            Self::Amount(clause) => clause.lower(),
            Self::Object(clause) => clause.lower(),
            Self::Zone(clause) => clause.lower(),
            Self::Choice(clause) => clause.lower(),
            Self::Payment(clause) => clause.lower(),
            Self::State(clause) => clause.lower(),
        }
    }
}

fn parse_play_exiled_cards_for_as_long_as_exiled_clause(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    (clause_grammar::parse_tagged_permission_shape(tokens)
        == Some(clause_grammar::TaggedPermissionShape::PlayExiledForAsLongAsExiled))
    .then(|| {
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            true,
            false,
            false,
            None,
        )
    })
}

fn parse_mana_any_type_cast_tagged_this_way_clause(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    (clause_grammar::parse_tagged_permission_shape(tokens)
        == Some(clause_grammar::TaggedPermissionShape::ManaAnyTypeCastsTaggedThisWay))
    .then(|| {
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            false,
            false,
            true,
            None,
        )
    })
}

pub fn parse_for_each_prevent_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_for_each_prevent_shape(tokens) else {
        return Ok(None);
    };
    let Some(filter) = parse_for_each_object_subject(shape.subject_tokens)? else {
        return Ok(None);
    };

    let Some(prevent_effect) = parse_prevent_damage_sentence(shape.prevent_tokens)? else {
        return Ok(None);
    };

    let effects = if let Some(idx) = shape.unless_token {
        if let Some(unless_effect) = try_build_unless(
            vec![prevent_effect.clone()],
            SubjectVerbPrimitiveClause::new(tokens),
            idx,
        )? {
            vec![unless_effect]
        } else {
            vec![prevent_effect]
        }
    } else {
        vec![prevent_effect]
    };
    Ok(Some(EffectAst::ForEachObject { filter, effects }))
}

pub fn parse_for_each_counter_group_removed_this_way_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_counter_group_removed_shape(tokens) else {
        return Ok(None);
    };
    if shape.group_size == 0 {
        return Err(CardTextError::ParseError(format!(
            "counter group size must be positive (clause: '{}')",
            render_lower_words(tokens)
        )));
    }
    if shape.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after counter group clause (clause: '{}')",
            render_lower_words(tokens)
        )));
    }

    let effects = parse_effect_chain_with_subject_verb_primitives(shape.effect_tokens)?;
    Ok(Some(EffectAst::RepeatEffects {
        count: Value::DividedRoundedDown(Box::new(Value::X), shape.group_size as i32)
            .with_surface_hint(ValueSurfaceHint::CountersRemovedThisWay),
        effects,
    }))
}

fn parse_cast_any_number_from_among_tagged_clause(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let shape = clause_grammar::parse_cast_any_tagged_shape(tokens)?;

    let mut filter = ObjectFilter::nonland().in_zone(Zone::Exile).match_tagged(
        TagKey::from(IT_TAG),
        crate::target::TaggedOpbjectRelation::IsTaggedObject,
    );

    filter.mana_value = shape.mana_value;

    Some(EffectAst::ForEachObject {
        filter,
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::subject_verb_cast_tagged(
                TagKey::from(IT_TAG),
                PlayerAst::You,
                false,
                false,
                true,
                None,
            )],
        }],
    })
}

fn parse_cast_single_spell_from_among_hand_cards_clause(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if clause_grammar::parse_tagged_permission_shape(tokens)
        != Some(clause_grammar::TaggedPermissionShape::CastSingleFromAmongHandCards)
    {
        return None;
    }

    Some(
        EffectAst::may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
            PlayerAst::You,
            PlayerAst::That,
            ObjectFilter::nonland().in_zone(Zone::Hand),
            Zone::Hand,
        ),
    )
}

fn parse_passive_sacrifice_by_controller_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_passive_sacrifice_shape(tokens) else {
        return Ok(None);
    };

    let filter = parse_object_filter(shape.object_tokens, false)?;
    Ok(Some(EffectAst::ForEachObject {
        filter,
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::ItsController,
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            1,
            None,
        )],
    }))
}

/// Split a conjoined block-permission tail off a pump modifier ("+2/+2 until
/// end of turn and can block an additional creature this turn"), returning
/// the pump head and the number of additional blockable attackers.
fn split_trailing_can_block_additional_tail(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], u32)> {
    for (idx, token) in tokens.iter().enumerate().rev() {
        if token.as_word() != Some("and") {
            continue;
        }
        let Some(shape) = effect_grammar::clause_pattern_shapes::parse_can_block_additional_tokens(
            &tokens[idx + 1..],
        ) else {
            continue;
        };
        if !shape.subject_tokens.is_empty() {
            return None;
        }
        let head = trim_lexed_commas(&tokens[..idx]);
        if head.is_empty() {
            return None;
        }
        return Some((head, shape.additional));
    }
    None
}

pub(crate) fn parse_get_pump_clause(
    subject_tokens: &[OwnedLexToken],
    action_tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // "It gets +2/+2 until end of turn and can block an additional creature
    // this turn" (Act of Heroism) — the block permission is its own granted
    // effect on the pump subject, not part of the P/T modifier tail.
    if let Some((pump_tokens, additional)) = split_trailing_can_block_additional_tail(action_tokens)
    {
        let Some(pump) = parse_get_pump_clause(subject_tokens, pump_tokens, full_tokens)? else {
            return Ok(None);
        };
        let EffectAst::SubjectVerb(subject_verb) = &pump else {
            return Ok(None);
        };
        let SubjectVerbActionAst::Pump { target, .. } = &subject_verb.action else {
            return Ok(None);
        };
        let grant = EffectAst::subject_verb_grant_abilities_to_target(
            target.clone(),
            vec![GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat {
                additional: additional as usize,
            }],
            Until::EndOfTurn,
        );
        return Ok(Some(EffectAst::Sequence {
            effects: vec![pump, grant],
        }));
    }
    let Some(subject_shape) = clause_grammar::parse_pump_subject_shape(subject_tokens) else {
        parser_trace("parse_get_pump_clause:subject-shape-miss", subject_tokens);
        return Ok(None);
    };
    let (modifier_tokens, additional_modifier) = match action_tokens {
        [first, second, rest @ ..]
            if first.as_word() == Some("an") && second.as_word() == Some("additional") =>
        {
            (rest, true)
        }
        [first, rest @ ..] if first.as_word() == Some("additional") => (rest, true),
        _ => (action_tokens, false),
    };
    let collapsed_modifier_tail = collapse_leading_signed_pt_modifier_tokens(modifier_tokens);
    let modifier_tail = collapsed_modifier_tail
        .as_deref()
        .unwrap_or(modifier_tokens);

    // An inline P/T alternative is one choice with two resolution branches,
    // not a tail that can be discarded after the first modifier. Copy the
    // shared authored tail (normally the duration) onto both branches and
    // reuse the ordinary pump parser for each branch.
    if !additional_modifier
        && let Some(alternative) =
            effect_grammar::for_each_shapes::parse_fixed_pt_alternative_shape(modifier_tail)
    {
        let branch_tokens = |modifier: &OwnedLexToken| {
            let mut tokens = Vec::with_capacity(1 + alternative.trailing_tokens.len());
            tokens.push(modifier.clone());
            tokens.extend_from_slice(alternative.trailing_tokens);
            tokens
        };
        let first_tokens = branch_tokens(&alternative.first_modifier);
        let second_tokens = branch_tokens(&alternative.second_modifier);
        let first = parse_get_pump_clause(subject_tokens, &first_tokens, full_tokens)?;
        let second = parse_get_pump_clause(subject_tokens, &second_tokens, full_tokens)?;
        if let (Some(first), Some(second)) = (first, second) {
            return Ok(Some(EffectAst::ChooseOneOf {
                modes: vec![
                    ChooseOneModeAst {
                        description: String::new(),
                        effects: vec![first],
                    },
                    ChooseOneModeAst {
                        description: String::new(),
                        effects: vec![second],
                    },
                ],
            }));
        }
    }

    if let Some(modifier) = clause_grammar::parse_discarded_this_way_modifier_shape(modifier_tail) {
        let target = parse_target_phrase(subject_shape.subject_tokens)?;
        return Ok(Some(EffectAst::subject_verb_pump_for_each(
            modifier.power,
            modifier.toughness,
            target,
            Value::EventValue(EventValueSpec::Amount)
                .with_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay),
            subject_shape.duration.unwrap_or(Until::EndOfTurn),
        )));
    }

    let Some(mod_token) = modifier_tail.first().map(OwnedLexToken::parser_text) else {
        parser_trace("parse_get_pump_clause:missing-modifier", action_tokens);
        return Ok(None);
    };
    let Ok((power, toughness)) = parse_pt_modifier_values(mod_token) else {
        parser_trace("parse_get_pump_clause:modifier-shape-miss", modifier_tail);
        return Ok(None);
    };
    let mut duration_before_for_each = false;
    let mut count = parse_get_for_each_count_value(modifier_tail)?;
    if count.is_none()
        && let Some(for_each_tokens) =
            clause_grammar::parse_modifier_duration_for_each_tokens(modifier_tail)
    {
        count = parse_get_for_each_count_value(for_each_tokens)?;
        duration_before_for_each = count.is_some();
    }
    if let Some(count) = count {
        let mut count = count;
        if duration_before_for_each {
            count = count.with_surface_hint(ValueSurfaceHint::DurationBeforeForEach);
        }
        if additional_modifier {
            count = count.with_surface_hint(ValueSurfaceHint::AdditionalPowerToughnessModifier);
        }
        let power_per = match power {
            Value::Fixed(value) => value,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported dynamic gets-for-each power modifier (clause: '{}')",
                    render_lower_words(full_tokens)
                )));
            }
        };
        let toughness_per = match toughness {
            Value::Fixed(value) => value,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported dynamic gets-for-each toughness modifier (clause: '{}')",
                    render_lower_words(full_tokens)
                )));
            }
        };
        let duration = subject_shape.duration.unwrap_or(Until::EndOfTurn);
        if count.has_surface_hint(ValueSurfaceHint::CreaturesChosenBeforeIt)
            && subject_shape
                .subject_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .any(|word| word == "those")
        {
            return Ok(Some(EffectAst::ForEachTagged {
                tag: TagKey::from(IT_TAG),
                effects: vec![EffectAst::subject_verb_pump_for_each(
                    power_per,
                    toughness_per,
                    TargetAst::Tagged(
                        TagKey::from(IT_TAG),
                        span_from_tokens(subject_shape.subject_tokens),
                    ),
                    count,
                    duration,
                )],
            }));
        }
        let scale_count = |per: i32| match per {
            0 => Value::Fixed(0),
            1 => count.clone().with_surface_hint(ValueSurfaceHint::ForEach),
            multiplier => Value::Scaled(Box::new(count.clone()), multiplier)
                .with_surface_hint(ValueSurfaceHint::ForEach),
        };
        let mut effect = match subject_shape.kind {
            clause_grammar::PumpSubjectKind::Tagged => EffectAst::subject_verb_pump_for_each(
                power_per,
                toughness_per,
                TargetAst::Tagged(
                    TagKey::from(IT_TAG),
                    span_from_tokens(subject_shape.subject_tokens),
                ),
                count,
                duration,
            ),
            clause_grammar::PumpSubjectKind::DemonstrativeTarget
                if subject_shape
                    .subject_tokens
                    .iter()
                    .filter_map(OwnedLexToken::as_word)
                    .any(|word| word == "those") =>
            {
                let filter = match parse_target_phrase(subject_shape.subject_tokens)? {
                    TargetAst::Object(filter, None, _) => filter,
                    TargetAst::Tagged(tag, _) => ObjectFilter::tagged(tag),
                    _ => return Ok(None),
                };
                EffectAst::subject_verb_pump_all(
                    filter,
                    scale_count(power_per),
                    scale_count(toughness_per),
                    duration,
                )
            }
            clause_grammar::PumpSubjectKind::DemonstrativeTarget => {
                EffectAst::subject_verb_pump_for_each(
                    power_per,
                    toughness_per,
                    parse_target_phrase(subject_shape.subject_tokens)?,
                    count,
                    duration,
                )
            }
            clause_grammar::PumpSubjectKind::ControlledFilter {
                filter_tokens,
                controller,
            } => {
                let Ok(mut filter) = parse_object_filter(filter_tokens, false) else {
                    return Ok(None);
                };
                if filter == ObjectFilter::default() {
                    return Ok(None);
                }
                filter.controller = Some(controller);
                EffectAst::subject_verb_pump_all(
                    filter,
                    scale_count(power_per),
                    scale_count(toughness_per),
                    duration,
                )
            }
            clause_grammar::PumpSubjectKind::DirectTarget(target_tokens) => {
                EffectAst::subject_verb_pump_for_each(
                    power_per,
                    toughness_per,
                    parse_target_phrase(target_tokens)?,
                    count,
                    duration,
                )
            }
            clause_grammar::PumpSubjectKind::Equipped => EffectAst::subject_verb_pump_for_each(
                power_per,
                toughness_per,
                TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::Equipped.key(),
                    span_from_tokens(subject_shape.subject_tokens),
                ),
                count,
                duration,
            ),
            clause_grammar::PumpSubjectKind::Enchanted => EffectAst::subject_verb_pump_for_each(
                power_per,
                toughness_per,
                TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::Enchanted.key(),
                    span_from_tokens(subject_shape.subject_tokens),
                ),
                count,
                duration,
            ),
            clause_grammar::PumpSubjectKind::FilterCandidate {
                filter_tokens,
                mentions_this,
                disallowed_pronoun,
                demonstrative_reference,
            } => {
                if demonstrative_reference {
                    return Ok(None);
                }
                let definite_singular_antecedent = subject_shape
                    .subject_tokens
                    .iter()
                    .filter_map(OwnedLexToken::as_word)
                    .eq(["the", "creature"]);
                if definite_singular_antecedent {
                    EffectAst::subject_verb_pump_for_each(
                        power_per,
                        toughness_per,
                        TargetAst::Tagged(
                            TagKey::from(IT_TAG),
                            span_from_tokens(subject_shape.subject_tokens),
                        ),
                        count,
                        duration,
                    )
                } else if mentions_this
                    && let Some(surface) = source_reference_surface_for_words(
                        &crate::lexer::parser_token_word_refs(filter_tokens),
                    )
                {
                    EffectAst::subject_verb_pump_for_each(
                        power_per,
                        toughness_per,
                        TargetAst::Object(
                            ObjectFilter::source_with_surface(surface),
                            None,
                            span_from_tokens(filter_tokens),
                        ),
                        count,
                        duration,
                    )
                } else {
                    let Ok(filter) = parse_object_filter(filter_tokens, false) else {
                        return Ok(None);
                    };
                    let directional_combat_relation =
                        filter.blocking && filter.in_combat_with_source;
                    if filter == ObjectFilter::default()
                        || (mentions_this && !filter.other && !directional_combat_relation)
                        || (disallowed_pronoun && !filter.other && !directional_combat_relation)
                    {
                        return Ok(None);
                    }
                    EffectAst::subject_verb_pump_all(
                        filter,
                        scale_count(power_per),
                        scale_count(toughness_per),
                        duration,
                    )
                }
            }
        };
        let set_quantifier_surface = match full_tokens.first() {
            Some(token) if token.is_word("all") => Some(ironsmith_core::SetQuantifierSurface::All),
            Some(token) if token.is_word("each") => {
                Some(ironsmith_core::SetQuantifierSurface::Each)
            }
            Some(token) if token.is_word("those") => {
                Some(ironsmith_core::SetQuantifierSurface::Those)
            }
            _ => None,
        };
        if let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PumpAll {
                    set_quantifier_surface: surface,
                    ..
                },
            ..
        }) = &mut effect
        {
            *surface = set_quantifier_surface;
        }
        return Ok(Some(effect));
    }

    let (power, toughness, parsed_duration, condition) =
        parse_get_modifier_values_with_tail(modifier_tail, power, toughness)?;
    let duration = subject_shape.duration.unwrap_or(parsed_duration);
    let demonstrative_set_surface = full_tokens
        .first()
        .is_some_and(|token| token.is_word("those"));
    let mut effect = match subject_shape.kind {
        clause_grammar::PumpSubjectKind::Tagged => EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Tagged(
                TagKey::from(IT_TAG),
                span_from_tokens(subject_shape.subject_tokens),
            ),
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::DemonstrativeTarget
            if subject_shape
                .subject_tokens
                .first()
                .and_then(OwnedLexToken::as_word)
                == Some("those")
                && condition.is_none() =>
        {
            let filter = match parse_target_phrase(subject_shape.subject_tokens)? {
                TargetAst::Object(filter, None, _) => filter,
                TargetAst::Tagged(tag, _) => ObjectFilter::tagged(tag),
                _ => return Ok(None),
            };
            EffectAst::subject_verb_pump_all(filter, power, toughness, duration)
        }
        clause_grammar::PumpSubjectKind::DemonstrativeTarget => EffectAst::subject_verb_pump(
            power,
            toughness,
            parse_target_phrase(subject_shape.subject_tokens)?,
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::ControlledFilter {
            filter_tokens,
            controller,
        } => {
            let Ok(mut filter) = parse_object_filter(filter_tokens, false) else {
                return Ok(None);
            };
            if filter == ObjectFilter::default() {
                return Ok(None);
            }
            filter.controller = Some(controller);
            EffectAst::subject_verb_pump_all(filter, power, toughness, duration)
        }
        clause_grammar::PumpSubjectKind::DirectTarget(target_tokens) => {
            EffectAst::subject_verb_pump(
                power,
                toughness,
                parse_target_phrase(target_tokens)?,
                duration,
                condition,
            )
        }
        clause_grammar::PumpSubjectKind::Equipped => EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::Equipped.key(),
                span_from_tokens(subject_shape.subject_tokens),
            ),
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::Enchanted => EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::Enchanted.key(),
                span_from_tokens(subject_shape.subject_tokens),
            ),
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::FilterCandidate {
            filter_tokens,
            mentions_this,
            disallowed_pronoun,
            demonstrative_reference,
        } => {
            if demonstrative_reference {
                return Ok(None);
            }
            if mentions_this
                && let Some(surface) = source_reference_surface_for_words(
                    &crate::lexer::parser_token_word_refs(filter_tokens),
                )
            {
                return Ok(Some(EffectAst::subject_verb_pump(
                    power,
                    toughness,
                    TargetAst::Object(
                        ObjectFilter::source_with_surface(surface),
                        None,
                        span_from_tokens(filter_tokens),
                    ),
                    duration,
                    condition,
                )));
            }
            let Ok(filter) = parse_object_filter(filter_tokens, false) else {
                return Ok(None);
            };
            let directional_combat_relation = filter.blocking && filter.in_combat_with_source;
            if filter == ObjectFilter::default()
                || (mentions_this && !filter.other && !directional_combat_relation)
                || (disallowed_pronoun && !filter.other && !directional_combat_relation)
            {
                return Ok(None);
            }
            EffectAst::subject_verb_pump_all(filter, power, toughness, duration)
        }
    };
    if demonstrative_set_surface
        && let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Pump {
                    set_quantifier_surface,
                    ..
                }
                | SubjectVerbActionAst::PumpAll {
                    set_quantifier_surface,
                    ..
                },
            ..
        }) = &mut effect
    {
        *set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Those);
    }
    Ok(Some(effect))
}

fn lower_direct_clause_shape(
    shape: clause_grammar::DirectClauseShape,
    tokens: &[OwnedLexToken],
) -> EffectAst {
    match shape {
        clause_grammar::DirectClauseShape::RingTemptsYou => {
            EffectAst::subject_verb_ring_tempts_you(PlayerAst::You)
        }
        clause_grammar::DirectClauseShape::TakeInitiative => {
            EffectAst::subject_verb_take_initiative(PlayerAst::You)
        }
        clause_grammar::DirectClauseShape::ChooseOddOrEven => {
            EffectAst::subject_verb_choose_named_option(
                PlayerAst::Implicit,
                vec!["odd".to_string(), "even".to_string()],
            )
        }
        clause_grammar::DirectClauseShape::ChooseLeftOrRight => {
            EffectAst::subject_verb_choose_named_option(
                PlayerAst::You,
                vec!["left".to_string(), "right".to_string()],
            )
        }
        clause_grammar::DirectClauseShape::ClearSuspected => {
            EffectAst::subject_verb_clear_suspected(None)
        }
        clause_grammar::DirectClauseShape::CopySourceExiledCard => {
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::default().in_zone(Zone::Exile).match_tagged(
                    TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    crate::target::TaggedOpbjectRelation::IsTaggedObject,
                ),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                zones: vec![Zone::Exile],
                search_mode: None,
            }
        }
        clause_grammar::DirectClauseShape::PutTaggedPlusOneCounter => {
            EffectAst::subject_verb_put_counters(
                CounterType::PlusOnePlusOne,
                Value::Fixed(1),
                TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                None,
                false,
            )
        }
        clause_grammar::DirectClauseShape::DamagedPlayersCantGainLife => {
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::gain_life(PlayerFilter::DamagedPlayer),
                Until::EndOfTurn,
                None,
            )
        }
        clause_grammar::DirectClauseShape::DamageCantBePrevented => EffectAst::subject_verb_cant(
            crate::effect::Restriction::prevent_damage(),
            Until::EndOfTurn,
            None,
        ),
        clause_grammar::DirectClauseShape::TurnSourceExiledFaceUp => EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::TurnFaceUp {
                target: TargetAst::Tagged(
                    TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    span_from_tokens(tokens),
                ),
            },
        ),
        clause_grammar::DirectClauseShape::TurnTaggedFaceUp => EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::TurnFaceUp {
                target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
            },
        ),
        clause_grammar::DirectClauseShape::Planeswalk => {
            EffectAst::subject_verb_emit_keyword_action(
                crate::events::KeywordActionKind::Planeswalk,
                1,
            )
        }
        clause_grammar::DirectClauseShape::AssembleContraption => {
            EffectAst::subject_verb_emit_keyword_action(
                crate::events::KeywordActionKind::AssembleContraption,
                1,
            )
        }
        clause_grammar::DirectClauseShape::ChaosEnsues => {
            EffectAst::subject_verb_emit_keyword_action(
                crate::events::KeywordActionKind::ChaosEnsues,
                1,
            )
        }
        clause_grammar::DirectClauseShape::AbandonScheme => {
            EffectAst::subject_verb_emit_keyword_action(
                crate::events::KeywordActionKind::AbandonScheme,
                1,
            )
        }
        clause_grammar::DirectClauseShape::DoubleX => EffectAst::subject_verb_scale_x_value(
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::Triggering.key(),
                span_from_tokens(tokens),
            ),
            2,
        ),
        clause_grammar::DirectClauseShape::OnlyChosenCanAttack => EffectAst::subject_verb_cant(
            crate::effect::Restriction::attack(
                ObjectFilter::creature().not_tagged(TagKey::from(IT_TAG)),
            ),
            Until::EndOfCombat,
            None,
        ),
        clause_grammar::DirectClauseShape::OnlyChosenCanBlock => EffectAst::subject_verb_cant(
            crate::effect::Restriction::block(
                ObjectFilter::creature().not_tagged(TagKey::from(IT_TAG)),
            ),
            Until::EndOfCombat,
            None,
        ),
        clause_grammar::DirectClauseShape::CastNonlandTaggedThisWay => {
            let filter = ObjectFilter::nonland().in_zone(Zone::Exile).match_tagged(
                TagKey::from(IT_TAG),
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            );
            EffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::May {
                    effects: vec![EffectAst::subject_verb_cast_tagged(
                        TagKey::from(IT_TAG),
                        PlayerAst::You,
                        false,
                        false,
                        true,
                        None,
                    )],
                }],
            }
        }
    }
}

pub fn parse_effect_clause(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    // Clause parsing is a public compiler boundary as well as an internal
    // sentence-parser stage. Lower-level callers and focused tests can enter
    parse_effect_clause_unstacked(tokens)
}

#[cfg(test)]
#[path = "clause_dispatch_inline_tests.rs"]
mod tests;

#[path = "clause_dispatch/clause_dispatch_core_programs.rs"]
mod clause_dispatch_core_programs;
pub use clause_dispatch_core_programs::parse_effect_clause_lexed;
use clause_dispatch_core_programs::{parse_effect_clause_unstacked, parse_passive_goad_clause};
#[path = "clause_dispatch/clause_dispatch_reference_programs.rs"]
mod clause_dispatch_reference_programs;
pub(super) use clause_dispatch_reference_programs::parse_hexproof_targeting_override_clause;
pub use clause_dispatch_reference_programs::parse_targeting_as_though_no_ability_spec;
#[path = "clause_dispatch/clause_dispatch_object_action_programs.rs"]
mod clause_dispatch_object_action_programs;
pub use clause_dispatch_object_action_programs::parse_conditional_become_pair;
use clause_dispatch_object_action_programs::parse_conditional_become_pair_impl;
