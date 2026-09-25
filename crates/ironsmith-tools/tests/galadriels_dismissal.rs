//! Galadriel's Dismissal: "Kicker {2}{W}. Target creature phases out. If this
//! spell was kicked, each creature target player controls phases out
//! instead."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Galadriel's Dismissal",
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
    creature: ObjectId,
    player: PlayerId,
    requirement_kinds: Vec<&'static str>,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
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
        ctx.requirements
            .iter()
            .map(|requirement| {
                if requirement
                    .legal_targets
                    .contains(&Target::Player(self.player))
                {
                    self.requirement_kinds.push("player");
                    Target::Player(self.player)
                } else {
                    assert!(
                        requirement
                            .legal_targets
                            .contains(&Target::Object(self.creature))
                    );
                    self.requirement_kinds.push("creature");
                    Target::Object(self.creature)
                }
            })
            .collect()
    }
}

fn creature(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

/// Casts the spell targeting Bob's first creature (and Bob, when kicked).
/// Returns which of Bob's two creatures and Alice's creature phased out, and
/// the kinds of targets that were requested.
fn cast(kick: bool) -> ([bool; 3], Vec<&'static str>) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::White, 2);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
    let first = game.create_object_from_definition(&creature("Bob Bear"), bob, Zone::Battlefield);
    let second = game.create_object_from_definition(&creature("Bob Wolf"), bob, Zone::Battlefield);
    let mine =
        game.create_object_from_definition(&creature("Alice Bear"), alice, Zone::Battlefield);

    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices {
        kick,
        creature: first,
        player: bob,
        requirement_kinds: Vec::new(),
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
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "kick={kick} result={result:?}");
    assert_eq!(game.stack[0].optional_costs_paid.was_kicked(), kick);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    (
        [
            game.is_phased_out(first),
            game.is_phased_out(second),
            game.is_phased_out(mine),
        ],
        dm.requirement_kinds,
    )
}

#[test]
fn unkicked_phases_out_only_the_target_creature() {
    let (phased, kinds) = cast(false);
    assert_eq!(phased, [true, false, false]);
    assert_eq!(kinds, vec!["creature"]);
}

#[test]
fn kicked_phases_out_each_creature_the_target_player_controls_instead() {
    let (phased, kinds) = cast(true);
    assert_eq!(phased, [true, true, false]);
    assert!(
        kinds.contains(&"player"),
        "kicked targets a player: {kinds:?}"
    );
}
