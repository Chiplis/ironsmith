//! Monstrous Emergence: "As an additional cost to cast this spell, choose a
//! creature you control or reveal a creature card from your hand. Monstrous
//! Emergence deals damage equal to the power of the creature you chose or the
//! card you revealed to target creature."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::{CardId, ObjectId};
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Monstrous Emergence",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(snapshot.parse_status, ironsmith_tools::ParseStatus::StrictCompiled, "{snapshot:#?}");
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

/// Picks the cost option whose description mentions `option_word`, targets
/// `victim`, and chooses the first legal object.
struct Choices {
    option_word: &'static str,
    victim: ObjectId,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(confirm) = ctx.options.iter().find(|o| o.description == "Confirm payment") {
            return vec![confirm.index];
        }
        // The cost's two modes, in printed order: choose, then reveal.
        if ctx.options.len() == 2 && ctx.options.iter().all(|o| o.legal && o.description.is_empty()) {
            return vec![ctx.options[usize::from(self.option_word == "reveal")].index];
        }
        ctx.options.iter().filter(|o| o.legal).take(ctx.min.max(1)).map(|o| o.index).collect()
    }
    fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
        vec![Target::Object(self.victim)]
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).take(1).collect()
    }
}

fn creature(name: &str, power: i32) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, 10))
        .build()
}

/// Alice controls a 4-power creature and holds a 6-power creature card;
/// returns the damage marked on Bob's 1/10 after the spell resolves.
fn damage_with(option_word: &'static str) -> u32 {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.create_object_from_definition(&creature("Four Power", 4), alice, Zone::Battlefield);
    game.create_object_from_definition(&creature("Six Power", 6), alice, Zone::Hand);
    let victim = game.create_object_from_definition(&creature("Victim", 1), bob, Zone::Battlefield);
    let spell = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Hand,
    );
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Green, 6);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices { option_word, victim };
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
        result = ironsmith::game_loop::apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm);
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    game.damage_on(victim)
}

#[test]
fn choosing_a_creature_uses_its_power() {
    assert_eq!(damage_with("choose"), 4);
}

#[test]
fn revealing_a_creature_card_uses_its_power() {
    assert_eq!(damage_with("reveal"), 6);
}
