use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::EffectContext as ExecutionContext;
use ironsmith::effects::EffectExecutor;
use ironsmith::game_loop::resolve_stack_entry_with;
use ironsmith::game_state::StackEntry;
use ironsmith::{CardDefinition, CardId, CardType, CounterType, GameState, PlayerId, Zone};
use serde_json::json;

fn setup() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.turn_number = 3;
    g
}
fn compile_test_card(name: &str, text: &str) -> CardDefinition {
    ironsmith_tools::parse_card_definition_with_runtime_builder(name, text, false).unwrap()
}

fn load(name: &str) -> CardDefinition {
    let text = match name {
        "Ajani's Pridemate" => {
            r###"Mana cost: {1}{W}
Type: Creature — Cat Soldier
First printed set: Magic 2011
Power/Toughness: 2/2
Whenever you gain life, put a +1/+1 counter on this creature."###
        }
        "Awaken the Blood Avatar" => {
            r###"Mana cost: {6}{B}{R}
Type: Sorcery
First printed set: Strixhaven: School of Mages Promos
As an additional cost to cast this spell, you may sacrifice any number of creatures. This spell costs {2} less to cast for each creature sacrificed this way.
Each opponent sacrifices a creature of their choice. Create a 3/6 black and red Avatar creature token with haste and "Whenever this token attacks, it deals 3 damage to each opponent.""###
        }
        "Char" => {
            r###"Mana cost: {2}{R}
Type: Instant
First printed set: Ravnica: City of Guilds
Char deals 4 damage to any target and 2 damage to you."###
        }
        "Dargo, the Shipwrecker" => {
            r###"Mana cost: {6}{R}
Type: Legendary Creature — Giant Pirate
First printed set: Commander Legends
Power/Toughness: 7/5
As an additional cost to cast this spell, you may sacrifice any number of artifacts and/or creatures. This spell costs {2} less to cast for each permanent sacrificed this way and {2} less to cast for each other artifact or creature you've sacrificed this turn.
Trample
Partner (You can have two commanders if both have partner.)"###
        }
        "Lightning Bolt" => {
            r###"Mana cost: {R}
Type: Instant
First printed set: Secrets of Strixhaven Promos
Lightning Bolt deals 3 damage to any target."###
        }
        "Rottenmouth Viper" => {
            r###"Mana cost: {5}{B}
Type: Creature — Elemental Snake
First printed set: Bloomburrow
Power/Toughness: 6/6
As an additional cost to cast this spell, you may sacrifice any number of nonland permanents. This spell costs {1} less to cast for each permanent sacrificed this way.
Whenever this creature enters or attacks, put a blight counter on it. Then for each blight counter on it, each opponent loses 4 life unless that player sacrifices a nonland permanent of their choice or discards a card."###
        }
        "Tamanoa" => {
            r###"Mana cost: {R}{G}{W}
Type: Creature — Spirit
First printed set: Coldsnap
Power/Toughness: 2/4
Whenever a noncreature source you control deals damage, you gain that much life."###
        }

        "Temple of Cyclical Time" => {
            r###"Type: Land
First printed set: The Lost Caverns of Ixalan
(Transforms from Ojer Pakpatiq, Deepest Epoch.)
{T}: Add {U}. Remove a time counter from this land.
{2}{U}, {T}: Transform this land. Activate only if it has no time counters on it and only as a sorcery."###
        }
        "Carpet of Flowers" => {
            r###"Mana cost: {G}
Type: Enchantment
First printed set: Urza's Saga
At the beginning of each of your main phases, if you haven't added mana with this ability this turn, you may add X mana of any one color, where X is the number of Islands target opponent controls."###
        }
        "Island" => {
            r###"Type: Basic Land — Island
First printed set: Limited Edition Alpha
({T}: Add {U}.)"###
        }
        "Jace, Reality Sculptor" => {
            r###"Mana cost: {3}{U}{U}
Type: Legendary Planeswalker — Jace
First printed set: Reality Fracture
Loyalty: 5
+1: Empower Jace X, where X is the number of Islands you control.
−3: Until your next turn, whenever a creature attacks you or a planeswalker you control, it gets -5/-0 until end of turn.
0: Exile all but the bottom card of each opponent's library. Activate only if there are twenty-five or more loyalty counters among Jaces you control."###
        }
        "Mossbridge Troll" => {
            r###"Mana cost: {5}{G}{G}
Type: Creature — Troll
First printed set: Shadowmoor
Power/Toughness: 5/5
If this creature would be destroyed, regenerate it.
Tap any number of untapped creatures you control other than this creature with total power 10 or greater: This creature gets +20/+20 until end of turn."###
        }
        "Mysterious Stranger" => {
            r###"Mana cost: {2}{R}{R}
Type: Creature — Human Rogue
First printed set: Fallout
Power/Toughness: 3/2
Flash
When this creature enters, for each graveyard with an instant or sorcery card in it, exile target instant or sorcery card from that graveyard. If two or more cards are exiled this way, choose one of them at random and copy it. You may cast the copy without paying its mana cost."###
        }
        "Osseous Sticktwister" => {
            r###"Mana cost: {1}{B}
Type: Artifact Creature — Scarecrow
First printed set: Duskmourn: House of Horror
Power/Toughness: 2/2
Lifelink
Delirium — At the beginning of your end step, if there are four or more card types among cards in your graveyard, each opponent may sacrifice a nonland permanent of their choice or discard a card. Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way."###
        }
        "Plargg, Dean of Chaos" => {
            r###"Mana cost: {1}{R}
Type: Legendary Creature — Orc Shaman
First printed set: Strixhaven: School of Mages Promos
Power/Toughness: 2/2
{T}, Discard a card: Draw a card.
{4}{R}, {T}: Reveal cards from the top of your library until you reveal a nonlegendary, nonland card with mana value 3 or less. You may cast that card without paying its mana cost. Put all revealed cards not cast this way on the bottom of your library in a random order."###
        }
        "Tectonic Edge" => {
            r###"Type: Land
First printed set: Worldwake
{T}: Add {C}.
{1}, {T}, Sacrifice this land: Destroy target nonbasic land. Activate only if an opponent controls four or more lands."###
        }
        "Valentin, Dean of the Vein" => {
            r###"Mana cost: {B}
Type: Legendary Creature — Vampire Warlock
First printed set: Strixhaven: School of Mages Promos
Power/Toughness: 1/1
Menace, lifelink
If a nontoken creature an opponent controls would die, exile it instead. When you do, you may pay {2}. If you do, create a 1/1 black and green Pest creature token with "When this token dies, you gain 1 life.""###
        }
        _ => panic!("missing test input: {name}"),
    };
    compile_test_card(name, text)
}
fn filler(name: &str, kind: CardType, power: i32, mv: u32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ironsmith::mana::ManaSymbol::Generic(mv as u8),
        ]))
        .power_toughness(ironsmith::PowerToughness::fixed(power, 6))
        .build()
}
fn current_card_rows() -> Vec<serde_json::Value> {
    let mut rows = vec![];
    let troll = load("Mossbridge Troll");
    for powers in [vec![5, 5], vec![4, 3, 3], vec![10], vec![9]] {
        let mut g = setup();
        let source = g.create_object_from_definition(&troll, PlayerId(0), Zone::Battlefield);
        g.remove_summoning_sickness(source);
        for (i, p) in powers.iter().enumerate() {
            let d = filler(&format!("Power {p} fixture {i}"), CardType::Creature, *p, 0);
            g.create_object_from_definition(&d, PlayerId(0), Zone::Battlefield);
        }
        let legal = compute_legal_actions(&g, PlayerId(0)).unwrap();
        let offered = legal
            .iter()
            .any(|a| matches!(a,LegalAction::ActivateAbility{source:s,..} if *s==source));
        rows.push(json!({"card":"Mossbridge Troll","scenario":{"other_untapped_creature_powers":powers},"expected":{"activation_offered":powers.iter().sum::<i32>()>=10},"actual":{"activation_offered":offered},"scope":"Real legal-action computation from canonical compiled card"}));
    }
    let plargg = load("Plargg, Dean of Chaos");
    let AbilityKind::Activated(a) = &plargg.abilities[1].kind else {
        panic!("second ability")
    };
    let first = &a.effects.segments[0].default_effects[0];
    let consult = first
        .downcast_ref::<ironsmith::effects::ConsultTopOfLibraryEffect>()
        .unwrap();
    for (kind, mv) in [
        (CardType::Land, 0),
        (CardType::Sorcery, 6),
        (CardType::Sorcery, 2),
    ] {
        let mut g = setup();
        let source = g.create_object_from_definition(&plargg, PlayerId(0), Zone::Battlefield);
        let eligible = filler("Eligible two-mana spell", CardType::Sorcery, 0, 2);
        let expected_match = g.create_object_from_definition(&eligible, PlayerId(0), Zone::Library);
        let top = filler("Top test card", kind, 0, mv);
        let top_id = g.create_object_from_definition(&top, PlayerId(0), Zone::Library);
        assert_eq!(g.player(PlayerId(0)).unwrap().library.last(), Some(&top_id));
        let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
        let _result = consult.execute(&mut g, &mut ctx).unwrap();
        let exposed = ctx
            .tagged_objects
            .get(&consult.all_tag)
            .unwrap()
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>();
        let matched = ctx
            .tagged_objects
            .get(&consult.match_tag)
            .unwrap()
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>();
        let expected_name = if kind == CardType::Land || mv > 3 {
            "Eligible two-mana spell"
        } else {
            "Top test card"
        };
        let _ = expected_match;
        rows.push(json!({"card":"Plargg, Dean of Chaos","scenario":{"top_card_type":format!("{kind:?}"),"top_mana_value":mv,"next_card":"Eligible two-mana spell"},"expected":{"matched_card":expected_name},"actual":{"matched_card":matched.first()},"exposed":exposed,"scope":"Execute actual compiled reveal-until effect, inspect its produced tags; subsequent optional cast not exercised"}));
    }
    let jace = load("Jace, Reality Sculptor");
    for loyalty in [5, 25] {
        let mut g = setup();
        let source = g.create_object_from_definition(&jace, PlayerId(0), Zone::Battlefield);
        g.object_mut(source)
            .unwrap()
            .counters
            .insert(CounterType::Loyalty, loyalty);
        for seat in [1, 2] {
            for i in 0..3 {
                g.create_object_from_definition(
                    &filler(&format!("Library {seat} card {i}"), CardType::Land, 0, 0),
                    PlayerId(seat),
                    Zone::Library,
                );
            }
        }
        let offered = compute_legal_actions(&g, PlayerId(0)).unwrap().iter().any(
            |a| matches!(a,LegalAction::ActivateAbility{source:s,ability_index:2} if *s==source),
        );
        rows.push(json!({"card":"Jace, Reality Sculptor","scenario":{"total_jace_loyalty":loyalty},"expected":{"activation_offered":loyalty>=25},"actual":{"activation_offered":offered},"scope":"Real legal-action computation"}));
        if loyalty == 25 {
            let AbilityKind::Activated(a) = &jace.abilities[2].kind else {
                panic!()
            };
            g.push_to_stack(StackEntry::ability(source, PlayerId(0), a.effects.clone()));
            let result = resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker);
            rows.push(json!({"card":"Jace, Reality Sculptor","scenario":{"total_jace_loyalty":25,"opponent_library_sizes_before":[3,3]},"expected":{"opponent_library_sizes_after":[1,1]},"actual":{"opponent_library_sizes_after":[g.player(PlayerId(1)).unwrap().library.len(),g.player(PlayerId(2)).unwrap().library.len()]},"resolution":format!("{result:?}"),"scope":"Actual compiled ultimate resolved on stack; announcement bypassed, 25-loyalty precondition satisfied"}));
        }
    }
    for row in &mut rows {
        row["status"] = json!(if row["actual"] == row["expected"] {
            "passed"
        } else {
            "semantic_mismatch"
        });
    }
    rows
}

struct ExplicitChoices {
    accept: bool,
    booleans: Vec<String>,
}
impl ironsmith::decision::DecisionMaker for ExplicitChoices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.booleans.push(format!(
            "{}: can_accept={} answer={}",
            c.description,
            c.can_accept,
            self.accept && c.can_accept
        ));
        self.accept && c.can_accept
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        c.max
    }
    fn decide_objects(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ironsmith::ObjectId> {
        SelectFirstDecisionMaker.decide_objects(g, c)
    }
    fn decide_options(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        SelectFirstDecisionMaker.decide_targets(g, c)
    }
}
fn trigger_step(g: &mut GameState, dm: &mut ExplicitChoices) -> Vec<String> {
    use ironsmith::game_loop::{drain_pending_trigger_events, put_triggers_on_stack_with_dm};
    let event = ironsmith::triggers::generate_step_trigger_events(g).unwrap();
    let mut q = ironsmith::triggers::TriggerQueue::new();
    for t in ironsmith::triggers::check_triggers(g, &event) {
        q.add(t);
    }
    put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
    let mut trace = vec![];
    for _ in 0..20 {
        drain_pending_trigger_events(g, &mut q);
        put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
        if g.stack.is_empty() {
            break;
        };
        trace.push(format!("{:?}", g.stack.last().unwrap().targets));
        resolve_stack_entry_with(g, dm).unwrap();
    }
    trace
}
fn trigger_and_condition_rows() -> Vec<serde_json::Value> {
    use ironsmith::game_loop::{drain_pending_trigger_events, put_triggers_on_stack_with_dm};
    let mut rows = vec![];
    let carpet = load("Carpet of Flowers");
    let island = load("Island");
    for n in [0, 1, 2] {
        for accept_first in [false, true] {
            let mut g = setup();
            g.create_object_from_definition(&carpet, PlayerId(0), Zone::Battlefield);
            for _ in 0..n {
                g.create_object_from_definition(&island, PlayerId(1), Zone::Battlefield);
            }
            let mut dm = ExplicitChoices {
                accept: accept_first,
                booleans: vec![],
            };
            let first_targets = trigger_step(&mut g, &mut dm);
            let first_mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
            let expected_first = if accept_first { n } else { 0 };
            if n == 0 {
                g.create_object_from_definition(&island, PlayerId(1), Zone::Battlefield);
            }
            g.turn.phase = ironsmith::Phase::NextMain;
            dm.accept = true;
            let second_targets = trigger_step(&mut g, &mut dm);
            let expected_second_triggers = if expected_first > 0 { 0 } else { 1 };
            rows.push(json!({"card":"Carpet of Flowers",
          "scenario":{"first_main_islands":n,"accept_first":accept_first,"second_main_islands":n.max(1)},
          "expected":{"first_mana":expected_first,"final_mana":n.max(1),"second_main_triggers":expected_second_triggers},
          "actual":{"first_mana":first_mana,"final_mana":g.player(PlayerId(0)).unwrap().mana_pool.total(),"second_main_triggers":second_targets.len()},
          "first_targets":first_targets,"second_targets":second_targets,"choice_trace":dm.booleans}));
        }
    }
    let stick = load("Osseous Sticktwister");
    for accept in [false, true] {
        let mut g = setup();
        g.create_object_from_definition(&stick, PlayerId(0), Zone::Battlefield);
        for kind in [
            CardType::Land,
            CardType::Instant,
            CardType::Sorcery,
            CardType::Enchantment,
        ] {
            g.create_object_from_definition(
                &filler("Graveyard type", kind, 0, 0),
                PlayerId(0),
                Zone::Graveyard,
            );
        }
        g.turn.phase = ironsmith::Phase::Ending;
        g.turn.step = Some(ironsmith::Step::End);
        let mut dm = ExplicitChoices {
            accept,
            booleans: vec![],
        };
        let trace = trigger_step(&mut g, &mut dm);
        rows.push(json!({"card":"Osseous Sticktwister","scenario":{"opponent_hands_and_battlefields":"empty","graveyard_types":4,"accept_if_legal":accept},"expected":{"opponent_life":[18,18]},"actual":{"opponent_life":[g.player(PlayerId(1)).unwrap().life,g.player(PlayerId(2)).unwrap().life]},"choice_trace":dm.booleans,"trigger_targets":trace}));
    }
    // Payable alternatives must still work, including when the first mode
    // (sacrifice) is unavailable but the second mode (discard) is available.
    for zone in [Zone::Hand, Zone::Battlefield] {
        for accept in [false, true] {
            let mut g = setup();
            g.create_object_from_definition(&stick, PlayerId(0), Zone::Battlefield);
            for kind in [
                CardType::Land,
                CardType::Instant,
                CardType::Sorcery,
                CardType::Enchantment,
            ] {
                g.create_object_from_definition(
                    &filler("Graveyard type", kind, 0, 0),
                    PlayerId(0),
                    Zone::Graveyard,
                );
            }
            for opponent in [PlayerId(1), PlayerId(2)] {
                g.create_object_from_definition(
                    &filler("Payment witness", CardType::Creature, 2, 2),
                    opponent,
                    zone,
                );
            }
            g.turn.phase = ironsmith::Phase::Ending;
            g.turn.step = Some(ironsmith::Step::End);
            let mut dm = ExplicitChoices {
                accept,
                booleans: vec![],
            };
            trigger_step(&mut g, &mut dm);
            let remaining = g
                .objects_in_deterministic_order()
                .into_iter()
                .filter(|o| o.name == "Payment witness" && o.zone == zone)
                .count();
            rows.push(json!({"card":"Osseous Sticktwister", "scenario":{"payment_zone":format!("{zone:?}"),"accept":accept},
                "expected":{"opponent_life":if accept { vec![20,20] } else { vec![18,18] },"remaining_resources":if accept {0} else {2}},
                "actual":{"opponent_life":[g.player(PlayerId(1)).unwrap().life,g.player(PlayerId(2)).unwrap().life],"remaining_resources":remaining},"choice_trace":dm.booleans}));
        }
    }
    let stranger = load("Mysterious Stranger");
    for counts in [[0_u32, 2, 1], [1, 2, 1], [0, 0, 1], [0, 0, 0]] {
        let mut g = setup();
        for (seat, n) in counts.into_iter().enumerate() {
            for i in 0..n {
                g.create_object_from_definition(
                    &filler(
                        &format!("Grave {seat} instant {i}"),
                        CardType::Instant,
                        0,
                        1,
                    ),
                    PlayerId(seat as u8),
                    Zone::Graveyard,
                );
            }
        }
        let source = g.create_object_from_definition(&stranger, PlayerId(0), Zone::Stack);
        g.move_object_by_effect(source, Zone::Battlefield).unwrap();
        let mut q = ironsmith::triggers::TriggerQueue::new();
        drain_pending_trigger_events(&mut g, &mut q);
        let mut dm = ExplicitChoices {
            accept: false,
            booleans: vec![],
        };
        put_triggers_on_stack_with_dm(&mut g, &mut q, &mut dm).unwrap();
        let targets = g.stack.last().expect("ETB stack entry").targets.clone();
        let owners = targets
            .iter()
            .filter_map(|t| match t {
                ironsmith::game_state::Target::Object(id) => g.object(*id).map(|o| o.owner.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let expected_owners = counts
            .iter()
            .enumerate()
            .filter_map(|(seat, n)| (*n > 0).then_some(seat))
            .collect::<Vec<_>>();
        let error = resolve_stack_entry_with(&mut g, &mut dm)
            .err()
            .map(|e| e.to_string());
        let remaining = (0..3)
            .map(|seat| g.player(PlayerId(seat)).unwrap().graveyard.len())
            .collect::<Vec<_>>();
        let expected_remaining = counts.map(|n| n.saturating_sub(1));
        rows.push(json!({"card":"Mysterious Stranger","scenario":{"instant_cards_in_graveyards":counts,"accept_optional_copy_cast":false},
          "expected":{"target_graveyard_owners":expected_owners,"remaining_in_graveyards":expected_remaining,"resolution_error":null},
          "actual":{"target_graveyard_owners":owners,"remaining_in_graveyards":remaining,"resolution_error":error},
          "scope":"Actual ETB announcement and resolution; exile one from each qualifying graveyard"}));
    }
    let edge = load("Tectonic Edge");
    for (own, opponent) in [(1, 4), (4, 4), (4, 3)] {
        let mut g = setup();
        let source = g.create_object_from_definition(&edge, PlayerId(0), Zone::Battlefield);
        for seat in [0, 1] {
            let n = if seat == 0 { own - 1 } else { opponent };
            for _ in 0..n {
                g.create_object_from_definition(
                    &filler("Nonbasic land witness", CardType::Land, 0, 0),
                    PlayerId(seat),
                    Zone::Battlefield,
                );
            }
        }
        g.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 5);
        let offered = compute_legal_actions(&g, PlayerId(0)).unwrap().iter().any(
            |a| matches!(a,LegalAction::ActivateAbility{source:s,ability_index:1}if *s==source),
        );
        rows.push(json!({"card":"Tectonic Edge","scenario":{"own_lands":own,"opponent_lands":opponent,"all_lands_nonbasic":true},"expected":{"activation_offered":opponent>=4},"actual":{"activation_offered":offered}}));
    }
    for row in &mut rows {
        row["status"] = json!(if row["actual"] == row["expected"] {
            "passed"
        } else {
            "semantic_mismatch"
        });
    }
    rows
}
#[test]
fn targeted_valentin_followup() {
    use ironsmith::game_loop::{drain_pending_trigger_events, put_triggers_on_stack_with_dm};
    let def = load("Valentin, Dean of the Vein");
    let mut rows = vec![];
    for (accept, mana) in [(false, 2), (true, 2), (true, 0)] {
        let mut g = setup();
        let source = g.create_object_from_definition(&def, PlayerId(0), Zone::Battlefield);
        let victim = g.create_object_from_definition(
            &filler("Opposing nontoken creature", CardType::Creature, 2, 2),
            PlayerId(1),
            Zone::Battlefield,
        );
        let stable = g.object(victim).unwrap().stable_id;
        g.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, mana);
        let filter = ironsmith::target::ObjectFilter {
            specific: Some(victim),
            ..Default::default()
        };
        g.push_to_stack(StackEntry::ability(
            source,
            PlayerId(0),
            vec![ironsmith::Effect::destroy_all(filter)],
        ));
        let mut dm = ExplicitChoices {
            accept,
            booleans: vec![],
        };
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for _ in 0..20 {
            drain_pending_trigger_events(&mut g, &mut queue);
            put_triggers_on_stack_with_dm(&mut g, &mut queue, &mut dm).unwrap();
            if g.stack.is_empty() {
                break;
            };
            resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        }
        let zone = g
            .find_object_by_stable_id(stable)
            .and_then(|id| g.object(id))
            .map(|o| format!("{:?}", o.zone));
        let pests = g
            .battlefield
            .iter()
            .filter(|id| {
                g.object(**id).is_some_and(|o| {
                    o.kind == ironsmith::object::ObjectKind::Token && o.name.contains("Pest")
                })
            })
            .count();
        rows.push(json!({"card":"Valentin, Dean of the Vein","scenario":{"accept_pay_two":accept,"opposing_creature":"nontoken","available_mana":mana},"expected":{"victim_zone":"Exile","pest_tokens":usize::from(accept && mana >= 2),"remaining_mana":if accept && mana >= 2 {mana-2} else {mana}},"actual":{"victim_zone":zone,"pest_tokens":pests,"remaining_mana":g.player(PlayerId(0)).unwrap().mana_pool.total()},"choice_trace":dm.booleans,"scope":"Canonical replacement card active; neutral destroy ability resolves through normal stack and trigger processing"}));
    }
    for r in &mut rows {
        r["status"] = json!(if r["expected"] == r["actual"] {
            "passed"
        } else {
            "semantic_mismatch"
        });
    }
    for row in &rows {
        assert_eq!(row["actual"], row["expected"], "{row:#}");
    }
}

#[test]
fn source_counter_gate_controls_activation_availability() {
    let def = load("Temple of Cyclical Time");
    for counters in [0, 1, 2] {
        let mut game = setup();
        let source = game.create_object_from_definition(&def, PlayerId(0), Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .counters
            .insert(CounterType::Time, counters);
        game.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Blue, 3);
        let offered = compute_legal_actions(&game, PlayerId(0)).unwrap().iter().any(|action|
   matches!(action, LegalAction::ActivateAbility { source: id, ability_index: 1 } if *id == source));
        assert_eq!(
            offered,
            counters == 0,
            "source has {counters} time counters"
        );
    }
}
#[test]
fn opponent_land_threshold_controls_activation_availability() {
    let def = load("Tectonic Edge");
    for count in [3, 4, 5] {
        let mut game = setup();
        let source = game.create_object_from_definition(&def, PlayerId(0), Zone::Battlefield);
        for _ in 0..count {
            game.create_object_from_definition(
                &filler("Nonbasic land", CardType::Land, 0, 0),
                PlayerId(1),
                Zone::Battlefield,
            );
        }
        game.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let offered = compute_legal_actions(&game, PlayerId(0)).unwrap().iter().any(|action|
   matches!(action, LegalAction::ActivateAbility { source: id, ability_index: 1 } if *id == source));
        assert_eq!(offered, count >= 4, "opponent has {count} lands");
    }
}

fn assert_card_rows(rows: Vec<serde_json::Value>, name: &str) {
    let selected: Vec<_> = rows.into_iter().filter(|row| row["card"] == name).collect();
    assert!(!selected.is_empty(), "No scenarios for {name}");
    let failures: Vec<_> = selected
        .into_iter()
        .filter(|row| row["actual"] != row["expected"])
        .collect();
    assert!(
        failures.is_empty(),
        "{name}: {}",
        serde_json::to_string_pretty(&failures).unwrap()
    );
}
#[test]
fn aggregate_tap_cost() {
    assert_card_rows(current_card_rows(), "Mossbridge Troll");
}
#[test]
fn bounded_library_search() {
    assert_card_rows(current_card_rows(), "Plargg, Dean of Chaos");
}
#[test]
fn loyalty_gate_and_library_remainder() {
    assert_card_rows(current_card_rows(), "Jace, Reality Sculptor");
}
#[test]
fn first_main_optional_mana() {
    assert_card_rows(trigger_and_condition_rows(), "Carpet of Flowers");
}
#[test]
fn impossible_optional_discard_or_sacrifice() {
    assert_card_rows(trigger_and_condition_rows(), "Osseous Sticktwister");
}
#[test]
fn target_each_graveyard() {
    assert_card_rows(trigger_and_condition_rows(), "Mysterious Stranger");
}

#[test]
fn loyalty_threshold_sums_only_matching_planeswalkers() {
    use ironsmith::Subtype;
    let def = load("Jace, Reality Sculptor");
    let mut game = setup();
    let source = game.create_object_from_definition(&def, PlayerId(0), Zone::Battlefield);
    game.object_mut(source)
        .unwrap()
        .counters
        .insert(CounterType::Loyalty, 10);
    let other_jace = CardDefinitionBuilder::new(CardId::new(), "Other Jace witness")
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Jace])
        .build();
    let matching = game.create_object_from_definition(&other_jace, PlayerId(0), Zone::Battlefield);
    let nonmatching = CardDefinitionBuilder::new(CardId::new(), "Other planeswalker witness")
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Chandra])
        .build();
    let ignored = game.create_object_from_definition(&nonmatching, PlayerId(0), Zone::Battlefield);
    game.object_mut(ignored)
        .unwrap()
        .counters
        .insert(CounterType::Loyalty, 100);
    for counters in [14, 15, 16] {
        game.object_mut(matching)
            .unwrap()
            .counters
            .insert(CounterType::Loyalty, counters);
        let offered = compute_legal_actions(&game, PlayerId(0)).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateAbility { source: id, ability_index: 2 } if *id == source));
        assert_eq!(
            offered,
            counters >= 15,
            "10 source loyalty + {counters} other Jace loyalty; ignore 100 on a different subtype"
        );
    }
}

#[path = "runtime_behavior_regressions/damage_batch.rs"]
mod damage_batch;
#[path = "runtime_behavior_regressions/sacrifice_cost.rs"]
mod sacrifice_cost;
