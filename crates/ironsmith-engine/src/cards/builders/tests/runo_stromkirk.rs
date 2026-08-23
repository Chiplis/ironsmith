#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const RUNO_FRONT_ID: u32 = 79_300;
const KROTHUSS_BACK_ID: u32 = 79_301;

fn linked_runo_pair() -> (CardDefinition, CardDefinition) {
    let mut runo = parse_oracle_card_definition("Runo Stromkirk");
    let mut krothuss = parse_oracle_card_definition("Krothuss, Lord of the Deep");
    let runo_id = CardId::from_raw(RUNO_FRONT_ID);
    let krothuss_id = CardId::from_raw(KROTHUSS_BACK_ID);

    runo.card.id = runo_id;
    runo.card.other_face = Some(krothuss_id);
    runo.card.other_face_name = Some("Krothuss, Lord of the Deep".to_string());
    runo.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;

    krothuss.card.id = krothuss_id;
    krothuss.card.other_face = Some(runo_id);
    krothuss.card.other_face_name = Some("Runo Stromkirk".to_string());
    krothuss.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;

    (runo, krothuss)
}

fn upkeep_event(player: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn runo_upkeep_reveal_of_six_mana_creature_transforms_to_krothuss() {
    let (runo_definition, krothuss_definition) = linked_runo_pair();
    crate::cards::register_runtime_custom_card(runo_definition.clone());
    crate::cards::register_runtime_custom_card(krothuss_definition);

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    let runo = game.create_object_from_definition(&runo_definition, alice, Zone::Battlefield);
    let six_drop = CardDefinitionBuilder::new(CardId::new(), "Six-Mana Sea Monster")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(6),
        ]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Serpent])
        .power_toughness(PowerToughness::fixed(6, 6))
        .build();
    game.create_object_from_definition(&six_drop, alice, Zone::Library);

    let event = upkeep_event(alice);
    let triggers = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == runo)
        .collect::<Vec<_>>();
    assert_eq!(
        triggers.len(),
        1,
        "Runo should trigger on its controller's upkeep"
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Runo's upkeep trigger should go on the stack");
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Runo's upkeep trigger should resolve");

    assert_eq!(game.transform_count(runo), 1);
    assert_eq!(
        game.object(runo)
            .expect("Runo should remain on the battlefield")
            .name,
        "Krothuss, Lord of the Deep"
    );
}

fn resolve_krothuss_copy_trigger(target_subtype: Subtype) -> (crate::GameState, usize) {
    let krothuss_definition = parse_oracle_card_definition("Krothuss, Lord of the Deep");
    let ability_debug = format!("{:#?}", krothuss_definition.abilities);
    assert!(
        ability_debug.contains("condition: TargetMatches")
            && !ability_debug.contains("condition: TaggedObjectMatches"),
        "the subtype condition must inspect the selected copy source, not Krothuss: {ability_debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    let krothuss =
        game.create_object_from_definition(&krothuss_definition, alice, Zone::Battlefield);
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Chosen Attacker")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![target_subtype])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let target = game.create_object_from_definition(&target_definition, alice, Zone::Battlefield);
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![
            crate::combat_state::AttackerInfo {
                creature: krothuss,
                target: crate::combat_state::AttackTarget::Player(bob),
            },
            crate::combat_state::AttackerInfo {
                creature: target,
                target: crate::combat_state::AttackTarget::Player(bob),
            },
        ],
        ..crate::combat_state::CombatState::default()
    });

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::new(
            krothuss,
            crate::triggers::AttackEventTarget::Player(bob),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let triggers = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == krothuss)
        .collect::<Vec<_>>();
    assert_eq!(triggers.len(), 1, "Krothuss should trigger when it attacks");
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Krothuss's attack trigger should go on the stack");
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Krothuss's attack trigger should resolve");

    let tokens = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == crate::object::ObjectKind::Token && object.name == "Chosen Attacker"
            })
        })
        .collect::<Vec<_>>();
    assert!(tokens.iter().all(|id| game.is_tapped(*id)));
    let combat = game.combat.as_ref().expect("combat should remain active");
    assert!(tokens.iter().all(|id| {
        combat
            .attackers
            .iter()
            .any(|attacker| attacker.creature == *id)
    }));

    let token_count = tokens.len();
    (game, token_count)
}

#[test]
fn krothuss_copies_an_ordinary_attacker_once_and_a_sea_monster_twice() {
    let (_ordinary_game, ordinary_count) = resolve_krothuss_copy_trigger(Subtype::Bear);
    assert_eq!(
        ordinary_count, 1,
        "an ordinary attacking creature should produce one tapped and attacking copy"
    );

    let (_kraken_game, kraken_count) = resolve_krothuss_copy_trigger(Subtype::Kraken);
    assert_eq!(
        kraken_count, 2,
        "a Kraken should replace the one-copy event with exactly two copies"
    );
}
