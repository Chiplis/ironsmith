//! Prepared designation effects.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub use ironsmith_core::PrepareEffect;

impl EffectExecutor for PrepareEffect {
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
            // A permanent without a prepare spell can't become prepared, and one
            // that already is doesn't prepare a second copy.
            if object.zone != Zone::Battlefield || !game.has_prepare_spell(object_id) {
                continue;
            }
            if game.set_prepared(object_id) {
                count += 1;
            }
        }

        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to prepare"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, LinkedFaceLayout, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;

    const PREPARE_SPELL_NAME: &str = "Raise Dead";

    /// The prepare spell face: a real card of its own name lives elsewhere, so
    /// this face is only ever reachable through the creature.
    fn prepare_spell_definition() -> crate::cards::CardDefinition {
        crate::cards::CardDefinition::new(
            CardBuilder::new(CardId::from_raw(9_001), PREPARE_SPELL_NAME)
                .card_types(vec![CardType::Sorcery])
                .mana_cost(crate::mana::ManaCost::new())
                .build(),
        )
    }

    fn enter_prepared_creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let spell = prepare_spell_definition();
        game.register_linked_face_definition(&spell);

        let id = game.new_object_id();
        let mut card = CardBuilder::new(CardId::from_raw(9_000), "Cheerful Osteomancer")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 2))
            .build();
        card.linked_face_layout = LinkedFaceLayout::Prepare;
        card.other_face = Some(spell.card.id);
        let mut object = Object::from_card(id, &card, owner, Zone::Hand);
        object.abilities_mut().push(Ability::static_ability(
            StaticAbility::enters_prepared_ability(),
        ));
        game.add_object(object);
        id
    }

    fn exiled_names(game: &GameState) -> Vec<String> {
        game.exile
            .iter()
            .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
            .collect()
    }

    #[test]
    fn entering_prepared_puts_one_prepare_spell_copy_in_exile() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = enter_prepared_creature(&mut game, alice);

        let result = game
            .move_object_with_etb_processing_with_dm(
                entrant,
                Zone::Battlefield,
                &mut crate::decision::SelectFirstDecisionMaker,
            )
            .expect("the creature should enter the battlefield");

        assert!(game.is_prepared(result.new_id), "it should enter prepared");
        assert_eq!(exiled_names(&game), vec![PREPARE_SPELL_NAME.to_string()]);

        // A permanent can't become prepared twice, so no second copy appears.
        assert!(!game.set_prepared(result.new_id));
        assert_eq!(exiled_names(&game), vec![PREPARE_SPELL_NAME.to_string()]);
    }

    #[test]
    fn the_copy_ceases_to_exist_when_the_permanent_stops_being_prepared() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = enter_prepared_creature(&mut game, alice);
        let id = game
            .move_object_with_etb_processing_with_dm(
                entrant,
                Zone::Battlefield,
                &mut crate::decision::SelectFirstDecisionMaker,
            )
            .expect("the creature should enter the battlefield")
            .new_id;
        assert_eq!(exiled_names(&game).len(), 1);

        assert!(game.clear_prepared(id));
        assert!(!game.is_prepared(id));
        assert!(
            exiled_names(&game).is_empty(),
            "unpreparing removes the exiled copy"
        );
    }

    #[test]
    fn leaving_the_battlefield_takes_the_copy_with_it() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = enter_prepared_creature(&mut game, alice);
        let id = game
            .move_object_with_etb_processing_with_dm(
                entrant,
                Zone::Battlefield,
                &mut crate::decision::SelectFirstDecisionMaker,
            )
            .expect("the creature should enter the battlefield")
            .new_id;
        assert_eq!(exiled_names(&game).len(), 1);

        game.move_object(
            id,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
        );

        assert!(
            exiled_names(&game).is_empty(),
            "the copy exists only while the permanent is on the battlefield"
        );
    }

    #[test]
    fn the_controller_may_cast_the_prepare_spell_copy_from_exile() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let entrant = enter_prepared_creature(&mut game, alice);
        game.move_object_with_etb_processing_with_dm(
            entrant,
            Zone::Battlefield,
            &mut crate::decision::SelectFirstDecisionMaker,
        )
        .expect("the creature should enter the battlefield");
        let copy = *game
            .exile
            .first()
            .expect("the prepare spell copy is in exile");

        let casts_copy = |actions: &[crate::decision::LegalAction]| {
            actions.iter().any(|action| {
                matches!(
                    action,
                    crate::decision::LegalAction::CastSpell { spell_id, from_zone, .. }
                        if *spell_id == copy && *from_zone == Zone::Exile
                )
            })
        };

        assert!(
            casts_copy(&crate::decision::compute_legal_actions(&game, alice)),
            "the prepared permanent's controller may cast the copy from exile"
        );
        assert!(
            !casts_copy(&crate::decision::compute_legal_actions(&game, bob)),
            "only the controller of the prepared permanent may cast the copy"
        );
    }

    #[test]
    fn a_permanent_without_a_prepare_spell_cannot_become_prepared() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(9_100), "Plain Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let bear = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = PrepareEffect::new(ChooseSpec::SpecificObject(bear))
            .execute(&mut game, &mut ctx)
            .expect("prepare should execute");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert!(!game.is_prepared(bear));
        assert!(exiled_names(&game).is_empty());
    }
}
