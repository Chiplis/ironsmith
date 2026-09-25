//! Vines of Vastwood: "Kicker {G}. Target creature can't be the target of
//! spells or abilities your opponents control this turn. If this spell was
//! kicked, that creature gets +4/+4 until end of turn."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effects::ResolvedTarget;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn load(name: &str) -> ironsmith::cards::CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Vines of Vastwood",
    )
    .unwrap()
    .remove(0);
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload);
    assert_eq!(snapshot.parse_status, ironsmith_tools::ParseStatus::StrictCompiled, "{snapshot:#?}");
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

struct Record(Vec<Target>);

impl DecisionMaker for Record {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.0 = ctx.requirements[0].legal_targets.clone();
        vec![self.0[0]]
    }
}

/// Alice resolves Vines on `owner`'s Bear; then `caster` casts Shock.
/// Returns whether the Bear was a legal Shock target.
fn shock_can_target_after_vines(owner: PlayerId, caster: PlayerId) -> bool {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = caster;
    game.turn.priority_player = Some(caster);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let bear = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        owner,
        Zone::Battlefield,
    );
    let vines_def = load("Vines of Vastwood");
    let vines = game.create_object_from_definition(&vines_def, alice, Zone::Stack);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(vines, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Object(bear)]);
    for effect in vines_def.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    game.refresh_continuous_state();
    game.update_cant_effects();
    let shock: ObjectId = game.create_object_from_definition(&load("Shock"), caster, Zone::Hand);
    game.player_mut(caster).unwrap().mana_pool.add(ManaSymbol::Red, 1);
    let action = compute_legal_actions(&game, caster)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == shock))
        .expect("Shock castable (players are legal targets)");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Record(Vec::new());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..8 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm);
    }
    dm.0.contains(&Target::Object(bear))
}

#[test]
fn your_opponents_cannot_target_it() {
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    assert!(!shock_can_target_after_vines(alice, bob));
}

#[test]
fn you_still_can() {
    let alice = PlayerId::from_index(0);
    assert!(shock_can_target_after_vines(alice, alice));
}

#[test]
fn opponents_are_relative_to_the_vines_caster_not_the_creatures_controller() {
    // Alice casts Vines on Bob's creature: Bob, her opponent, can't target it.
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    assert!(!shock_can_target_after_vines(bob, bob));
    assert!(shock_can_target_after_vines(bob, alice));
}
