use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, generated_definition_has_unimplemented_content};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::events::{ZoneChangeEvent, cause::EventCause};
use ironsmith::game_loop::{
    check_and_apply_sbas, put_triggers_on_stack, resolve_stack_entry, resolve_stack_entry_with,
};
use ironsmith::object::{AttachmentTarget, ObjectKind};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::types::{CardType, Subtype};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

const PROBE: &str = "Mana cost: {2}\nType: Artifact — Equipment\nJob select\nEquip {1}";
const FIXTURES: &str = include_str!("../../../fixtures/job_select_materialization.json.fixture");

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("direct {name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false)
        .unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    let bytes = serde_json::to_vec(&artifact).unwrap();
    let restored: CompiledCardArtifact = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    [direct, registry.get(name).unwrap().clone()]
}

fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}

fn queue_entry(game: &mut GameState, equipment: ObjectId) {
    let event = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            equipment,
            Zone::Stack,
            Zone::Battlefield,
            EventCause::from_game_rule(),
            None,
        ),
        Default::default(),
    );
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack(game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "exactly one job select ETB trigger");
}

fn heroes(game: &GameState) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.kind == ObjectKind::Token && object.name == "Hero")
        })
        .collect()
}

fn assert_hero(game: &GameState, hero: ObjectId, player: PlayerId) {
    let token = game.object(hero).unwrap();
    assert_eq!(token.owner, player);
    assert_eq!(game.controller_of(token), player);
    assert!(token.has_subtype(Subtype::Hero));
    assert_eq!(token.card_types.as_slice(), &[CardType::Creature]);
    assert_eq!(token.base_power, Some(ironsmith::card::PtValue::Fixed(1)));
    assert_eq!(
        token.base_toughness,
        Some(ironsmith::card::PtValue::Fixed(1))
    );
    assert!(token.colors().is_empty());
}

fn assert_attached(game: &GameState, equipment: ObjectId, hero: ObjectId) {
    assert_eq!(
        game.object(equipment).unwrap().attached_to,
        Some(AttachmentTarget::Object(hero))
    );
    assert!(game.object(hero).unwrap().attachments.contains(&equipment));
}

#[test]
fn job_select_materialization_all_12_frozen_cards_compile_and_create_heroes() {
    let cards: serde_json::Value = serde_json::from_str(FIXTURES).unwrap();
    let cards = cards.as_array().unwrap();
    assert_eq!(cards.len(), 12);
    let alice = PlayerId::from_index(0);
    for card in cards {
        let name = card["name"].as_str().unwrap();
        for definition in definitions(name, card["text"].as_str().unwrap()) {
            assert!(
                !generated_definition_has_unimplemented_content(&definition),
                "{name}"
            );
            // Runtime definitions omit source Oracle text; rendering follows the executable structure.
            let lines = ironsmith_text::compiled_text_lines(&definition);
            assert!(
                lines
                    .iter()
                    .any(|line| line.trim_end_matches('.') == "Job select"),
                "{name}: {lines:?}"
            );
            let mut game = game();
            let equipment =
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            assert!(heroes(&game).is_empty(), "job select uses the stack");
            queue_entry(&mut game, equipment);
            assert!(heroes(&game).is_empty(), "token is created on resolution");
            resolve_stack_entry(&mut game).unwrap();
            let created = heroes(&game);
            assert_eq!(created.len(), 1, "{name}");
            assert_hero(&game, created[0], alice);
            assert_attached(&game, equipment, created[0]);
        }
    }
}

#[test]
fn job_select_materialization_tracks_trigger_controller_not_equipment_owner_or_new_controller() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Borrowed job select", PROBE) {
        let mut game = game();
        let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.set_current_controller(equipment, bob).unwrap();
        queue_entry(&mut game, equipment);
        game.set_current_controller(equipment, alice).unwrap();
        resolve_stack_entry(&mut game).unwrap();
        let created = heroes(&game);
        assert_eq!(created.len(), 1);
        assert_hero(&game, created[0], bob);
        assert_eq!(game.object(equipment).unwrap().owner, alice);
        assert_eq!(game.controller_of_id(equipment), Some(alice));
        assert_attached(&game, equipment, created[0]);
    }
}

#[test]
fn job_select_materialization_resolves_after_source_removal_without_attaching_new_identity() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Removed job select", PROBE) {
        for blink in [false, true] {
            let mut game = game();
            let equipment =
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            queue_entry(&mut game, equipment);
            let moved = game.move_object_by_effect(equipment, Zone::Exile).unwrap();
            let returned = blink.then(|| {
                game.move_object_by_effect(moved, Zone::Battlefield)
                    .unwrap()
            });
            resolve_stack_entry(&mut game).unwrap();
            let created = heroes(&game);
            assert_eq!(created.len(), 1, "the trigger survives its source");
            assert_hero(&game, created[0], alice);
            assert!(game.object(created[0]).unwrap().attachments.is_empty());
            if let Some(returned) = returned {
                assert_ne!(equipment, returned);
                assert_eq!(game.object(returned).unwrap().attached_to, None);
            }
        }
    }
}

#[test]
fn job_select_materialization_attachment_adds_stats_and_types_and_detaches_on_death() {
    let cards: serde_json::Value = serde_json::from_str(FIXTURES).unwrap();
    let card = cards
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["name"] == "Bard's Bow")
        .unwrap();
    let alice = PlayerId::from_index(0);
    for definition in definitions("Bard's Bow", card["text"].as_str().unwrap()) {
        let mut game = game();
        let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        queue_entry(&mut game, equipment);
        resolve_stack_entry(&mut game).unwrap();
        let hero = heroes(&game)[0];
        assert_eq!(game.calculated_power(hero), Some(3));
        assert_eq!(game.calculated_toughness(hero), Some(3));
        assert!(
            game.current_subtypes(hero)
                .unwrap()
                .contains(&Subtype::Bard)
        );
        assert!(game.current_has_static_ability_id(
            hero,
            ironsmith::static_abilities::StaticAbilityId::Reach
        ));
        game.move_object_by_effect(hero, Zone::Graveyard).unwrap();
        check_and_apply_sbas(&mut game, &mut TriggerQueue::new()).unwrap();
        assert!(game.battlefield.contains(&equipment));
        assert_eq!(game.object(equipment).unwrap().attached_to, None);
    }
}

#[test]
fn job_select_materialization_doubled_tokens_offer_one_attachment_to_current_equipment_controller()
{
    struct ChooseLast {
        expected_player: PlayerId,
        chosen: Option<ObjectId>,
        pause: bool,
        pending: bool,
    }
    impl DecisionMaker for ChooseLast {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            context: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(context.player, self.expected_player);
            assert_eq!(context.candidates.len(), 2);
            assert_eq!(context.min, 1);
            assert_eq!(context.max, Some(1));
            if self.pause {
                self.pending = true;
                return Vec::new();
            }
            let id = context.candidates.last().unwrap().id;
            self.chosen = Some(id);
            vec![id]
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let doubler = compile_to_runtime_definition(
        "Token doubler",
        "Type: Enchantment\nIf an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.",
        false,
    ).unwrap();
    for keyword in ["Job select", "Living weapon", "For Mirrodin!"] {
        for definition in definitions("Token Equipment", &PROBE.replace("Job select", keyword)) {
            let mut game = game();
            game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
            let equipment =
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            queue_entry(&mut game, equipment);
            game.set_current_controller(equipment, bob).unwrap();
            let mut dm = ChooseLast {
                expected_player: bob,
                chosen: None,
                pause: true,
                pending: false,
            };
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(dm.awaiting_choice());
            assert_eq!(
                game.stack.len(),
                1,
                "an unanswered attachment choice preserves the trigger"
            );
            assert!(
                !game
                    .battlefield
                    .iter()
                    .any(|id| game.object(*id).unwrap().kind == ObjectKind::Token),
                "choice suspension rolls back uncommitted tokens"
            );
            assert_eq!(game.object(equipment).unwrap().attached_to, None);
            dm.pause = false;
            dm.pending = false;
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let chosen = dm.chosen.expect("doubled tokens must offer a choice");
            assert_attached(&game, equipment, chosen);
            let tokens = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.kind == ObjectKind::Token)
                })
                .collect::<Vec<_>>();
            assert_eq!(tokens.len(), 2);
            assert_eq!(
                tokens
                    .iter()
                    .filter(|id| !game.object(**id).unwrap().attachments.is_empty())
                    .count(),
                1
            );
            assert!(
                tokens
                    .iter()
                    .all(|id| game.object(*id).unwrap().owner == alice)
            );
        }
    }
}

#[test]
fn job_select_materialization_does_not_hide_unsupported_markers() {
    let mut definition =
        compile_to_runtime_definition("Unsupported control", PROBE, false).unwrap();
    definition
        .abilities
        .push(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::keyword_fallback_text(
                "Unknown job select extension",
            ),
        ));
    assert!(generated_definition_has_unimplemented_content(&definition));
    assert!(
        definition
            .abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
    );
}

#[test]
fn job_select_materialization_attachment_is_nontargeting_and_obeys_creature_equipment_legality() {
    let alice = PlayerId::from_index(0);
    let shroud = compile_to_runtime_definition(
        "Shroud anthem",
        "Type: Enchantment\nCreatures you control have shroud.",
        false,
    )
    .unwrap();
    for animated in [false, true] {
        let text = if animated {
            PROBE.replace(
                "Type: Artifact — Equipment",
                "Type: Artifact Creature — Equipment\nPower/Toughness: 2/2",
            )
        } else {
            PROBE.to_string()
        };
        for definition in definitions("Job select legality", &text) {
            let mut game = game();
            game.create_object_from_definition(&shroud, alice, Zone::Battlefield);
            let equipment =
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            queue_entry(&mut game, equipment);
            resolve_stack_entry(&mut game).unwrap();
            let created = heroes(&game);
            assert_eq!(
                created.len(),
                1,
                "illegal attachment does not undo creation"
            );
            assert_hero(&game, created[0], alice);
            assert!(game.current_has_static_ability_id(
                created[0],
                ironsmith::static_abilities::StaticAbilityId::Shroud
            ));
            if animated {
                assert_eq!(game.object(equipment).unwrap().attached_to, None);
            } else {
                assert_attached(&game, equipment, created[0]);
            }
        }
    }
}
