//! Deem Inferior: "This spell costs {1} less to cast for each card you've
//! drawn this turn. The owner of target nonland permanent puts it into their
//! library second from the top or on the bottom."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::{CardId, StableId};
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Deem Inferior",
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

/// Targets `target`; when asked where the card goes, records who chose and
/// picks `bottom` or the top-side option.
struct Choices {
    target: ObjectId,
    bottom: bool,
    chooser: Option<PlayerId>,
}

impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
        vec![Target::Object(self.target)]
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx
            .options
            .iter()
            .any(|option| option.description.contains("Bottom of library"))
        {
            self.chooser = Some(ctx.player);
            return vec![if self.bottom { 1 } else { 0 }];
        }
        ctx.options
            .iter()
            .filter(|o| o.legal)
            .take(ctx.min)
            .map(|o| o.index)
            .collect()
    }
}

fn filler(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

/// Casts Deem Inferior on Bob's creature after Alice drew `drawn` cards this
/// turn, with `blue_mana` blue mana available. Returns Bob's library (top
/// first) by name and who chose the position, or None if not castable.
fn cast(drawn: u32, mana: u32, bottom: bool) -> Option<(Vec<String>, Option<PlayerId>, StableId)> {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    for i in 0..5 {
        game.create_object_from_definition(
            &filler(&format!("Alice Card {i}")),
            alice,
            Zone::Library,
        );
    }
    // Bob's library bottom-first: Bottom, Middle, Top.
    for name in ["Bob Bottom", "Bob Middle", "Bob Top"] {
        game.create_object_from_definition(&filler(name), bob, Zone::Library);
    }
    let bear = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Bob Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let bear_stable = game.object(bear).unwrap().stable_id;
    if drawn > 0 {
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut ctx = ironsmith::effects::EffectContext::new(bear, alice, &mut dm);
        ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::Effect::draw(drawn as i32),
            &mut ctx,
        )
        .unwrap();
    }
    let spell = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Hand,
    );
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, mana);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))?;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices {
        target: bear,
        bottom,
        chooser: None,
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
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let library: Vec<String> = game
        .player(bob)
        .unwrap()
        .library
        .iter()
        .rev()
        .map(|id| game.object(*id).unwrap().name.to_string())
        .collect();
    Some((library, dm.chooser, bear_stable))
}

#[test]
fn owner_can_put_it_second_from_the_top() {
    let (library, chooser, _) = cast(0, 4, false).expect("castable for {3}{U}");
    assert_eq!(chooser, Some(PlayerId::from_index(1)), "the owner chooses");
    assert_eq!(
        library[..2],
        ["Bob Top".to_string(), "Bob Bear".to_string()]
    );
}

#[test]
fn owner_can_put_it_on_the_bottom() {
    let (library, _, _) = cast(0, 4, true).expect("castable");
    assert_eq!(library.last().map(String::as_str), Some("Bob Bear"));
}

#[test]
fn each_card_drawn_this_turn_reduces_the_cost() {
    assert!(cast(0, 2, false).is_none(), "{{3}}{{U}} needs four mana");
    assert!(cast(2, 2, false).is_some(), "two draws make it {{1}}{{U}}");
}
