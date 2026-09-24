//! Overload: "Kicker {2}. Destroy target artifact if its mana value is 2 or
//! less. If this spell was kicked, destroy that artifact if its mana value is 5
//! or less instead."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Overload",
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

struct Choices {
    kick: bool,
    target: ObjectId,
    offered_kicker: bool,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            self.offered_kicker = true;
            return if self.kick { vec![0] } else { Vec::new() };
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .take(ctx.min)
            .map(|option| option.index)
            .collect()
    }

    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert!(
            ctx.requirements[0]
                .legal_targets
                .contains(&Target::Object(self.target)),
            "the artifact must be a legal target regardless of its mana value"
        );
        vec![Target::Object(self.target)]
    }
}

fn artifact(mana_value: u32) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), &format!("Artifact MV {mana_value}"))
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_symbols(if mana_value == 0 {
            Vec::new()
        } else {
            vec![ManaSymbol::Generic(mana_value as u8)]
        }))
        .build()
}

/// Casts Overload from hand at `artifact MV`, returning whether it was destroyed
/// and the mana left in the pool.
fn cast_overload(kick: bool, mana_value: u32) -> (bool, u32) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 3);
    let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
    let target = game.create_object_from_definition(&artifact(mana_value), bob, Zone::Battlefield);
    let target_stable = game.object(target).unwrap().stable_id;

    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("Overload is castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices {
        kick,
        target,
        offered_kicker: false,
    };
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..32 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        assert!(!matches!(ctx, DecisionContext::Number(_)));
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert!(dm.offered_kicker, "kicker must be offered");
    assert_eq!(game.stack.len(), 1, "kick={kick} mv={mana_value} result={result:?}");
    assert_eq!(game.stack[0].optional_costs_paid.was_kicked(), kick);
    let mana_left = game.player(alice).unwrap().mana_pool.total();
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let zone = game
        .object(game.find_object_by_stable_id(target_stable).unwrap())
        .unwrap()
        .zone;
    (zone == Zone::Graveyard, mana_left)
}

#[test]
fn unkicked_destroys_only_mana_value_two_or_less() {
    for (mana_value, destroyed) in [(0, true), (2, true), (3, false), (5, false)] {
        let (was_destroyed, mana_left) = cast_overload(false, mana_value);
        assert_eq!(was_destroyed, destroyed, "unkicked mv={mana_value}");
        assert_eq!(mana_left, 2, "unkicked costs only R");
    }
}

#[test]
fn kicked_destroys_mana_value_five_or_less_instead() {
    for (mana_value, destroyed) in [(1, true), (2, true), (3, true), (5, true), (6, false)] {
        let (was_destroyed, mana_left) = cast_overload(true, mana_value);
        assert_eq!(was_destroyed, destroyed, "kicked mv={mana_value}");
        assert_eq!(mana_left, 0, "kicked costs R plus 2");
    }
}

