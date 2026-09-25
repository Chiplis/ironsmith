//! Regression tests for the targeting / ward / stack-object / trigger-framework
//! mechanics audit (ledger_targeting.md). Each `mod` covers one finding.
use std::collections::VecDeque;

use ironsmith::card::PowerToughness;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::{Phase, Step, Target};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

fn card(name: &str) -> CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

fn free() -> ManaCost {
    ManaCost::from_pips(vec![vec![ManaSymbol::Generic(0)]])
}

/// A card with custom rules text, compiled by the real parser.
fn custom(name: &str, types: Vec<CardType>, cost: ManaCost, text: &str) -> CardDefinition {
    let mut builder =
        ironsmith_registry::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(types.clone())
            .mana_cost(cost);
    if types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.parse_text(text).unwrap()
}

fn vanilla(name: &str, power: i32, toughness: i32) -> CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn filler(name: &str) -> CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn artifact(name: &str) -> CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

/// Two-player game in Alice's first main phase with a few library cards each.
fn new_game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = ALICE;
    game.turn.priority_player = Some(ALICE);
    game.turn.phase = Phase::FirstMain;
    for player in [ALICE, BOB] {
        for i in 0..6 {
            game.create_object_from_definition(
                &filler(&format!("Library {i}")),
                player,
                Zone::Library,
            );
        }
    }
    game
}

/// Scripted answers: queued target lists (else the first legal ones), one
/// boolean answer, and a record of what was asked.
#[derive(Default)]
struct Script {
    targets: VecDeque<Vec<Target>>,
    yes: bool,
    target_prompts: Vec<(PlayerId, Vec<Target>)>,
    option_prompts: Vec<(PlayerId, Vec<String>)>,
    boolean_prompts: Vec<PlayerId>,
}

impl Script {
    fn targeting(targets: Vec<Vec<Target>>) -> Self {
        Self {
            targets: targets.into(),
            ..Self::default()
        }
    }
}

impl DecisionMaker for Script {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.boolean_prompts.push(ctx.player);
        self.yes
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let wanted = ctx.min.max(1).min(ctx.max.unwrap_or(usize::MAX));
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .take(wanted)
            .collect()
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.option_prompts.push((
            ctx.player,
            ctx.options
                .iter()
                .map(|option| option.description.clone())
                .collect(),
        ));
        ctx.options
            .iter()
            .filter(|o| o.legal)
            .take(ctx.min)
            .map(|o| o.index)
            .collect()
    }

    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let legal: Vec<Target> = ctx
            .requirements
            .iter()
            .flat_map(|requirement| requirement.legal_targets.iter().copied())
            .collect();
        // Queued targets are handed out prompt by prompt: each requirement
        // takes, in requirement order, the queued targets legal for it.
        if let Some(queued) = self.targets.front_mut() {
            let mut chosen = Vec::new();
            for requirement in &ctx.requirements {
                let max = requirement.max_targets.unwrap_or(usize::MAX);
                let mut taken = 0;
                queued.retain(|target| {
                    if taken < max && requirement.legal_targets.contains(target) {
                        chosen.push(*target);
                        taken += 1;
                        false
                    } else {
                        true
                    }
                });
            }
            if !chosen.is_empty() {
                if queued.is_empty() {
                    self.targets.pop_front();
                }
                self.target_prompts.push((ctx.player, legal));
                return chosen;
            }
        }
        self.target_prompts.push((ctx.player, legal));
        ctx.requirements
            .iter()
            .flat_map(|requirement| {
                let count = requirement
                    .min_targets
                    .max(1)
                    .min(requirement.legal_targets.len());
                requirement.legal_targets.iter().take(count).copied()
            })
            .collect()
    }
}

fn find_cast(game: &GameState, player: PlayerId, spell: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, player).into_iter().find(
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell),
    )
}

/// `player` casts `spell` (answering its choices with `dm`), then the game
/// runs up to the next priority so triggered abilities (ward) are stacked.
fn cast(
    game: &mut GameState,
    queue: &mut TriggerQueue,
    player: PlayerId,
    spell: ObjectId,
    dm: &mut Script,
) {
    game.turn.priority_player = Some(player);
    let action = find_cast(game, player, spell).expect("the cast must be legal");
    let before = game.stack.len();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..24 {
        if game.stack.len() > before {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress =
            ironsmith::game_loop::apply_decision_context_with_dm(game, queue, &mut state, &ctx, dm)
                .unwrap();
    }
    assert!(game.stack.len() > before, "the spell must be on the stack");
    settle(game, queue, dm);
}

/// Advance to the next priority: SBAs, then pending triggers go on the stack.
fn settle(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut Script) {
    ironsmith::game_loop::advance_priority_with_dm(game, queue, dm).unwrap();
}

/// Everyone passes until the stack is empty.
fn resolve_all(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut Script) {
    ironsmith::game_loop::run_priority_loop_with(game, queue, dm).unwrap();
}

/// Resolve only the top object, then stack whatever triggered.
fn resolve_top(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut Script) {
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    settle(game, queue, dm);
}

fn life(game: &GameState, player: PlayerId) -> i32 {
    game.player(player).unwrap().life
}

fn zone_of_card(game: &GameState, stable: ironsmith::ids::StableId) -> Option<Zone> {
    game.find_object_by_stable_id(stable)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

fn plus_counters(game: &GameState, id: ObjectId) -> u32 {
    game.object(id)
        .and_then(|object| object.counters.get(&CounterType::PlusOnePlusOne).copied())
        .unwrap_or(0)
}

fn in_graveyard(game: &GameState, player: PlayerId, name: &str) -> bool {
    game.player(player)
        .unwrap()
        .graveyard
        .iter()
        .any(|id| game.object(*id).is_some_and(|o| o.name.to_string() == name))
}

fn on_battlefield(game: &GameState, name: &str) -> Option<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).is_some_and(|o| o.name.to_string() == name))
}

// ---------------------------------------------------------------------------

/// B1: hexproof and shroud work only on the battlefield (CR 113.6, 702.11b),
/// so a hexproof creature spell can be countered.
mod b1_hexproof_spell_can_be_countered {
    use super::*;

    #[test]
    fn counterspell_targets_a_hexproof_creature_spell() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        game.turn.active_player = BOB;
        let scout = game.create_object_from_definition(&card("Gladecover Scout"), BOB, Zone::Hand);
        let counterspell =
            game.create_object_from_definition(&card("Counterspell"), ALICE, Zone::Hand);
        game.player_mut(BOB)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        let scout_stable = game.object(scout).unwrap().stable_id;
        cast(&mut game, &mut queue, BOB, scout, &mut Script::default());
        let scout_spell = game.stack[0].object_id;
        let mut dm = Script::targeting(vec![vec![Target::Object(scout_spell)]]);
        cast(&mut game, &mut queue, ALICE, counterspell, &mut dm);
        assert!(
            dm.target_prompts[0]
                .1
                .contains(&Target::Object(scout_spell))
        );
        resolve_all(&mut game, &mut queue, &mut Script::default());
        assert_eq!(zone_of_card(&game, scout_stable), Some(Zone::Graveyard));
    }
}

/// B2: ward is an ability of a permanent (CR 702.21a, 113.6).
mod b2_ward_only_on_battlefield {
    use super::*;

    #[test]
    fn graveyard_ward_card_does_not_tax_a_graveyard_targeting_spell() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let terror =
            game.create_object_from_definition(&card("Tolarian Terror"), BOB, Zone::Graveyard);
        let stable = game.object(terror).unwrap().stable_id;
        let cremate = game.create_object_from_definition(&card("Cremate"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            cremate,
            &mut Script::targeting(vec![vec![Target::Object(terror)]]),
        );
        assert_eq!(game.stack.len(), 1, "no ward trigger from a graveyard card");
        resolve_all(&mut game, &mut queue, &mut Script::default());
        assert_eq!(zone_of_card(&game, stable), Some(Zone::Exile));
    }

    #[test]
    fn countering_a_ward_creature_spell_is_not_taxed() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        game.turn.active_player = BOB;
        let terror = game.create_object_from_definition(&card("Tolarian Terror"), BOB, Zone::Hand);
        let stable = game.object(terror).unwrap().stable_id;
        let counterspell =
            game.create_object_from_definition(&card("Counterspell"), ALICE, Zone::Hand);
        game.player_mut(BOB)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 7);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        cast(&mut game, &mut queue, BOB, terror, &mut Script::default());
        let spell = game.stack[0].object_id;
        cast(
            &mut game,
            &mut queue,
            ALICE,
            counterspell,
            &mut Script::targeting(vec![vec![Target::Object(spell)]]),
        );
        assert_eq!(game.stack.len(), 2, "no ward trigger from a spell");
        resolve_all(&mut game, &mut queue, &mut Script::default());
        assert_eq!(zone_of_card(&game, stable), Some(Zone::Graveyard));
        assert!(in_graveyard(&game, ALICE, "Counterspell"));
    }
}

fn ward_bear(text: &str) -> CardDefinition {
    custom(
        "Warded Bear",
        vec![CardType::Creature],
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)], vec![ManaSymbol::Green]]),
        text,
    )
}

/// B3: the ward counter is an ordinary counter, so "can't be countered" wins.
mod b3_ward_respects_cant_be_countered {
    use super::*;

    #[test]
    fn abrupt_decay_resolves_through_unpaid_ward() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let bear =
            game.create_object_from_definition(&ward_bear("Ward {2}"), BOB, Zone::Battlefield);
        let stable = game.object(bear).unwrap().stable_id;
        let decay = game.create_object_from_definition(&card("Abrupt Decay"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            decay,
            &mut Script::targeting(vec![vec![Target::Object(bear)]]),
        );
        assert_eq!(game.stack.len(), 2, "the ward trigger still triggers");
        resolve_all(&mut game, &mut queue, &mut Script::default());
        assert_eq!(zone_of_card(&game, stable), Some(Zone::Graveyard));
    }
}

/// B4: ward is a triggered ability that goes on the stack when the permanent
/// becomes a target, one per ward instance (CR 702.21a).
mod b4_ward_is_a_trigger {
    use super::*;

    #[test]
    fn unpaid_ward_trigger_counters_the_targeting_spell() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let terror =
            game.create_object_from_definition(&card("Tolarian Terror"), BOB, Zone::Battlefield);
        let shock = game.create_object_from_definition(&card("Shock"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            shock,
            &mut Script::targeting(vec![vec![Target::Object(terror)]]),
        );
        assert_eq!(game.stack.len(), 2);
        let ward = game.stack.last().unwrap();
        assert!(
            ward.is_ability,
            "ward is a triggered ability above the spell"
        );
        assert_eq!(ward.controller, BOB);
        resolve_all(&mut game, &mut queue, &mut Script::default());
        assert!(in_graveyard(&game, ALICE, "Shock"));
        assert_eq!(game.damage_on(terror), 0);
    }

    #[test]
    fn each_ward_instance_triggers() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let bear = game.create_object_from_definition(
            &ward_bear("Ward {1}\nWard—Pay 2 life."),
            BOB,
            Zone::Battlefield,
        );
        let shock = game.create_object_from_definition(&card("Shock"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            shock,
            &mut Script::targeting(vec![vec![Target::Object(bear)]]),
        );
        assert_eq!(game.stack.len(), 3, "two ward instances, two triggers");
    }
}

/// B5: a battle is cast at sorcery speed (CR 310.1, 307.1).
mod b5_battle_timing {
    use super::*;

    #[test]
    fn battle_is_not_castable_on_an_opponents_turn() {
        let mut game = new_game();
        let invasion =
            game.create_object_from_definition(&card("Invasion of Zendikar"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 4);
        assert!(
            find_cast(&game, ALICE, invasion).is_some(),
            "main phase, empty stack"
        );
        game.turn.active_player = BOB;
        game.turn.priority_player = Some(ALICE);
        assert!(find_cast(&game, ALICE, invasion).is_none(), "no flash");
    }
}

/// B6: a player with protection from everything can't be enchanted; Auras
/// attached to them are put into the graveyard (CR 702.16c).
mod b6_player_protection_drops_curses {
    use super::*;

    #[test]
    fn teferis_protection_removes_an_attached_curse() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let curse = game.create_object_from_definition(
            &card("Curse of Death's Hold"),
            BOB,
            Zone::Battlefield,
        );
        assert!(
            game.attach_object_to_target(curse, ironsmith::object::AttachmentTarget::Player(ALICE))
        );
        let protection =
            game.create_object_from_definition(&card("Teferi's Protection"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 3);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            protection,
            &mut Script::default(),
        );
        resolve_all(&mut game, &mut queue, &mut Script::default());
        assert!(in_graveyard(&game, BOB, "Curse of Death's Hold"));
    }
}

/// B8: an Aura put onto the battlefield without being cast can't be attached
/// to something with protection from it; with no legal object it stays put
/// (CR 303.4f, 303.4g).
mod b8_uncast_aura_respects_protection {
    use super::*;

    #[test]
    fn pacifism_stays_in_graveyard_when_only_a_pro_white_creature_exists() {
        let mut game = new_game();
        game.create_object_from_definition(&card("Black Knight"), BOB, Zone::Battlefield);
        let pacifism =
            game.create_object_from_definition(&card("Pacifism"), ALICE, Zone::Graveyard);
        let stable = game.object(pacifism).unwrap().stable_id;
        let _ = game.move_object_with_etb_processing_with_dm(
            pacifism,
            Zone::Battlefield,
            &mut Script::default(),
        );
        assert_eq!(zone_of_card(&game, stable), Some(Zone::Graveyard));
    }

    #[test]
    fn pacifism_attaches_to_the_creature_without_protection() {
        let mut game = new_game();
        game.create_object_from_definition(&card("Black Knight"), BOB, Zone::Battlefield);
        let bears = game.create_object_from_definition(
            &vanilla("Grizzly Bears", 2, 2),
            BOB,
            Zone::Battlefield,
        );
        let pacifism =
            game.create_object_from_definition(&card("Pacifism"), ALICE, Zone::Graveyard);
        let stable = game.object(pacifism).unwrap().stable_id;
        let _ = game.move_object_with_etb_processing_with_dm(
            pacifism,
            Zone::Battlefield,
            &mut Script::default(),
        );
        let aura = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(aura).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.object(aura).unwrap().attached_to,
            Some(ironsmith::object::AttachmentTarget::Object(bears))
        );
    }
}

/// B9: "as though it didn't have hexproof" also covers hexproof from a
/// quality (CR 702.11e).
mod b9_hexproof_from_permission {
    use super::*;

    fn disfigure_can_target_harbinger(spotlight: bool) -> bool {
        let mut game = new_game();
        if spotlight {
            game.create_object_from_definition(
                &card("Glaring Spotlight"),
                ALICE,
                Zone::Battlefield,
            );
        }
        game.create_object_from_definition(&card("Garruk's Harbinger"), BOB, Zone::Battlefield);
        let disfigure = game.create_object_from_definition(&card("Disfigure"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        game.refresh_continuous_state();
        find_cast(&game, ALICE, disfigure).is_some()
    }

    #[test]
    fn spotlight_lets_a_black_spell_target_hexproof_from_black() {
        assert!(!disfigure_can_target_harbinger(false));
        assert!(disfigure_can_target_harbinger(true));
    }
}

/// E1-03: if a fighter is an illegal target, neither creature fights
/// (CR 701.14b); no other creature is picked instead.
mod e1_03_fight_with_illegal_target {
    use super::*;

    #[test]
    fn prey_upon_does_nothing_when_its_target_gains_hexproof() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let mine = game.create_object_from_definition(
            &vanilla("Alice Bear", 3, 3),
            ALICE,
            Zone::Battlefield,
        );
        let targeted = game.create_object_from_definition(
            &vanilla("Bob Target", 2, 2),
            BOB,
            Zone::Battlefield,
        );
        let other =
            game.create_object_from_definition(&vanilla("Bob Other", 2, 2), BOB, Zone::Battlefield);
        let prey = game.create_object_from_definition(&card("Prey Upon"), ALICE, Zone::Hand);
        let defense =
            game.create_object_from_definition(&card("Blossoming Defense"), BOB, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        game.player_mut(BOB)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            prey,
            &mut Script::targeting(vec![vec![Target::Object(mine), Target::Object(targeted)]]),
        );
        cast(
            &mut game,
            &mut queue,
            BOB,
            defense,
            &mut Script::targeting(vec![vec![Target::Object(targeted)]]),
        );
        resolve_all(&mut game, &mut queue, &mut Script::default());
        for id in [mine, targeted, other] {
            assert_eq!(
                game.damage_on(id),
                0,
                "{:?}",
                game.object(id).map(|o| o.name.to_string())
            );
        }
    }
}

/// E1-06: every ability on the stack is its own object with its own target
/// identity (CR 113.1a, 701.6a).
mod e1_06_stack_ability_identity {
    use super::*;

    #[test]
    fn stifle_counters_a_storm_trigger_above_its_spell() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let first = game.create_object_from_definition(&card("Shock"), ALICE, Zone::Hand);
        let grapeshot = game.create_object_from_definition(&card("Grapeshot"), ALICE, Zone::Hand);
        let stifle = game.create_object_from_definition(&card("Stifle"), BOB, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 3);
        game.player_mut(BOB)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            first,
            &mut Script::targeting(vec![vec![Target::Player(BOB)]]),
        );
        resolve_all(&mut game, &mut queue, &mut Script::default());
        game.turn.priority_player = Some(ALICE);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            grapeshot,
            &mut Script::targeting(vec![vec![Target::Player(BOB)]]),
        );
        assert_eq!(game.stack.len(), 2, "Grapeshot and its storm trigger");
        let storm = game.stack[1].clone();
        assert!(storm.is_ability);
        let storm_id = storm.ability_id.expect("an ability has its own stack id");
        assert_ne!(storm_id, storm.object_id);
        let mut dm = Script::targeting(vec![vec![Target::Object(storm_id)]]);
        cast(&mut game, &mut queue, BOB, stifle, &mut dm);
        assert!(dm.target_prompts[0].1.contains(&Target::Object(storm_id)));
        resolve_all(
            &mut game,
            &mut queue,
            &mut Script::targeting(vec![vec![Target::Player(BOB)]; 4]),
        );
        // Shock (2) + Grapeshot itself (1); no storm copy.
        assert_eq!(life(&game, BOB), 17);
    }

    #[test]
    fn stifle_counters_exactly_the_chosen_one_of_two_abilities_from_one_source() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        game.create_object_from_definition(&card("Soul Warden"), ALICE, Zone::Battlefield);
        let tokens = game.create_object_from_definition(
            &custom(
                "Two Soldiers",
                vec![CardType::Sorcery],
                free(),
                "Create a 1/1 white Soldier creature token. Create a 1/1 white Soldier creature token.",
            ),
            ALICE,
            Zone::Hand,
        );
        let stifle = game.create_object_from_definition(&card("Stifle"), BOB, Zone::Hand);
        game.player_mut(BOB)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        cast(&mut game, &mut queue, ALICE, tokens, &mut Script::default());
        resolve_top(&mut game, &mut queue, &mut Script::default());
        assert_eq!(game.stack.len(), 2, "one Soul Warden trigger per token");
        let bottom = game.stack[0].ability_id.unwrap();
        let top = game.stack[1].ability_id.unwrap();
        assert_ne!(bottom, top);
        let mut dm = Script::targeting(vec![vec![Target::Object(top)]]);
        cast(&mut game, &mut queue, BOB, stifle, &mut dm);
        let offered = &dm.target_prompts[0].1;
        assert!(
            offered.contains(&Target::Object(top)) && offered.contains(&Target::Object(bottom))
        );
        resolve_top(&mut game, &mut queue, &mut Script::default());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(
            game.stack[0].ability_id,
            Some(bottom),
            "the other ability remains"
        );
        resolve_all(&mut game, &mut queue, &mut Script::default());
        assert_eq!(life(&game, ALICE), 21);
    }
}

/// E1-08: exchange targets are rechecked against their own requirements; if
/// one is illegal, no part of the exchange happens (CR 608.2b, 701.12a).
mod e1_08_exchange_rechecks_targets {
    use super::*;

    #[test]
    fn switcheroo_does_nothing_when_one_target_gains_hexproof() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let mine = game.create_object_from_definition(
            &vanilla("Alice Bear", 2, 2),
            ALICE,
            Zone::Battlefield,
        );
        let theirs =
            game.create_object_from_definition(&vanilla("Bob Bear", 2, 2), BOB, Zone::Battlefield);
        let switcheroo = game.create_object_from_definition(&card("Switcheroo"), ALICE, Zone::Hand);
        let defense =
            game.create_object_from_definition(&card("Blossoming Defense"), BOB, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 5);
        game.player_mut(BOB)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            switcheroo,
            &mut Script::targeting(vec![vec![Target::Object(mine), Target::Object(theirs)]]),
        );
        cast(
            &mut game,
            &mut queue,
            BOB,
            defense,
            &mut Script::targeting(vec![vec![Target::Object(theirs)]]),
        );
        resolve_all(&mut game, &mut queue, &mut Script::default());
        assert_eq!(game.current_controller(mine), Some(ALICE));
        assert_eq!(game.current_controller(theirs), Some(BOB));
    }
}

/// C4: the controller of a permanent entering under another player's control
/// orders its ETB replacement effects (CR 616.1, 110.2a).
mod c4_replacement_order_for_stolen_entry {
    use super::*;

    #[test]
    fn reanimating_player_orders_counter_replacements() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        game.create_object_from_definition(&card("Hardened Scales"), ALICE, Zone::Battlefield);
        game.create_object_from_definition(&card("Doubling Season"), ALICE, Zone::Battlefield);
        let feeder =
            game.create_object_from_definition(&card("Spike Feeder"), BOB, Zone::Graveyard);
        let stable = game.object(feeder).unwrap().stable_id;
        let reanimate = game.create_object_from_definition(&card("Reanimate"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        cast(
            &mut game,
            &mut queue,
            ALICE,
            reanimate,
            &mut Script::targeting(vec![vec![Target::Object(feeder)]]),
        );
        let mut dm = Script::default();
        resolve_all(&mut game, &mut queue, &mut dm);
        let feeder = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.current_controller(feeder), Some(ALICE));
        let order_prompts: Vec<_> = dm
            .option_prompts
            .iter()
            .filter(|(_, options)| {
                options
                    .iter()
                    .any(|o| o.contains("Hardened Scales") || o.contains("Doubling Season"))
            })
            .collect();
        assert!(!order_prompts.is_empty(), "{:?}", dm.option_prompts);
        assert!(
            order_prompts.iter().all(|(player, _)| *player == ALICE),
            "{order_prompts:?}"
        );
    }
}

/// C8: the legend rule is applied simultaneously with the other SBAs of the
/// same check (CR 704.3), so the leaving legend still sees the other death.
mod c8_legend_rule_simultaneous {
    use super::*;

    #[test]
    fn legend_leaving_by_legend_rule_sees_a_simultaneous_death() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let konrad = card("Syr Konrad, the Grim");
        let first = game.create_object_from_definition(&konrad, ALICE, Zone::Battlefield);
        let second = game.create_object_from_definition(&konrad, ALICE, Zone::Battlefield);
        let victim = game.create_object_from_definition(
            &vanilla("Doomed Bear", 2, 2),
            BOB,
            Zone::Battlefield,
        );
        game.mark_damage(victim, 2);
        let mut dm = Script::default();
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        assert!(game.object(victim).is_none());
        let kept = [first, second]
            .into_iter()
            .filter(|id| game.object(*id).is_some())
            .count();
        assert_eq!(kept, 1);
        // Kept Konrad: the bear and the other Konrad died. Departed Konrad:
        // the bear died simultaneously with it (look-back, CR 603.10a).
        let konrad_triggers = queue.entries.len();
        assert_eq!(konrad_triggers, 3, "{:?}", queue.entries.len());
    }
}

/// T1: a delayed trigger doesn't follow an object that left and came back as
/// a new object (CR 603.7c, 400.7).
mod t1_delayed_trigger_incarnation {
    use super::*;

    fn end_step(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut Script) {
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::End);
        game.turn.priority_player = Some(ALICE);
        ironsmith::game_loop::generate_and_queue_step_triggers(game, queue);
        resolve_all(game, queue, dm);
    }

    fn breach(blink: bool) -> Option<Zone> {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let bear =
            game.create_object_from_definition(&vanilla("Breach Bear", 2, 2), ALICE, Zone::Hand);
        let stable = game.object(bear).unwrap().stable_id;
        let breach =
            game.create_object_from_definition(&card("Through the Breach"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 5);
        let mut dm = Script {
            yes: true,
            ..Script::default()
        };
        cast(&mut game, &mut queue, ALICE, breach, &mut dm);
        resolve_all(&mut game, &mut queue, &mut dm);
        let entered = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        if blink {
            let exiled = game.move_object_by_effect(entered, Zone::Exile).unwrap();
            game.move_object_with_etb_processing(exiled, Zone::Battlefield)
                .unwrap();
        }
        end_step(&mut game, &mut queue, &mut dm);
        zone_of_card(&game, stable)
    }

    #[test]
    fn breached_creature_is_sacrificed_at_end_step() {
        assert_eq!(breach(false), Some(Zone::Graveyard));
    }

    #[test]
    fn a_blinked_creature_is_a_new_object_and_stays() {
        assert_eq!(breach(true), Some(Zone::Battlefield));
    }
}

/// T2: a state trigger triggers again once its instance has left the stack
/// while the condition still holds (CR 603.8).
mod t2_state_trigger_retriggers {
    use super::*;

    #[test]
    fn stifled_crocodile_trigger_triggers_again() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        game.create_object_from_definition(&card("Emperor Crocodile"), ALICE, Zone::Battlefield);
        let stifle = game.create_object_from_definition(&card("Stifle"), BOB, Zone::Hand);
        game.player_mut(BOB)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        let mut dm = Script::default();
        settle(&mut game, &mut queue, &mut dm);
        assert_eq!(game.stack.len(), 1, "no other creatures: the state trigger");
        settle(&mut game, &mut queue, &mut dm);
        assert_eq!(
            game.stack.len(),
            1,
            "no second instance while one is on the stack"
        );
        let trigger = game.stack[0].ability_id.unwrap();
        cast(
            &mut game,
            &mut queue,
            BOB,
            stifle,
            &mut Script::targeting(vec![vec![Target::Object(trigger)]]),
        );
        resolve_top(&mut game, &mut queue, &mut dm);
        assert_eq!(game.stack.len(), 1, "it triggered again");
        assert_ne!(game.stack[0].ability_id, Some(trigger));
        resolve_all(&mut game, &mut queue, &mut dm);
        assert!(in_graveyard(&game, ALICE, "Emperor Crocodile"));
    }
}

/// T3: a reflexive triggered ability is put on the stack the next time a
/// player would receive priority, with targets chosen then (CR 603.12, 603.3).
mod t3_reflexive_trigger_uses_the_stack {
    use super::*;

    #[test]
    fn manticore_reflexive_damage_waits_on_the_stack() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let fodder = game.create_object_from_definition(
            &vanilla("Big Fodder", 4, 4),
            ALICE,
            Zone::Battlefield,
        );
        let manticore =
            game.create_object_from_definition(&card("Heart-Piercer Manticore"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 4);
        let mut dm = Script {
            yes: true,
            ..Script::default()
        };
        dm.targets.push_back(vec![Target::Player(BOB)]);
        cast(&mut game, &mut queue, ALICE, manticore, &mut dm);
        resolve_top(&mut game, &mut queue, &mut dm); // the creature spell
        assert_eq!(game.stack.len(), 1, "the ETB trigger");
        assert!(
            dm.target_prompts.is_empty(),
            "the ETB trigger has no targets"
        );
        resolve_top(&mut game, &mut queue, &mut dm); // sacrifice: reflexive trigger
        assert!(game.object(fodder).is_none(), "the fodder was sacrificed");
        assert_eq!(
            game.stack.len(),
            1,
            "the reflexive trigger is its own stack object"
        );
        assert!(game.stack[0].is_ability);
        assert_eq!(life(&game, BOB), 20, "it has not resolved yet");
        assert_eq!(
            dm.target_prompts.len(),
            1,
            "targets were chosen as it was put on the stack"
        );
        assert!(!dm.target_prompts[0].1.contains(&Target::Object(fodder)));
        resolve_all(&mut game, &mut queue, &mut dm);
        assert_eq!(life(&game, BOB), 16);
    }
}

/// T4: an intervening-if condition is checked when the event happens and on
/// resolution, not again when the ability is put on the stack (CR 603.4).
mod t4_intervening_if_not_rechecked_when_stacked {
    use super::*;

    #[test]
    fn trigger_goes_on_the_stack_after_the_condition_became_false() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        game.create_object_from_definition(
            &custom(
                "Lucky Watcher",
                vec![CardType::Enchantment],
                free(),
                "Whenever a creature you control enters, if you have 20 or more life, draw a card.",
            ),
            ALICE,
            Zone::Battlefield,
        );
        let spell = game.create_object_from_definition(
            &custom(
                "Costly Soldier",
                vec![CardType::Sorcery],
                free(),
                "Create a 1/1 white Soldier creature token. You lose 5 life.",
            ),
            ALICE,
            Zone::Hand,
        );
        let mut dm = Script::default();
        cast(&mut game, &mut queue, ALICE, spell, &mut dm);
        resolve_top(&mut game, &mut queue, &mut dm);
        assert_eq!(life(&game, ALICE), 15);
        assert_eq!(game.stack.len(), 1, "the trigger was put on the stack");
        // A response makes the condition true again before it resolves.
        game.player_mut(ALICE).unwrap().life = 20;
        let hand = game.player(ALICE).unwrap().hand.len();
        resolve_all(&mut game, &mut queue, &mut dm);
        assert_eq!(game.player(ALICE).unwrap().hand.len(), hand + 1);
    }
}

/// E2-6: "when this enters, it explores" uses last known information once the
/// creature has left; the new object isn't affected (CR 701.44c, 400.7).
mod e2_6_explore_after_leaving {
    use super::*;

    #[test]
    fn jadelight_killed_in_response_gets_no_counters_in_the_graveyard() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let ranger =
            game.create_object_from_definition(&card("Jadelight Ranger"), ALICE, Zone::Hand);
        let stable = game.object(ranger).unwrap().stable_id;
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 3);
        let mut dm = Script::default();
        cast(&mut game, &mut queue, ALICE, ranger, &mut dm);
        resolve_top(&mut game, &mut queue, &mut dm);
        assert_eq!(game.stack.len(), 1, "the explore trigger");
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        let card_in_graveyard = game
            .move_object_by_effect(permanent, Zone::Graveyard)
            .unwrap();
        resolve_all(&mut game, &mut queue, &mut dm);
        assert_eq!(
            game.object(card_in_graveyard).map(|o| o.zone),
            Some(Zone::Graveyard)
        );
        assert_eq!(plus_counters(&game, card_in_graveyard), 0);
    }
}

/// E2-7: taking the initiative triggers the inherent "venture into Undercity"
/// ability, including when the initiative passes because a player left
/// (CR 725.2, 725.4).
mod e2_7_initiative_venture_trigger {
    use super::*;

    /// A two-room stand-in for the Undercity dungeon card.
    fn register_undercity() {
        let mut definition =
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Undercity")
                .card_types(vec![CardType::Dungeon])
                .build();
        for (room, leads_to) in [
            ("Secret Entrance", vec!["Forge".to_string()]),
            ("Forge", Vec::new()),
        ] {
            definition
                .abilities
                .push(ironsmith::ability::Ability::triggered(
                    ironsmith::triggers::Trigger::dungeon_room(room, leads_to),
                    vec![ironsmith::Effect::gain_life(1)],
                ));
        }
        ironsmith::dungeon::register_dungeon_definition(&definition).unwrap();
    }

    #[test]
    fn taking_the_initiative_puts_a_venture_trigger_on_the_stack() {
        register_undercity();
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let sneak = game.create_object_from_definition(&card("Aarakocra Sneak"), ALICE, Zone::Hand);
        game.player_mut(ALICE)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 4);
        let mut dm = Script::default();
        cast(&mut game, &mut queue, ALICE, sneak, &mut dm);
        resolve_top(&mut game, &mut queue, &mut dm); // creature
        resolve_top(&mut game, &mut queue, &mut dm); // take the initiative
        assert!(game.has_initiative(ALICE));
        assert!(
            game.active_dungeon(ALICE).is_none(),
            "the venture has not happened yet"
        );
        assert_eq!(game.stack.len(), 1, "the venture trigger");
        resolve_all(&mut game, &mut queue, &mut dm);
        assert_eq!(
            game.active_dungeon(ALICE).map(|d| d.dungeon_name.as_str()),
            Some("Undercity")
        );
    }

    #[test]
    fn initiative_passed_by_a_leaving_player_ventures() {
        register_undercity();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        game.turn.turn_number = 3;
        game.turn.active_player = ALICE;
        game.turn.priority_player = Some(ALICE);
        game.turn.phase = Phase::FirstMain;
        let mut queue = TriggerQueue::new();
        game.set_initiative(Some(BOB));
        assert!(game.concede_game(BOB));
        let mut dm = Script::default();
        settle(&mut game, &mut queue, &mut dm);
        assert!(
            game.has_initiative(ALICE),
            "the active player takes the initiative"
        );
        resolve_all(&mut game, &mut queue, &mut dm);
        assert_eq!(
            game.active_dungeon(ALICE).map(|d| d.dungeon_name.as_str()),
            Some("Undercity")
        );
    }
}

/// D1-09: an ability triggers the moment its event happens (CR 603.2), so only
/// permanents on the battlefield right then can trigger, and a watcher that
/// leaves later in the same resolution still triggers.
mod d1_09_per_event_trigger_matching {
    use super::*;

    fn soul_warden_spell(game: &mut GameState, text: &str) -> ObjectId {
        game.create_object_from_definition(
            &custom("Two-Step Spell", vec![CardType::Sorcery], free(), text),
            ALICE,
            Zone::Hand,
        )
    }

    #[test]
    fn a_watcher_arriving_later_does_not_see_an_earlier_entry() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let warden =
            game.create_object_from_definition(&card("Soul Warden"), ALICE, Zone::Graveyard);
        let spell = soul_warden_spell(
            &mut game,
            "Create a 1/1 white Soldier creature token. Return target creature card from your graveyard to the battlefield.",
        );
        let mut dm = Script::targeting(vec![vec![Target::Object(warden)]]);
        cast(&mut game, &mut queue, ALICE, spell, &mut dm);
        resolve_all(&mut game, &mut queue, &mut dm);
        assert!(on_battlefield(&game, "Soul Warden").is_some());
        assert_eq!(life(&game, ALICE), 20);
    }

    #[test]
    fn a_watcher_leaving_later_still_sees_an_earlier_entry() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        let warden =
            game.create_object_from_definition(&card("Soul Warden"), ALICE, Zone::Battlefield);
        let spell = soul_warden_spell(
            &mut game,
            "Create a 1/1 white Soldier creature token. Then destroy target creature.",
        );
        let mut dm = Script::targeting(vec![vec![Target::Object(warden)]]);
        cast(&mut game, &mut queue, ALICE, spell, &mut dm);
        resolve_all(&mut game, &mut queue, &mut dm);
        assert!(in_graveyard(&game, ALICE, "Soul Warden"));
        assert_eq!(life(&game, ALICE), 21);
    }

    /// Round 2: events an effect reports in its result from a step nested in a
    /// larger instruction are matched at that step, like queued events.
    #[test]
    fn nested_scry_is_matched_before_the_next_nested_step() {
        let mut game = new_game();
        let mut queue = TriggerQueue::new();
        game.create_object_from_definition(&artifact("Trinket"), ALICE, Zone::Battlefield);
        let watcher = game.create_object_from_definition(
            &custom(
                "Scry Watcher",
                vec![CardType::Creature],
                free(),
                "Whenever you scry, you gain 1 life.",
            ),
            ALICE,
            Zone::Graveyard,
        );
        let spell = soul_warden_spell(
            &mut game,
            "If you control an artifact, scry 1, then return target creature card from your graveyard to the battlefield.",
        );
        let mut dm = Script::targeting(vec![vec![Target::Object(watcher)]]);
        cast(&mut game, &mut queue, ALICE, spell, &mut dm);
        resolve_all(&mut game, &mut queue, &mut dm);
        assert!(on_battlefield(&game, "Scry Watcher").is_some());
        assert_eq!(
            life(&game, ALICE),
            20,
            "the watcher wasn't there when Alice scried"
        );
    }
}
