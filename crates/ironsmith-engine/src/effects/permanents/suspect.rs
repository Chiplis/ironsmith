//! Suspect designation effects.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub use ironsmith_core::{ClearSuspectedEffect, SuspectEffect};

include!("suspect_targets.rs");

impl EffectExecutor for SuspectEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = match resolve_objects_for_effect(game, ctx, &self.target) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) if !self.target.is_target() => {
                return Ok(EffectOutcome::count(0));
            }
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };

        let mut count = 0_i32;
        for object_id in objects {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.zone != Zone::Battlefield || !game.current_is_creature(object_id) {
                continue;
            }
            if game.is_suspected(object_id) {
                continue;
            }
            if game.set_suspected(object_id) { count += 1; }
        }

        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to suspect"
    }
}

impl EffectExecutor for ClearSuspectedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = if let Some(target) = &self.target {
            match resolve_suspected_designation_objects(game, ctx, target) {
                Ok(objects) => objects,
                Err(ExecutionError::InvalidTarget) if !target.is_target() => {
                    return Ok(EffectOutcome::count(0));
                }
                Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
                Err(err) => return Err(err),
            }
        } else {
            game.suspected_ids().collect()
        };

        let mut count = 0_i32;
        for object_id in objects {
            if game.clear_suspected(object_id) {
                count += 1;
            }
        }

        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "suspected creature"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::OutcomeValue;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbilityId;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        owner: PlayerId,
        card_id: u32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), format!("Creature {card_id}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn suspect_marks_creature_and_grants_menace_and_cant_block() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, alice, 1);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let first = SuspectEffect::new(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut ctx)
            .expect("suspect should execute");
        assert_eq!(first.value, OutcomeValue::Count(1));
        assert!(game.is_suspected(creature));
        assert!(game.current_has_static_ability_id(creature, StaticAbilityId::Menace));
        assert!(game.current_has_static_ability_id(creature, StaticAbilityId::CantBlock));

        let second = SuspectEffect::new(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut ctx)
            .expect("second suspect should execute");
        assert_eq!(second.value, OutcomeValue::Count(0));

        let cleared = ClearSuspectedEffect::new(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut ctx)
            .expect("clear suspected should execute");
        assert_eq!(cleared.value, OutcomeValue::Count(1));
        assert!(!game.is_suspected(creature));
        assert!(!game.current_has_static_ability_id(creature, StaticAbilityId::Menace));
        assert!(!game.current_has_static_ability_id(creature, StaticAbilityId::CantBlock));
    }

    #[test]
    fn clear_all_suspected_clears_every_designation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_creature(&mut game, alice, 1);
        let second = create_creature(&mut game, alice, 2);
        let mut ctx = ExecutionContext::new_default(source, alice);

        SuspectEffect::new(ChooseSpec::SpecificObject(first))
            .execute(&mut game, &mut ctx)
            .expect("suspect first");
        SuspectEffect::new(ChooseSpec::SpecificObject(second))
            .execute(&mut game, &mut ctx)
            .expect("suspect second");

        let cleared = ClearSuspectedEffect::all()
            .execute(&mut game, &mut ctx)
            .expect("clear all suspected");
        assert_eq!(cleared.value, OutcomeValue::Count(2));
        assert!(!game.is_suspected(first));
        assert!(!game.is_suspected(second));
    }
}

#[cfg(test)]
mod designation_history_tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::condition_eval::evaluate_condition_resolution;
    use crate::effect::{Condition, Until};
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ObjectFilter;
    use crate::types::CardType;

    fn creature(game: &mut GameState) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Designation history")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
        game.create_object_from_card(&card, PlayerId(0), Zone::Battlefield)
    }
    #[test]
    fn suspected_lki_is_exact_and_unknown_evidence_is_an_error() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = creature(&mut game);
        assert!(game.set_suspected(source));
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        assert_eq!(snapshot.suspected, Some(true));
        let mut ctx = ExecutionContext::new_default(source, PlayerId(0)).with_source_snapshot(snapshot.clone());
        ctx.tag_object("paid", snapshot.clone());
        let paid = Condition::TaggedObjectMatchedLastKnown("paid".into(), ObjectFilter::default().suspected());
        game.clear_suspected(source);
        assert_eq!(evaluate_condition_resolution(&game, &paid, &ctx), Ok(true));
        assert_eq!(evaluate_condition_resolution(&game, &Condition::SourceSuspected, &ctx), Ok(false));
        game.set_suspected(source);
        let graveyard = game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        let returned = game.move_object_by_game_rule(graveyard, Zone::Battlefield).unwrap();
        assert!(!game.is_suspected(returned));
        assert_eq!(evaluate_condition_resolution(&game, &Condition::SourceSuspected, &ctx), Ok(true));
        assert_eq!(evaluate_condition_resolution(&game, &paid, &ctx), Ok(true));
        let mut unknown = snapshot;
        unknown.suspected = None;
        let mut unknown_ctx = ExecutionContext::new_default(source, PlayerId(0));
        unknown_ctx.tag_object("paid", unknown);
        assert!(matches!(evaluate_condition_resolution(&game, &paid, &unknown_ctx), Err(ExecutionError::IncompleteEvidence(_))));
    }
    #[test]
    fn phased_designation_is_preserved_and_blocked_suspect_reports_no_change() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = creature(&mut game);
        game.set_suspected(source);
        game.phase_out(source);
        assert!(!game.clear_suspected(source));
        assert!(!game.set_suspected(source));
        assert!(game.is_suspected(source));
        game.phase_in(source);
        assert!(game.clear_suspected(source));
        game.add_restriction_effect(crate::effect::Restriction::BecomeSuspected(ObjectFilter::source()), Until::Forever, source, PlayerId(0), None);
        let outcome = SuspectEffect::new(ChooseSpec::SpecificObject(source)).execute(&mut game, &mut ExecutionContext::new_default(source, PlayerId(0))).unwrap();
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert!(!game.is_suspected(source));
    }
}
