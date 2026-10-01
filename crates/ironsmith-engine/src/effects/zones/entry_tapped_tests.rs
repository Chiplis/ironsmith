//! Regression coverage for authored tapped instructions across entry paths.
use crate::ability::Ability;
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::CardDefinitionBuilder;
use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
use crate::effects::{CreateTokenCopyEffect, CreateTokenEffect, EffectExecutor, ExecutionContext};
use crate::events::EnterBattlefieldEvent;
use crate::game_state::GameState;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::CardType;
use crate::zone::Zone;

fn untapper(game: &mut GameState, player: PlayerId) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Entry Untapper")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::enters_untapped_for_filter(ObjectFilter::default().you_control()),
        ))
        .build();
    game.create_object_from_definition(&definition, player, Zone::Battlefield)
}

struct EntryOrder {
    source: ObjectId,
    untapped_last: bool,
    choices: usize,
}

impl DecisionMaker for EntryOrder {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.choices += 1;
        vec![
            ctx.options
                .iter()
                .find(|option| {
                    option.legal && (option.object_id == Some(self.source)) != self.untapped_last
                })
                .expect("both replacement effects offered")
                .index,
        ]
    }
}

fn token_entry_case(copy: bool, suppress_attachment: bool, intrinsic: bool, untapped_last: bool) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = untapper(&mut game, alice);
    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Entry Token")
        .token()
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2));
    if intrinsic {
        builder = builder.with_ability(Ability::static_ability(
            StaticAbility::enters_tapped_ability(),
        ));
    }
    let definition = builder.build();
    let original = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.combat = Some(Default::default());
    game.turn.phase = crate::game_state::Phase::Combat;
    let mut decisions = EntryOrder {
        source,
        untapped_last,
        choices: 0,
    };
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
    let outcome = if copy {
        CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(original))
            .enters_tapped(true)
            .attacking(true)
            .execute(&mut game, &mut ctx)
            .unwrap()
    } else {
        let mut effect = CreateTokenEffect::one(definition).tapped().attacking();
        effect.suppress_aura_attachment_choice = suppress_attachment;
        effect.execute(&mut game, &mut ctx).unwrap()
    };
    let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
        panic!("token should be created");
    };
    assert_eq!(ids.len(), 1);
    let tapped = intrinsic && !untapped_last;
    assert_eq!(game.is_tapped(ids[0]), tapped);
    assert!(
        game.combat
            .as_ref()
            .unwrap()
            .attackers
            .iter()
            .any(|a| a.creature == ids[0])
    );
    let entries: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<EnterBattlefieldEvent>())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].enters_tapped, tapped);
}

#[test]
fn token_tapped_instruction_obeys_replacements_and_preserves_attacking() {
    for intrinsic in [false, true] {
        for untapped_last in [false, true] {
            token_entry_case(false, false, intrinsic, untapped_last);
            token_entry_case(false, true, intrinsic, untapped_last);
        }
    }
}

#[test]
fn token_copy_tapped_instruction_obeys_replacements_and_preserves_attacking() {
    for intrinsic in [false, true] {
        for untapped_last in [false, true] {
            token_entry_case(true, false, intrinsic, untapped_last);
        }
    }
}

#[test]
fn permission_tapped_instruction_obeys_replacement_order() {
    for intrinsic in [false, true] {
        for untapped_last in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let source = untapper(&mut game, alice);
            let mut builder = CardDefinitionBuilder::new(CardId::new(), "Exiled Land")
                .card_types(vec![CardType::Land]);
            if intrinsic {
                builder = builder.with_ability(Ability::static_ability(
                    StaticAbility::enters_tapped_ability(),
                ));
            }
            let id = game.create_object_from_definition(&builder.build(), alice, Zone::Exile);
            let mut decisions = EntryOrder {
                source,
                untapped_last,
                choices: 0,
            };
            let result = game
                .move_object_with_etb_processing_with_entry_options(
                    id,
                    Zone::Battlefield,
                    &mut decisions,
                    true,
                    true,
                ).expect("replacement operation must execute successfully in this scenario")
                .assert_completed_without_additions().unwrap();
            let tapped = intrinsic && !untapped_last;
            assert_eq!(result.enters_tapped, tapped);
            assert_eq!(game.is_tapped(result.new_id), tapped);
            assert_eq!(decisions.choices, usize::from(intrinsic));
        }
    }
}

#[test]
fn tapped_instruction_does_not_apply_an_opponents_untapper() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    untapper(&mut game, PlayerId::from_index(1));
    let card = CardBuilder::new(CardId::new(), "Land")
        .card_types(vec![CardType::Land])
        .build();
    let id = game.create_object_from_card(&card, alice, Zone::Exile);
    let result = game
        .move_object_with_etb_processing_with_entry_options(
            id,
            Zone::Battlefield,
            &mut SelectFirstDecisionMaker,
            true,
            true,
        ).expect("replacement operation must execute successfully in this scenario")
        .assert_completed_without_additions().unwrap();
    assert!(result.enters_tapped);
    assert!(game.is_tapped(result.new_id));
}

#[test]
fn both_land_play_apis_honor_tapped_permissions_and_untapped_replacements() {
    for priority_api in [false, true] {
        for untap in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            game.turn.phase = crate::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            let source = if untap {
                untapper(&mut game, alice)
            } else {
                game.new_object_id()
            };
            let card = CardBuilder::new(CardId::new(), "Permission Land")
                .card_types(vec![CardType::Land])
                .build();
            let id = game.create_object_from_card(&card, alice, Zone::Exile);
            game.effect_store
                .grant_registry
                .grant_to_filter_until_end_of_turn(
                    ObjectFilter::default().with_type(CardType::Land),
                    Zone::Exile,
                    alice,
                    crate::grant::Grantable::play_from(),
                    source,
                    game.turn.turn_number,
                );
            game.effect_store
                .grant_registry
                .grants
                .last_mut()
                .unwrap()
                .play_from_constraints
                .lands_enter_tapped = true;
            let mut dm = SelectFirstDecisionMaker;
            if priority_api {
                let mut queue = crate::triggers::TriggerQueue::new();
                let mut state = crate::game_loop::PriorityLoopState::new(2);
                crate::game_loop::apply_priority_response_with_dm(
                    &mut game,
                    &mut queue,
                    &mut state,
                    &crate::PriorityResponse::PriorityAction(
                        crate::decision::LegalAction::PlayLand { land_id: id },
                    ),
                    &mut dm,
                )
                .unwrap();
            } else {
                crate::special_actions::perform(
                    crate::special_actions::SpecialAction::PlayLand { card_id: id },
                    &mut game,
                    alice,
                    &mut dm,
                )
                .unwrap();
                let events = game.take_pending_trigger_events();
                let entries: Vec<_> = events
                    .iter()
                    .filter_map(|event| event.downcast::<EnterBattlefieldEvent>())
                    .collect();
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].enters_tapped, !untap);
            }
            let entered = game
                .battlefield
                .iter()
                .copied()
                .find(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name == "Permission Land")
                })
                .expect("land should enter");
            assert_eq!(game.is_tapped(entered), !untap);
            assert!(!game.player(alice).unwrap().can_play_land());
        }
    }
}
