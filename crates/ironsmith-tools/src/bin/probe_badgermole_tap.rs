//! Reproduces: Badgermole Cub enters, earthbend 1 animates a basic Swamp, then
//! tapping that Swamp for mana should also add the Cub's additional {G}.
//!
//! Run with:
//!   cargo run --release -p ironsmith-tools --bin probe_badgermole_tap

use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm};
use ironsmith::game_state::GameState;
use ironsmith::ids::{CardId, ObjectId, PlayerId};
use ironsmith::triggers::TriggerQueue;
use ironsmith::zone::Zone;
use ironsmith_registry::card::PowerToughness;
use ironsmith_registry::cards::CardDefinition;
use ironsmith_registry::cards::builders::CardDefinitionBuilder;
use ironsmith_registry::types::{CardType, Subtype, Supertype};

#[derive(serde::Deserialize)]
struct RawCard {
    name: String,
    #[serde(default)]
    type_line: Option<String>,
    #[serde(default)]
    oracle_text: Option<String>,
    #[serde(default)]
    power: Option<String>,
    #[serde(default)]
    toughness: Option<String>,
}

fn oracle_index() -> std::collections::HashMap<String, RawCard> {
    let path = format!("{}/../../cards.json", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(&path).expect("read cards.json");
    let cards: Vec<RawCard> = serde_json::from_str(&raw).expect("parse cards.json");
    let mut out = std::collections::HashMap::new();
    for card in cards {
        out.entry(card.name.clone()).or_insert(card);
    }
    out
}

fn types_for(type_line: &str) -> (Vec<Supertype>, Vec<CardType>, Vec<Subtype>) {
    let mut supertypes = Vec::new();
    let mut types = Vec::new();
    let mut subtypes = Vec::new();
    let mut parts = type_line.split('—');
    let head = parts.next().unwrap_or(type_line);
    for word in head.split_whitespace() {
        match word {
            "Legendary" => supertypes.push(Supertype::Legendary),
            "Basic" => supertypes.push(Supertype::Basic),
            "Snow" => supertypes.push(Supertype::Snow),
            "Artifact" => types.push(CardType::Artifact),
            "Creature" => types.push(CardType::Creature),
            "Land" => types.push(CardType::Land),
            "Enchantment" => types.push(CardType::Enchantment),
            "Instant" => types.push(CardType::Instant),
            "Sorcery" => types.push(CardType::Sorcery),
            _ => {}
        }
    }
    if let Some(tail) = parts.next() {
        for word in tail.split_whitespace() {
            match word {
                "Swamp" => subtypes.push(Subtype::Swamp),
                "Forest" => subtypes.push(Subtype::Forest),
                "Island" => subtypes.push(Subtype::Island),
                "Mountain" => subtypes.push(Subtype::Mountain),
                "Plains" => subtypes.push(Subtype::Plains),
                _ => {}
            }
        }
    }
    (supertypes, types, subtypes)
}

fn compile(index: &std::collections::HashMap<String, RawCard>, name: &str) -> CardDefinition {
    let card = index
        .get(name)
        .unwrap_or_else(|| panic!("card '{name}' missing from cards.json"));
    let type_line = card.type_line.clone().unwrap_or_default();
    let (supertypes, types, subtypes) = types_for(&type_line);
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types)
        .supertypes(supertypes)
        .subtypes(subtypes);
    if let (Some(power), Some(toughness)) = (card.power.as_deref(), card.toughness.as_deref())
        && let (Ok(power), Ok(toughness)) = (power.parse::<i32>(), toughness.parse::<i32>())
    {
        builder = builder.power_toughness(PowerToughness::fixed(power, toughness));
    }
    let text = card.oracle_text.clone().unwrap_or_default();
    if text.trim().is_empty() {
        return builder.build();
    }
    builder
        .clone()
        .parse_text(text.clone())
        .unwrap_or_else(|err| panic!("compile '{name}' failed: {err:?}"))
}

fn add_with_etb(
    game: &mut GameState,
    definition: &CardDefinition,
    player: PlayerId,
    queue: &mut TriggerQueue,
    dm: &mut SelectFirstDecisionMaker,
) -> Option<ObjectId> {
    let temp = game.create_object_from_definition(definition, player, Zone::Command);
    let result = game.move_object_with_etb_processing_with_dm(temp, Zone::Battlefield, dm)?;
    let entered = result.new_id;
    let provenance = game
        .provenance_graph_mut()
        .alloc_root_event(ironsmith::events::EventKind::EnterBattlefield);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::EnterBattlefieldEvent::new(entered, Zone::Command),
        provenance,
    );
    game.queue_trigger_event(provenance, event);
    ironsmith::game_loop::drain_pending_trigger_events(game, queue);
    Some(entered)
}

fn main() {
    let index = oracle_index();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);

    let swamp =
        game.create_object_from_definition(&compile(&index, "Swamp"), alice, Zone::Battlefield);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state();

    let mut queue = TriggerQueue::default();
    let mut dm = SelectFirstDecisionMaker;
    let cub = compile(&index, "Badgermole Cub");
    let cub_id = add_with_etb(&mut game, &cub, alice, &mut queue, &mut dm).expect("cub entered");
    let _ = ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm);
    let mut resolved = 0;
    while !game.stack.is_empty() && resolved < 8 {
        if ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).is_err() {
            break;
        }
        resolved += 1;
    }
    game.refresh_continuous_state();

    println!("cub={cub_id:?} swamp={swamp:?}");
    let raw_types = game
        .object(swamp)
        .map(|object| format!("{:?}", object.card_types))
        .unwrap_or_default();
    let calc_types = game
        .calculated_characteristics_arc(swamp)
        .map(|chars| format!("{:?}", chars.card_types))
        .unwrap_or_default();
    println!("swamp raw card_types        = {raw_types}");
    println!("swamp calculated card_types = {calc_types}");
    println!(
        "swamp counters = {}",
        game.counter_count(swamp, ironsmith::object::CounterType::PlusOnePlusOne)
    );

    let actions = compute_legal_actions(&game, alice);
    let Some(action) = actions.iter().find(|action| {
        matches!(action, LegalAction::ActivateManaAbility { source, .. } if *source == swamp)
    }) else {
        println!("no mana ability available on the swamp; actions={actions:?}");
        return;
    };

    let mut state = PriorityLoopState::new(game.players_in_game());
    apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action.clone()),
        &mut dm,
    )
    .expect("tapping the swamp for mana should work");

    let pool = &game.player(alice).expect("alice").mana_pool;
    println!(
        "pool after tapping the earthbent swamp: W={} U={} B={} R={} G={} C={}",
        pool.white, pool.blue, pool.black, pool.red, pool.green, pool.colorless
    );
    println!("expected: B=1 G=1");
}
