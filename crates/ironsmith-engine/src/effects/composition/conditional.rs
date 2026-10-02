//! Conditional effect implementation.

use crate::effect::{Condition, EffectOutcome};
use crate::effects::{EffectExecutor, ModalSpec};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::ChooseSpec;
pub type ConditionalEffect = ironsmith_core::ConditionalEffect<crate::effect::Effect>;

fn unwrapped_effect(mut effect: &crate::effect::Effect) -> &crate::effect::Effect {
    while let Some(inner) = effect.transparent_child_effect() {
        effect = inner;
    }
    effect
}

fn reveal_claim_filter(
    filter: &crate::target::ObjectFilter,
    game: &GameState,
    source: ObjectId,
) -> crate::target::ObjectFilter {
    let mut claim = filter.clone();
    claim.any_of = filter
        .any_of
        .iter()
        .map(|branch| reveal_claim_filter(branch, game, source))
        .collect();
    if claim.shares_creature_type_with_source {
        // Claims can be checked after the source leaves or changes types.
        // Capture the public creature types at the instruction, rather than
        // consulting the later source (or a placeholder's cached subtypes)
        // when validating the opened card.
        let subtypes = game
            .current_subtypes(source)
            .or_else(|| game.object(source).map(|object| object.subtypes.to_vec()))
            .unwrap_or_default()
            .into_iter()
            .filter(|subtype| subtype.is_creature_type())
            .collect::<Vec<_>>();
        claim.shares_creature_type_with_source = false;
        let mut relation = if subtypes.is_empty() {
            crate::target::ObjectFilter {
                source: true,
                other: true,
                ..Default::default()
            }
        } else {
            crate::target::ObjectFilter {
                subtypes,
                ..Default::default()
            }
        };
        // The filter's base qualities are ANDed with its any_of group.
        // Nest the old group to preserve its constraints while adding this
        // captured relation, including when it already contains a union.
        relation.any_of = std::mem::take(&mut claim.any_of);
        claim.any_of = vec![relation];
    }
    claim
}

/// "If the privately looked-at card matches, you may reveal it. If you do,
/// ..." must reach the optional reveal on every peer. Even a known mismatch
/// needs an explicit decline: skipping it would disclose the conditional's
/// result through the decision sequence and strand a concealed peer.
///
/// Only fold a condition into the offer when revealing the same single card
/// is its first action and every subsequent instruction depends on accepting
/// that offer. Arbitrary conditionals and else branches cannot be folded this
/// way without changing their semantics.
fn optional_hidden_reveal_guard(
    effect: &ConditionalEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    condition_matches: bool,
) -> Option<crate::effects::context::OptionalIdentityGuard> {
    use crate::effects::{IfEffect, MayEffect, RevealTaggedEffect, WithIdEffect};
    if !effect.if_false.is_empty() {
        return None;
    }
    let Condition::TaggedObjectMatches(tag, filter) = &effect.condition else {
        return None;
    };
    if !filter.has_search_stated_quality() {
        return None;
    }
    let first = effect.if_true.first()?;
    let may = unwrapped_effect(first).downcast_ref::<MayEffect>()?;
    if may.effects.len() != 1 {
        return None;
    }
    let reveal = unwrapped_effect(&may.effects[0]).downcast_ref::<RevealTaggedEffect>()?;
    if &reveal.tag != tag {
        return None;
    }
    let mut outcome_ids = Vec::new();
    let mut wrapped = first;
    loop {
        if let Some(with_id) = wrapped.downcast_ref::<WithIdEffect>() {
            outcome_ids.push(with_id.id);
        }
        let Some(inner) = wrapped.transparent_child_effect() else {
            break;
        };
        wrapped = inner;
    }
    if !effect.if_true.iter().skip(1).all(|effect| {
        unwrapped_effect(effect)
            .downcast_ref::<IfEffect>()
            .is_some_and(|if_effect| {
                outcome_ids.contains(&if_effect.condition)
                    && matches!(
                        if_effect.predicate,
                        crate::effect::EffectPredicate::Happened
                    )
                    && if_effect.else_.is_empty()
            })
    }) {
        return None;
    }
    let snapshots = ctx.get_tagged_all(tag)?;
    if snapshots.len() != 1 {
        return None;
    }
    let snapshot = &snapshots[0];
    let object = game.object(snapshot.object_id).or_else(|| {
        game.find_object_by_stable_id(snapshot.stable_id)
            .and_then(|id| game.object(id))
    })?;
    // Tracking and zone membership are symmetric; local knowledge and public
    // openings are not a reason to remove this decision during replay.
    if game.hidden_card_info(object.id).is_none() || !object.zone.is_hidden() {
        return None;
    }
    Some(crate::effects::context::OptionalIdentityGuard {
        object: object.id,
        filter: reveal_claim_filter(filter, game, ctx.source),
        filter_ctx: ctx.filter_context(game),
        can_accept: game.is_hidden_card_placeholder(object.id) || condition_matches,
    })
}

/// Effect that branches based on game state conditions.
///
/// Unlike `If` which checks the result of a prior effect, `Conditional`
/// evaluates game state conditions like "if you control a creature" or
/// "if your life total is 10 or less".
///
/// # Fields
///
/// * `condition` - The game state condition to check
/// * `if_true` - Effects to execute if condition is true
/// * `if_false` - Effects to execute if condition is false
///
/// # Example
///
/// ```ignore
/// // If you control a creature, draw a card. Otherwise, gain 2 life.
/// let effect = ConditionalEffect::new(
///     Condition::YouControl(ObjectFilter::creature()),
///     vec![Effect::draw(1)],
///     vec![Effect::gain_life(2)],
/// );
/// ```
impl EffectExecutor for ConditionalEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for effect in &self.if_true {
            visitor(effect);
        }
        for effect in &self.if_false {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut result = evaluate_condition(game, &self.condition, ctx)?;
        let identity_guard = optional_hidden_reveal_guard(self, game, ctx, result);

        // CR 700.2 / 601.2b: "If [condition] as you cast this spell, you may
        // choose both instead" fixes how many modes may be chosen during
        // casting (603.3c for triggers). When the announced modes only fit
        // the other branch's mode choice, that branch was the one in force at
        // announcement; don't let a changed condition reject the choice.
        if let Some(chosen) = ctx.chosen_modes.as_deref() {
            let (current, other) = if result {
                (&self.if_true, &self.if_false)
            } else {
                (&self.if_false, &self.if_true)
            };
            if let (Some(current_max), Some(other_max)) = (
                announced_mode_choice_max(game, current, ctx)?,
                announced_mode_choice_max(game, other, ctx)?,
            ) && chosen.len() > current_max
                && chosen.len() <= other_max
            {
                result = !result;
            }
        }

        let effects_to_execute = if result || identity_guard.is_some() {
            &self.if_true
        } else {
            &self.if_false
        };

        let mut outcomes = Vec::new();
        for (index, effect) in effects_to_execute.iter().enumerate() {
            if index == 0 && identity_guard.is_some() {
                let previous_guard = ctx.optional_identity_guard.take();
                ctx.optional_identity_guard = identity_guard.clone();
                let outcome = execute_effect(game, effect, ctx);
                ctx.optional_identity_guard = previous_guard;
                outcomes.push(outcome?);
            } else {
                outcomes.push(execute_effect(game, effect, ctx)?);
            }
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if let Some(next) = effects_to_execute.get(index + 1) {
                crate::effects::match_triggers_at_instruction_boundary(
                    game,
                    ctx,
                    Some(next),
                    outcomes.iter().flat_map(|outcome| outcome.events.iter()),
                );
            }
        }

        if outcomes.is_empty() {
            // A false conditional with no else branch did not perform the
            // guarded action. Preserve that distinction for trailing
            // "otherwise" effects and other result predicates.
            return Ok(EffectOutcome::count(0));
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.if_true, &self.if_false])
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.if_true, &self.if_false])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.if_true, &self.if_false], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.if_true, &self.if_false])
    }

    fn get_modal_spec_with_context(
        &self,
        game: &GameState,
        controller: PlayerId,
        source: ObjectId,
    ) -> Option<ModalSpec> {
        // Evaluate the condition at cast time to determine which branch to use
        let condition_result = evaluate_condition_simple(game, &self.condition, controller, source);

        // Search the appropriate branch for modal specs
        let effects_to_search = if condition_result {
            &self.if_true
        } else {
            &self.if_false
        };

        // Recursively search through the effects in this branch
        for effect in effects_to_search {
            if let Some(spec) = effect
                .0
                .get_modal_spec_with_context(game, controller, source)
            {
                return Some(spec);
            }
        }

        None
    }
}

/// The most modes the branch's cast-time mode choice allows, if the branch
/// opens with one.
fn announced_mode_choice_max(
    game: &GameState,
    effects: &[crate::effect::Effect],
    ctx: &ExecutionContext,
) -> Result<Option<usize>, ExecutionError> {
    let Some(choose) = effects.iter().find_map(|effect| {
        effect
            .downcast_ref::<crate::effects::ChooseModeEffect>()
            .filter(|choose| choose.chooser.is_none())
    }) else {
        return Ok(None);
    };
    Ok(Some(
        crate::effects::helpers::resolve_value(game, &choose.choose_count, ctx)?.max(0) as usize,
    ))
}

fn evaluate_condition_simple(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    crate::condition_eval::evaluate_condition_cast_time(game, condition, controller, source)
}

fn evaluate_condition(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    crate::condition_eval::evaluate_condition_resolution(game, condition, ctx)
}

#[cfg(test)]
#[path = "conditional_hidden_tests.rs"]
mod hidden_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{ChoiceCount, Condition};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::ObjectFilter;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn make_creature_card(card_id: u32, name: &str, symbol: ManaSymbol) -> crate::card::Card {
        make_creature_card_with_symbols(card_id, name, &[symbol])
    }

    fn make_creature_card_with_symbols(
        card_id: u32,
        name: &str,
        symbols: &[ManaSymbol],
    ) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(
                symbols.iter().copied().map(|symbol| vec![symbol]).collect(),
            ))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
        symbol: ManaSymbol,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name, symbol);
        let obj = crate::object::Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_creature_with_symbols(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
        symbols: &[ManaSymbol],
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card_with_symbols(id.0 as u32, name, symbols);
        let obj = crate::object::Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn conditional_forwards_inner_target_spec_from_if_true() {
        let effect = ConditionalEffect::if_only(
            Condition::YourTurn,
            vec![Effect::counter(ChooseSpec::target_spell())],
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn conditional_forwards_inner_target_spec_from_if_false() {
        let effect = ConditionalEffect::new(
            Condition::YourTurn,
            vec![Effect::draw(1)],
            vec![Effect::counter(ChooseSpec::target_spell())],
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn conditional_shares_color_with_tagged_target_gates_combat_prevention() {
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let tagged_permanent = create_creature(&mut game, "Guard Marker", alice, ManaSymbol::Red);
        let matching_target =
            create_creature(&mut game, "Matching Attacker", alice, ManaSymbol::Red);
        let tagged_snapshot = ObjectSnapshot::from_object(
            game.object(tagged_permanent).expect("tagged permanent"),
            &game,
        );
        let matching_tags: HashMap<TagKey, Vec<ObjectSnapshot>> =
            HashMap::from([(TagKey::from("it"), vec![tagged_snapshot.clone()])]);
        let mut matching_ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(matching_target)])
            .with_tagged_objects(matching_tags);

        let effect = Effect::new(ConditionalEffect::if_only(
            Condition::TargetMatches(
                ObjectFilter::creature().shares_color_with_tagged(TagKey::from("it")),
            ),
            vec![Effect::prevent_all_combat_damage_from(
                ChooseSpec::target_creature(),
                crate::effect::Until::EndOfTurn,
            )],
        ));

        execute_effect(&mut game, &effect, &mut matching_ctx)
            .expect("matching target should resolve");
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);

        let matching_source_colors = game
            .object(matching_target)
            .expect("matching target")
            .colors();
        let matching_source_types = game
            .object(matching_target)
            .expect("matching target")
            .card_types
            .clone();
        let prevented = game
            .effect_store
            .prevention_effects
            .apply_prevention_to_player(
                alice,
                3,
                true,
                matching_target,
                &matching_source_colors,
                &matching_source_types,
                true,
            );
        assert_eq!(prevented, 0);

        let mut nonmatching_game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice2 = PlayerId::from_index(0);
        let source2 = nonmatching_game.new_object_id();
        let tagged_permanent2 = create_creature(
            &mut nonmatching_game,
            "Guard Marker",
            alice2,
            ManaSymbol::Red,
        );
        let _matching_target2 = create_creature(
            &mut nonmatching_game,
            "Matching Attacker",
            alice2,
            ManaSymbol::Red,
        );
        let nonmatching_target2 = create_creature(
            &mut nonmatching_game,
            "Nonmatching Attacker",
            alice2,
            ManaSymbol::Blue,
        );
        let tagged_snapshot2 = ObjectSnapshot::from_object(
            nonmatching_game
                .object(tagged_permanent2)
                .expect("tagged permanent"),
            &nonmatching_game,
        );
        let nonmatching_tags2: HashMap<TagKey, Vec<ObjectSnapshot>> =
            HashMap::from([(TagKey::from("it"), vec![tagged_snapshot2])]);
        let mut nonmatching_ctx = ExecutionContext::new_default(source2, alice2)
            .with_targets(vec![ResolvedTarget::Object(nonmatching_target2)])
            .with_tagged_objects(nonmatching_tags2);

        execute_effect(&mut nonmatching_game, &effect, &mut nonmatching_ctx)
            .expect("nonmatching target should resolve");
        assert!(
            nonmatching_game
                .effect_store
                .prevention_effects
                .shields()
                .is_empty(),
            "expected no shield for nonmatching target"
        );
    }

    #[test]
    fn conditional_target_color_sets_destroy_only_when_sets_are_equal() {
        let mut same_game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let same_first = create_creature_with_symbols(
            &mut same_game,
            "First Blue-Red Creature",
            bob,
            &[ManaSymbol::Blue, ManaSymbol::Red],
        );
        let same_second = create_creature_with_symbols(
            &mut same_game,
            "Second Blue-Red Creature",
            bob,
            &[ManaSymbol::Red, ManaSymbol::Blue],
        );
        let same_spec =
            ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2));
        let same_effect = Effect::new(ConditionalEffect::if_only(
            Condition::Not(Box::new(Condition::TargetObjectsHaveDifferentColorSets)),
            vec![Effect::new(crate::effects::DestroyEffect::with_spec(
                same_spec.clone(),
            ))],
        ));
        let mut same_ctx = ExecutionContext::new_default(same_game.new_object_id(), alice)
            .with_targets(vec![
                ResolvedTarget::Object(same_first),
                ResolvedTarget::Object(same_second),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: same_spec,
                range: 0..2,
            }]);

        execute_effect(&mut same_game, &same_effect, &mut same_ctx)
            .expect("equal target color sets should resolve");
        assert!(
            [same_first, same_second].into_iter().all(|id| same_game
                .object(id)
                .is_none_or(|object| object.zone != Zone::Battlefield)),
            "both equal-color-set targets should be destroyed"
        );

        let mut different_game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let different_first = create_creature_with_symbols(
            &mut different_game,
            "Blue-Red Creature",
            bob,
            &[ManaSymbol::Blue, ManaSymbol::Red],
        );
        let different_second = create_creature_with_symbols(
            &mut different_game,
            "Red Creature",
            bob,
            &[ManaSymbol::Red],
        );
        let different_spec =
            ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2));
        let different_effect = Effect::new(ConditionalEffect::if_only(
            Condition::Not(Box::new(Condition::TargetObjectsHaveDifferentColorSets)),
            vec![Effect::new(crate::effects::DestroyEffect::with_spec(
                different_spec.clone(),
            ))],
        ));
        let mut different_ctx =
            ExecutionContext::new_default(different_game.new_object_id(), alice)
                .with_targets(vec![
                    ResolvedTarget::Object(different_first),
                    ResolvedTarget::Object(different_second),
                ])
                .with_target_assignments(vec![crate::game_state::TargetAssignment {
                    spec: different_spec,
                    range: 0..2,
                }]);

        execute_effect(&mut different_game, &different_effect, &mut different_ctx)
            .expect("different target color sets should resolve");
        assert!(
            [different_first, different_second]
                .into_iter()
                .all(|id| different_game
                    .object(id)
                    .is_some_and(|object| object.zone == Zone::Battlefield)),
            "overlapping but unequal color sets must prevent destruction"
        );
    }
}
