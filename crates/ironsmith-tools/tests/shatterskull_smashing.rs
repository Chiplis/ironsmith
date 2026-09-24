//! Shatterskull Smashing: "Shatterskull Smashing deals X damage divided as you
//! choose among up to two target creatures and/or planeswalkers. If X is 6 or
//! more, Shatterskull Smashing deals twice X damage divided as you choose among
//! them instead."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DistributeContext, NumberContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Shatterskull Smashing",
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

/// Picks X, both creatures as targets, and puts `first_share` of the total on
/// the first one. Records every distribution prompt as (total, stack size).
struct Caster {
    x: u32,
    targets: Vec<ObjectId>,
    first_share: u32,
    prompts: Vec<(u32, usize)>,
}

impl DecisionMaker for Caster {
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value { self.x } else { ctx.min }
    }

    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        for id in &self.targets {
            assert!(ctx.requirements[0].legal_targets.contains(&Target::Object(*id)));
        }
        self.targets.iter().map(|id| Target::Object(*id)).collect()
    }

    fn decide_distribute(&mut self, game: &GameState, ctx: &DistributeContext) -> Vec<(Target, u32)> {
        self.prompts.push((ctx.total, game.stack.len()));
        let first = self.first_share.min(ctx.total);
        let mut shares = vec![(Target::Object(self.targets[0]), first)];
        if let Some(second) = self.targets.get(1) {
            shares.push((Target::Object(*second), ctx.total - first));
        }
        shares
    }
}

fn creature(name: &str, toughness: i32) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, toughness))
        .build()
}

/// Bob controls two 1/20 walls; Alice casts Shatterskull Smashing for `x`.
/// Returns the damage marked on each wall and the distribution prompts.
fn cast(x: u32, both: bool, first_share: u32) -> (u32, u32, Vec<(u32, usize)>) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let a = game.create_object_from_definition(&creature("Wall A", 30), bob, Zone::Battlefield);
    let b = game.create_object_from_definition(&creature("Wall B", 30), bob, Zone::Battlefield);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, x + 2);
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Caster {
        x,
        targets: if both { vec![a, b] } else { vec![a] },
        first_share,
        prompts: Vec::new(),
    };
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
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0, "paid X + {{R}}{{R}}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    (game.damage_on(a), game.damage_on(b), dm.prompts)
}

#[test]
fn x_below_six_divides_x_as_announced_while_casting() {
    let (a, b, prompts) = cast(5, true, 2);
    assert_eq!((a, b), (2, 3));
    assert_eq!(prompts, vec![(5, 0)], "divided once, during casting (CR 601.2d)");
}

#[test]
fn x_six_or_more_divides_twice_x_among_the_same_targets() {
    let (a, b, prompts) = cast(6, true, 1);
    assert_eq!((a, b), (1, 11), "twice X = 12 split as announced; prompts {prompts:?}");
    assert_eq!(prompts, vec![(12, 0)], "the doubled total is announced while casting");
}

#[test]
fn a_single_target_takes_all_of_twice_x() {
    let (a, b, _) = cast(7, false, 14);
    assert_eq!((a, b), (14, 0));
}
