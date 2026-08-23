#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::game_state::{StackEntry, Target};
use crate::object::{CounterType, ObjectKind};

#[derive(Default)]
struct ChooseNamedObjects {
    names: Vec<String>,
}

impl ChooseNamedObjects {
    fn new(names: &[&str]) -> Self {
        Self {
            names: names.iter().map(|name| (*name).to_string()).collect(),
        }
    }
}

impl DecisionMaker for ChooseNamedObjects {
    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let max = ctx.max.unwrap_or(usize::MAX);
        let mut selected = Vec::new();
        for name in &self.names {
            if selected.len() >= max {
                break;
            }
            if let Some(candidate) = ctx
                .candidates
                .iter()
                .find(|candidate| candidate.legal && candidate.name == *name)
                && !selected.contains(&candidate.id)
            {
                selected.push(candidate.id);
            }
        }
        if selected.len() < ctx.min {
            for candidate in ctx.candidates.iter().filter(|candidate| candidate.legal) {
                if selected.len() >= ctx.min || selected.len() >= max {
                    break;
                }
                if !selected.contains(&candidate.id) {
                    selected.push(candidate.id);
                }
            }
        }
        selected
    }

    fn decide_options(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.description == "Choose a color"
            && let Some(green) = ctx
                .options
                .iter()
                .find(|option| option.legal && option.description == "Green")
        {
            return vec![green.index];
        }

        ctx.options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.index)
            .take(ctx.min)
            .collect()
    }
}

fn card(name: &str, card_types: Vec<CardType>, mana_value: u8) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(card_types);
    if mana_value > 0 {
        builder = builder.mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value,
        )]));
    }
    builder.build()
}

fn creature(name: &str, controller_color: Option<ManaSymbol>) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2));
    if let Some(color) = controller_color {
        builder = builder.mana_cost(ManaCost::from_symbols(vec![color]));
    }
    builder.build()
}

fn zone_by_stable(game: &crate::GameState, stable: StableId) -> Zone {
    let object = game
        .find_object_by_stable_id(stable)
        .and_then(|id| game.object(id))
        .expect("the moved card should remain findable by stable id");
    object.zone
}

fn resolve_named_spell(
    game: &mut crate::GameState,
    definition: &CardDefinition,
    controller: PlayerId,
    targets: Vec<Target>,
    decisions: &mut dyn DecisionMaker,
) {
    let source = game.create_object_from_definition(definition, controller, Zone::Stack);
    game.push_to_stack(StackEntry::new(source, controller).with_targets(targets));
    crate::game_loop::resolve_stack_entry_with(game, decisions)
        .unwrap_or_else(|error| panic!("{} should resolve: {error}", definition.name()));
}

#[test]
fn mystic_genesis_uses_the_countered_spells_mana_value_for_its_ooze() {
    let definition = parse_oracle_card_definition("Mystic Genesis");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target_definition = card("Genesis Target", vec![CardType::Sorcery], 3);
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Stack);
    let target_stable = game.object(target).expect("target spell").stable_id;
    game.push_to_stack(StackEntry::new(target, bob));

    resolve_named_spell(
        &mut game,
        &definition,
        alice,
        vec![Target::Object(target)],
        &mut ChooseNamedObjects::default(),
    );

    assert_eq!(zone_by_stable(&game, target_stable), Zone::Graveyard);
    let ooze = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .find(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == "Ooze"
                    && object.kind == ObjectKind::Token
                    && game.controller_of(object) == alice
            })
        })
        .expect("Mystic Genesis should create one Ooze token");
    assert_eq!(game.calculated_power(ooze), Some(3));
    assert_eq!(game.calculated_toughness(ooze), Some(3));
}

#[test]
fn photon_blast_barrage_cast_trigger_copies_the_real_spell_x_times() {
    let definition = parse_oracle_card_definition("Photon Blast Barrage");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let victim = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Photon Victim")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 10))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.object_mut(spell).expect("Photon spell").x_value = Some(2);
    game.push_to_stack(
        StackEntry::new(spell, alice)
            .with_x(2)
            .with_targets(vec![Target::Object(victim)]),
    );
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell).expect("Photon spell should exist"),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            spell,
            alice,
            Zone::Hand,
            snapshot,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let matching = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == spell)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1, "the cast should trigger exactly once");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in matching {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Photon's cast trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Photon's copy trigger should resolve");

    assert_eq!(
        game.objects_in_zone(Zone::Stack)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|object| object.name == "Photon Blast Barrage")
            .count(),
        3,
        "the original plus exactly X=2 copies must remain on the stack"
    );
    assert_eq!(
        game.objects_in_zone(Zone::Stack)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|object| object.kind == ObjectKind::SpellCopy)
            .count(),
        2
    );

    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry(&mut game)
            .expect("each Photon copy and the original should resolve");
    }
    assert_eq!(game.damage_on(victim), 3);
}

#[test]
fn pieces_of_the_puzzle_moves_only_two_matching_cards_to_hand_and_the_rest_to_graveyard() {
    let definition = parse_oracle_card_definition("Pieces of the Puzzle");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let cards = [
        ("Pieces Instant", CardType::Instant),
        ("Pieces Sorcery", CardType::Sorcery),
        ("Pieces Extra Instant", CardType::Instant),
        ("Pieces Land One", CardType::Land),
        ("Pieces Land Two", CardType::Land),
    ];
    let stables = cards
        .iter()
        .map(|(name, card_type)| {
            let id = game.create_object_from_definition(
                &card(name, vec![*card_type], 1),
                alice,
                Zone::Library,
            );
            (*name, game.object(id).expect("library card").stable_id)
        })
        .collect::<Vec<_>>();
    let mut decisions = ChooseNamedObjects::new(&["Pieces Instant", "Pieces Sorcery"]);
    resolve_named_spell(&mut game, &definition, alice, Vec::new(), &mut decisions);

    for (name, stable) in stables {
        let expected = if matches!(name, "Pieces Instant" | "Pieces Sorcery") {
            Zone::Hand
        } else {
            Zone::Graveyard
        };
        assert_eq!(
            zone_by_stable(&game, stable),
            expected,
            "wrong zone for {name}"
        );
    }
}

#[test]
fn renounce_gains_life_only_for_the_selected_permanents_actually_sacrificed() {
    let definition = parse_oracle_card_definition("Renounce");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let selected_one = game.create_object_from_definition(
        &card("Renounce One", vec![CardType::Artifact], 1),
        alice,
        Zone::Battlefield,
    );
    let selected_two = game.create_object_from_definition(
        &card("Renounce Two", vec![CardType::Enchantment], 1),
        alice,
        Zone::Battlefield,
    );
    let survivor = game.create_object_from_definition(
        &card("Renounce Survivor", vec![CardType::Land], 0),
        alice,
        Zone::Battlefield,
    );
    let opposing = game.create_object_from_definition(
        &card("Renounce Opposing", vec![CardType::Artifact], 1),
        bob,
        Zone::Battlefield,
    );
    let selected_stables = [selected_one, selected_two]
        .map(|id| game.object(id).expect("selected permanent").stable_id);
    let mut decisions = ChooseNamedObjects::new(&["Renounce One", "Renounce Two"]);
    resolve_named_spell(&mut game, &definition, alice, Vec::new(), &mut decisions);

    for stable in selected_stables {
        assert_eq!(zone_by_stable(&game, stable), Zone::Graveyard);
    }
    assert_eq!(
        game.object(survivor).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        game.object(opposing).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(game.life_total(alice), 24);
}

fn artifact(name: &str, controller: PlayerId, game: &mut crate::GameState) -> ObjectId {
    game.create_object_from_definition(
        &card(name, vec![CardType::Artifact], 1),
        controller,
        Zone::Battlefield,
    )
}

#[test]
fn rise_and_shine_tracks_exactly_the_artifacts_that_became_creatures_in_both_cast_modes() {
    let definition = parse_oracle_card_definition("Rise and Shine");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mut ordinary = crate::tests::test_helpers::setup_two_player_game();
    let target = artifact("Rise Target", alice, &mut ordinary);
    let untargeted = artifact("Rise Untargeted", alice, &mut ordinary);
    let opposing = artifact("Rise Opposing", bob, &mut ordinary);
    resolve_named_spell(
        &mut ordinary,
        &definition,
        alice,
        vec![Target::Object(target)],
        &mut ChooseNamedObjects::default(),
    );
    assert!(
        ordinary
            .calculated_card_types(target)
            .contains(&CardType::Creature)
    );
    assert_eq!(
        ordinary.counter_count(target, CounterType::PlusOnePlusOne),
        4
    );
    for excluded in [untargeted, opposing] {
        assert!(
            !ordinary
                .calculated_card_types(excluded)
                .contains(&CardType::Creature)
        );
        assert_eq!(
            ordinary.counter_count(excluded, CounterType::PlusOnePlusOne),
            0
        );
    }

    let mut overloaded = crate::tests::test_helpers::setup_two_player_game();
    overloaded.turn.phase = crate::Phase::FirstMain;
    overloaded.turn.step = None;
    overloaded.turn.active_player = alice;
    overloaded.turn.priority_player = Some(alice);
    let spell_in_hand = overloaded.create_object_from_definition(&definition, alice, Zone::Hand);
    let affected_one = artifact("Rise All One", alice, &mut overloaded);
    let affected_two = artifact("Rise All Two", alice, &mut overloaded);
    let opponent_artifact = artifact("Rise Other Player", bob, &mut overloaded);
    let already_creature = overloaded.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Rise Artifact Creature")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let stack_id = crate::game_loop::propose_spell_cast(
        &mut overloaded,
        spell_in_hand,
        Zone::Hand,
        alice,
        &crate::CastingMethod::Alternative(0),
    )
    .expect("the production overload method should move Rise and Shine to the stack");
    overloaded.push_to_stack(
        StackEntry::new(stack_id, alice).with_casting_method(crate::CastingMethod::Alternative(0)),
    );
    crate::game_loop::resolve_stack_entry(&mut overloaded)
        .expect("overloaded Rise and Shine should resolve");

    for affected in [affected_one, affected_two] {
        assert!(
            overloaded
                .calculated_card_types(affected)
                .contains(&CardType::Creature)
        );
        assert_eq!(
            overloaded.counter_count(affected, CounterType::PlusOnePlusOne),
            4
        );
    }
    for excluded in [opponent_artifact, already_creature] {
        assert_eq!(
            overloaded.counter_count(excluded, CounterType::PlusOnePlusOne),
            0
        );
    }
}

#[test]
fn searing_rays_counts_the_chosen_color_separately_for_each_player() {
    let definition = parse_oracle_card_definition("Searing Rays");
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    for (name, controller, color) in [
        ("Alice Green", alice, ManaSymbol::Green),
        ("Bob Green One", bob, ManaSymbol::Green),
        ("Bob Green Two", bob, ManaSymbol::Green),
        ("Bob Red Decoy", bob, ManaSymbol::Red),
        ("Cara Green", cara, ManaSymbol::Green),
    ] {
        game.create_object_from_definition(
            &creature(name, Some(color)),
            controller,
            Zone::Battlefield,
        );
    }
    resolve_named_spell(
        &mut game,
        &definition,
        alice,
        Vec::new(),
        &mut ChooseNamedObjects::default(),
    );
    assert_eq!(game.life_total(alice), 19);
    assert_eq!(game.life_total(bob), 18);
    assert_eq!(game.life_total(cara), 19);
}
