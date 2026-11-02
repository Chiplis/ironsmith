//! Actual MSEM source861: independent type/attachment-filter/attach primitives.
//! Parent owns registration and execution after applying the scratch patch.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions() -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-final-abilities.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "msem-861")
        .unwrap();
    let card = &row["representative_cards"][0];
    assert_eq!(card["name"], "Honey Pot");
    source_support::definitions(card).unwrap()
}
struct Choices {
    accept: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.accept
    }
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..16 {
        let mut q = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut q, dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut q, dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
        }
    }
    panic!("trigger chain did not settle");
}
fn tokens(game: &GameState, watcher: ObjectId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| *id != watcher)
        .collect()
}
#[test]
fn source_token_breach_becomes_a_curse_attaches_only_to_opponents_and_expires_next_turn() {
    for definition in definitions() {
        for mode in ["accept", "decline", "noncombat", "creature"] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.turn_number = 3;
            game.turn.active_player = A;
            let mut dm = Choices {
                accept: mode != "decline",
            };
            let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
            let receipt = game
                .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
                .unwrap();
            let watcher = receipt.original.into_result().unwrap().new_id;
            settle(&mut game, &mut dm);
            let recipients = tokens(&game, watcher);
            assert_eq!(recipients.len(), 2);
            let source = recipients[0];
            let other = recipients[1];
            let target = if mode == "creature" {
                ironsmith::events::DamageTarget::Object(other)
            } else {
                ironsmith::events::DamageTarget::Player(B)
            };
            let combat = mode != "noncombat";
            game.queue_trigger_event(
                Default::default(),
                ironsmith::triggers::TriggerEvent::new(
                    ironsmith::events::DamageEvent::with_cause(
                        source,
                        target,
                        1,
                        combat,
                        if combat {
                            ironsmith::events::EventCause::combat_damage(source)
                        } else {
                            ironsmith::events::EventCause::effect()
                        },
                    ),
                    Default::default(),
                ),
            );
            settle(&mut game, &mut dm);
            let converted = mode == "accept";
            assert_eq!(
                game.object_has_card_type(source, ironsmith::CardType::Creature),
                !converted,
                "{mode}"
            );
            assert!(game.object_has_card_type(source, ironsmith::CardType::Enchantment));
            assert_eq!(
                game.calculated_subtypes(source)
                    .contains(&ironsmith::Subtype::Aura),
                converted
            );
            assert_eq!(
                game.calculated_subtypes(source)
                    .contains(&ironsmith::Subtype::Curse),
                converted
            );
            assert_eq!(
                game.object(source).unwrap().attached_to,
                converted.then_some(ironsmith::object::AttachmentTarget::Player(B))
            );
            assert_eq!(
                game.player(A).unwrap().life,
                if converted { 21 } else { 20 }
            );
            assert_eq!(
                game.player(B).unwrap().life,
                if converted { 19 } else { 20 }
            );
            assert!(
                game.object_has_card_type(other, ironsmith::CardType::Creature),
                "the ability changes only its own token"
            );
            if converted {
                // Enchant opponent is a legality constraint: an effect trying to attach to
                // the controller must preserve the current legal attachment.
                let attach = Effect::new(ironsmith::effects::AttachObjectsEffect::new(
                    ChooseSpec::SpecificObject(source),
                    ChooseSpec::SpecificPlayer(A),
                ));
                execute_effect(
                    &mut game,
                    &attach,
                    &mut EffectContext::new_default(source, A),
                )
                .unwrap();
                assert_eq!(
                    game.object(source).unwrap().attached_to,
                    Some(ironsmith::object::AttachmentTarget::Player(B))
                );
                game.next_turn();
                assert_eq!(game.turn.turn_number,4);
                assert_eq!(game.turn.active_player,B);
                game.refresh_continuous_state().unwrap();
                assert!(
                    !game.object_has_card_type(source, ironsmith::CardType::Creature),
                    "opponent turn is before your next turn"
                );
                game.next_turn();
                assert_eq!(game.turn.turn_number,5);
                assert_eq!(game.turn.active_player,A);
                game.refresh_continuous_state().unwrap();
                assert!(game.object_has_card_type(source, ironsmith::CardType::Creature));
                assert!(
                    !game
                        .calculated_subtypes(source)
                        .contains(&ironsmith::Subtype::Aura)
                );
                assert!(
                    !game
                        .calculated_subtypes(source)
                        .contains(&ironsmith::Subtype::Curse)
                );
            }
        }
    }
}
fn compile_named(name: &str, text: &str) -> [CardDefinition; 2] {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()
}
#[test]
fn actual_blind_observer_counts_each_cursed_opponent_once_without_controller_or_owner_requirements()
{
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-final-abilities.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "msem-860")
        .unwrap();
    let card = &row["representative_cards"][0];
    for definition in compile_named("Blind Observer", card["executable_text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
        let c = PlayerId(2);
        let d = PlayerId(3);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices { accept: false };
        let curse = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Attachment witness",
            "Type: Enchantment — Aura Curse\nEnchant player",
            false,
        )
        .unwrap();
        for target in [A, B, B, c] {
            let aura = game.create_object_from_definition(&curse, A, Zone::Battlefield);
            assert!(game.attach_object_to_target(
                aura,
                ironsmith::object::AttachmentTarget::Player(target)
            ));
        }
        game.queue_trigger_event(
            Default::default(),
            ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::BeginningOfEndStepEvent::new(A),
                Default::default(),
            ),
        );
        settle(&mut game, &mut dm);
        assert_eq!(
            game.player(A).unwrap().life,
            20,
            "only opponents, even when the controller is cursed"
        );
        assert_eq!(
            game.player(B).unwrap().life,
            18,
            "two Curses select one player once"
        );
        assert_eq!(
            game.player(c).unwrap().life,
            18,
            "controller and owner of a Curse need not equal its recipient"
        );
        assert_eq!(
            game.player(d).unwrap().life,
            20,
            "uncursed opponent excluded"
        );
        // The same filter capability accepts any attachment quality, independently
        // of the special adjective's Curse shorthand.
        let aura_def = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Noncurse Aura witness",
            "Type: Enchantment — Aura\nEnchant player",
            false,
        )
        .unwrap();
        let aura = game.create_object_from_definition(&aura_def, A, Zone::Battlefield);
        game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Player(d));
        for program in compile_named(
            "Qualified attachment action",
            "Type: Sorcery\nEach opponent with an Aura attached to them gains 3 life.",
        ) {
            let before = [B, c, d].map(|id| game.player(id).unwrap().life);
            let mut ctx = EffectContext::new_default(source, A);
            for effect in program.spell_effect.as_ref().unwrap() {
                execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            for (index, id) in [B, c, d].into_iter().enumerate() {
                assert_eq!(game.player(id).unwrap().life, before[index] + 3);
            }
            assert_eq!(game.player(A).unwrap().life, 20);
        }
    }
}
