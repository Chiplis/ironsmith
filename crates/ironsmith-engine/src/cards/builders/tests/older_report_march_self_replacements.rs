#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::game_state::{StackEntry, Target};

fn creature(name: &str, power: i32, toughness: i32, subtypes: Vec<Subtype>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn noncreature(name: &str, card_type: CardType, mana_value: u8) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if mana_value > 0 {
        builder = builder.mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value,
        )]));
    }
    builder.build()
}

fn zone_by_stable(game: &crate::GameState, stable: StableId) -> Option<Zone> {
    game.find_object_by_stable_id(stable)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

fn move_to_graveyard_through_effect(
    game: &mut crate::GameState,
    object: ObjectId,
    controller: PlayerId,
) {
    let effect = crate::effect::Effect::move_to_zone(
        crate::target::ChooseSpec::SpecificObject(object),
        Zone::Graveyard,
        false,
    );
    let mut context = crate::effects::ExecutionContext::new_default(object, controller);
    crate::effects::execute_effect(game, &effect, &mut context)
        .expect("the later graveyard move should resolve through replacement processing");
}

#[derive(Default)]
struct NamedDecisions {
    names: Vec<String>,
    accept_optional: bool,
}

impl NamedDecisions {
    fn choosing(names: &[&str]) -> Self {
        Self {
            names: names.iter().map(|name| (*name).to_string()).collect(),
            accept_optional: false,
        }
    }
}

impl DecisionMaker for NamedDecisions {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept_optional
    }

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
        if ctx
            .description
            .contains("onto the battlefield instead of into your hand")
        {
            let destination = if self.accept_optional {
                "Battlefield"
            } else {
                "Hand"
            };
            return ctx
                .options
                .iter()
                .find(|option| option.legal && option.description == destination)
                .map(|option| vec![option.index])
                .unwrap_or_default();
        }

        ctx.options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.index)
            .take(ctx.min)
            .collect()
    }
}

fn resolve_named_spell(
    game: &mut crate::GameState,
    name: &str,
    controller: PlayerId,
    targets: Vec<Target>,
    kicked: bool,
    decisions: &mut dyn DecisionMaker,
) {
    let definition = parse_oracle_card_definition(name);
    let source = game.create_object_from_definition(&definition, controller, Zone::Stack);
    let mut entry = StackEntry::new(source, controller).with_targets(targets);
    if kicked {
        let mut paid = crate::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
        assert!(
            !definition.optional_costs.is_empty(),
            "{name} should expose its kicker as an optional cost"
        );
        paid.pay(0);
        game.object_mut(source)
            .expect("the spell should exist on the stack")
            .optional_costs_paid = paid.clone();
        entry = entry.with_optional_costs_paid(paid);
    }
    game.push_to_stack(entry);
    crate::game_loop::resolve_stack_entry_with(game, decisions)
        .unwrap_or_else(|error| panic!("{name} should resolve: {error}"));
}

#[test]
fn bleed_dry_exiles_only_the_marked_creature_if_it_dies_later_that_turn() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = game.create_object_from_definition(
        &creature("Bleed Dry Target", 2, 20, Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let target_stable = game.object(target).expect("target").stable_id;
    let unrelated = game.create_object_from_definition(
        &creature("Unrelated Dying Creature", 2, 2, Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let unrelated_stable = game
        .object(unrelated)
        .expect("unrelated creature")
        .stable_id;

    resolve_named_spell(
        &mut game,
        "Bleed Dry",
        alice,
        vec![Target::Object(target)],
        false,
        &mut NamedDecisions::default(),
    );
    assert_eq!(game.current_toughness(target), Some(7));

    move_to_graveyard_through_effect(&mut game, unrelated, bob);
    move_to_graveyard_through_effect(&mut game, target, bob);
    assert_eq!(
        zone_by_stable(&game, unrelated_stable),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        zone_by_stable(&game, target_stable),
        Some(Zone::Exile),
        "the later death of the marked creature should be replaced with exile"
    );
}

fn run_caravan_vigil(morbid: bool, accept_replacement: bool) -> (Zone, Zone) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let nonbasic = game.create_object_from_definition(
        &noncreature("Caravan Nonbasic", CardType::Land, 0),
        alice,
        Zone::Library,
    );
    let basic_definition = CardDefinitionBuilder::new(CardId::new(), "Caravan Basic")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .subtypes(vec![Subtype::Forest])
        .build();
    let basic = game.create_object_from_definition(&basic_definition, alice, Zone::Library);
    let basic_stable = game.object(basic).expect("basic land").stable_id;
    let nonbasic_stable = game.object(nonbasic).expect("nonbasic land").stable_id;
    if morbid {
        let doomed = game.create_object_from_definition(
            &creature("Caravan Morbid Creature", 1, 1, Vec::new()),
            alice,
            Zone::Battlefield,
        );
        game.move_object_by_effect(doomed, Zone::Graveyard);
    }
    let mut decisions = NamedDecisions::choosing(&["Caravan Basic"]);
    decisions.accept_optional = accept_replacement;
    resolve_named_spell(
        &mut game,
        "Caravan Vigil",
        alice,
        Vec::new(),
        false,
        &mut decisions,
    );
    (
        zone_by_stable(&game, basic_stable).expect("basic land should remain in the game"),
        zone_by_stable(&game, nonbasic_stable).expect("nonbasic land should remain in the game"),
    )
}

#[test]
fn caravan_vigil_replaces_hand_with_battlefield_only_when_morbid_is_accepted() {
    assert_eq!(run_caravan_vigil(false, true), (Zone::Hand, Zone::Library));
    assert_eq!(run_caravan_vigil(true, false), (Zone::Hand, Zone::Library));
    assert_eq!(
        run_caravan_vigil(true, true),
        (Zone::Battlefield, Zone::Library)
    );
}

fn run_dispense_justice(artifact_count: usize) -> (Zone, Zone, Zone) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for index in 0..artifact_count {
        game.create_object_from_definition(
            &noncreature(
                &format!("Metalcraft Artifact {index}"),
                CardType::Artifact,
                1,
            ),
            alice,
            Zone::Battlefield,
        );
    }
    let attacker_one = game.create_object_from_definition(
        &creature("Justice Attacker One", 2, 2, Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let attacker_two = game.create_object_from_definition(
        &creature("Justice Attacker Two", 2, 2, Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let nonattacker = game.create_object_from_definition(
        &creature("Justice Nonattacker", 2, 2, Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let stables = [attacker_one, attacker_two, nonattacker]
        .map(|id| game.object(id).expect("creature").stable_id);
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![
            crate::combat_state::AttackerInfo {
                creature: attacker_one,
                target: crate::combat_state::AttackTarget::Player(alice),
            },
            crate::combat_state::AttackerInfo {
                creature: attacker_two,
                target: crate::combat_state::AttackTarget::Player(alice),
            },
        ],
        ..Default::default()
    });
    let mut decisions = NamedDecisions::choosing(&["Justice Attacker One", "Justice Attacker Two"]);
    resolve_named_spell(
        &mut game,
        "Dispense Justice",
        alice,
        vec![Target::Player(bob)],
        false,
        &mut decisions,
    );
    (
        zone_by_stable(&game, stables[0]).unwrap(),
        zone_by_stable(&game, stables[1]).unwrap(),
        zone_by_stable(&game, stables[2]).unwrap(),
    )
}

#[test]
fn dispense_justice_metalcraft_replaces_one_sacrifice_with_exactly_two_attackers() {
    assert_eq!(
        run_dispense_justice(2),
        (Zone::Graveyard, Zone::Battlefield, Zone::Battlefield)
    );
    assert_eq!(
        run_dispense_justice(3),
        (Zone::Graveyard, Zone::Graveyard, Zone::Battlefield)
    );
}

fn run_gather_the_pack(spell_mastery: bool) -> (Zone, Zone, Vec<Zone>, Zone) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    if spell_mastery {
        for index in 0..2 {
            game.create_object_from_definition(
                &noncreature(&format!("Mastery Instant {index}"), CardType::Instant, 1),
                alice,
                Zone::Graveyard,
            );
        }
    }
    let deeper = game.create_object_from_definition(
        &creature("Outside Gather Top Five", 2, 2, Vec::new()),
        alice,
        Zone::Library,
    );
    let mut filler_stables = Vec::new();
    for index in 0..3 {
        let filler = game.create_object_from_definition(
            &noncreature(&format!("Gather Filler {index}"), CardType::Land, 0),
            alice,
            Zone::Library,
        );
        filler_stables.push(game.object(filler).expect("filler").stable_id);
    }
    let first = game.create_object_from_definition(
        &creature("Gather Creature One", 2, 2, Vec::new()),
        alice,
        Zone::Library,
    );
    let second = game.create_object_from_definition(
        &creature("Gather Creature Two", 2, 2, Vec::new()),
        alice,
        Zone::Library,
    );
    let first_stable = game.object(first).expect("first creature").stable_id;
    let second_stable = game.object(second).expect("second creature").stable_id;
    let deeper_stable = game.object(deeper).expect("deeper card").stable_id;
    let mut decisions = NamedDecisions::choosing(&["Gather Creature One", "Gather Creature Two"]);
    resolve_named_spell(
        &mut game,
        "Gather the Pack",
        alice,
        Vec::new(),
        false,
        &mut decisions,
    );
    (
        zone_by_stable(&game, first_stable).unwrap(),
        zone_by_stable(&game, second_stable).unwrap(),
        filler_stables
            .into_iter()
            .map(|stable| zone_by_stable(&game, stable).unwrap())
            .collect(),
        zone_by_stable(&game, deeper_stable).unwrap(),
    )
}

#[test]
fn gather_the_pack_spell_mastery_replaces_one_pick_with_up_to_two_from_the_same_five() {
    let ordinary = run_gather_the_pack(false);
    assert_eq!((ordinary.0, ordinary.1), (Zone::Hand, Zone::Graveyard));
    assert_eq!(ordinary.2, vec![Zone::Graveyard; 3]);
    assert_eq!(ordinary.3, Zone::Library);

    let mastery = run_gather_the_pack(true);
    assert_eq!((mastery.0, mastery.1), (Zone::Hand, Zone::Hand));
    assert_eq!(mastery.2, vec![Zone::Graveyard; 3]);
    assert_eq!(mastery.3, Zone::Library);
}

fn run_kirtars_wrath(graveyard_count: usize) -> usize {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for index in 0..graveyard_count {
        game.create_object_from_definition(
            &noncreature(&format!("Threshold Card {index}"), CardType::Instant, 1),
            alice,
            Zone::Graveyard,
        );
    }
    let alice_victim = game.create_object_from_definition(
        &creature("Kirtar Alice Victim", 2, 2, Vec::new()),
        alice,
        Zone::Battlefield,
    );
    let bob_victim = game.create_object_from_definition(
        &creature("Kirtar Bob Victim", 2, 2, Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let victim_stables =
        [alice_victim, bob_victim].map(|id| game.object(id).expect("victim").stable_id);
    resolve_named_spell(
        &mut game,
        "Kirtar's Wrath",
        alice,
        Vec::new(),
        false,
        &mut NamedDecisions::default(),
    );
    assert!(
        victim_stables
            .iter()
            .all(|stable| zone_by_stable(&game, *stable) == Some(Zone::Graveyard))
    );
    game.objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                game.controller_of(object) == alice && object.subtypes.contains(&Subtype::Spirit)
            })
        })
        .count()
}

#[test]
fn kirtars_wrath_threshold_replaces_the_plain_sweep_with_exactly_two_spirits() {
    assert_eq!(run_kirtars_wrath(6), 0);
    assert_eq!(run_kirtars_wrath(7), 2);
}

fn resolve_mana_replacement(
    name: &str,
    setup: impl FnOnce(&mut crate::GameState, PlayerId),
) -> u32 {
    let definition = parse_oracle_card_definition(name);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    setup(&mut game, alice);
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} should have an activated mana ability"));
    assert!(
        format!("{:?}", activated.mana_cost).contains("Tap"),
        "{name}'s mana ability should retain its tap cost"
    );
    game.push_to_stack(StackEntry::ability(
        source,
        alice,
        activated.effects.clone(),
    ));
    crate::game_loop::resolve_stack_entry(&mut game)
        .unwrap_or_else(|error| panic!("{name}'s mana ability should resolve: {error}"));
    game.player(alice).expect("Alice").mana_pool.green
}

#[test]
fn leafkin_druid_replaces_one_green_with_two_at_four_creatures() {
    let below = resolve_mana_replacement("Leafkin Druid", |game, alice| {
        for index in 0..2 {
            game.create_object_from_definition(
                &creature(&format!("Leafkin Ally {index}"), 2, 2, Vec::new()),
                alice,
                Zone::Battlefield,
            );
        }
    });
    let enabled = resolve_mana_replacement("Leafkin Druid", |game, alice| {
        for index in 0..3 {
            game.create_object_from_definition(
                &creature(&format!("Leafkin Ally {index}"), 2, 2, Vec::new()),
                alice,
                Zone::Battlefield,
            );
        }
    });
    assert_eq!(below, 1);
    assert_eq!(enabled, 2, "the replacement must not add one plus two");
}

#[test]
fn raucous_audience_replaces_one_green_with_two_only_for_your_power_four_creature() {
    let opponent_only = resolve_mana_replacement("Raucous Audience", |game, _alice| {
        let bob = PlayerId::from_index(1);
        game.create_object_from_definition(
            &creature("Opponent Power Four", 4, 4, Vec::new()),
            bob,
            Zone::Battlefield,
        );
    });
    let controlled = resolve_mana_replacement("Raucous Audience", |game, alice| {
        game.create_object_from_definition(
            &creature("Controlled Power Four", 4, 4, Vec::new()),
            alice,
            Zone::Battlefield,
        );
    });
    assert_eq!(opponent_only, 1);
    assert_eq!(controlled, 2, "the replacement must not add one plus two");
}

fn run_shepherd_of_the_clouds(control_mount: bool) -> (Zone, bool, bool) {
    let definition = parse_oracle_card_definition("Shepherd of the Clouds");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    if control_mount {
        game.create_object_from_definition(
            &creature("Shepherd Mount", 2, 2, vec![Subtype::Mount]),
            alice,
            Zone::Battlefield,
        );
    }
    let target = game.create_object_from_definition(
        &noncreature("Shepherd Target", CardType::Artifact, 3),
        alice,
        Zone::Graveyard,
    );
    let expensive = game.create_object_from_definition(
        &noncreature("Shepherd Expensive", CardType::Artifact, 4),
        alice,
        Zone::Graveyard,
    );
    let instant = game.create_object_from_definition(
        &noncreature("Shepherd Instant", CardType::Instant, 1),
        alice,
        Zone::Graveyard,
    );
    let target_stable = game.object(target).expect("target").stable_id;
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Shepherd should have an enters trigger");
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &triggered.effects,
        alice,
        Some(source),
        None,
    );
    let legal = requirements
        .first()
        .expect("Shepherd should require one graveyard target")
        .legal_targets
        .clone();
    game.push_to_stack(
        StackEntry::ability(source, alice, triggered.effects.clone())
            .with_targets(vec![Target::Object(target)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Shepherd trigger should resolve");
    (
        zone_by_stable(&game, target_stable).unwrap(),
        legal.contains(&Target::Object(expensive)),
        legal.contains(&Target::Object(instant)),
    )
}

#[test]
fn shepherd_of_the_clouds_replaces_hand_with_battlefield_only_while_you_control_a_mount() {
    assert_eq!(
        run_shepherd_of_the_clouds(false),
        (Zone::Hand, false, false)
    );
    assert_eq!(
        run_shepherd_of_the_clouds(true),
        (Zone::Battlefield, false, false)
    );
}

fn run_destined_warrior(full_party: bool) -> (i32, i32) {
    let definition = parse_oracle_card_definition("The Destined Warrior");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ally = game.create_object_from_definition(
        &creature("Destined Ally", 2, 2, Vec::new()),
        alice,
        Zone::Battlefield,
    );
    let opponent = game.create_object_from_definition(
        &creature("Destined Opponent", 2, 2, Vec::new()),
        bob,
        Zone::Battlefield,
    );
    if full_party {
        for (name, subtype) in [
            ("Party Cleric", Subtype::Cleric),
            ("Party Rogue", Subtype::Rogue),
            ("Party Warrior", Subtype::Warrior),
            ("Party Wizard", Subtype::Wizard),
        ] {
            game.create_object_from_definition(
                &creature(name, 1, 1, vec![subtype]),
                alice,
                Zone::Battlefield,
            );
        }
    }
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("The Destined Warrior should have a combat trigger");
    game.push_to_stack(StackEntry::ability(
        source,
        alice,
        triggered.effects.clone(),
    ));
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("The Destined Warrior trigger should resolve");
    (
        game.current_power(ally).expect("ally should have power"),
        game.current_power(opponent)
            .expect("opponent should have power"),
    )
}

#[test]
fn the_destined_warrior_full_party_replaces_plus_one_with_plus_three_for_your_creatures() {
    assert_eq!(run_destined_warrior(false), (3, 2));
    assert_eq!(
        run_destined_warrior(true),
        (5, 2),
        "full party should produce +3/+0, not the additive +4/+0"
    );
}

fn run_five_doctors(kicked: bool) -> (Zone, Zone, Zone) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let library_doctor = game.create_object_from_definition(
        &creature("Library Doctor Pick", 2, 2, vec![Subtype::Doctor]),
        alice,
        Zone::Library,
    );
    let graveyard_doctor = game.create_object_from_definition(
        &creature("Graveyard Doctor Pick", 2, 2, vec![Subtype::Doctor]),
        alice,
        Zone::Graveyard,
    );
    let non_doctor = game.create_object_from_definition(
        &creature("Non-Doctor Decoy", 2, 2, Vec::new()),
        alice,
        Zone::Library,
    );
    let stables = [library_doctor, graveyard_doctor, non_doctor]
        .map(|id| game.object(id).expect("search card").stable_id);
    let mut decisions = NamedDecisions::choosing(&["Library Doctor Pick", "Graveyard Doctor Pick"]);
    resolve_named_spell(
        &mut game,
        "The Five Doctors",
        alice,
        Vec::new(),
        kicked,
        &mut decisions,
    );
    (
        zone_by_stable(&game, stables[0]).unwrap(),
        zone_by_stable(&game, stables[1]).unwrap(),
        zone_by_stable(&game, stables[2]).unwrap(),
    )
}

#[test]
fn the_five_doctors_kicker_replaces_hand_with_battlefield_for_the_same_searched_cards() {
    assert_eq!(
        run_five_doctors(false),
        (Zone::Hand, Zone::Hand, Zone::Library)
    );
    assert_eq!(
        run_five_doctors(true),
        (Zone::Battlefield, Zone::Battlefield, Zone::Library)
    );
}
