//! UNVALIDATED implementation-first coverage (cf8 p09): per-player named
//! choices. "For each player, choose friend or foe. Each friend …. Each foe
//! …." and "each opponent chooses money, friends, or secrets. For each player
//! who chose money, …" make one named choice per player (not a vote,
//! CR 701.38; made in APNAP order, CR 101.4) and restrict each following
//! participant statement to the players with that option.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::effects::{ChoosePlayerOptionEffect, PlayerOptionChooser, player_option_choice_tag};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};

#[path = "p09_common/mod.rs"]
mod common;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn rows() -> Vec<serde_json::Value> {
    common::rows(include_str!("../../../fixtures/player_option_choices.json.fixture"))
}

fn option_reads(definition: &CardDefinition, option: &str) -> bool {
    let tag = player_option_choice_tag(option);
    let needle = format!("{:?}", PlayerFilter::TaggedPlayer(tag));
    common::all_effects(definition)
        .iter()
        .any(|effect| format!("{effect:?}").contains(&needle))
}

#[test]
fn friend_or_foe_cards_choose_per_player_and_restrict_each_statement() {
    let rows = rows();
    for name in [
        "Khorvath's Fury",
        "Pir's Whim",
        "Regna's Sanction",
        "Virtus's Maneuver",
        "Zndrsplt's Judgment",
    ] {
        for definition in common::definitions(common::row(&rows, name)) {
            let effects = common::all_effects(&definition);
            let choice = effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChoosePlayerOptionEffect>())
                .unwrap_or_else(|| panic!("{name}: missing per-player choice"));
            assert_eq!(choice.chooser, PlayerOptionChooser::Controller, "{name}");
            assert_eq!(choice.participants, PlayerFilter::Any, "{name}");
            assert_eq!(choice.options, vec!["friend".to_string(), "foe".to_string()], "{name}");
            assert!(option_reads(&definition, "friend"), "{name}: friends unread");
            assert!(option_reads(&definition, "foe"), "{name}: foes unread");
        }
    }
}

#[test]
fn master_of_ceremonies_opponents_choose_for_themselves() {
    let rows = rows();
    let name = "Master of Ceremonies";
    for definition in common::definitions(common::row(&rows, name)) {
        let effects = common::all_effects(&definition);
        let choice = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ChoosePlayerOptionEffect>())
            .unwrap_or_else(|| panic!("{name}: missing per-player choice"));
        assert_eq!(choice.chooser, PlayerOptionChooser::Participant);
        assert_eq!(choice.participants, PlayerFilter::Opponent);
        assert_eq!(
            choice.options,
            vec!["money".to_string(), "friends".to_string(), "secrets".to_string()]
        );
        for option in ["money", "friends", "secrets"] {
            assert!(option_reads(&definition, option), "{name}: {option} unread");
        }
    }
}

/// Alice names herself friend and Bob foe.
struct FriendFoe;

impl DecisionMaker for FriendFoe {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(for_player) = ctx.description.strip_prefix("Choose one for ") {
            let wanted = if for_player == "Alice" { "friend" } else { "foe" };
            return vec![
                ctx.options
                    .iter()
                    .find(|option| option.description == wanted)
                    .unwrap()
                    .index,
            ];
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [ManaSymbol::Black, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(color, 10);
    }
    game
}

fn creature(name: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(
        name,
        "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/2",
        false,
    )
    .unwrap()
}

fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut FriendFoe) {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
}

fn in_zone(game: &GameState, player: PlayerId, zone: Zone, name: &str) -> bool {
    let player = game.player(player).unwrap();
    let ids: Vec<ObjectId> = match zone {
        Zone::Hand => player.hand.iter().copied().collect(),
        Zone::Graveyard => player.graveyard.iter().copied().collect(),
        _ => game.battlefield.iter().copied().collect(),
    };
    ids.iter()
        .any(|id| game.object(*id).is_some_and(|object| object.name == name))
}

#[test]
fn virtus_maneuver_friend_returns_and_foe_sacrifices() {
    let rows = rows();
    for definition in common::definitions(common::row(&rows, "Virtus's Maneuver")) {
        let mut game = game();
        let alice_card = creature("Alice Bear");
        let bob_creature = creature("Bob Bear");
        game.create_object_from_definition(&alice_card, A, Zone::Graveyard);
        let _: ObjectId = game.create_object_from_definition(&bob_creature, B, Zone::Battlefield);
        let mut dm = FriendFoe;
        cast(&mut game, &definition, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        // Alice (friend) returned her creature card; Bob (foe) sacrificed his.
        assert!(in_zone(&game, A, Zone::Hand, "Alice Bear"));
        assert!(in_zone(&game, B, Zone::Graveyard, "Bob Bear"));
    }
}
