#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::GameState;
use crate::game_state::Target;

fn creature(
    name: &str,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    power: i32,
    toughness: i32,
) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn zone_of_stable(game: &GameState, stable_id: crate::ids::StableId) -> Zone {
    let id = game
        .find_object_by_stable_id(stable_id)
        .expect("object should remain findable by stable ID");
    game.object(id).expect("object should exist").zone
}

fn trigger_event<E: crate::events::GameEventType + 'static>(
    event: E,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        event,
        crate::provenance::ProvNodeId::default(),
    )
}

fn source_triggers(
    game: &GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

fn resolve_triggers_with_dm(
    game: &mut GameState,
    entries: Vec<crate::triggers::TriggeredAbilityEntry>,
    decisions: &mut impl crate::decision::DecisionMaker,
) {
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, decisions)
        .expect("trigger should be put on the stack");
    crate::game_loop::resolve_stack_entry_with(game, decisions).expect("trigger should resolve");
}

#[test]
fn older_floor_candidate_definitions_preserve_the_reported_semantic_boundaries() {
    let mageta = parse_oracle_card_definition("Mageta the Lion");
    assert_eq!(
        canonical_compiled_lines(&mageta).join("\n"),
        "{2}{W}{W}, {T}, Discard two cards: Destroy all other creatures. They can't be regenerated."
    );
    let mageta_debug = format!("{mageta:#?}");
    assert!(
        mageta_debug.contains("DestroyNoRegenerationEffect")
            && mageta_debug.contains("other: true")
            && mageta_debug.contains("DiscardEffect")
    );

    let tidecaller = parse_oracle_card_definition("Exhibition Tidecaller");
    let tidecaller_debug = format!("{tidecaller:#?}");
    assert!(
        tidecaller_debug.contains("TriggeringSpellManaSpentToCastAtLeast")
            && tidecaller_debug.contains("amount: 5")
            && canonical_compiled_lines(&tidecaller)
                .join(" ")
                .contains("that player mills ten cards instead")
    );

    let cid = parse_oracle_card_definition("Cid, Timeless Artificer");
    assert!(canonical_compiled_lines(&cid).iter().any(|line| {
        line.contains("Artifact creatures")
            && line.contains("Heroes you control get +1/+1")
            && line.contains("for each Artificer you control")
            && line.contains("each Artificer card in your graveyard")
    }));

    let krydle = parse_oracle_card_definition("Krydle of Baldur's Gate");
    let krydle_debug = format!("{krydle:#?}");
    assert!(
        krydle_debug.contains("ThisDealsCombatDamageToPlayerTrigger")
            && krydle_debug.contains("GainLifeEffect")
            && krydle_debug.contains("ScryEffect")
            && krydle_debug.contains("DamagedPlayer")
            && krydle_debug.contains("You")
    );

    let ran_and_shaw = parse_oracle_card_definition("Ran and Shaw");
    let ran_debug = format!("{ran_and_shaw:#?}");
    assert!(
        ran_debug.contains("SourceWasCastByController")
            && ran_debug.contains("GreaterThanOrEqual")
            && ran_debug.contains("CreateTokenCopyEffect")
            && ran_debug.contains("removed_supertypes")
            && ran_debug.contains("Legendary")
    );
}

#[test]
fn mageta_destroys_every_other_creature_without_allowing_regeneration() {
    let definition = parse_oracle_card_definition("Mageta the Lion");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mageta = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let shielded = game.create_object_from_definition(
        &creature(
            "Shielded Creature",
            vec![CardType::Creature],
            Vec::new(),
            2,
            2,
        ),
        bob,
        Zone::Battlefield,
    );
    let shielded_stable = game.object(shielded).expect("shielded creature").stable_id;
    let ally = game.create_object_from_definition(
        &creature("Mageta Ally", vec![CardType::Creature], Vec::new(), 2, 2),
        alice,
        Zone::Battlefield,
    );
    let ally_stable = game.object(ally).expect("ally").stable_id;
    let artifact = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Noncreature Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    let artifact_stable = game.object(artifact).expect("artifact").stable_id;
    game.add_regeneration_shield(shielded, 1);

    let effects = game
        .object(mageta)
        .expect("Mageta should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.effects.clone()),
            _ => None,
        })
        .expect("Mageta should have its activated ability");
    game.push_to_stack(crate::game_state::StackEntry::ability(
        mageta, alice, effects,
    ));
    crate::game_loop::resolve_stack_entry(&mut game).expect("Mageta's ability should resolve");

    assert_eq!(
        game.object(mageta).expect("Mageta survives").zone,
        Zone::Battlefield
    );
    assert_eq!(zone_of_stable(&game, shielded_stable), Zone::Graveyard);
    assert_eq!(zone_of_stable(&game, ally_stable), Zone::Graveyard);
    assert_eq!(zone_of_stable(&game, artifact_stable), Zone::Battlefield);
    assert_eq!(
        game.regenerated_this_turn_count(shielded),
        0,
        "the regeneration shield must not replace Mageta's destruction"
    );
}

struct TargetPlayerDecisionMaker {
    target: PlayerId,
}

impl crate::decision::DecisionMaker for TargetPlayerDecisionMaker {
    fn decide_targets(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        vec![Target::Player(self.target)]
    }
}

fn run_tidecaller_mill_case(mana_spent: u32, expected_milled: usize) {
    let definition = parse_oracle_card_definition("Exhibition Tidecaller");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let tidecaller = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let library_card = CardDefinitionBuilder::new(CardId::new(), "Mill Fodder")
        .card_types(vec![CardType::Sorcery])
        .build();
    for _ in 0..12 {
        game.create_object_from_definition(&library_card, bob, Zone::Library);
    }
    let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Opus Trigger Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    game.object_mut(spell)
        .expect("triggering spell should exist")
        .mana_spent_to_cast = crate::player::ManaPool {
        colorless: mana_spent,
        ..crate::player::ManaPool::default()
    };
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell).expect("triggering spell should exist"),
        &game,
    );
    let event = trigger_event(crate::events::spells::SpellCastEvent::new_with_snapshot(
        spell,
        alice,
        Zone::Hand,
        snapshot,
    ));
    let entries = source_triggers(&game, tidecaller, &event);
    assert_eq!(
        entries.len(),
        1,
        "Tidecaller should trigger for its controller's instant"
    );

    let library_before = game.player(bob).expect("Bob").library.len();
    let graveyard_before = game.player(bob).expect("Bob").graveyard.len();
    let mut decisions = TargetPlayerDecisionMaker { target: bob };
    resolve_triggers_with_dm(&mut game, entries, &mut decisions);
    assert_eq!(
        game.player(bob).expect("Bob").library.len(),
        library_before - expected_milled
    );
    assert_eq!(
        game.player(bob).expect("Bob").graveyard.len(),
        graveyard_before + expected_milled
    );
}

#[test]
fn exhibition_tidecaller_uses_the_triggering_spells_mana_for_three_or_ten_cards() {
    run_tidecaller_mill_case(4, 3);
    run_tidecaller_mill_case(5, 10);
}

#[test]
fn cid_buffs_only_your_artifact_creatures_and_heroes_using_both_artificer_counts() {
    let definition = parse_oracle_card_definition("Cid, Timeless Artificer");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cid = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let artifact_creature = game.create_object_from_definition(
        &creature(
            "Artifact Creature",
            vec![CardType::Artifact, CardType::Creature],
            Vec::new(),
            1,
            1,
        ),
        alice,
        Zone::Battlefield,
    );
    let hero = game.create_object_from_definition(
        &creature("Hero", vec![CardType::Creature], vec![Subtype::Hero], 2, 2),
        alice,
        Zone::Battlefield,
    );
    let ordinary = game.create_object_from_definition(
        &creature(
            "Ordinary Creature",
            vec![CardType::Creature],
            Vec::new(),
            2,
            2,
        ),
        alice,
        Zone::Battlefield,
    );
    let opponent_artifact_creature = game.create_object_from_definition(
        &creature(
            "Opponent Artifact Creature",
            vec![CardType::Artifact, CardType::Creature],
            Vec::new(),
            1,
            1,
        ),
        bob,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &creature(
            "Battlefield Artificer",
            vec![CardType::Creature],
            vec![Subtype::Artificer],
            1,
            1,
        ),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &creature(
            "Graveyard Artificer",
            vec![CardType::Creature],
            vec![Subtype::Artificer],
            1,
            1,
        ),
        alice,
        Zone::Graveyard,
    );
    game.create_object_from_definition(
        &creature(
            "Opponent Graveyard Artificer",
            vec![CardType::Creature],
            vec![Subtype::Artificer],
            1,
            1,
        ),
        bob,
        Zone::Graveyard,
    );

    game.refresh_continuous_state();
    assert_eq!(
        (
            game.current_power(artifact_creature),
            game.current_toughness(artifact_creature)
        ),
        (Some(4), Some(4)),
        "Cid, a second controlled Artificer, and one Artificer card in your graveyard should provide +3/+3"
    );
    assert_eq!(
        (game.current_power(hero), game.current_toughness(hero)),
        (Some(5), Some(5))
    );
    assert_eq!(
        (
            game.current_power(ordinary),
            game.current_toughness(ordinary)
        ),
        (Some(2), Some(2))
    );
    assert_eq!(
        (
            game.current_power(opponent_artifact_creature),
            game.current_toughness(opponent_artifact_creature)
        ),
        (Some(1), Some(1))
    );
    assert!(
        game.object(cid)
            .expect("Cid should remain on the battlefield")
            .subtypes
            .contains(&Subtype::Artificer),
        "the source itself supplies one of the three counted Artificers"
    );
}

struct KrydleDecisionMaker {
    controller: PlayerId,
    saw_controller_scry: bool,
}

impl crate::decision::DecisionMaker for KrydleDecisionMaker {
    fn decide_partition(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        assert_eq!(
            ctx.player, self.controller,
            "Krydle's controller must make the scry choice"
        );
        assert_eq!(ctx.description, "Scry 1");
        self.saw_controller_scry = true;
        ctx.cards.iter().map(|(id, _)| *id).collect()
    }
}

#[test]
fn krydle_binds_loss_and_mill_to_the_damaged_player_but_gain_and_scry_to_you() {
    let definition = parse_oracle_card_definition("Krydle of Baldur's Gate");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let krydle = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let library_card = CardDefinitionBuilder::new(CardId::new(), "Library Card")
        .card_types(vec![CardType::Sorcery])
        .build();
    for _ in 0..3 {
        game.create_object_from_definition(&library_card, bob, Zone::Library);
    }
    for _ in 0..2 {
        game.create_object_from_definition(&library_card, alice, Zone::Library);
    }

    let noncombat = trigger_event(crate::events::DamageEvent::with_cause(
        krydle,
        crate::events::DamageTarget::Player(bob),
        1,
        false,
        crate::events::cause::EventCause::effect(),
    ));
    assert!(
        source_triggers(&game, krydle, &noncombat).is_empty(),
        "noncombat damage from Krydle must not trigger the combat-damage ability"
    );

    let (dealt, prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        krydle,
        crate::events::DamageTarget::Player(bob),
        1,
        true,
        crate::events::cause::EventCause::combat_damage(krydle),
    );
    assert_eq!((dealt, prevented), (1, false));
    game.lose_life(bob, dealt);
    let combat_event = trigger_event(crate::events::DamageEvent::with_cause(
        krydle,
        crate::events::DamageTarget::Player(bob),
        1,
        true,
        crate::events::cause::EventCause::combat_damage(krydle),
    ));
    let entries = source_triggers(&game, krydle, &combat_event);
    assert_eq!(entries.len(), 1);
    let bob_library_before = game.player(bob).expect("Bob").library.len();
    let bob_graveyard_before = game.player(bob).expect("Bob").graveyard.len();
    let mut decisions = KrydleDecisionMaker {
        controller: alice,
        saw_controller_scry: false,
    };
    resolve_triggers_with_dm(&mut game, entries, &mut decisions);

    assert_eq!(
        game.life_total(bob),
        18,
        "combat damage plus Krydle's loss applies to Bob"
    );
    assert_eq!(game.life_total(alice), 21, "Alice gains the life");
    assert_eq!(
        game.player(bob).expect("Bob").library.len(),
        bob_library_before - 1
    );
    assert_eq!(
        game.player(bob).expect("Bob").graveyard.len(),
        bob_graveyard_before + 1
    );
    assert!(decisions.saw_controller_scry, "Alice, not Bob, should scry");
}

fn ran_and_shaw_entries(
    matching_grave_cards: usize,
    entered_from: Zone,
) -> (
    GameState,
    ObjectId,
    Vec<crate::triggers::TriggeredAbilityEntry>,
) {
    let definition = parse_oracle_card_definition("Ran and Shaw");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for index in 0..matching_grave_cards {
        let subtype = if index % 2 == 0 {
            Subtype::Dragon
        } else {
            Subtype::Lesson
        };
        game.create_object_from_definition(
            &creature(
                &format!("Matching Grave Card {index}"),
                vec![CardType::Creature],
                vec![subtype],
                1,
                1,
            ),
            alice,
            Zone::Graveyard,
        );
    }
    game.create_object_from_definition(
        &creature(
            "Opponent Graveyard Dragon",
            vec![CardType::Creature],
            vec![Subtype::Dragon],
            1,
            1,
        ),
        bob,
        Zone::Graveyard,
    );
    let ran_and_shaw = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(ran_and_shaw)
            .expect("Ran and Shaw should exist"),
        &game,
    );
    let event = trigger_event(crate::events::ZoneChangeEvent::with_cause(
        ran_and_shaw,
        entered_from,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        Some(snapshot),
    ));
    let entries = source_triggers(&game, ran_and_shaw, &event);
    (game, ran_and_shaw, entries)
}

#[test]
fn ran_and_shaw_copy_only_after_a_cast_entry_with_three_of_your_dragons_or_lessons() {
    let (mut game, source, entries) = ran_and_shaw_entries(3, Zone::Stack);
    assert_eq!(
        entries.len(),
        1,
        "cast entry plus three matching grave cards should trigger"
    );
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    resolve_triggers_with_dm(&mut game, entries, &mut decisions);
    let copies: Vec<_> = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| *id != source)
        .filter_map(|id| game.object(id))
        .filter(|object| object.name == "Ran and Shaw")
        .collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].kind, crate::object::ObjectKind::Token);
    assert!(
        !copies[0].supertypes.contains(&Supertype::Legendary),
        "the copy must be nonlegendary so both permanents remain"
    );

    let (_, _, only_two) = ran_and_shaw_entries(2, Zone::Stack);
    assert!(
        only_two.is_empty(),
        "two matching grave cards are below the threshold"
    );
    let (_, _, not_cast) = ran_and_shaw_entries(3, Zone::Graveyard);
    assert!(
        not_cast.is_empty(),
        "an entry that was not cast must not trigger"
    );
}
