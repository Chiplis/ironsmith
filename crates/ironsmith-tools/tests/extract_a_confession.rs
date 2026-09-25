//! Extract a Confession: "As an additional cost to cast this spell, you may
//! collect evidence 6. Each opponent sacrifices a creature of their choice. If
//! evidence was collected, instead each opponent sacrifices a creature with the
//! greatest power among creatures they control."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Extract a Confession",
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

#[derive(Default)]
struct Choices {
    collect: bool,
    offered_evidence: bool,
    evidence_names: Vec<String>,
    /// (chooser, legal candidate names) for each sacrifice choice.
    sacrifice_candidates: Vec<(PlayerId, Vec<String>)>,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            self.offered_evidence = true;
            return if self.collect { vec![0] } else { Vec::new() };
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .take(ctx.min)
            .map(|option| option.index)
            .collect()
    }

    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal: Vec<_> = ctx.candidates.iter().filter(|c| c.legal).collect();
        if ctx.aggregate_constraint.is_some() {
            let picked: Vec<_> = legal
                .iter()
                .filter(|c| self.evidence_names.contains(&c.name))
                .map(|c| c.id)
                .collect();
            return picked;
        }
        let owner_is_creature_side = legal.iter().all(|c| {
            game.object(c.id)
                .is_some_and(|o| o.zone == Zone::Battlefield)
        });
        if owner_is_creature_side {
            self.sacrifice_candidates
                .push((ctx.player, legal.iter().map(|c| c.name.clone()).collect()));
        }
        legal.iter().take(ctx.min.max(1)).map(|c| c.id).collect()
    }
}

fn card(name: &str, types: Vec<CardType>, mana_value: u8) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types.clone())
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value,
        )]));
    if types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(mana_value as i32, 1));
    }
    builder.build()
}

struct Outcome {
    game: GameState,
    dm: Choices,
    evidence: Vec<ObjectId>,
}

/// Alice casts Extract a Confession against Bob and Cara, who each control a
/// 1-power creature and two 3-power creatures.
fn cast(collect: bool, graveyard_mvs: &[u8]) -> Result<Outcome, String> {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 2);
    let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
    let mut evidence = Vec::new();
    let mut evidence_names = Vec::new();
    for (i, mv) in graveyard_mvs.iter().enumerate() {
        let name = format!("Evidence {i}");
        evidence.push(game.create_object_from_definition(
            &card(&name, vec![CardType::Instant], *mv),
            alice,
            Zone::Graveyard,
        ));
        evidence_names.push(name);
    }
    for index in [1, 2] {
        let player = PlayerId::from_index(index);
        game.create_object_from_definition(
            &card("Small", vec![CardType::Creature], 1),
            player,
            Zone::Battlefield,
        );
        game.create_object_from_definition(
            &card("Big A", vec![CardType::Creature], 3),
            player,
            Zone::Battlefield,
        );
        game.create_object_from_definition(
            &card("Big B", vec![CardType::Creature], 3),
            player,
            Zone::Battlefield,
        );
    }

    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices {
        collect,
        evidence_names,
        ..Default::default()
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
    if game.stack.len() != 1 {
        return Err(format!("{result:?}"));
    }
    assert!(dm.offered_evidence);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    Ok(Outcome { game, dm, evidence })
}

fn creature_names(game: &GameState, player: PlayerId) -> Vec<String> {
    let mut names: Vec<_> = game
        .battlefield
        .iter()
        .filter(|id| game.current_controller(**id) == Some(player))
        .map(|id| game.object(*id).unwrap().name.to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn without_evidence_each_opponent_sacrifices_a_creature_of_their_choice() {
    let outcome = cast(false, &[3, 3]).unwrap();
    for index in [1, 2] {
        let player = PlayerId::from_index(index);
        assert_eq!(creature_names(&outcome.game, player).len(), 2);
    }
    assert_eq!(
        outcome.dm.sacrifice_candidates.len(),
        2,
        "each opponent chooses"
    );
    for (_, candidates) in &outcome.dm.sacrifice_candidates {
        assert_eq!(
            candidates.len(),
            3,
            "any creature may be sacrificed: {candidates:?}"
        );
    }
    for id in &outcome.evidence {
        assert_eq!(outcome.game.object(*id).unwrap().zone, Zone::Graveyard);
    }
}

#[test]
fn collected_evidence_forces_greatest_power_sacrifice_and_exiles_evidence() {
    let outcome = cast(true, &[3, 3]).unwrap();
    for index in [1, 2] {
        let player = PlayerId::from_index(index);
        let remaining = creature_names(&outcome.game, player);
        assert_eq!(remaining.len(), 2);
        assert!(
            remaining.contains(&"Small".to_string()),
            "a greatest-power creature was sacrificed"
        );
    }
    for (_, candidates) in &outcome.dm.sacrifice_candidates {
        assert!(
            candidates.iter().all(|name| name.starts_with("Big")),
            "only greatest-power creatures are legal: {candidates:?}"
        );
        assert_eq!(candidates.len(), 2, "tied creatures remain a choice");
    }
    let alice = PlayerId::from_index(0);
    assert!(
        outcome
            .game
            .player(alice)
            .unwrap()
            .graveyard
            .iter()
            .all(|id| { !outcome.evidence.contains(id) })
    );
    assert_eq!(
        outcome
            .game
            .exile
            .iter()
            .filter(|id| outcome
                .game
                .object(**id)
                .unwrap()
                .name
                .starts_with("Evidence"))
            .count(),
        2,
        "the evidence cards were exiled as the cost"
    );
    assert_eq!(outcome.game.player(alice).unwrap().mana_pool.total(), 0);
}

#[test]
fn evidence_below_six_cannot_be_collected() {
    // Total mana value 5 cannot pay collect evidence 6: the proposal is illegal.
    assert!(cast(true, &[3, 2]).is_err());
}
