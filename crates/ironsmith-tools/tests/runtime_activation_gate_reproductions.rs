//! Actual activation announcements at authored conditional-gate boundaries.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct OneX;
impl DecisionMaker for OneX {
    fn decide_number(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        1u32.clamp(ctx.min, ctx.max)
    }
}

fn fixture(kind: CardType) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Activation boundary resource")
        .card_types(vec![kind]);
    if kind == CardType::Creature {
        builder
            .power_toughness(ironsmith::PowerToughness::fixed(2, 6))
            .build()
    } else {
        builder.build()
    }
}
fn put(game: &mut GameState, kind: CardType, player: PlayerId, zone: Zone, n: usize) {
    for _ in 0..n {
        game.create_object_from_definition(&fixture(kind), player, zone);
    }
}
fn setup(
    def: &CardDefinition,
    kind: &str,
    n: usize,
    other: usize,
) -> Result<(GameState, ObjectId, usize, Value), String> {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(symbol, 12);
    }
    let index = def
        .abilities
        .iter()
        .position(|a| match &a.kind {
            AbilityKind::Activated(a) => a
                .additional_restrictions
                .iter()
                .any(|r| r.to_lowercase().contains("only if")),
            _ => false,
        })
        .ok_or("no authored conditional activation")?;
    let zone = if kind.starts_with("zone_") || kind == "actual_suspend" {
        match kind {
            "zone_hand" | "actual_suspend" => Zone::Hand,
            "zone_graveyard" => Zone::Graveyard,
            "zone_exile" => Zone::Exile,
            _ => Zone::Battlefield,
        }
    } else if def.abilities[index]
        .functional_zones
        .contains(&Zone::Graveyard)
        && !def.abilities[index]
            .functional_zones
            .contains(&Zone::Battlefield)
    {
        Zone::Graveyard
    } else {
        Zone::Battlefield
    };
    let mut source = game.create_object_from_definition(def, PlayerId(0), zone);
    game.remove_summoning_sickness(source);
    game.untap(source);
    // These permanents and cards model an already-established board. No spell or
    // qualifying historical event has occurred in the current turn.
    put(
        &mut game,
        CardType::Creature,
        PlayerId(0),
        Zone::Battlefield,
        if kind == "own_creatures" { n } else { 1 },
    );
    put(
        &mut game,
        CardType::Creature,
        PlayerId(1),
        Zone::Battlefield,
        1,
    );
    put(
        &mut game,
        CardType::Artifact,
        PlayerId(1),
        Zone::Battlefield,
        1,
    );
    if def.name() == "The Seedcore" {
        let one = CardDefinitionBuilder::new(CardId::new(), "Seedcore eligible target")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_definition(&one, PlayerId(0), Zone::Battlefield);
    }
    if def.name() == "Skirsdag High Priest" {
        put(
            &mut game,
            CardType::Creature,
            PlayerId(0),
            Zone::Battlefield,
            1,
        );
    }
    for player in [PlayerId(0), PlayerId(1)] {
        put(&mut game, CardType::Instant, player, Zone::Library, 16);
        put(
            &mut game,
            CardType::Instant,
            player,
            Zone::Hand,
            if kind == "hand" && player == PlayerId(0) {
                n
            } else {
                2
            },
        );
    }
    match kind {
        "life" => game.player_mut(PlayerId(0)).unwrap().life = n as i32,
        "hand" => {}
        "graveyard_types" => {
            for card_type in [
                CardType::Creature,
                CardType::Artifact,
                CardType::Land,
                CardType::Instant,
                CardType::Sorcery,
            ]
            .into_iter()
            .take(n)
            {
                put(&mut game, card_type, PlayerId(0), Zone::Graveyard, 1);
            }
        }
        "graveyard_permanent_types" => {
            for card_type in [
                CardType::Creature,
                CardType::Artifact,
                CardType::Land,
                CardType::Enchantment,
                CardType::Planeswalker,
            ]
            .into_iter()
            .take(n)
            {
                put(&mut game, card_type, PlayerId(0), Zone::Graveyard, 1);
            }
        }
        "graveyard_permanent_count" => {
            let current = game.player(PlayerId(0)).unwrap().graveyard.len();
            if current > n {
                return Err("graveyard starts above requested count".into());
            }
            put(
                &mut game,
                CardType::Artifact,
                PlayerId(0),
                Zone::Graveyard,
                n - current,
            );
        }
        "own_exile" => put(&mut game, CardType::Instant, PlayerId(0), Zone::Exile, n),
        "source_time" => {
            game.add_counters(source, CounterType::Time, n as u32);
        }
        "city_blessing" => {
            let own = game
                .battlefield
                .iter()
                .filter(|id| game.controller_of_id(**id) == Some(PlayerId(0)))
                .count();
            if n < own {
                return Err("owned board starts above requested count".into());
            }
            put(
                &mut game,
                CardType::Artifact,
                PlayerId(0),
                Zone::Battlefield,
                n - own,
            );
            game.refresh_continuous_state();
            if game.has_citys_blessing(PlayerId(0)) != (n >= 10) {
                return Err("ascend designation did not match setup".into());
            }
        }
        "own_upkeep_grave_order" => {
            game.turn.phase = ironsmith::Phase::Beginning;
            game.turn.step = Some(ironsmith::Step::Upkeep);
            put(
                &mut game,
                CardType::Creature,
                PlayerId(0),
                Zone::Graveyard,
                n,
            );
            put(
                &mut game,
                CardType::Instant,
                PlayerId(0),
                Zone::Graveyard,
                1,
            );
        }
        "hakim_enchanted" => {
            game.turn.phase = ironsmith::Phase::Beginning;
            game.turn.step = Some(ironsmith::Step::Upkeep);
            let (_,aura)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),"Holy Strength"),"Mana cost: {W}\nType: Enchantment — Aura\nFirst printed set: Limited Edition Alpha\nEnchant creature\nEnchanted creature gets +1/+2.",false).map_err(|e|format!("Aura fixture:{e:?}"))?;
            game.create_object_from_definition(&aura, PlayerId(0), Zone::Graveyard);
            if n > 0 {
                let id = game.create_object_from_definition(&aura, PlayerId(0), Zone::Battlefield);
                if !game.attach_object_to_target(
                    id,
                    ironsmith::object::AttachmentTarget::Object(source),
                ) {
                    return Err("aura attachment fixture failed".into());
                }
            }
        }
        "own_turn" => {
            if n == 0 {
                game.turn.active_player = PlayerId(1);
            }
        }
        "no_dungeon" => {}
        "actual_suspend" => {
            let stable = game.object(source).unwrap().stable_id;
            ironsmith::special_actions::perform(
                ironsmith::special_actions::SpecialAction::Suspend { card_id: source },
                &mut game,
                PlayerId(0),
                &mut SelectFirstDecisionMaker,
            )
            .map_err(|e| format!("suspend setup:{e:?}"))?;
            source = game
                .find_object_by_stable_id(stable)
                .ok_or("suspended card missing")?;
            if game.object(source).unwrap().zone != Zone::Exile
                || game.counter_count(source, CounterType::Time) != 10
            {
                return Err("suspend did not create expected exile/time-counter state".into());
            }
        }
        "zone_hand" | "zone_battlefield" | "zone_graveyard" | "zone_exile" => {}
        "power" => {
            let base = game
                .calculated_power(source)
                .ok_or("power source missing")?;
            if (n as i32) < base {
                return Err("requested power lower than printed".into());
            }
            game.add_counters(source, CounterType::PlusOnePlusOne, n as u32 - base as u32);
        }
        "poison" => {
            game.add_player_counters_with_source(
                PlayerId(1),
                CounterType::Poison,
                n as u32,
                None,
                None,
            ).unwrap();
        }
        "opponent_graveyard" => put(
            &mut game,
            CardType::Instant,
            PlayerId(1),
            Zone::Graveyard,
            n,
        ),
        "lands" => {
            let own = usize::from(def.card.card_types.contains(&CardType::Land));
            if n < own {
                return Err("land count smaller than source".into());
            }
            put(
                &mut game,
                CardType::Land,
                PlayerId(0),
                Zone::Battlefield,
                n - own,
            );
            put(
                &mut game,
                CardType::Land,
                PlayerId(1),
                Zone::Battlefield,
                other,
            );
        }
        "no_history" => {}
        "own_creatures" => {}
        _ => return Err(format!("unknown recipe {kind}")),
    }
    let evidence = json!({"ability_index":index,"source_zone":format!("{:?}",game.object(source).unwrap().zone),"source_power":game.calculated_power(source),"source_time_counters":game.counter_count(source,CounterType::Time),"source_attachments":format!("{:?}",game.object(source).unwrap().attachments),"alice_hand":game.player(PlayerId(0)).unwrap().hand.len(),"alice_life":game.player(PlayerId(0)).unwrap().life,"bob_poison":game.player(PlayerId(1)).unwrap().counter_count(CounterType::Poison),"alice_graveyard":game.player(PlayerId(0)).unwrap().graveyard.len(),"bob_graveyard":game.player(PlayerId(1)).unwrap().graveyard.len(),"citys_blessing":game.has_citys_blessing(PlayerId(0)),"turn_history":format!("{:?}",game.turn_store.turn_history),"combat":format!("{:?}",game.combat)});
    game.effect_store.pending_trigger_events.clear();
    Ok((game, source, index, evidence))
}
fn run(def: &CardDefinition, kind: &str, n: usize, other: usize) -> Result<(Value, Value), String> {
    let (mut game, source, index, mut evidence) = setup(def, kind, n, other)?;
    let action=compute_legal_actions(&game,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,ability_index}|LegalAction::ActivateManaAbility{source:id,ability_index} if *id==source && *ability_index==index));
    let legal = action.is_some();
    let mut announced = false;
    if let Some(action) = action {
        let is_mana = matches!(action, LegalAction::ActivateManaAbility { .. });
        evidence["selected_action"] = json!(format!("{action:?}"));
        let mut dm: Box<dyn DecisionMaker> =
            if ["Barad-dûr", "Defenders of Humanity"].contains(&def.name()) {
                Box::new(OneX)
            } else {
                Box::new(SelectFirstDecisionMaker)
            };
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .map_err(|e| format!("announcement:{e}"))?;
        for _ in 0..24 {
            if is_mana
                && game.player(PlayerId(0)).unwrap().mana_pool.total() > 72
                && game.is_tapped(source)
            {
                announced = true;
                break;
            }
            if state.pending_activation.is_none() && !game.stack.is_empty() {
                announced = true;
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                return Err(format!("announcement stalled:{progress:?}"));
            };
            progress =
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
                    .map_err(|e| format!("announcement:{e}"))?;
        }
        if !announced {
            return Err("announcement budget".into());
        }
        evidence["stack_targets"] = json!(game.stack.last().map(|e| format!("{:?}", e.targets)));
        evidence["mana_spent"] =
            json!(72i64 - game.player(PlayerId(0)).unwrap().mana_pool.total() as i64);
    }
    Ok((
        json!({"activation_offered":legal,"activation_announced":announced}),
        evidence,
    ))
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "report generator, not a card correctness test"]
fn report_activation_gates() {
    let cases: Vec<(&str, &str, usize, usize, bool)> = vec![
        ("Speaker of the Heavens", "life", 26, 0, false),
        ("Speaker of the Heavens", "life", 27, 0, true),
        ("Speaker of the Heavens", "life", 30, 0, true),
        ("Ayli, Eternal Pilgrim", "life", 29, 0, false),
        ("Ayli, Eternal Pilgrim", "life", 30, 0, true),
        ("Ayli, Eternal Pilgrim", "life", 31, 0, true),
        ("Fool's Tome", "hand", 0, 0, true),
        ("Fool's Tome", "hand", 1, 0, false),
        ("Fool's Tome", "hand", 2, 0, false),
        ("Sea Gate Wreckage", "hand", 0, 0, true),
        ("Sea Gate Wreckage", "hand", 1, 0, false),
        ("Nihilistic Glee", "hand", 0, 0, true),
        ("Nihilistic Glee", "hand", 1, 0, false),
        ("Keldon Megaliths", "hand", 0, 0, true),
        ("Keldon Megaliths", "hand", 1, 0, false),
        ("The Biblioplex", "hand", 0, 0, true),
        ("The Biblioplex", "hand", 1, 0, false),
        ("The Biblioplex", "hand", 6, 0, false),
        ("The Biblioplex", "hand", 7, 0, true),
        ("The Biblioplex", "hand", 8, 0, false),
        ("Dread Wanderer", "hand", 0, 0, true),
        ("Dread Wanderer", "hand", 1, 0, true),
        ("Dread Wanderer", "hand", 2, 0, false),
        ("Resonating Lute", "hand", 6, 0, false),
        ("Resonating Lute", "hand", 7, 0, true),
        ("Resonating Lute", "hand", 8, 0, true),
        ("Bloodshot Trainee", "power", 2, 0, false),
        ("Bloodshot Trainee", "power", 3, 0, false),
        ("Bloodshot Trainee", "power", 4, 0, true),
        ("Bloodshot Trainee", "power", 5, 0, true),
        ("Greenhilt Trainee", "power", 2, 0, false),
        ("Greenhilt Trainee", "power", 3, 0, false),
        ("Greenhilt Trainee", "power", 4, 0, true),
        ("Stallion of Ashmouth", "graveyard_types", 0, 0, false),
        ("Stallion of Ashmouth", "graveyard_types", 3, 0, false),
        ("Stallion of Ashmouth", "graveyard_types", 4, 0, true),
        ("Stallion of Ashmouth", "graveyard_types", 5, 0, true),
        ("Sinew Dancer", "poison", 0, 0, false),
        ("Sinew Dancer", "poison", 2, 0, false),
        ("Sinew Dancer", "poison", 3, 0, true),
        ("Weathered Wayfarer", "lands", 0, 0, false),
        ("Weathered Wayfarer", "lands", 1, 1, false),
        ("Weathered Wayfarer", "lands", 1, 2, true),
        ("Tectonic Edge", "lands", 1, 3, false),
        ("Tectonic Edge", "lands", 1, 4, true),
        ("Merfolk Windrobber", "opponent_graveyard", 0, 0, false),
        ("Merfolk Windrobber", "opponent_graveyard", 7, 0, false),
        ("Merfolk Windrobber", "opponent_graveyard", 8, 0, true),
        ("Idol of Oblivion", "no_history", 0, 0, false),
        ("Caged Zombie", "no_history", 0, 0, false),
        ("Security Detail", "own_creatures", 0, 0, true),
        ("Security Detail", "own_creatures", 1, 0, false),
        ("Haunted Plate Mail", "own_creatures", 0, 0, true),
        ("Haunted Plate Mail", "own_creatures", 1, 0, false),
    ];
    report_cases(cases, "activation-gate-reproductions.json");
}
fn report_cases(cases: Vec<(&str, &str, usize, usize, bool)>, out: &str) {
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let mut definitions = HashMap::new();
    let mut artifacts = Vec::new();
    for payload in input["cards"].as_array().unwrap() {
        let name = payload["name"].as_str().unwrap();
        if !cases.iter().any(|c| c.0 == name) {
            continue;
        }
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            payload["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum}));
        definitions.insert(name.to_string(), def);
    }
    let mut rows = Vec::new();
    for (name, kind, n, other, allowed) in cases {
        let expected = json!({"activation_offered":allowed,"activation_announced":allowed});
        let (status, actual, evidence) = match run(&definitions[name], kind, n, other) {
            Ok((actual, evidence)) => (
                if actual == expected {
                    "expected_result_observed"
                } else {
                    "semantic_mismatch"
                },
                actual,
                evidence,
            ),
            Err(error) => (
                "fixture_or_announcement_error",
                json!({"error":error}),
                Value::Null,
            ),
        };
        rows.push(json!({"card":name,"scenario":{"condition_kind":kind,"value":n,"other_value":other,"scope":"activation legality and actual announcement; effect resolution not covered"},"expected":expected,"actual":actual,"status":status,"fixture_evidence":evidence}));
    }
    let binary = std::env::current_exe().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = json!({"scope":"Strict artifact gate boundaries. Seeded reachable board and resource states, real legal-action computation, real cost/target announcement. A legal activation is not an effect correctness pass; failed fixtures are not promoted.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory)}});
    std::fs::write(
        root.join("reports/runtime-audit").join(out),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

#[test]
#[ignore = "report generator, not a card correctness test"]
fn report_activation_history_gates() {
    let cases = vec![
        ("Barad-dûr", "no_history", 0, 0, false),
        ("Bloodsoaked Champion", "no_history", 0, 0, false),
        ("Brackish Trudge", "no_history", 0, 0, false),
        ("Cleaving Reaper", "no_history", 0, 0, false),
        ("Cult Conscript", "no_history", 0, 0, false),
        ("Detective's Satchel", "no_history", 0, 0, false),
        ("Essence Anchor", "no_history", 0, 0, false),
        ("Falkenrath Pit Fighter", "no_history", 0, 0, false),
        ("Gilt-Blade Prowler", "no_history", 0, 0, false),
        ("Gutterbones", "no_history", 0, 0, false),
        ("Hall of Oracles", "no_history", 0, 0, false),
        ("Hired Claw", "no_history", 0, 0, false),
        ("Lagomos, Hand of Hatred", "no_history", 0, 0, false),
        ("Lilypad Village", "no_history", 0, 0, false),
        ("Master's Manufactory", "no_history", 0, 0, false),
        ("Minas Tirith", "no_history", 0, 0, false),
        ("Repeating Barrage", "no_history", 0, 0, false),
        ("Potioner's Trove", "no_history", 0, 0, false),
        ("Proctor of Potential", "no_history", 0, 0, false),
        ("Seeker of Insight", "no_history", 0, 0, false),
        ("Skarrgan Firebird", "no_history", 0, 0, false),
        ("Tapestry of the Ages", "no_history", 0, 0, false),
        ("Undercity Scrounger", "no_history", 0, 0, false),
        ("Vengeful Devil", "no_history", 0, 0, false),
        ("Xerex Strobe-Knight", "no_history", 0, 0, false),
        ("Zhalfirin Decoy", "no_history", 0, 0, false),
        ("Grim Repriser", "no_history", 0, 0, false),
        ("Fixer, Techno Terror", "no_history", 0, 0, false),
        ("Bonecache Overseer", "no_history", 0, 0, false),
    ];
    report_cases(cases, "activation-history-gate-reproductions.json");
}

#[test]
#[ignore = "report generator, not a card correctness test"]
fn report_remaining_state_gates() {
    let mut cases = Vec::new();
    for name in ["Fleshless Gladiator", "The Seedcore", "Glistening Sphere"] {
        for n in [2, 3] {
            cases.push((name, "poison", n, 0, n >= 3));
        }
    }
    for name in [
        "Balustrade Wurm",
        "Kindly Stranger",
        "Crop Sigil",
        "Raving Visionary",
        "Reaper of Flight Moonsilver",
        "Shifting Woodland",
        "Resurrected Cultist",
    ] {
        for n in [3, 4] {
            cases.push((name, "graveyard_types", n, 0, n >= 4));
        }
    }
    for n in [3, 4] {
        cases.push((
            "Matzalantli, the Great Door",
            "graveyard_permanent_types",
            n,
            0,
            n >= 4,
        ));
    }
    for n in [7, 8] {
        cases.push((
            "Uchbenbak, the Great Mistake",
            "graveyard_permanent_count",
            n,
            0,
            n >= 8,
        ));
    }
    for n in [0, 1] {
        cases.push(("Dreadlight Monstrosity", "own_exile", n, 0, n >= 1));
    }
    for name in ["Arch of Orazca", "Orazca Relic", "Timestream Navigator"] {
        for n in [9, 10] {
            cases.push((name, "city_blessing", n, 0, n >= 10));
        }
    }
    for n in [0, 1, 2] {
        cases.push(("Temple of Cyclical Time", "source_time", n, 0, n == 0));
    }
    for n in [1, 2] {
        cases.push(("Temple of the Dead", "hand", n, 0, n <= 1));
    }
    for n in [6, 7] {
        cases.push(("Jin-Gitaxias", "hand", n, 0, n >= 7));
    }
    for n in [7, 8] {
        cases.push(("Sheoldred", "opponent_graveyard", n, 0, n >= 8));
    }
    for n in [2, 3] {
        cases.push(("Kitsune Bonesetter", "hand", n, 0, n > 2));
    }
    for n in [2, 3] {
        cases.push(("Isolated Watchtower", "lands", 1, n, n >= 3));
    }
    for n in [0, 1] {
        cases.push(("Defenders of Humanity", "own_creatures", n, 0, n == 0));
    }
    report_cases(cases, "remaining-state-gate-reproductions.json");
}

#[test]
#[ignore = "report generator, not a card correctness test"]
fn report_special_zone_gates() {
    let mut cases = Vec::new();
    for name in [
        "Blitzball",
        "Churning Reservoir",
        "Sanar, Unfinished Genius",
        "Skirsdag High Priest",
        "Temple of Civilization",
        "Urabrask",
    ] {
        cases.push((name, "no_history", 0, 0, false));
    }
    for n in [2, 3] {
        cases.push(("Chittering Skitterling", "poison", n, 0, n >= 3));
    }
    for n in [0, 1] {
        cases.push(("Ragamuffyn", "hand", n, 0, n == 0));
    }
    cases.push(("Sarevok's Tome", "no_dungeon", 0, 0, false));
    for n in [0, 1] {
        cases.push(("Ghost Town", "own_turn", n, 0, n == 0));
    }
    for n in [0, 2, 3] {
        cases.push(("Ashen Ghoul", "own_upkeep_grave_order", n, 0, n >= 3));
    }
    for n in [0, 1] {
        cases.push(("Hakim, Loreweaver", "hakim_enchanted", n, 0, n == 0));
    }
    for name in ["Carrionette", "Glory", "Loathsome Troll"] {
        cases.push((name, "zone_battlefield", 0, 0, false));
        cases.push((name, "zone_hand", 0, 0, false));
        cases.push((name, "zone_graveyard", 0, 0, true));
    }
    cases.push(("Skyblade's Boon", "zone_hand", 0, 0, false));
    cases.push(("Skyblade's Boon", "zone_graveyard", 0, 0, true));
    cases.push(("Skyblade's Boon", "zone_exile", 0, 0, false));
    cases.push(("Greater Gargadon", "zone_battlefield", 0, 0, false));
    cases.push(("Greater Gargadon", "zone_exile", 0, 0, false));
    cases.push(("Greater Gargadon", "actual_suspend", 0, 0, true));
    report_cases(cases, "special-zone-gate-reproductions.json");
}
