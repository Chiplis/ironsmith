//! Every exact physical source and real intrinsic hand→exile→delayed-recast lifecycle.
#[path = "custom_final_next_wave_support.rs"]
mod source;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::ids::StableId;
use ironsmith::{CardDefinition, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-leitmotif-complete.json"
    ))
    .unwrap()
}
fn definitions(card: &serde_json::Value) -> [CardDefinition; 2] {
    assert!(
        card["executable_text"].is_string(),
        "unmaterialized exact source {}",
        card["source_card_key"]
    );
    source::definitions(card).unwrap_or_else(|error| panic!("{}: {error}", card["source_card_key"]))
}
struct Choices {
    accept: bool,
    preferred: Vec<StableId>,
}
impl DecisionMaker for Choices {
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
    fn decide_options(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.accept
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        let mut all = Vec::new();
        for requirement in &context.requirements {
            let mut legal = Vec::new();
            if requirement.legal_targets.contains(&Target::Player(B)) {
                legal.push(Target::Player(B));
            }
            for stable in &self.preferred {
                if let Some(id) = game.find_object_by_stable_id(*stable) {
                    let target = Target::Object(id);
                    if requirement.legal_targets.contains(&target) && !legal.contains(&target) {
                        legal.push(target);
                    }
                }
            }
            for target in &requirement.legal_targets {
                if !legal.contains(target) {
                    legal.push(*target);
                }
            }
            let count = requirement
                .max_targets
                .unwrap_or(requirement.min_targets.max(1))
                .min(legal.len());
            all.extend(legal.into_iter().take(count));
        }
        all
    }
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn compile(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false).unwrap()
}
fn add(game: &mut GameState, name: &str, text: &str, owner: PlayerId, zone: Zone) -> StableId {
    let id = game.create_object_from_definition(&compile(name, text), owner, zone);
    game.object(id).unwrap().stable_id
}
fn current(game: &GameState, id: StableId) -> ObjectId {
    game.find_object_by_stable_id(id).unwrap()
}
fn flush(game: &mut GameState, dm: &mut Choices) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn announce(game: &mut GameState, spell: ObjectId, dm: &mut Choices) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
        .expect("complete actual spell must be castable");
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..96 {
        if !state.has_pending_action() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
            return;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    panic!("cast announcement stalled");
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    flush(game, dm);
}
fn prepare(game: &mut GameState, dm: &mut Choices) -> (StableId, StableId, StableId) {
    let anchor = add(
        game,
        "Target witness",
        "Mana cost: {W}\nType: Legendary Creature — Human\nPower/Toughness: 2/8",
        A,
        Zone::Battlefield,
    );
    let enemy = add(
        game,
        "Opponent witness",
        "Type: Creature — Human\nPower/Toughness: 3/8",
        B,
        Zone::Battlefield,
    );
    let grave = add(
        game,
        "Grave permanent",
        "Mana cost: {2}\nType: Creature — Human\nPower/Toughness: 1/8",
        A,
        Zone::Graveyard,
    );
    let spell = add(
        game,
        "Grave spell",
        "Mana cost: {U}\nType: Instant\nYou gain 1 life.",
        A,
        Zone::Graveyard,
    );
    dm.preferred.extend([anchor, grave, spell, enemy]);
    for owner in [A, B] {
        for index in 0..20 {
            add(
                game,
                &format!("Library witness {index}"),
                "Type: Land",
                owner,
                Zone::Library,
            );
        }
    }
    add(game, "Discard witness", "Type: Land", A, Zone::Hand);
    for symbol in [
        ironsmith::mana::ManaSymbol::White,
        ironsmith::mana::ManaSymbol::Blue,
        ironsmith::mana::ManaSymbol::Black,
        ironsmith::mana::ManaSymbol::Red,
        ironsmith::mana::ManaSymbol::Green,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 100);
    }
    (anchor, grave, spell)
}
fn token_count(game: &GameState) -> usize {
    game.battlefield
        .iter()
        .filter(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token)
        .count()
}
fn enemy_creatures(game: &GameState) -> usize {
    game.battlefield
        .iter()
        .filter(|id| {
            game.object(**id).is_some_and(|object| {
                game.controller_of(object) == B
                    && object.card_types.contains(&ironsmith::CardType::Creature)
            })
        })
        .count()
}
struct Before {
    hand: usize,
    grave_b: usize,
    life_a: i32,
    life_b: i32,
    power: i32,
    damage: u32,
    tokens: usize,
    enemies: usize,
    anchor: ObjectId,
}
fn before(game: &GameState, anchor: StableId) -> Before {
    let id = current(game, anchor);
    Before {
        hand: game.player(A).unwrap().hand.len(),
        grave_b: game.player(B).unwrap().graveyard.len(),
        life_a: game.player(A).unwrap().life,
        life_b: game.player(B).unwrap().life,
        power: game.calculated_power(id).unwrap(),
        damage: game.damage_on(id),
        tokens: token_count(game),
        enemies: enemy_creatures(game),
        anchor: id,
    }
}
fn assert_source_effect(
    game: &GameState,
    key: &str,
    before: &Before,
    anchor: StableId,
    grave: StableId,
    spell: StableId,
) {
    let id = current(game, anchor);
    match key {
        "A Planet Far, Far Away_LVS" | "Turning of the Cosmos_HQZ" => {
            assert_eq!(game.player(A).unwrap().hand.len(), before.hand + 1)
        }
        "Start the Fire_OTH" => assert_eq!(
            game.player(A).unwrap().hand.len(),
            before.hand + 1,
            "discard1 then draw2"
        ),
        "Dead on Arrival_LVS" => assert_eq!(enemy_creatures(game), before.enemies - 1),
        "Find Happiness in Misery_LVS" => {
            assert_eq!(game.player(A).unwrap().hand.len(), before.hand + 1);
            assert_eq!(game.player(A).unwrap().life, before.life_a - 1);
            assert_eq!(game.player(B).unwrap().graveyard.len(), before.grave_b + 2);
        }
        "Ground Bound_LVS" => assert!(game.is_tapped(id)),
        "Growing Up_LVS" => {
            assert_eq!(game.calculated_power(id), Some(before.power + 3));
            assert!(
                game.current_has_static_ability_id(id, ironsmith_core::StaticAbilityId::Trample)
            );
        }
        "Head on Collision_LVS" => assert_eq!(game.player(B).unwrap().life, before.life_b - 2),
        "Hostile Habitat_LVS" => assert_eq!(game.damage_on(id), before.damage + 2),
        "Lead the Nation_LVS" => assert_eq!(game.calculated_power(id), Some(before.power + 2)),
        "No Rest_LVS" => assert_eq!(
            game.object(current(game, grave)).unwrap().zone,
            Zone::Battlefield
        ),
        "Ready to Go_LVS" => assert_eq!(token_count(game), before.tokens + 2),
        "Remember for Centuries_LVS" => {
            assert_eq!(game.object(current(game, spell)).unwrap().zone, Zone::Hand)
        }
        "Take a Walk_LVS" => {
            assert_ne!(id, before.anchor);
            assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.controller_of(game.object(id).unwrap()), A);
        }
        "Picture Perfect_MSC" => {
            assert_eq!(token_count(game), before.tokens + 1);
            assert!(
                game.battlefield
                    .iter()
                    .filter_map(|id| game.object(*id))
                    .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
                    .all(|object| !object.supertypes.contains(&ironsmith::Supertype::Legendary))
            );
        }
        _ => panic!("missing complete source behavior assertion for {key}"),
    }
}
fn witness(game: &mut GameState, source: ObjectId, type_variant: bool, matching: bool) -> ObjectId {
    let (mana, kind) = if type_variant {
        if matching {
            ("U", "Sorcery")
        } else {
            ("R", "Instant")
        }
    } else if !matching {
        ("0", "Instant")
    } else {
        let colors = game.object(source).unwrap().colors();
        let pip = [
            (ironsmith::Color::White, "W"),
            (ironsmith::Color::Blue, "U"),
            (ironsmith::Color::Black, "B"),
            (ironsmith::Color::Red, "R"),
            (ironsmith::Color::Green, "G"),
        ]
        .into_iter()
        .find(|(color, _)| colors.contains(*color))
        .unwrap()
        .1;
        (pip, "Instant")
    };
    if matching {
        game.player_mut(A).unwrap().mana_pool.empty();
        let symbol = match mana {
            "W" => ironsmith::mana::ManaSymbol::White,
            "U" => ironsmith::mana::ManaSymbol::Blue,
            "B" => ironsmith::mana::ManaSymbol::Black,
            "R" => ironsmith::mana::ManaSymbol::Red,
            "G" => ironsmith::mana::ManaSymbol::Green,
            _ => unreachable!(),
        };
        game.player_mut(A).unwrap().mana_pool.add(symbol, 1);
    }
    game.create_object_from_definition(
        &compile(
            "Cast witness",
            &format!("Mana cost: {{{mana}}}\nType: {kind}\nYou gain 1 life."),
        ),
        A,
        Zone::Hand,
    )
}
#[test]
fn all_sixteen_exact_physical_sources_compile_without_availability_skips() {
    let data = fixture();
    let mut count = 0;
    let mut added = 0;
    for row in data["mechanics"].as_array().unwrap() {
        for card in row["representative_cards"].as_array().unwrap() {
            assert_eq!(definitions(card).len(), 2);
            count += 1;
            if card["previously_in_seed"] == false {
                added += 1;
            }
        }
    }
    assert_eq!(count, 16);
    assert_eq!(added, 13);
}
#[test]
fn every_intrinsic_source_keeps_primary_effects_and_exact_single_use_delayed_recast() {
    let data = fixture();
    let mut exercised = 0;
    for row in data["mechanics"].as_array().unwrap() {
        for card in row["representative_cards"].as_array().unwrap() {
            if card["variant"] == "grant" {
                continue;
            }
            let key = card["source_card_key"].as_str().unwrap();
            exercised += 1;
            for definition in definitions(card) {
                for accept in [false, true] {
                    let mut game = game();
                    let mut dm = Choices {
                        accept: true,
                        preferred: Vec::new(),
                    };
                    let (anchor, grave, spell) = prepare(&mut game, &mut dm);
                    let source = game.create_object_from_definition(&definition, A, Zone::Hand);
                    let stable = game.object(source).unwrap().stable_id;
                    announce(&mut game, source, &mut dm);
                    let first = before(&game, anchor);
                    resolve(&mut game, &mut dm);
                    assert_source_effect(&game, key, &first, anchor, grave, spell);
                    let exiled = current(&game, stable);
                    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "{key}");
                    assert_eq!(game.effect_store.delayed_triggers.len(), 1, "{key}");
                    let mismatch = witness(&mut game, exiled, card["variant"] == "type", false);
                    announce(&mut game, mismatch, &mut dm);
                    flush(&mut game, &mut dm);
                    assert_eq!(game.stack.len(), 1, "mismatched cast cannot consume {key}");
                    resolve(&mut game, &mut dm);
                    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
                    let new_grave = add(
                        &mut game,
                        "Fresh grave permanent",
                        "Mana cost: {2}\nType: Creature — Human\nPower/Toughness: 1/8",
                        A,
                        Zone::Graveyard,
                    );
                    let new_spell = add(
                        &mut game,
                        "Fresh grave spell",
                        "Mana cost: {U}\nType: Instant\nYou gain 1 life.",
                        A,
                        Zone::Graveyard,
                    );
                    add(
                        &mut game,
                        "Fresh opponent witness",
                        "Type: Creature — Human\nPower/Toughness: 3/8",
                        B,
                        Zone::Battlefield,
                    );
                    dm.preferred.extend([new_grave, new_spell]);
                    game.turn.turn_number += 2;
                    let matching = witness(&mut game, exiled, card["variant"] == "type", true);
                    announce(&mut game, matching, &mut dm);
                    flush(&mut game, &mut dm);
                    assert_eq!(game.stack.len(), 2, "{key}");
                    dm.accept = accept;
                    resolve(&mut game, &mut dm);
                    assert!(game.effect_store.delayed_triggers.is_empty());
                    if !accept {
                        assert_eq!(
                            game.object(current(&game, stable)).unwrap().zone,
                            Zone::Exile
                        );
                        assert_eq!(game.stack.len(), 1);
                        continue;
                    }
                    assert_eq!(
                        game.object(current(&game, stable)).unwrap().zone,
                        Zone::Stack
                    );
                    assert_eq!(
                        game.player(A).unwrap().mana_pool.total(),
                        0,
                        "free recast still pays no printed mana"
                    );
                    let second = before(&game, anchor);
                    resolve(&mut game, &mut dm);
                    assert_source_effect(&game, key, &second, anchor, new_grave, new_spell);
                    let finished = current(&game, stable);
                    let expected = if key == "Picture Perfect_MSC" {
                        Zone::Library
                    } else {
                        Zone::Graveyard
                    };
                    assert_eq!(
                        game.object(finished).unwrap().zone,
                        expected,
                        "recast {key}"
                    );
                    if expected == Zone::Library {
                        assert_eq!(
                            game.player(A).unwrap().library.first(),
                            Some(&finished),
                            "PicturePerfect uses actual bottom, not graveyard or top"
                        );
                    }
                    assert!(
                        game.effect_store.delayed_triggers.is_empty(),
                        "exile cast never schedules another hand-only recipe"
                    );
                }
            }
        }
    }
    assert_eq!(exercised, 15);
}
#[test]
fn countering_each_actual_intrinsic_source_never_exiles_or_registers_a_followup() {
    let data = fixture();
    for row in data["mechanics"].as_array().unwrap() {
        for card in row["representative_cards"].as_array().unwrap() {
            if card["variant"] == "grant" {
                continue;
            }
            for definition in definitions(card) {
                let mut game = game();
                let mut dm = Choices {
                    accept: true,
                    preferred: Vec::new(),
                };
                prepare(&mut game, &mut dm);
                let source = game.create_object_from_definition(&definition, A, Zone::Hand);
                let stable = game.object(source).unwrap().stable_id;
                announce(&mut game, source, &mut dm);
                ironsmith::effects::execute_effect(
                    &mut game,
                    &ironsmith::Effect::new(ironsmith::effects::CounterEffect::new(
                        ironsmith::target::ChooseSpec::SpecificObject(source),
                    )),
                    &mut ironsmith::effects::EffectContext::new_default(source, A),
                )
                .unwrap();
                assert_eq!(
                    game.object(current(&game, stable)).unwrap().zone,
                    Zone::Graveyard
                );
                assert!(game.effect_store.delayed_triggers.is_empty());
            }
        }
    }
}

fn source_card(data: &serde_json::Value, key: &str) -> serde_json::Value {
    data["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["source_card_key"] == key)
        .unwrap()
        .clone()
}
fn move_object(game: &mut GameState, id: ObjectId, zone: Zone) -> ObjectId {
    let stable = game.object(id).unwrap().stable_id;
    ironsmith::effects::execute_effect(
        game,
        &ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(
            ironsmith::target::ChooseSpec::SpecificObject(id),
            zone,
            false,
        )),
        &mut ironsmith::effects::EffectContext::new_default(id, A),
    )
    .unwrap();
    current(game, stable)
}
#[test]
fn independently_scheduled_actual_sources_recast_their_own_card_not_the_last_registration() {
    let data = fixture();
    let one = definitions(&source_card(&data, "A Planet Far, Far Away_LVS"));
    let two = definitions(&source_card(&data, "Take a Walk_LVS"));
    for route in 0..2 {
        let mut game = game();
        let mut dm = Choices {
            accept: true,
            preferred: Vec::new(),
        };
        prepare(&mut game, &mut dm);
        let first = game.create_object_from_definition(&one[route], A, Zone::Hand);
        let first_stable = game.object(first).unwrap().stable_id;
        let second = game.create_object_from_definition(&two[route], A, Zone::Hand);
        let second_stable = game.object(second).unwrap().stable_id;
        // Both actual instant casts precede both resolutions, so neither
        // already-installed delayed trigger can consume the other's cast.
        announce(&mut game, first, &mut dm);
        announce(&mut game, second, &mut dm);
        resolve(&mut game, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.effect_store.delayed_triggers.len(), 2);
        let first_exiled = current(&game, first_stable);
        let matching = witness(&mut game, first_exiled, false, true);
        announce(&mut game, matching, &mut dm);
        flush(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 3);
        for _ in 0..8 {
            if game.stack.len() == 1 {
                break;
            }
            resolve(&mut game, &mut dm);
        }
        assert_eq!(
            game.object(current(&game, first_stable)).unwrap().zone,
            Zone::Graveyard
        );
        assert_eq!(
            game.object(current(&game, second_stable)).unwrap().zone,
            Zone::Graveyard
        );
        assert!(game.effect_store.delayed_triggers.is_empty());
        assert_eq!(game.stack.len(), 1);
    }
}
#[test]
fn departing_and_reexiling_the_original_physical_card_does_not_revive_the_old_permission() {
    let data = fixture();
    for definition in definitions(&source_card(&data, "A Planet Far, Far Away_LVS")) {
        let mut game = game();
        let mut dm = Choices {
            accept: true,
            preferred: Vec::new(),
        };
        prepare(&mut game, &mut dm);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        announce(&mut game, source, &mut dm);
        resolve(&mut game, &mut dm);
        let old = current(&game, stable);
        let hand = move_object(&mut game, old, Zone::Hand);
        let fresh = move_object(&mut game, hand, Zone::Exile);
        assert_ne!(fresh, old);
        let matching = witness(&mut game, fresh, false, true);
        announce(&mut game, matching, &mut dm);
        flush(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 2);
        resolve(&mut game, &mut dm);
        assert_eq!(game.object(fresh).unwrap().zone, Zone::Exile);
        assert_eq!(game.stack.len(), 1);
        assert!(game.effect_store.delayed_triggers.is_empty());
    }
}
#[test]
fn authoritative_redundant_recipe_key_coalesces_intrinsic_and_multiple_real_time_after_time_grants()
{
    let data = fixture();
    let grants = definitions(&source_card(&data, "Time After Time_LVS"));
    let intrinsic = definitions(&source_card(&data, "A Planet Far, Far Away_LVS"));
    for route in 0..2 {
        for providers in [1, 2] {
            for printed in [false, true] {
                let mut game = game();
                let mut dm = Choices {
                    accept: true,
                    preferred: Vec::new(),
                };
                prepare(&mut game, &mut dm);
                let grantors = (0..providers)
                    .map(|_| {
                        game.create_object_from_definition(&grants[route], A, Zone::Battlefield)
                    })
                    .collect::<Vec<_>>();
                let ordinary = compile(
                    "Recipient witness",
                    "Mana cost: {1}{U}\nType: Instant\nYou gain 1 life.",
                );
                let source = game.create_object_from_definition(
                    if printed {
                        &intrinsic[route]
                    } else {
                        &ordinary
                    },
                    A,
                    Zone::Hand,
                );
                let stable = game.object(source).unwrap().stable_id;
                announce(&mut game, source, &mut dm);
                resolve(&mut game, &mut dm);
                assert_eq!(
                    game.object(current(&game, stable)).unwrap().zone,
                    Zone::Exile
                );
                assert_eq!(
                    game.effect_store.delayed_triggers.len(),
                    1,
                    "redundant instances share the declared recipe, not all same-destination callback programs"
                );
                for grantor in grantors {
                    move_object(&mut game, grantor, Zone::Graveyard);
                }
                let exiled = current(&game, stable);
                let matching = witness(&mut game, exiled, false, true);
                announce(&mut game, matching, &mut dm);
                flush(&mut game, &mut dm);
                assert_eq!(game.stack.len(), 2);
                resolve(&mut game, &mut dm);
                assert_eq!(
                    game.object(current(&game, stable)).unwrap().zone,
                    Zone::Stack
                );
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                resolve(&mut game, &mut dm);
                assert_eq!(
                    game.object(current(&game, stable)).unwrap().zone,
                    Zone::Graveyard
                );
                assert!(game.effect_store.delayed_triggers.is_empty());
            }
        }
    }
}

#[test]
fn actual_type_leitmotif_and_redundant_color_grants_keep_independent_observers() {
    let data = fixture();
    let typed = definitions(&source_card(&data, "Start the Fire_OTH"));
    let grants = definitions(&source_card(&data, "Time After Time_LVS"));
    for route in 0..2 {
        for first_type in [true, false] {
            let mut game = game();
            let mut dm = Choices {
                accept: false,
                preferred: Vec::new(),
            };
            prepare(&mut game, &mut dm);
            let providers = (0..2)
                .map(|_| game.create_object_from_definition(&grants[route], A, Zone::Battlefield))
                .collect::<Vec<_>>();
            let source = game.create_object_from_definition(&typed[route], A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            announce(&mut game, source, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(
                game.effect_store.delayed_triggers.len(),
                2,
                "one intrinsic type recipe plus one redundant color recipe from two actual providers"
            );
            for provider in providers {
                move_object(&mut game, provider, Zone::Graveyard);
            }
            for (index, type_match) in [first_type, !first_type].into_iter().enumerate() {
                let exiled = current(&game, stable);
                // Blue sorcery shares only type; red instant shares only color.
                let next = witness(&mut game, exiled, type_match, true);
                announce(&mut game, next, &mut dm);
                flush(&mut game, &mut dm);
                assert_eq!(
                    game.stack.len(),
                    2,
                    "exactly one independent observer matches"
                );
                assert_eq!(
                    game.effect_store.delayed_triggers.len(),
                    1 - index,
                    "the other observer remains until its own matching cast"
                );
                resolve(&mut game, &mut dm); // decline only the matching recipe
                assert_eq!(
                    game.object(current(&game, stable)).unwrap().zone,
                    Zone::Exile
                );
                assert_eq!(game.stack.len(), 1);
                resolve(&mut game, &mut dm);
                assert!(game.stack.is_empty());
            }
        }
    }
}


#[test]
fn generic_terminal_recipe_keeps_an_unbounded_next_cast_and_exact_exile_reference_under_strict_claims(){
    fn collect(effect:&ironsmith::Effect,out:&mut Vec<ironsmith::effects::ScheduleDelayedTriggerEffect>){
        if let Some(schedule)=effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>(){out.push(schedule.clone());}
        effect.0.visit_child_effects(&mut |child|collect(child,out));
    }
    for relation in ["color","card type"] {
        let text=format!("Mana cost: {{1}}{{U}}\nType: Instant\nDraw a card.\nIf this spell was cast from your hand, instead of putting it into your graveyard as it resolves, exile it once for the resolution recipe named \"independent remembered relation\" and when you next cast a spell that shares a {relation} with the card exiled this way, you may cast the card exiled this way without paying its mana cost.");
        let native=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_runtime_definition("Independent terminal recipe",&text,false)).unwrap();
        let(artifact,_)=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Independent terminal recipe",&text,false)).unwrap();
        let artifact=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        let wire=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap();
        for definition in[native,wire]{
            let mut schedules=Vec::new();
            for ability in &definition.abilities{let ironsmith::ability::AbilityKind::Static(ability)=&ability.kind else{continue};let Some(model)=ability.compiled_model()else{continue};if let ironsmith_core::StaticAbilityPayload::SpellResolutionDestinationProgram{follow_up,..}=&model.payload{for effect in follow_up.all_effects(){collect(effect,&mut schedules);}}}
            assert_eq!(schedules.len(),1);let schedule=&schedules[0];assert!(schedule.one_shot);assert!(!schedule.until_end_of_turn&&!schedule.until_end_of_combat);assert_eq!(schedule.duration,ironsmith_core::DelayedTriggerDuration::Forever);
            let trigger=schedule.trigger.compiled_model().cloned().unwrap();let ironsmith_core::trigger_model::TriggerKind::SpellCast{filter:Some(filter),caster}=trigger.kind else{panic!("exact cast filter required")};assert_eq!(caster,ironsmith::target::PlayerFilter::You);assert!(format!("{filter:?}").contains("__prior_exiled_card__"),"the predicate must bind its captured original card");
        }
    }
}
