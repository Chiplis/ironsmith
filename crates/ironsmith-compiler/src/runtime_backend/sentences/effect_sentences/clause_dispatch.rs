pub(crate) use self::become_clause::parse_become_clause;
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
use super::super::grammar::choices::parse_choice_land_type_phrase_words;
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
    span_from_tokens, trim_commas,
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
use super::verb_handlers::parse_control_duration;
use super::zone_counter_helpers::parse_put_counters;
use super::zone_handlers::{
    collapse_leading_signed_pt_modifier_tokens,
    parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control, parse_sacrifice,
};
use super::{
    Verb, bind_implicit_player_context, find_verb, parse_effect_chain_with_subject_verb_primitives,
    parse_simple_gain_ability_clause, parse_simple_lose_ability_clause,
};
use crate::TagKey;
use crate::cards::builders::{
    ABILITY_CONTROLLER_TARGET_CHOICE_TAG, CardTextError, ChooseOneModeAst, EffectAst,
    GrantedAbilityAst, IT_TAG, KeywordAction, OPPONENT_TARGET_CHOICE_TAG, PlayerAst,
    ReturnControllerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst, TargetAst,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::object::CounterType;
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
        | PlayerFilter::MaxSpeed { base, .. } => player_filter_mentions_source_object(base),
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

    crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
        effect,
        true,
        |effects| {
            for effect in effects {
                bind_gain_control_pronoun_to_source(effect);
            }
        },
    );
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
        let Some(damage_word) = words.iter().position(|word| *word == "damage") else {
            return Ok(None);
        };
        if damage_word <= 1 {
            return Ok(None);
        }
        (1, damage_word, words.len())
    } else if words.ends_with(&["is", "healed"]) {
        let Some(damage_word) = words[..words.len() - 2]
            .iter()
            .position(|word| *word == "damage")
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
    let amount = if words[amount_start..amount_end] == ["all"] {
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
    if words
        .get(damage_word..damage_word + 2)
        .is_some_and(|tail| tail == ["already", "dealt"])
    {
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
struct PlayerAmountClause<'a> {
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
struct PlayerObjectClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerZoneClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerChoiceClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerPaymentClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerStateClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
enum CommonPlayerActionClause<'a> {
    Amount(PlayerAmountClause<'a>),
    Object(PlayerObjectClause<'a>),
    Zone(PlayerZoneClause<'a>),
    Choice(PlayerChoiceClause<'a>),
    Payment(PlayerPaymentClause<'a>),
    State(PlayerStateClause<'a>),
}

impl<'a> PlayerAmountClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerObjectClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerZoneClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerChoiceClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerPaymentClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerStateClause<'a> {
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
        Some(match pattern {
            CommonPlayerActionPattern::Amount => Self::Amount(PlayerAmountClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::ObjectSelection => Self::Object(PlayerObjectClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::ZoneMovement => Self::Zone(PlayerZoneClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::Choice => Self::Choice(PlayerChoiceClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::Payment => Self::Payment(PlayerPaymentClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::StateChange => Self::State(PlayerStateClause {
                subject,
                verb,
                action_tokens,
            }),
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

pub(crate) fn parse_for_each_prevent_damage_clause(
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

pub(crate) fn parse_for_each_counter_group_removed_this_way_clause(
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

fn parse_get_pump_clause(
    subject_tokens: &[OwnedLexToken],
    action_tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
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
    let mut count = parse_get_for_each_count_value(modifier_tail)?;
    if count.is_none()
        && let Some(for_each_tokens) =
            clause_grammar::parse_modifier_duration_for_each_tokens(modifier_tail)
    {
        count = parse_get_for_each_count_value(for_each_tokens)?;
    }
    if let Some(count) = count {
        let count = if additional_modifier {
            count.with_surface_hint(ValueSurfaceHint::AdditionalPowerToughnessModifier)
        } else {
            count
        };
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
                    TagKey::from("equipped"),
                    span_from_tokens(subject_shape.subject_tokens),
                ),
                count,
                duration,
            ),
            clause_grammar::PumpSubjectKind::Enchanted => EffectAst::subject_verb_pump_for_each(
                power_per,
                toughness_per,
                TargetAst::Tagged(
                    TagKey::from("enchanted"),
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
                } else {
                    let Ok(filter) = parse_object_filter(filter_tokens, false) else {
                        return Ok(None);
                    };
                    if filter == ObjectFilter::default()
                        || (mentions_this && !filter.other)
                        || (disallowed_pronoun && !filter.other)
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
        let set_quantifier_surface = match subject_shape
            .subject_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
        {
            Some("all") => Some(ironsmith_core::SetQuantifierSurface::All),
            Some("each") | Some("those") => Some(ironsmith_core::SetQuantifierSurface::Each),
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
    let demonstrative_set_surface = subject_shape
        .subject_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        == Some("those");
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
                TagKey::from("equipped"),
                span_from_tokens(subject_shape.subject_tokens),
            ),
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::Enchanted => EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Tagged(
                TagKey::from("enchanted"),
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
            let Ok(filter) = parse_object_filter(filter_tokens, false) else {
                return Ok(None);
            };
            if filter == ObjectFilter::default()
                || (mentions_this && !filter.other)
                || (disallowed_pronoun && !filter.other)
            {
                return Ok(None);
            }
            EffectAst::subject_verb_pump_all(filter, power, toughness, duration)
        }
    };
    if demonstrative_set_surface
        && let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PumpAll {
                    set_quantifier_surface,
                    ..
                },
            ..
        }) = &mut effect
    {
        *set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);
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
            TargetAst::Tagged(TagKey::from("triggering"), span_from_tokens(tokens)),
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

pub(crate) fn parse_effect_clause(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    let stripped_instead = super::strip_leading_instead_prefix(tokens);
    let tokens = stripped_instead.as_deref().unwrap_or(tokens);
    let tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };

    // A standalone effect sentence reaches clause dispatch directly, without
    // passing through the coordinated-chain parser. Preserve the dedicated
    // sequential-offer model for "any player may sacrifice ..." here too:
    // PlayerFilter::Any is not itself an actor and must not become the chooser
    // or sacrificing player for a single MayEffect.
    if let Some(shape) = effect_grammar::parse_any_player_may_sacrifice_shape(tokens) {
        let sacrifice = parse_sacrifice(
            shape.action_tokens,
            Some(SubjectAst::Player(PlayerAst::That)),
            None,
        )?;
        return Ok(EffectAst::AnyPlayerMay {
            players: PlayerFilter::Any,
            effects: vec![sacrifice],
        });
    }

    // `assigns no combat damage` is a complete effect even when Oracle
    // coordinates another effect after it. The direct shape intentionally
    // requires a sentence boundary, so split this prefix before dispatching
    // the rest of the coordinated clause.
    for (and_idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let prefix = trim_edge_punctuation(&tokens[..and_idx]);
        let suffix = trim_edge_punctuation(&tokens[and_idx + 1..]);
        if suffix.is_empty()
            || !matches!(
                clause_grammar::parse_assigns_no_combat_damage_shape(&prefix),
                Some(clause_grammar::AssignsNoCombatDamageShape::Supported { .. })
            )
        {
            continue;
        }
        let first = parse_effect_clause(&prefix)?;
        let mut effects = vec![first];
        effects.extend(
            crate::runtime_backend::sentences::effect_sentences::parse_effect_chain_lexed(&suffix)?,
        );
        if effects.len() > 1 {
            return Ok(EffectAst::Sequence { effects });
        }
    }

    if let Some(effect) = parse_conditional_become_pair(tokens)? {
        return Ok(effect);
    }

    if let Some(shape) = followup_grammar::parse_counter_linked_land_subtype_followup(tokens) {
        return Ok(EffectAst::subject_verb_add_subtypes(
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
            vec![shape.subtype],
            Until::ForAsLongAs(
                ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                    shape.counter_type,
                ),
            ),
        ));
    }

    if let Some(effect) = effect_grammar::parse_prevent_damage_sentence_lexed(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_heal_damage_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens)
        && let Ok(base_effect) = parse_effect_clause(trailing_if.leading_tokens)
    {
        return Ok(EffectAst::TrailingIf {
            predicate: trailing_if.predicate,
            effects: vec![base_effect],
        });
    }

    if let Some(spec) = parse_may_cast_it_sentence(tokens) {
        return Ok(build_may_cast_tagged_effect(&spec));
    }

    if let Some(effect) = parse_play_exiled_cards_for_as_long_as_exiled_clause(tokens) {
        return Ok(effect);
    }

    if let Some(shape) =
        clause_grammar::parse_cast_target_from_your_graveyard_this_turn_shape(tokens)
    {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                    TagKey::from(IT_TAG),
                    PlayerAst::You,
                    false,
                    false,
                    false,
                ),
            ],
        });
    }

    if let Some(effect) = parse_cast_or_play_tagged_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_cast_any_number_from_among_tagged_clause(tokens) {
        return Ok(effect);
    }

    if let Some(effect) = parse_cast_single_spell_from_among_hand_cards_clause(tokens) {
        return Ok(effect);
    }

    if let Some(effect) = parse_mana_any_type_cast_tagged_this_way_clause(tokens) {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_leading_may_clause_shape(tokens) {
        // In permission text such as "You may play an additional land this
        // turn", "may" describes the granted game-rule permission. It is not
        // an optional resolution action and therefore must not become a
        // MayEffect decision at resolution time.
        if let Some(mut permission) = parse_additional_land_plays_clause(shape.effect_tokens)? {
            if let clause_grammar::LeadingMayActorShape::Player(player) = shape.actor {
                bind_implicit_player_context(&mut permission, player);
            }
            return Ok(permission);
        }
        let mut effects = parse_effect_chain_with_subject_verb_primitives(shape.effect_tokens)?;
        return Ok(match shape.actor {
            clause_grammar::LeadingMayActorShape::Player(player) => {
                for effect in &mut effects {
                    bind_implicit_player_context(effect, player);
                }
                EffectAst::MayByPlayer { player, effects }
            }
            clause_grammar::LeadingMayActorShape::Implicit => EffectAst::May { effects },
        });
    }

    if let Some(shape) = clause_grammar::parse_tagged_plural_pump_shape(tokens)
        && let Some(effect) =
            parse_get_pump_clause(shape.subject_tokens, shape.modifier_tokens, tokens)?
    {
        return Ok(effect);
    }

    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();

    if let Some(effect) = parse_for_each_prevent_damage_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_for_each_counter_group_removed_this_way_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_direct_clause_shape(tokens) {
        return Ok(lower_direct_clause_shape(shape, tokens));
    }

    if let Some(shape) = clause_grammar::parse_shared_ability_gain_shape(tokens) {
        return Ok(EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Tagged(
                TagKey::from(IT_TAG),
                Some(crate::cards::builders::TextSpan::synthetic()),
            ),
            shape
                .abilities
                .into_iter()
                .map(GrantedAbilityAst::KeywordAction)
                .collect(),
            Until::Forever,
        ));
    }
    if let Some(effect) = parse_take_extra_turn_sentence(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(effect);
    }
    if let Some(spec) = parse_mana_replacement_clause_spec_lexed(tokens) {
        return Ok(EffectAst::subject_verb_register_mana_replacement(
            ObjectFilter::land().you_control(),
            vec![spec.replacement_mana],
            crate::effects::ReplacementApplyMode::UntilEndOfTurn,
        ));
    }
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}') [rule=mana-replacement]",
            clause_words.join(" ")
        )));
    }

    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}') [rule=mana-trigger-additional]",
            clause_words.join(" ")
        )));
    }

    if let Some(shape) = clause_grammar::parse_for_each_card_payment_shape(tokens) {
        let mut filter = ObjectFilter::default();
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(EffectAst::ForEachObject {
            filter,
            effects: vec![EffectAst::UnlessAction {
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                    crate::zone::Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                alternative: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LoseLife {
                        amount: Value::Fixed(shape.life_amount as i32),
                    },
                )],
                player: PlayerAst::You,
            }],
        });
    }

    if let Some(shape) = clause_grammar::parse_opponent_return_choice_shape(tokens) {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::UnlessAction {
                    effects: vec![EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        false,
                    )],
                    alternative: vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::Draw {
                            count: Value::Fixed(1),
                        },
                    )],
                    player: PlayerAst::ItsController,
                },
            ],
        });
    }

    if let Some(effects) =
        parse_sentence_delayed_next_step_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(match effects.as_slice() {
            [effect] => effect.clone(),
            _ => EffectAst::Sequence { effects },
        });
    }

    if let Some(effect) =
        parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(tokens)
    {
        return Ok(effect);
    }

    if let Some(effect) = run_clause_primitives(tokens)? {
        return Ok(effect);
    }

    let clause = SubjectVerbPrimitiveClause::new(tokens);
    if let Some(unless_idx) = find_unquoted_token_word(clause, "unless") {
        let main_tokens = trim_commas(&tokens[..unless_idx]);
        if !main_tokens.is_empty()
            && let Ok(main_effect) = parse_effect_clause(&main_tokens)
            && let Some(unless_effect) = try_build_unless(vec![main_effect], clause, unless_idx)?
        {
            return Ok(unless_effect);
        }
    }

    if let Some(effect) = parse_has_base_power_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_passive_sacrifice_by_controller_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_copular_base_pt_animation_clause(tokens)? {
        return Ok(effect);
    }

    let choice_tokens = clause_grammar::strip_optional_you_choice_tokens(tokens);
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();

    if let Some((consumed, excluded_color)) = parse_choose_color_phrase_words(&choice_words)?
        && consumed == choice_words.len()
        && excluded_color.is_none()
    {
        return Ok(EffectAst::subject_verb_choose_color(
            crate::cards::builders::PlayerAst::Implicit,
        ));
    }

    if let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_creature_type(
            crate::cards::builders::PlayerAst::Implicit,
            excluded_subtypes,
        ));
    }

    if let Some(parsed) = parse_choice_land_type_phrase_words(&choice_words)
        && parsed.consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_land_type(
            crate::cards::builders::PlayerAst::Implicit,
            parsed.exclude_basic,
        ));
    }

    if let Some((consumed, options)) = parse_choose_card_type_phrase_words(&choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_card_type(
            crate::cards::builders::PlayerAst::Implicit,
            options,
        ));
    }

    if let Some(consumed) = parse_choose_player_phrase_words(&choice_words)
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_player(
            crate::cards::builders::PlayerAst::Implicit,
            PlayerFilter::Any,
            TagKey::from(IT_TAG),
            false,
            0,
        ));
    }

    if let Some(shape) = clause_grammar::parse_ordered_choose_all_shape(tokens) {
        let filter = parse_object_filter(shape.filter_tokens, false)?;
        let repeated_filter = parse_object_filter(shape.repeated_filter_tokens, false)?;
        if filter != repeated_filter {
            return Err(CardTextError::ParseError(format!(
                "ordered choice stopping filter differs from chosen filter (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(EffectAst::ChooseObjects {
            filter: filter.clone(),
            count: ChoiceCount::dynamic_x(),
            count_value: Some(
                Value::Count(filter).with_surface_hint(ValueSurfaceHint::ChooseAllInOrder),
            ),
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        });
    }

    if let Some(shape) = clause_grammar::parse_choose_target_shape(tokens)
        && let Ok(mut target) = parse_target_phrase(shape.target_tokens)
    {
        if shape.excludes_chooser_controller {
            preserve_target_choice_controller_exclusion(&mut target, shape.chooser);
        }
        let player_target = match &target {
            TargetAst::Player(_, _) => true,
            TargetAst::WithCount(inner, _) => matches!(inner.as_ref(), TargetAst::Player(_, _)),
            _ => false,
        };
        if player_target
            || clause_grammar::parse_clause_subject_verb_shape(shape.target_tokens).is_none()
        {
            return Ok(explicit_target_choice(shape, target));
        }
    }

    if let Some((chooser, choose_filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(tokens)?
    {
        return Ok(EffectAst::subject_verb_choose_player(
            chooser,
            choose_filter,
            TagKey::from(IT_TAG),
            random,
            exclude_previous_choices,
        ));
    }

    if let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_target_player_choose_objects_clause_with_count_value(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_you_choose_objects_clause_with_count_value(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if let Some(shape) = clause_grammar::parse_assigns_no_combat_damage_shape(tokens) {
        match shape {
            clause_grammar::AssignsNoCombatDamageShape::Unsupported => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported assigns-no-combat-damage clause tail (clause: '{}') [rule=assigns-no-combat-damage-tail]",
                    clause_words.join(" ")
                )));
            }
            clause_grammar::AssignsNoCombatDamageShape::Supported { source, duration } => {
                let source = match source {
                    clause_grammar::AssignDamageSourceShape::Source => TargetAst::Source(None),
                    clause_grammar::AssignDamageSourceShape::Tagged => {
                        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens))
                    }
                    clause_grammar::AssignDamageSourceShape::Target(target_tokens) => {
                        parse_target_phrase(target_tokens)?
                    }
                };
                return Ok(EffectAst::subject_verb_assign_no_combat_damage(
                    source, duration,
                ));
            }
        }
    }

    if starts_with_target_indicator(tokens)
        && find_negation_span(tokens)
            .is_some_and(|(neg_start, _)| find_verb(&tokens[..neg_start]).is_none())
        && let (duration, clause_tokens) =
            parse_restriction_duration(tokens)?.unwrap_or((Until::Forever, tokens.to_vec()))
        && let Some(restrictions) = parse_cant_restrictions(&clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && let Some(target) = parsed.target.clone()
    {
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_cant(parsed.restriction.clone(), duration, None),
            ],
        });
    }

    if let Some(shape) = clause_grammar::parse_target_only_shape(tokens) {
        if find_negation_span(tokens).is_some() || shape.restriction_like {
            return Err(CardTextError::ParseError(format!(
                "unsupported target-only restriction clause (clause: '{}') [rule=target-only-restriction]",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::subject_verb_target_only(target));
    }

    if let Some(shape) = clause_grammar::parse_embedded_choose_target_shape(tokens) {
        let mut target = parse_target_phrase(shape.target_tokens)?;
        if shape.excludes_chooser_controller {
            preserve_target_choice_controller_exclusion(&mut target, shape.chooser);
        }
        return Ok(match shape.chooser {
            _ => explicit_target_choice(shape, target),
        });
    }

    if let Some(effect) = parse_next_turn_cant_clause(tokens)? {
        return Ok(effect);
    }

    if let Some((duration, clause_tokens)) = parse_restriction_duration(tokens)?
        && find_negation_span(&clause_tokens).is_some()
        && let Some(restrictions) = parse_cant_restrictions(&clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && parsed.target.is_none()
    {
        return Ok(EffectAst::subject_verb_cant(
            parsed.restriction.clone(),
            duration,
            None,
        ));
    }

    if let Some(effect) = parse_hexproof_targeting_override_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_cast_target_without_paying_shape(tokens) {
        let _ = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::SubjectVerb(
            crate::runtime_backend::ast::SubjectVerbEffectAst {
                subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::Implicit,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: TagKey::from(IT_TAG),
                    player: PlayerAst::Implicit,
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
                },
            },
        ));
    }

    if let Some(effect) = parse_passive_goad_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_control_player_clause(tokens)? {
        return Ok(effect);
    }

    // Generic "X if <predicate>" fallback: clauses like "play the exiled card
    // without paying its mana cost if you attacked with three or more
    // creatures this turn" have no known leading verb, but the head parses on
    // its own and the tail is a recognizable predicate. Only attempted where
    // the clause would otherwise be a hard no-verb error.
    if clause_grammar::parse_clause_subject_verb_shape(tokens).is_none()
        && let Some(shape) = clause_grammar::parse_trailing_if_fallback_shape(tokens)
        && let Ok(head_effects) = super::parse_effect_sentence_lexed(shape.head_tokens)
        && !head_effects.is_empty()
    {
        parser_trace("parse_effect_clause:trailing-if-fallback", tokens);
        return Ok(EffectAst::Conditional {
            predicate: shape.predicate,
            if_true: head_effects,
            if_false: Vec::new(),
        });
    }

    let (verb, _) = find_verb(tokens).ok_or_else(|| {
        let clause = render_lower_words(tokens);
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "convert",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "look",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "unattach",
            "remove",
            "return",
            "exchange",
            "become",
            "switch",
            "skip",
            "surveil",
            "shuffle",
            "reorder",
            "pay",
            "detain",
            "goad",
            "suspect",
            "end",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    let verb_shape = clause_grammar::parse_clause_subject_verb_shape(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "could not split subject and verb in effect clause (clause: '{}')",
            render_lower_words(tokens)
        ))
    })?;
    let subject_tokens_storage = trim_commas(verb_shape.subject_tokens);
    let subject_tokens = subject_tokens_storage.as_slice();
    let rest = verb_shape.action_tokens;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb={verb:?} subject={}",
        if subject_tokens.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    ));

    if matches!(verb, Verb::Counter)
        && !subject_tokens.is_empty()
        && contains_token_word(tokens, "on")
        && let Ok(effect) = parse_put_counters(tokens)
    {
        parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
        return Ok(effect);
    }

    if matches!(verb, Verb::Get)
        && let Some(effect) = parse_get_pump_clause(subject_tokens, rest, tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Sacrifice)
        && let Some((subject, target)) = parse_controller_or_owner_of_target_subject(subject_tokens)
    {
        return parse_sacrifice(rest, Some(subject), Some(target));
    }
    if matches!(verb, Verb::Put)
        && let Some((SubjectAst::Player(PlayerAst::ItsOwner), target)) =
            parse_controller_or_owner_of_target_subject(subject_tokens)
    {
        if is_pronoun_top_or_bottom_library_choice_put_tail(rest) {
            return Ok(EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::ItsOwner,
                SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target },
            ));
        }
    }
    let subject_word_view = ClauseDispatchCompatWords::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    if is_target_player_dealt_damage_by_this_turn_subject(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history player subject (clause: '{}') [rule=combat-history-player-subject]",
            render_lower_words(tokens)
        )));
    }
    if matches!(verb, Verb::Gain) && !subject_tokens.is_empty() {
        if let Some(shape) = clause_grammar::parse_protection_choice_shape(rest) {
            let target = parse_target_phrase(subject_tokens)?;
            return Ok(EffectAst::subject_verb_grant_protection_choice(
                target,
                match shape.chooser {
                    clause_grammar::ProtectionChoiceChooserShape::You => PlayerAst::You,
                    clause_grammar::ProtectionChoiceChooserShape::TargetController => {
                        PlayerAst::ItsController
                    }
                },
                shape.includes_colorless,
                shape.includes_artifacts,
            ));
        }
    }
    if matches!(verb, Verb::Gain)
        && let Some(effects) =
            super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(EffectAst::Sequence { effects });
    }
    if matches!(verb, Verb::Gain)
        && let Some(effect) = parse_simple_gain_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Gain) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let parsed_actions = parse_ability_line(tail.ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(tail.ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !tail.ability_tokens.is_empty()
            && tail.trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_grant_abilities_to_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    if matches!(verb, Verb::Lose) && clause_grammar::parse_shared_ability_gain_shape(rest).is_some()
    {
        let target = match clause_grammar::parse_reference_subject_shape(subject_tokens) {
            clause_grammar::ReferenceSubjectShape::Source => {
                TargetAst::Source(span_from_tokens(subject_tokens))
            }
            clause_grammar::ReferenceSubjectShape::Tagged => {
                TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens))
            }
            clause_grammar::ReferenceSubjectShape::Other => parse_target_phrase(subject_tokens)?,
        };
        return Ok(EffectAst::subject_verb_remove_abilities_from_target(
            target,
            Vec::new(),
            Until::EndOfTurn,
        ));
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = parse_simple_lose_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Lose) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let ability_tokens = trim_edge_punctuation(tail.ability_tokens);
        let trailing_tokens = trim_edge_punctuation(tail.trailing_tokens);
        let parsed_actions = parse_ability_line(&ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(&ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !ability_tokens.is_empty()
            && trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_remove_abilities_from_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    let each_other_player = matches!(
        subject_words.as_slice(),
        ["each", "other", "player"] | ["each", "other", "players"]
    );
    if matches!(verb, Verb::Return)
        && clause_grammar::is_return_tagged_reference_shape(subject_tokens)
    {
        let mut return_tokens = subject_tokens.to_vec();
        return_tokens.extend(rest.iter().cloned());
        return parse_effect_with_verb(verb, Some(SubjectAst::This), &return_tokens);
    }
    if matches!(verb, Verb::Put)
        && clause_grammar::is_exiled_cards_to_hand_shape(subject_tokens, rest)
    {
        let filter = parse_object_filter(subject_tokens, false)?;
        return Ok(EffectAst::subject_verb_return_all_to_hand(filter));
    }
    let relative_player_subject = if matches!(verb, Verb::Gain)
        && rest.first().is_some_and(|token| token.is_word("control"))
        && subject_tokens
            .first()
            .is_some_and(|token| token.is_word(TARGET_WORD))
    {
        match parse_target_phrase(subject_tokens) {
            Ok(target) => match &target {
                TargetAst::Player(filter, _)
                    if !matches!(filter, PlayerFilter::Any | PlayerFilter::Opponent) =>
                {
                    Some(target)
                }
                _ => None,
            },
            Err(_) => None,
        }
    } else {
        None
    };
    let mut effect = if let Some(target) = relative_player_subject {
        let source_relative_target = target_player_mentions_source_object(&target);
        let mut gain_control =
            parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        if source_relative_target {
            bind_gain_control_pronoun_to_source(&mut gain_control);
        }
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), gain_control],
        }
    } else if matches!(verb, Verb::Become) {
        parse_become_clause(subject_tokens, rest)?
    } else {
        let subject = if each_other_player {
            SubjectAst::Player(PlayerAst::That)
        } else {
            parse_subject(subject_tokens)
        };
        if let Some(clause) = CommonPlayerActionClause::recognize(subject, verb, rest) {
            clause.lower()?
        } else {
            parse_effect_with_verb(verb, Some(subject), rest)?
        }
    };
    if let Some(filter) = for_each_subject_filter {
        effect = EffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        };
    }
    if each_other_player {
        effect = EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::NotYou,
            effects: vec![effect],
        };
    }
    Ok(effect)
}

/// Parse the coordinated conditional animation used by effects such as
/// "that permanent becomes saddled if it's a Mount and becomes an artifact
/// creature if it's a Vehicle".  The ordinary trailing-if splitter cannot
/// consume this shape because the first predicate is followed by another
/// effect rather than the end of the clause.
pub(crate) fn parse_conditional_become_pair(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some((verb, _)) = find_verb(tokens) else {
        return Ok(None);
    };
    if verb != Verb::Become {
        return Ok(None);
    }
    let Some(shape) = clause_grammar::parse_clause_subject_verb_shape(tokens) else {
        return Ok(None);
    };

    let words = parser_token_word_positions(shape.action_tokens);
    let Some((first_if_idx, _)) = words.iter().find(|(_, word)| *word == "if") else {
        return Ok(None);
    };
    let Some((and_idx, _)) = words
        .iter()
        .find(|(idx, word)| *idx > *first_if_idx && *word == "and")
    else {
        return Ok(None);
    };
    let Some((second_become_idx, _)) = words
        .iter()
        .find(|(idx, word)| *idx > *and_idx && *word == "becomes")
    else {
        return Ok(None);
    };
    let Some((second_if_idx, _)) = words
        .iter()
        .find(|(idx, word)| *idx > *second_become_idx && *word == "if")
    else {
        return Ok(None);
    };

    let first_body = trim_lexed_commas(&shape.action_tokens[..*first_if_idx]);
    let first_predicate_tokens =
        trim_lexed_commas(&shape.action_tokens[*first_if_idx + 1..*and_idx]);
    let second_body =
        trim_lexed_commas(&shape.action_tokens[*second_become_idx + 1..*second_if_idx]);
    let second_predicate_tokens = trim_lexed_commas(&shape.action_tokens[*second_if_idx + 1..]);
    if first_body.is_empty()
        || first_predicate_tokens.is_empty()
        || second_body.is_empty()
        || second_predicate_tokens.is_empty()
    {
        return Ok(None);
    }

    let first_predicate = parse_predicate_with_grammar_entrypoint_lexed(first_predicate_tokens)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported conditional become predicate (clause: '{}')",
                render_lower_words(first_predicate_tokens)
            ))
        })?;
    let second_predicate = parse_predicate_with_grammar_entrypoint_lexed(second_predicate_tokens)
        .map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported conditional become predicate (clause: '{}')",
            render_lower_words(second_predicate_tokens)
        ))
    })?;

    let first_effect = parse_become_clause(shape.subject_tokens, first_body)?;
    let second_effect = parse_become_clause(shape.subject_tokens, second_body)?;
    Ok(Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::Conditional {
                predicate: first_predicate,
                if_true: vec![first_effect],
                if_false: Vec::new(),
            },
            EffectAst::Conditional {
                predicate: second_predicate,
                if_true: vec![second_effect],
                if_false: Vec::new(),
            },
        ],
    }))
}

fn parse_passive_goad_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_passive_goad_shape(tokens) else {
        return Ok(None);
    };
    let target = match shape.target {
        clause_grammar::GoadTargetShape::TaggedToken => {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens))
        }
        clause_grammar::GoadTargetShape::Target(target_tokens) => {
            parse_target_phrase(target_tokens)?
        }
    };
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let duration = if shape.for_rest_of_game {
        Until::Forever
    } else {
        Until::YourNextTurn
    };
    Ok(Some(EffectAst::subject_verb_goad_for(target, duration)))
}

fn parse_hexproof_targeting_override_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let (duration, clause_tokens) =
        parse_restriction_duration(tokens)?.unwrap_or((Until::Forever, tokens.to_vec()));
    let Some(shape) = clause_grammar::parse_hexproof_targeting_override_shape(&clause_tokens)
    else {
        return Ok(None);
    };
    let filter = parse_object_filter(shape.filter_tokens, false)?;
    Ok(Some(EffectAst::subject_verb_remove_abilities_all(
        filter,
        vec![GrantedAbilityAst::KeywordAction(KeywordAction::Hexproof)],
        duration,
    )))
}

pub(crate) fn parse_effect_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_clause(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::ast::SubjectVerbEffectAst;
    use crate::runtime_backend::lexer::lex_line;

    fn lex_tail(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex test tail")
    }

    #[test]
    fn only_authored_choose_target_clauses_are_explicit_declarations() {
        let authored = parse_effect_clause(&lex_tail("Choose target opponent."))
            .expect("parse authored target declaration");
        let authored = match authored {
            EffectAst::TagAffected { effect, .. } => *effect,
            effect => effect,
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    explicit_declaration: true,
                    ..
                },
            ..
        }) = authored
        else {
            panic!("expected explicit target declaration");
        };

        let synthetic = EffectAst::subject_verb_target_only(TargetAst::Player(
            PlayerFilter::target_opponent(),
            None,
        ));
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    explicit_declaration: false,
                    ..
                },
            ..
        }) = synthetic
        else {
            panic!("expected synthetic target prelude");
        };
    }

    #[test]
    fn common_player_action_clause_classifies_core_shapes() {
        let subject = SubjectAst::Player(PlayerAst::TargetOpponent);
        for (verb, tail, expected) in [
            (
                Verb::Draw,
                "X cards where X is their devotion to black",
                CommonPlayerActionPattern::Amount,
            ),
            (
                Verb::Sacrifice,
                "a creature they control",
                CommonPlayerActionPattern::ObjectSelection,
            ),
            (
                Verb::Shuffle,
                "their graveyard into their library",
                CommonPlayerActionPattern::ZoneMovement,
            ),
            (Verb::Pay, "{2}", CommonPlayerActionPattern::Payment),
            (Verb::Scry, "X", CommonPlayerActionPattern::Choice),
        ] {
            let tail = lex_tail(tail);
            let clause = CommonPlayerActionClause::recognize(subject.clone(), verb, &tail)
                .expect("common player clause should be recognized");
            assert_eq!(clause.pattern(), expected, "{verb:?} {tail:?}");
        }
    }

    #[test]
    fn common_player_action_clause_recognizes_typed_clause_variants() {
        let subject = SubjectAst::Player(PlayerAst::TargetOpponent);
        for (verb, tail, assert_variant) in [
            (
                Verb::Draw,
                "X cards where X is their devotion to black",
                matches_amount as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Sacrifice,
                "a creature they control",
                matches_object as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Shuffle,
                "their graveyard into their library",
                matches_zone as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Scry,
                "X",
                matches_choice as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Pay,
                "{2}",
                matches_payment as fn(CommonPlayerActionClause<'_>),
            ),
        ] {
            let tail = lex_tail(tail);
            let clause = CommonPlayerActionClause::recognize(subject.clone(), verb, &tail)
                .expect("common player clause should be recognized");
            assert_variant(clause);
        }
    }

    fn matches_amount(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Amount(_)));
    }

    fn matches_object(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Object(_)));
    }

    fn matches_zone(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Zone(_)));
    }

    fn matches_choice(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Choice(_)));
    }

    fn matches_payment(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Payment(_)));
    }

    #[test]
    fn common_player_action_clause_delegates_to_effect_parser() {
        for text in [
            "Target opponent draws a card",
            "Target opponent sacrifices a creature they control",
            "Target opponent shuffles their library",
            "Target opponent pays {2}",
            "Each opponent scries 1",
        ] {
            let tokens = lex_line(text, 0).expect("lex clause");
            parse_effect_clause(&tokens)
                .unwrap_or_else(|err| panic!("common player clause should parse: {text}: {err:?}"));
        }
    }

    #[test]
    fn any_player_sacrifice_offer_keeps_sequential_player_semantics() {
        let tokens = lex_line("Any player may sacrifice two creatures of their choice.", 0)
            .expect("lex any-player sacrifice offer");
        let effect = parse_effect_clause(&tokens).expect("parse any-player sacrifice offer");

        let EffectAst::AnyPlayerMay { players, effects } = effect else {
            panic!("expected typed any-player offer, got {effect:#?}");
        };
        assert_eq!(players, PlayerFilter::Any);
        let [
            EffectAst::Sequence {
                effects: sacrifice_steps,
            },
        ] = effects.as_slice()
        else {
            panic!("expected a choose-and-sacrifice sequence, got {effects:#?}");
        };
        assert!(
            matches!(
                sacrifice_steps.as_slice(),
                [
                    EffectAst::ChooseObjects {
                        player: PlayerAst::That,
                        ..
                    },
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                            player: PlayerAst::That,
                            ..
                        },
                        action: SubjectVerbActionAst::SacrificeAll { .. },
                    })
                ]
            ),
            "expected both choice and sacrifice to stay bound to the offered player, got {sacrifice_steps:#?}"
        );
    }

    #[test]
    fn explicit_player_attach_clause_preserves_the_attachment_chooser() {
        let tokens = lex_line(
            "That player attaches this Aura to a land of their choice.",
            0,
        )
        .expect("lex explicit-player attach clause");
        let effect = parse_effect_clause(&tokens).expect("parse explicit-player attach clause");

        assert!(
            matches!(
                &effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                        player: PlayerAst::That,
                        ..
                    },
                    action: SubjectVerbActionAst::Attach {
                        target: TargetAst::WithCount(_, count),
                        ..
                    },
                }) if count.is_single()
            ),
            "explicit attach actor and counted destination must survive parsing: {effect:#?}"
        );
    }

    #[test]
    fn targeted_graveyard_cast_permission_preserves_one_target_and_duration() {
        use crate::types::{CardType, Subtype};

        let cases = [
            (
                "You may cast target nonland card from your graveyard this turn.",
                vec![],
                vec![],
                true,
            ),
            (
                "You may cast target artifact card from your graveyard this turn.",
                vec![CardType::Artifact],
                vec![],
                false,
            ),
            (
                "You may cast target enchantment card from your graveyard this turn.",
                vec![CardType::Enchantment],
                vec![],
                false,
            ),
            (
                "You may cast target Zombie creature card from your graveyard this turn.",
                vec![CardType::Creature],
                vec![Subtype::Zombie],
                false,
            ),
        ];

        for (text, card_types, subtypes, excludes_land) in cases {
            let tokens = lex_line(text, 0).expect("lex targeted graveyard permission");
            let effect = parse_effect_clause(&tokens).expect("targeted graveyard permission");
            let EffectAst::Sequence { effects } = effect else {
                panic!("expected target-plus-grant sequence for {text}");
            };
            let [
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::TargetOnly { target, .. },
                    ..
                }),
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                            tag,
                            player,
                            allow_land,
                            without_paying_mana_cost,
                            allow_any_color_for_cast,
                            while_on_top_of_library,
                        },
                    ..
                }),
            ] = effects.as_slice()
            else {
                panic!("expected one target followed by one tagged cast grant for {text}");
            };
            let TargetAst::Object(filter, _, _) = target else {
                panic!("expected a single object target for {text}");
            };
            assert_eq!(filter.zone, Some(Zone::Graveyard), "{text}");
            assert_eq!(filter.owner, Some(PlayerFilter::You), "{text}");
            assert_eq!(filter.card_types, card_types, "{text}");
            assert_eq!(filter.subtypes, subtypes, "{text}");
            assert_eq!(
                filter.excluded_card_types.contains(&CardType::Land),
                excludes_land,
                "{text}"
            );
            assert_eq!(tag.as_str(), IT_TAG, "{text}");
            assert_eq!(*player, PlayerAst::You, "{text}");
            assert!(!*allow_land, "{text}");
            assert!(!*without_paying_mana_cost, "{text}");
            assert_eq!(
                *allow_any_color_for_cast,
                ironsmith_core::value_model::ManaSpendMode::Normal,
                "{text}"
            );
            assert!(!*while_on_top_of_library, "{text}");
        }
    }

    #[test]
    fn leading_may_chain_reaches_targeted_graveyard_cast_permission() {
        let tokens = lex_line(
            "You may cast target Zombie creature card from your graveyard this turn.",
            0,
        )
        .expect("lex leading-may targeted graveyard permission");
        let effects =
            crate::runtime_backend::sentences::effect_sentences::parse_effect_chain_lexed(&tokens)
                .expect("parse through production leading-may chain");

        let [EffectAst::Sequence { effects }] = effects.as_slice() else {
            panic!("expected the chain to preserve a target-plus-grant sequence: {effects:#?}");
        };
        assert!(matches!(
            effects.as_slice(),
            [
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::TargetOnly { .. },
                    ..
                }),
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { .. },
                    ..
                }),
            ]
        ));
    }

    #[test]
    fn plural_demonstrative_pump_preserves_tagged_set() {
        let tokens =
            lex_line("Those creatures get +1/+1 until end of turn.", 0).expect("lex plural pump");
        let effect = parse_effect_clause(&tokens).expect("plural tagged pump should parse");
        let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Pump {
                    target: TargetAst::Object(filter, ..),
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected a mass pump for a plural demonstrative subject");
        };
        assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    #[test]
    fn plural_demonstrative_untap_preserves_typed_tagged_set() {
        let tokens = lex_line("Untap those creatures.", 0).expect("lex plural untap");
        let effect = parse_effect_clause(&tokens).expect("plural tagged untap should parse");

        let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: SubjectVerbActionAst::UntapAll { filter },
            ..
        }) = effect
        else {
            panic!("expected a mass untap for a plural demonstrative subject");
        };
        assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    #[test]
    fn each_other_player_subject_lowers_to_filtered_player_iteration() {
        let tokens = lex_line("Each other player loses X life.", 0).expect("lex clause");
        let effect = parse_effect_clause(&tokens).expect("each-other-player clause should parse");

        let EffectAst::ForEachPlayersFiltered { filter, effects } = effect else {
            panic!("expected filtered player iteration");
        };
        assert_eq!(filter, PlayerFilter::NotYou);
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    subject: crate::cards::builders::SubjectVerbSubjectAst {
                        player: PlayerAst::That,
                        ..
                    },
                    action: SubjectVerbActionAst::LoseLife { .. },
                }
            )]
        ));
    }

    #[test]
    fn parses_control_target_player_during_next_turn_clause() {
        let tokens = lex_line(
            "You control target player during that player's next turn.",
            0,
        )
        .expect("lex clause");
        let effect = parse_effect_clause(&tokens)
            .expect("control target player during next turn should parse");
        let debug = format!("{effect:?}").to_ascii_lowercase();
        assert!(
            debug.contains("controlplayer") && debug.contains("nextturn"),
            "expected control-player-next-turn effect, got {debug}"
        );
    }

    #[test]
    fn counter_linked_land_subtype_followup_lowers_to_prior_tagged_land() {
        let tokens = lex_line(
            "That land is an Island in addition to its other types for as long as it has a flood counter on it.",
            0,
        )
        .unwrap();
        let effect = parse_effect_clause(&tokens).expect("typed land subtype followup");
        let debug = format!("{effect:#?}");
        assert!(debug.contains("AddSubtypes"), "{debug}");
        assert!(debug.contains("Island"), "{debug}");
        assert!(debug.contains(IT_TAG), "{debug}");
        assert!(
            debug.contains("ForAsLongAs") && debug.contains("Flood"),
            "{debug}"
        );
    }

    #[test]
    fn filtered_combat_damage_prevention_keeps_non_subtype_source_filter() {
        let tokens = lex_line(
            "Prevent all combat damage non-Soldier creatures would deal this turn.",
            0,
        )
        .unwrap();
        effect_grammar::parse_prevent_damage_sentence_lexed(&tokens)
            .expect("typed prevention grammar should not error")
            .expect("typed prevention grammar should recognize filtered source");
        let effect = parse_effect_clause(&tokens).expect("typed filtered prevention");
        let debug = format!("{effect:#?}");
        assert!(debug.contains("PreventAllCombatDamage"), "{debug}");
        assert!(debug.contains("Soldier"), "{debug}");
        assert!(debug.contains("excluded_subtypes"), "{debug}");
    }

    #[test]
    fn discarded_this_way_pump_split_keeps_typed_modifier_tail() {
        let tokens = lex_line(
            "target creature gets +2/+0 until end of turn for each card discarded this way",
            0,
        )
        .unwrap();
        let shape = clause_grammar::parse_clause_subject_verb_shape(&tokens).unwrap();
        assert!(
            clause_grammar::parse_discarded_this_way_modifier_shape(shape.action_tokens).is_some(),
            "{:?}",
            shape.action_tokens
        );
    }

    #[test]
    fn tagged_plural_pump_clause_lowers_directly() {
        let tokens = lex_line("they each get +2/+2 until end of turn", 0).unwrap();
        let shape = clause_grammar::parse_clause_subject_verb_shape(&tokens).unwrap();
        assert_eq!(
            ClauseDispatchCompatWords::new(shape.subject_tokens).word_refs(),
            ["they", "each"]
        );
        let effect = parse_get_pump_clause(shape.subject_tokens, shape.action_tokens, &tokens)
            .expect("tagged plural pump should not error")
            .expect("tagged plural pump should be recognized");
        assert!(
            matches!(effect, EffectAst::SubjectVerb(_)),
            "expected typed subject-verb pump, got {effect:?}"
        );
    }
}
