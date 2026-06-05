//! Move to zone effect implementation.

use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::helpers::{resolve_objects_for_effect, resolve_tagged_object_id};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{EventOutcome, process_zone_change_with_additional_effects};
use crate::filter::FilterContext;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::SOURCE_EXILED_TAG;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

use super::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, finalize_zone_change_move,
    maybe_prompt_for_split_result_order, move_to_battlefield_with_options,
    take_recorded_zone_change,
};
pub use ironsmith_core::BattlefieldController;
pub type MoveToZoneEffect = ironsmith_core::MoveToZoneEffect;

fn fixed_cost_filter(effect: &MoveToZoneEffect) -> Option<(&ObjectFilter, usize)> {
    let ChooseSpec::Object(filter) = effect.target.base() else {
        return None;
    };
    let count = effect.target.count();
    if count.min == 0 || count.max != Some(count.min) {
        return None;
    }
    Some((filter, count.min as usize))
}

fn matching_cost_candidate_count(
    game: &GameState,
    filter: &ObjectFilter,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
) -> usize {
    let filter_ctx = FilterContext::new(controller).with_source(source);
    let candidate_ids: Vec<_> = match filter.zone {
        Some(Zone::Hand) => game
            .players
            .iter()
            .flat_map(|player| player.hand.iter().copied())
            .collect(),
        Some(Zone::Graveyard) => game
            .players
            .iter()
            .flat_map(|player| player.graveyard.iter().copied())
            .collect(),
        Some(Zone::Battlefield) => game.battlefield.clone(),
        Some(Zone::Library) => game
            .players
            .iter()
            .flat_map(|player| player.library.iter().copied())
            .collect(),
        Some(Zone::OutsideGame) => game
            .players
            .iter()
            .flat_map(|player| player.sideboard.iter().copied())
            .collect(),
        Some(Zone::Stack) => game.stack.iter().map(|entry| entry.object_id).collect(),
        Some(Zone::Exile) => game.exile.clone(),
        Some(Zone::Command) => game.command_zone.clone(),
        None => Vec::new(),
    };

    candidate_ids
        .into_iter()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
        })
        .count()
}

fn enters_attacking_targets(game: &GameState, combat: &CombatState) -> Vec<AttackTarget> {
    let mut defending_players = Vec::new();
    for attacker in &combat.attackers {
        let defending_player = match attacker.target {
            AttackTarget::Player(player) => Some(player),
            AttackTarget::Planeswalker(planeswalker) => game
                .object(planeswalker)
                .map(|object| game.controller_of(object)),
        };
        if let Some(player) = defending_player
            && !defending_players.contains(&player)
        {
            defending_players.push(player);
        }
    }

    let all_effects = game.all_continuous_effects();
    let mut targets = Vec::new();
    for defender in defending_players {
        targets.push(AttackTarget::Player(defender));
        for &object_id in &game.battlefield {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if game.controller_of(object) == defender
                && object.zone == Zone::Battlefield
                && game.object_has_card_type_with_effects(
                    object_id,
                    crate::types::CardType::Planeswalker,
                    &all_effects,
                )
            {
                targets.push(AttackTarget::Planeswalker(object_id));
            }
        }
    }
    targets
}

fn attack_target_description(game: &GameState, target: &AttackTarget) -> String {
    match target {
        AttackTarget::Player(player) => game
            .player(*player)
            .map(|player| player.name.clone())
            .unwrap_or_else(|| format!("player {}", player.0)),
        AttackTarget::Planeswalker(object_id) => game
            .object(*object_id)
            .map(|object| object.name.clone())
            .unwrap_or_else(|| format!("planeswalker #{}", object_id.0)),
    }
}

fn choose_enters_attacking_target(
    game: &GameState,
    ctx: &mut ExecutionContext<'_>,
    moved_id: crate::ids::ObjectId,
) -> Option<AttackTarget> {
    let combat = game.combat.as_ref()?;
    let targets = enters_attacking_targets(game, combat);
    if targets.len() <= 1 {
        return targets.first().cloned();
    }

    let options = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            crate::decisions::DisplayOption::new(index, attack_target_description(game, target))
        })
        .collect();
    let chooser = game
        .object(moved_id)
        .map(|object| game.controller_of(object))
        .unwrap_or(ctx.controller);
    let source = ctx.source;
    let selected = crate::decisions::make_decision(
        game,
        &mut *ctx.decision_maker,
        chooser,
        Some(source),
        crate::decisions::ChoiceSpec::single(source, options),
    );
    let selected_index = selected.into_iter().next().unwrap_or(0);
    targets
        .get(selected_index)
        .cloned()
        .or_else(|| targets.first().cloned())
}

impl EffectExecutor for MoveToZoneEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let moves_source = matches!(self.target.base(), ChooseSpec::Source);
        let mut object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        // When a tag snapshot carries a stale ObjectId (the tagged object
        // changed zones since the snapshot was taken), resolve through
        // stable_id so the move can find the actual game object.
        if let ChooseSpec::Tagged(tag) = &self.target {
            if let Some(tagged) = ctx.get_tagged_all(tag) {
                for (idx, snapshot) in tagged.iter().enumerate() {
                    if idx < object_ids.len() && game.object(object_ids[idx]).is_none() {
                        if let Some(resolved) = resolve_tagged_object_id(game, snapshot) {
                            object_ids[idx] = resolved;
                        }
                    }
                }
            }
        }
        if object_ids.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let mut moved_ids = Vec::new();
        let mut affected_ids = Vec::new();
        let mut affected_memory = Vec::new();
        let mut any_prevented = false;
        let mut any_replaced = false;
        let mut moved_source_lki = None;

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let stable_id = obj.stable_id;
            let from_zone = obj.zone;
            let source_lki_before_move = if moves_source && object_id == ctx.source {
                Some(
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        obj, game,
                    ),
                )
            } else {
                None
            };
            let target_lki_before_move =
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                );
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            // Process through replacement effects with decision maker
            let result = process_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                self.zone,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match result {
                EventOutcome::Prevented => {
                    return Ok(EffectOutcome::prevented());
                }
                EventOutcome::Proceed(final_zone) => {
                    if final_zone == Zone::Battlefield {
                        let conditional_tapped_and_attacking = self
                            .enters_tapped_and_attacking_if
                            .as_ref()
                            .is_some_and(|filter| {
                                let filter_ctx = FilterContext::new(ctx.controller)
                                    .with_source(ctx.source);
                                filter.matches_snapshot(&target_lki_before_move, &filter_ctx, game)
                            });
                        let enters_tapped = self.enters_tapped || conditional_tapped_and_attacking;
                        let enters_attacking =
                            self.enters_attacking || conditional_tapped_and_attacking;
                        let options = match self.battlefield_controller {
                            BattlefieldController::Preserve => {
                                BattlefieldEntryOptions::preserve(enters_tapped)
                            }
                            BattlefieldController::Owner => {
                                BattlefieldEntryOptions::owner(enters_tapped)
                            }
                            BattlefieldController::You => BattlefieldEntryOptions::specific(
                                ctx.controller,
                                enters_tapped,
                            ),
                        };
                        if self.enters_face_down
                            && let Some(card) = game.object_mut(object_id)
                        {
                            card.apply_face_down_cast_overlay();
                        }
                        match move_to_battlefield_with_options(game, ctx, object_id, options) {
                            BattlefieldEntryOutcome::Moved(new_id) => {
                                if enters_attacking
                                    && let Some(target) =
                                        choose_enters_attacking_target(game, ctx, new_id)
                                    && let Some(combat) = game.combat.as_mut()
                                {
                                    combat.attackers.push(AttackerInfo {
                                        creature: new_id,
                                        target,
                                    });
                                }
                                ctx.refresh_target_snapshot(target_lki_before_move.clone());
                                affected_memory.push(OutcomeObjectMemory::from_snapshot(
                                    &target_lki_before_move,
                                ));
                                if let Some(snapshot) = source_lki_before_move.clone() {
                                    moved_source_lki = Some(snapshot);
                                }
                                moved_ids.push(new_id);
                            }
                            BattlefieldEntryOutcome::Prevented => {
                                if self.enters_face_down
                                    && let Some(card) = game.object_mut(object_id)
                                {
                                    card.end_face_down_cast_overlay();
                                }
                                any_prevented = true;
                            }
                        }
                        continue;
                    }

                    let mut result =
                        finalize_zone_change_move(game, object_id, final_zone, ctx.cause.clone());
                    if !result.new_object_ids.is_empty() {
                        ctx.refresh_target_snapshot(target_lki_before_move.clone());
                        affected_memory
                            .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                        if let Some(snapshot) = source_lki_before_move.clone() {
                            moved_source_lki = Some(snapshot);
                        }
                    }
                    if !result.new_object_ids.is_empty() {
                        for &new_id in &result.new_object_ids {
                            if final_zone == Zone::Exile {
                                game.add_exiled_with_source_link(ctx.source, new_id);
                                if let Some(object) = game.object(new_id) {
                                    ctx.tag_object(
                                        SOURCE_EXILED_TAG,
                                        ObjectSnapshot::from_object(object, game),
                                    );
                                }
                            }
                            if final_zone == Zone::Library
                                && !self.to_top
                                && let Some(owner) = game.object(new_id).map(|obj| obj.owner)
                            {
                                game.move_library_card_to_bottom(
                                    owner,
                                    new_id,
                                    "card put on bottom of library",
                                );
                            }
                        }
                        if final_zone == Zone::Library && from_zone == Zone::Battlefield {
                            maybe_prompt_for_split_result_order(
                                game,
                                &mut ctx.decision_maker,
                                final_zone,
                                &ctx.cause,
                                &mut result,
                            );
                            game.record_zone_change_results(
                                object_id,
                                result.new_object_ids.clone(),
                            );
                        }
                        affected_ids.extend(result.new_object_ids.iter().copied());
                        moved_ids.extend(result.new_object_ids.iter().copied());
                        continue;
                    }

                    continue;
                }
                EventOutcome::Replaced => {
                    any_replaced = true;
                    if let Some(result) = take_recorded_zone_change(game, object_id) {
                        affected_ids.extend(result.new_object_ids);
                    } else if let Some(result_id) = game.find_object_by_stable_id(stable_id) {
                        affected_ids.push(result_id);
                    }
                    affected_memory
                        .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                }
                EventOutcome::NotApplicable => continue,
            }
        }

        if moves_source && let Some(new_source_id) = moved_ids.first().copied() {
            let old_source_id = ctx.source;
            if self.transfer_exiled_with_source_links {
                game.transfer_exiled_with_source_links(old_source_id, new_source_id);
            }
            ctx.source = new_source_id;
        }
        if let Some(snapshot) = moved_source_lki {
            ctx.refresh_source_snapshot(snapshot);
        }

        if !moved_ids.is_empty() {
            let mut outcome =
                EffectOutcome::with_objects(moved_ids).with_affected_objects(affected_ids);
            if !affected_memory.is_empty() {
                outcome = outcome.with_affected_object_memory(affected_memory);
            }
            return Ok(outcome);
        }
        if any_prevented {
            return Ok(EffectOutcome::prevented());
        }
        if any_replaced {
            let mut outcome = EffectOutcome::replaced().with_affected_objects(affected_ids);
            if !affected_memory.is_empty() {
                outcome = outcome.with_affected_object_memory(affected_memory);
            }
            return Ok(outcome);
        }
        Ok(EffectOutcome::target_invalid())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target to move"
    }
}

impl CostExecutableEffect for MoveToZoneEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        if matches!(self.target.base(), ChooseSpec::Source) && game.object(source).is_some() {
            return Ok(());
        }

        if let Some((filter, count)) = fixed_cost_filter(self) {
            let matching = matching_cost_candidate_count(game, filter, source, controller);
            if matching >= count {
                return Ok(());
            }
            return Err(CostValidationError::NotEnoughCards);
        }

        Err(CostValidationError::Other(
            "unsupported move-to-zone cost".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;
    use crate::events::zones::matchers::WouldGoToGraveyardMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), "Move Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        game.add_object(Object::from_card(id, &card, owner, Zone::Battlefield));
        id
    }

    fn create_named_creature_in_zone(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        zone: Zone,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        game.add_object(Object::from_card(id, &card, owner, zone));
        id
    }

    fn create_named_creature_with_types_in_zone(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        card_types: Vec<CardType>,
        zone: Zone,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(card_types)
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    struct ChooseLastOptionDecisionMaker;

    impl crate::decision::DecisionMaker for ChooseLastOptionDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .last()
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    #[test]
    fn paladin_elizabeth_taggerdy_move_enters_tapped_and_attacking_chosen_defender() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let paladin = create_named_creature_in_zone(
            &mut game,
            alice,
            "Paladin Elizabeth Taggerdy",
            Zone::Battlefield,
        );
        let other_attacker =
            create_named_creature_in_zone(&mut game, alice, "Wasteland Raider", Zone::Battlefield);
        let vault_dweller =
            create_named_creature_in_zone(&mut game, alice, "Vault Dweller", Zone::Hand);
        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: paladin,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: other_attacker,
                    target: AttackTarget::Player(cara),
                },
            ],
            ..CombatState::default()
        });

        let mut decision_maker = ChooseLastOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(paladin, alice, &mut decision_maker);
        let outcome = MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(vault_dweller),
            Zone::Battlefield,
            false,
        )
        .tapped()
        .attacking()
        .execute(&mut game, &mut ctx)
        .expect("Paladin Elizabeth Taggerdy move should resolve");

        let moved = outcome
            .affected_objects()
            .and_then(|ids| ids.first().copied())
            .or_else(|| match outcome.value {
                crate::effect::OutcomeValue::Objects(ref ids) => ids.first().copied(),
                _ => None,
            })
            .expect("moved creature id should be reported");
        assert!(game.battlefield.contains(&moved));
        assert!(game.is_tapped(moved), "moved creature should enter tapped");
        let combat = game.combat.as_ref().expect("combat should remain active");
        let moved_attacker = combat
            .attackers
            .iter()
            .find(|info| info.creature == moved)
            .expect("moved creature should enter attacking");
        assert_eq!(moved_attacker.target, AttackTarget::Player(cara));
    }

    #[test]
    fn paladin_elizabeth_taggerdy_move_without_active_combat_does_not_attack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let paladin = create_named_creature_in_zone(
            &mut game,
            alice,
            "Paladin Elizabeth Taggerdy",
            Zone::Battlefield,
        );
        let vault_dweller =
            create_named_creature_in_zone(&mut game, alice, "Vault Dweller", Zone::Hand);

        let mut ctx = ExecutionContext::new_default(paladin, alice);
        let outcome = MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(vault_dweller),
            Zone::Battlefield,
            false,
        )
        .tapped()
        .attacking()
        .execute(&mut game, &mut ctx)
        .expect("Paladin Elizabeth Taggerdy move should resolve outside combat");

        let moved = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids[0],
            _ => panic!("expected moved object id"),
        };
        assert!(game.battlefield.contains(&moved));
        assert!(
            game.is_tapped(moved),
            "moved creature should still enter tapped"
        );
        assert!(
            game.combat.is_none(),
            "no attacker should be added without combat"
        );
    }

    #[test]
    fn conditional_move_enters_tapped_and_attacking_when_filter_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_named_creature_in_zone(&mut game, alice, "Attack Source", Zone::Battlefield);
        let existing_attacker =
            create_named_creature_in_zone(&mut game, alice, "Existing Attacker", Zone::Battlefield);
        let enchantment_creature = create_named_creature_with_types_in_zone(
            &mut game,
            alice,
            "Enchantment Creature",
            vec![CardType::Enchantment, CardType::Creature],
            Zone::Hand,
        );
        game.combat = Some(CombatState {
            attackers: vec![AttackerInfo {
                creature: existing_attacker,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        });

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(enchantment_creature),
            Zone::Battlefield,
            false,
        )
        .tapped_and_attacking_if(ObjectFilter::default().with_type(CardType::Enchantment))
        .execute(&mut game, &mut ctx)
        .expect("conditional move should resolve");
        let moved = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids[0],
            _ => panic!("expected moved object id"),
        };

        assert!(game
            .object(moved)
            .is_some_and(|object| object.zone == Zone::Battlefield));
        assert!(
            game.is_tapped(moved),
            "matching enchantment creature should enter tapped"
        );
        assert!(
            game.combat.as_ref().is_some_and(|combat| combat
                .attackers
                .iter()
                .any(|info| info.creature == moved)),
            "matching enchantment creature should enter attacking"
        );
    }

    #[test]
    fn conditional_move_enters_normally_when_filter_does_not_match() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_named_creature_in_zone(&mut game, alice, "Attack Source", Zone::Battlefield);
        let existing_attacker =
            create_named_creature_in_zone(&mut game, alice, "Existing Attacker", Zone::Battlefield);
        let creature = create_named_creature_with_types_in_zone(
            &mut game,
            alice,
            "Plain Creature",
            vec![CardType::Creature],
            Zone::Hand,
        );
        game.combat = Some(CombatState {
            attackers: vec![AttackerInfo {
                creature: existing_attacker,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        });

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MoveToZoneEffect::new(ChooseSpec::SpecificObject(creature), Zone::Battlefield, false)
            .tapped_and_attacking_if(ObjectFilter::default().with_type(CardType::Enchantment))
            .execute(&mut game, &mut ctx)
            .expect("conditional move should resolve");
        let moved = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids[0],
            _ => panic!("expected moved object id"),
        };

        assert!(game
            .object(moved)
            .is_some_and(|object| object.zone == Zone::Battlefield));
        assert!(
            !game.is_tapped(moved),
            "nonmatching creature should not enter tapped"
        );
        assert!(
            game.combat.as_ref().is_some_and(|combat| !combat
                .attackers
                .iter()
                .any(|info| info.creature == moved)),
            "nonmatching creature should not enter attacking"
        );
    }

    #[test]
    fn non_target_move_to_zone_does_not_request_cast_time_targets() {
        let move_choice = MoveToZoneEffect::new(
            ChooseSpec::WithCount(
                Box::new(ChooseSpec::Object(crate::filter::ObjectFilter {
                    zone: Some(Zone::Exile),
                    ..crate::filter::ObjectFilter::default()
                })),
                crate::effect::ChoiceCount::exactly(1),
            ),
            Zone::Graveyard,
            false,
        );
        assert!(move_choice.target_selection_profile().is_none());

        let move_target = MoveToZoneEffect::new(
            ChooseSpec::target(ChooseSpec::Object(crate::filter::ObjectFilter {
                zone: Some(Zone::Battlefield),
                ..crate::filter::ObjectFilter::default()
            })),
            Zone::Graveyard,
            false,
        );
        assert!(move_target.target_selection_profile().is_some());
    }

    #[test]
    fn replaced_move_preserves_redirected_object_ids_in_outcome() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, alice);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldGoToGraveyardMatcher::new(crate::target::ObjectFilter::specific(creature)),
                ReplacementAction::Instead(vec![Effect::new(MoveToZoneEffect::to_exile(
                    ChooseSpec::SpecificObject(creature),
                ))]),
            ),
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = MoveToZoneEffect::to_graveyard(ChooseSpec::SpecificObject(creature));
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        let affected = outcome
            .affected_objects()
            .expect("redirected object ids should be preserved");
        assert_eq!(affected.len(), 1);
        assert!(
            game.object(affected[0])
                .is_some_and(|obj| obj.zone == Zone::Exile && obj.name == "Move Probe")
        );
        assert!(game.players[0].graveyard.is_empty());
    }
}
