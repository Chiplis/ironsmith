#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{parse_oracle_card_definition, resolve_triggers_for_source};
use super::*;
use crate::decision::{LegalAction, SelectFirstDecisionMaker};
use crate::effects::ExecutionContext;
use crate::mana::ManaCost;
use crate::snapshot::ObjectSnapshot;

const ORACLE_TEXT: &str = "When this artifact enters, you get {E}{E}.\nWhenever you cast a spell from anywhere other than your hand, this artifact deals 1 damage to each opponent.\n{T}, Pay {E}{E}: Exile the top card of your library. You may play it until you exile another card with this artifact.";

fn free_spell(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn cast_event(spell: ObjectId, caster: PlayerId, from_zone: Zone) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, caster, from_zone),
        crate::provenance::ProvNodeId::default(),
    )
}

fn activated_ability(definition: &CardDefinition) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Unstable Amulet should have its tap-and-energy ability")
}

fn activate_amulet(
    game: &mut crate::GameState,
    source: ObjectId,
    controller: PlayerId,
    activated: &crate::ability::ActivatedAbility,
) {
    let snapshot = ObjectSnapshot::from_object(
        game.object(source).expect("Unstable Amulet source exists"),
        game,
    );
    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx =
        ExecutionContext::new(source, controller, &mut decisions).with_source_snapshot(snapshot);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        game,
        controller,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut ctx,
    )
    .expect("Unstable Amulet's tap-and-two-energy cost should be payable");
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        controller,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Unstable Amulet's activated ability should resolve");
}

fn current_id(game: &crate::GameState, stable_id: StableId) -> ObjectId {
    game.find_object_by_stable_id(stable_id)
        .expect("the stable card should remain in the game")
}

fn can_play_from_exile(game: &crate::GameState, card: ObjectId, player: PlayerId) -> bool {
    game.effect_store
        .grant_registry
        .card_can_play_from_zone(game, card, Zone::Exile, player)
}

fn has_cast_action(game: &crate::GameState, card: ObjectId, player: PlayerId) -> bool {
    crate::decision::compute_legal_actions(game, player)
        .iter()
        .any(|action| matches!(action, LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == card))
}

#[test]
fn unstable_amulet_named_triggers_gain_energy_and_damage_only_for_nonhand_casts() {
    let definition = parse_oracle_card_definition("Unstable Amulet");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        ORACLE_TEXT
    );
    let ability_debug = format!("{:#?}", definition.abilities);
    assert!(
        ability_debug.contains("EnergyCountersEffect")
            && ability_debug.contains("from_not_hand: true")
            && ability_debug.contains("ForPlayersEffect")
            && ability_debug.contains("Opponent")
            && ability_debug.contains("PayEnergyEffect")
            && ability_debug.contains("UntilSourceExilesAnother"),
        "Unstable Amulet must retain all three typed abilities: {ability_debug}"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let amulet = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let enters = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            amulet,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(resolve_triggers_for_source(&mut game, amulet, &enters), 1);
    assert_eq!(
        game.player(alice).expect("Alice exists").energy_counters,
        2,
        "the named enters trigger must give exactly two energy"
    );

    let hand_spell =
        game.create_object_from_definition(&free_spell("Hand Cast"), alice, Zone::Stack);
    assert_eq!(
        resolve_triggers_for_source(
            &mut game,
            amulet,
            &cast_event(hand_spell, alice, Zone::Hand),
        ),
        0,
        "casting from hand must not trigger Unstable Amulet"
    );
    assert_eq!(game.player(alice).expect("Alice exists").life, 20);
    assert_eq!(game.player(bob).expect("Bob exists").life, 20);
    assert_eq!(game.player(charlie).expect("Charlie exists").life, 20);

    let opposing_exile_spell =
        game.create_object_from_definition(&free_spell("Bob Exile Cast"), bob, Zone::Stack);
    assert_eq!(
        resolve_triggers_for_source(
            &mut game,
            amulet,
            &cast_event(opposing_exile_spell, bob, Zone::Exile),
        ),
        0,
        "an opponent's non-hand cast must not trigger Alice's Amulet"
    );

    let own_exile_spell =
        game.create_object_from_definition(&free_spell("Alice Exile Cast"), alice, Zone::Stack);
    assert_eq!(
        resolve_triggers_for_source(
            &mut game,
            amulet,
            &cast_event(own_exile_spell, alice, Zone::Exile),
        ),
        1,
        "Alice's cast from exile must trigger her Amulet exactly once"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        20,
        "Unstable Amulet must not damage its controller"
    );
    assert_eq!(game.player(bob).expect("Bob exists").life, 19);
    assert_eq!(game.player(charlie).expect("Charlie exists").life, 19);
}

#[test]
fn unstable_amulet_activation_pays_tap_and_energy_and_expires_only_its_own_grant() {
    let definition = parse_oracle_card_definition("Unstable Amulet");
    let activated = activated_ability(&definition);
    assert_eq!(activated.mana_cost.costs().len(), 2);
    let cost_text = activated
        .mana_cost
        .costs()
        .iter()
        .map(|cost| cost.display())
        .collect::<Vec<_>>()
        .join(", ");
    assert!(
        cost_text.contains("{T}") && cost_text.contains("{E}{E}"),
        "{cost_text}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    let first_source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other_source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let third =
        game.create_object_from_definition(&free_spell("Amulet Third Exile"), alice, Zone::Library);
    let second = game.create_object_from_definition(
        &free_spell("Amulet Second Exile"),
        alice,
        Zone::Library,
    );
    let first = game.create_object_from_definition(
        &free_spell("Shared-Name Amulet Card"),
        alice,
        Zone::Library,
    );
    let bob_same_name = game.create_object_from_definition(
        &free_spell("Shared-Name Amulet Card"),
        bob,
        Zone::Exile,
    );
    let first_stable = game.object(first).expect("first top card").stable_id;
    let second_stable = game.object(second).expect("second top card").stable_id;
    let third_stable = game.object(third).expect("third top card").stable_id;
    assert!(game.set_player_library_order_with_audit(
        alice,
        vec![third, second, first],
        "Unstable Amulet source-link regression setup",
    ));

    game.player_mut(alice)
        .expect("Alice exists")
        .energy_counters = 1;
    assert!(
        crate::cost::can_pay_cost_with_reason(
            &game,
            first_source,
            alice,
            &activated.mana_cost,
            crate::costs::PaymentReason::ActivateAbility,
        )
        .is_err(),
        "one energy must not pay Unstable Amulet's two-energy activation"
    );
    assert!(!game.is_tapped(first_source));

    game.player_mut(alice)
        .expect("Alice exists")
        .energy_counters = 6;
    activate_amulet(&mut game, first_source, alice, activated);
    assert!(game.is_tapped(first_source));
    assert!(!game.is_tapped(other_source));
    assert_eq!(game.player(alice).expect("Alice exists").energy_counters, 4);
    let first_exiled = current_id(&game, first_stable);
    assert_eq!(
        game.object(first_exiled).expect("first exiled card").zone,
        Zone::Exile
    );
    assert_eq!(
        game.object(first_exiled).expect("first exiled card").owner,
        alice
    );
    assert!(can_play_from_exile(&game, first_exiled, alice));
    assert!(has_cast_action(&game, first_exiled, alice));
    assert!(!can_play_from_exile(&game, first_exiled, bob));
    assert!(
        !can_play_from_exile(&game, bob_same_name, alice),
        "the play grant must follow stable card identity rather than name"
    );

    activate_amulet(&mut game, other_source, alice, activated);
    assert!(game.is_tapped(other_source));
    assert_eq!(game.player(alice).expect("Alice exists").energy_counters, 2);
    let second_exiled = current_id(&game, second_stable);
    assert!(can_play_from_exile(&game, second_exiled, alice));
    assert!(
        can_play_from_exile(&game, first_exiled, alice),
        "another Amulet's exile must not expire the first source's permission"
    );

    game.untap(first_source);
    activate_amulet(&mut game, first_source, alice, activated);
    assert!(game.is_tapped(first_source));
    assert_eq!(game.player(alice).expect("Alice exists").energy_counters, 0);
    let third_exiled = current_id(&game, third_stable);
    assert!(
        !can_play_from_exile(&game, first_exiled, alice),
        "the first source's next exile must expire exactly its previous permission"
    );
    assert!(!has_cast_action(&game, first_exiled, alice));
    assert!(can_play_from_exile(&game, third_exiled, alice));
    assert!(has_cast_action(&game, third_exiled, alice));
    assert!(
        can_play_from_exile(&game, second_exiled, alice),
        "the other source's permission must remain active"
    );
}
