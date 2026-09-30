use super::*;
use crate::effect::{Effect, Value};
use crate::ids::{CardId, PlayerId};
use crate::replacement::{ReplacementAction, ReplacementEffect};

fn setup() -> (GameState, ObjectId, PlayerId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = crate::card::CardBuilder::new(CardId::new(), "Zone entry carrier fixture")
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let entrant = game.create_object_from_card(&card, alice, Zone::Hand);
    game.take_pending_trigger_events();
    (game, entrant, alice, bob)
}

fn redirect_to_battlefield(
    game: &mut GameState,
    entrant: ObjectId,
    alice: PlayerId,
) -> crate::replacement::ReplacementEffectId {
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            entrant,
            alice,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                crate::target::ObjectFilter::specific(entrant),
                Some(Zone::Hand),
                Some(Zone::Graveyard),
            ),
            ReplacementAction::ChangeDestination(Zone::Battlefield),
        ))
}

#[test]
fn zone_redirect_commits_the_resolved_entry_controller() {
    for changes_controller in [false, true] {
        let (mut game, entrant, alice, bob) = setup();
        let redirect = redirect_to_battlefield(&mut game, entrant, alice);
        let controller_replacement = changes_controller.then(|| {
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    entrant,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::EnterUnderControl(bob),
                ),
            )
        });
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = apply_zone_change(
            &mut game,
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_effect(entrant, alice),
            &mut dm,
        )
        .into_result()
        .expect("redirected zone change must proceed");
        let entered = result.new_object_id.unwrap();
        assert_eq!(result.final_zone, Zone::Battlefield);
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(entered).unwrap().owner, alice);
        assert_eq!(
            game.current_controller(entered),
            Some(if changes_controller { bob } else { alice })
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(redirect)
                .is_none()
        );
        if let Some(replacement) = controller_replacement {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_none()
            );
        }
    }
}

#[test]
fn zone_redirect_executes_entry_program_and_restores_the_whole_move_on_error() {
    for fails in [false, true] {
        let (mut game, entrant, alice, _) = setup();
        let stable = game.object(entrant).unwrap().stable_id;
        let redirect = redirect_to_battlefield(&mut game, entrant, alice);
        let mut effects = vec![Effect::gain_life(2)];
        if fails {
            effects.push(Effect::lose_life(Value::X));
        }
        let entry_program = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(effects),
                ),
            ),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = apply_zone_change(
            &mut game,
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_effect(entrant, alice),
            &mut dm,
        );
        assert!(!dm.awaiting_choice());
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        if fails {
            assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
            assert!(game.battlefield.is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(redirect)
                    .is_some()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(entry_program)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        } else {
            assert!(result.is_proceed());
            let entered = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(redirect)
                    .is_none()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(entry_program)
                    .is_none()
            );
            let events = game.take_pending_trigger_events();
            let gains = events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .collect::<Vec<_>>();
            assert_eq!(gains.len(), 1);
            assert_eq!(gains[0].amount, 2);
        }
    }
}
