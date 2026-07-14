use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutionCategory, EffectExecutor, ReplacementApplyMode};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;

/// Registers a concrete zone-change replacement effect for the currently resolved object(s).
#[derive(Debug, Clone, PartialEq)]
pub struct RegisterZoneReplacementEffect {
    pub target: ChooseSpec,
    pub from_zone: Option<Zone>,
    pub to_zone: Option<Zone>,
    pub replacement_zone: Zone,
    pub library_placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
    pub mode: ReplacementApplyMode,
    pub optional: bool,
    pub choice_description: Option<String>,
    pub counters: Vec<(CounterType, u32)>,
}

impl RegisterZoneReplacementEffect {
    pub fn new(
        target: ChooseSpec,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            target,
            from_zone,
            to_zone,
            replacement_zone,
            library_placement: None,
            mode,
            optional: false,
            choice_description: None,
            counters: Vec::new(),
        }
    }

    pub fn optional(mut self, description: impl Into<String>) -> Self {
        self.optional = true;
        self.choice_description = Some(description.into());
        self
    }

    pub fn with_counters(mut self, counters: Vec<(CounterType, u32)>) -> Self {
        self.counters = counters;
        self
    }

    pub fn with_library_placement(
        mut self,
        placement: ironsmith_core::ZoneReplacementLibraryPlacement,
    ) -> Self {
        self.library_placement = Some(placement);
        self
    }

    pub fn resolve_replacements(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Vec<ReplacementEffect>, ExecutionError> {
        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if object_ids.is_empty() {
            return Err(ExecutionError::InvalidTarget);
        }

        Ok(object_ids
            .into_iter()
            .map(|object_id| {
                let replacement = zone_replacement_action(
                    object_id,
                    self.to_zone,
                    self.replacement_zone,
                    self.library_placement,
                    self.optional,
                    self.choice_description.clone(),
                    self.counters.clone(),
                );
                ReplacementEffect::with_matcher(
                    ctx.source,
                    ctx.controller,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(object_id),
                        self.from_zone,
                        self.to_zone,
                    ),
                    replacement,
                )
            })
            .collect())
    }
}

pub(crate) fn zone_replacement_action(
    object_id: crate::ids::ObjectId,
    original_zone: Option<Zone>,
    replacement_zone: Zone,
    library_placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
    optional: bool,
    choice_description: Option<String>,
    counters: Vec<(CounterType, u32)>,
) -> ReplacementAction {
    if optional {
        let mut destinations = Vec::new();
        if let Some(zone) = original_zone {
            destinations.push(zone);
        }
        if !destinations.contains(&replacement_zone) {
            destinations.push(replacement_zone);
        }
        return ReplacementAction::InteractiveChooseDestination {
            destinations,
            description: choice_description.unwrap_or_else(|| "Choose a destination".to_string()),
        };
    }

    if !counters.is_empty() {
        return ReplacementAction::MoveToZoneWithCounters {
            zone: replacement_zone,
            counters,
        };
    }

    if replacement_zone == Zone::Library
        && let Some(placement) = library_placement
    {
        let target = ChooseSpec::SpecificObject(object_id);
        let move_effect = match placement {
            ironsmith_core::ZoneReplacementLibraryPlacement::Top => {
                crate::effect::Effect::move_to_zone(target, Zone::Library, true)
            }
            ironsmith_core::ZoneReplacementLibraryPlacement::Bottom => {
                crate::effect::Effect::move_to_zone(target, Zone::Library, false)
            }
            ironsmith_core::ZoneReplacementLibraryPlacement::TopOrBottom => {
                crate::effect::Effect::new(
                    crate::effects::MoveToLibraryTopOrBottomChoiceEffect::new(target)
                        .with_chooser(PlayerFilter::You),
                )
            }
        };
        return ReplacementAction::Instead(vec![move_effect]);
    }

    ReplacementAction::ChangeDestination(replacement_zone)
}

impl EffectExecutor for RegisterZoneReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let replacements = match self.resolve_replacements(game, ctx) {
            Ok(replacements) => replacements,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };

        for replacement in replacements {
            match self.mode {
                ReplacementApplyMode::OneShot => {
                    game.effect_store
                        .replacement_effects
                        .add_one_shot_effect(replacement);
                }
                ReplacementApplyMode::UntilEndOfTurn => {
                    game.effect_store
                        .replacement_effects
                        .add_until_end_of_turn_effect(replacement);
                }
                ReplacementApplyMode::Resolution => {
                    game.effect_store
                        .replacement_effects
                        .add_resolution_effect(replacement);
                }
            }
        }

        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        Ok(EffectOutcome::with_objects(object_ids))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target for replacement"
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::OutcomeStatus;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, owner: PlayerId, zone: Zone) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Replacement Test Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn test_registered_zone_replacement_exiles_matching_death_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let move_outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        assert!(
            move_outcome.status != OutcomeStatus::TargetInvalid,
            "expected move effect to resolve on the creature"
        );

        let exiled_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after replacement");
        assert_eq!(
            game.object(exiled_id)
                .expect("exiled creature should exist")
                .zone,
            Zone::Exile
        );
    }

    #[test]
    fn test_registered_zone_replacement_moves_with_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        )
        .with_counters(vec![(CounterType::Time, 3)]);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let move_outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        assert!(
            move_outcome.status != OutcomeStatus::TargetInvalid,
            "expected move effect to resolve on the creature"
        );

        let exiled_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after replacement");
        assert_eq!(
            game.object(exiled_id)
                .expect("exiled creature should exist")
                .zone,
            Zone::Exile
        );
        assert_eq!(game.counter_count(exiled_id, CounterType::Time), 3);
    }

    #[test]
    fn test_registered_zone_replacement_does_not_apply_to_nonmatching_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let move_outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Hand,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        assert!(
            move_outcome.status != OutcomeStatus::TargetInvalid,
            "expected move-to-hand effect to resolve on the creature"
        );
        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after moving to hand");
        assert_eq!(
            game.object(moved_id)
                .expect("moved creature should exist")
                .zone,
            Zone::Hand
        );
    }

    #[test]
    fn persistent_leave_battlefield_replacement_survives_cleanup_and_exiles_any_destination() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            None,
            Zone::Exile,
            ReplacementApplyMode::Resolution,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("persistent replacement registration should succeed");

        crate::turn::execute_cleanup_step(&mut game);

        execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Hand,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve through the replacement");

        let exiled_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should remain findable after the replacement");
        assert_eq!(
            game.object(exiled_id)
                .expect("replaced creature should exist")
                .zone,
            Zone::Exile,
            "leave-battlefield replacement must survive cleanup and replace a move to hand"
        );
    }

    #[test]
    fn library_destination_replacement_honors_top_and_bottom_placement() {
        for (placement, expect_top) in [
            (ironsmith_core::ZoneReplacementLibraryPlacement::Top, true),
            (
                ironsmith_core::ZoneReplacementLibraryPlacement::Bottom,
                false,
            ),
        ] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let _sentinel = create_creature(&mut game, alice, Zone::Library);
            let moving = create_creature(&mut game, alice, Zone::Graveyard);
            let stable_id = game.object(moving).expect("moving card").stable_id;

            let effect = RegisterZoneReplacementEffect::new(
                ChooseSpec::SpecificObject(moving),
                Some(Zone::Graveyard),
                Some(Zone::Hand),
                Zone::Library,
                ReplacementApplyMode::OneShot,
            )
            .with_library_placement(placement);
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(moving, alice, &mut dm);
            execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
                .expect("library replacement registration");
            execute_effect(
                &mut game,
                &crate::effect::Effect::move_to_zone(
                    ChooseSpec::SpecificObject(moving),
                    Zone::Hand,
                    false,
                ),
                &mut ctx,
            )
            .expect("move through library replacement");

            let moved = game
                .find_object_by_stable_id(stable_id)
                .expect("moved card remains findable");
            let library = &game.player(alice).expect("player").library;
            let expected = if expect_top {
                library.last()
            } else {
                library.first()
            };
            assert_eq!(expected, Some(&moved));
        }
    }
}
