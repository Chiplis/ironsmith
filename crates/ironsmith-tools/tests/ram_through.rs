//! Ram Through: "Target creature you control deals damage equal to its power
//! to target creature you don't control. If the creature you control has
//! trample, excess damage is dealt to that creature's controller instead."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::{CardId, StableId};
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Ram Through",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

struct Aim(Vec<ObjectId>);

impl DecisionMaker for Aim {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        ctx.requirements
            .iter()
            .zip(&self.0)
            .map(|(_, id)| Target::Object(*id))
            .collect()
    }
}

fn creature(
    name: &str,
    power: i32,
    toughness: i32,
    trample: bool,
) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness));
    if trample {
        builder = builder.trample();
    }
    builder.build()
}

/// Alice's 5/5 (with or without trample) rams Bob's 2/2. Returns the damage
/// marked on the 2/2 (or where it went) and Bob's life.
fn ram(trample: bool) -> (Zone, i32, u32) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let rammer = game.create_object_from_definition(
        &creature("Rammer", 5, 5, trample),
        alice,
        Zone::Battlefield,
    );
    let victim = game.create_object_from_definition(
        &creature("Victim", 2, 2, false),
        bob,
        Zone::Battlefield,
    );
    let victim_stable: StableId = game.object(victim).unwrap().stable_id;
    let spell = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Hand,
    );
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 1);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Aim(vec![rammer, victim]);
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let id = game.find_object_by_stable_id(victim_stable).unwrap();
    let marked = game.damage_on(id);
    (
        game.object(id).unwrap().zone,
        game.player(bob).unwrap().life,
        marked,
    )
}

#[test]
fn trample_sends_excess_damage_to_the_controller() {
    let (_, life, marked) = ram(true);
    assert_eq!(marked, 2, "the 2/2 is dealt only lethal damage");
    assert_eq!(life, 17, "the other 3 goes to Bob");
}

#[test]
fn without_trample_all_damage_goes_to_the_creature() {
    let (_, life, marked) = ram(false);
    assert_eq!(marked, 5);
    assert_eq!(life, 20);
}
