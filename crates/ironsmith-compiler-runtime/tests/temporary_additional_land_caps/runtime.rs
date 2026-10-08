use super::{definitions, fixtures};
use ironsmith::cards::CardDefinition;
use ironsmith::card::CardBuilder;
use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, ManaPaymentContext, SelectObjectsContext, SelectOptionsContext, ViewCardsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::ids::StableId;
use ironsmith::game_state::HiddenInfoOperation;
use ironsmith::special_actions::{SpecialAction, can_perform_check, perform};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, Phase, PlayerId, Supertype, Zone};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 20);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 20);
    game
}
fn land(game: &mut GameState, player: PlayerId, zone: Zone, basic: bool) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::new(), "Land witness").card_types(vec![CardType::Land]);
    if basic { builder = builder.supertypes(vec![Supertype::Basic]); }
    game.create_object_from_card(&builder.build(), player, zone)
}
fn bodies(name: &str) -> [CardDefinition; 2] {
    definitions(fixtures().iter().find(|row| row["name"] == name).unwrap())
}

struct Choices {
    mode: usize,
    entwine: bool,
    search_count: usize,
    searched: bool,
    revealed: usize,
    selected: Vec<ObjectId>,
    selected_stable: Vec<StableId>,
    public_viewers: Vec<PlayerId>,
    resolution_checkpoint: usize,
}
impl Choices {
    fn new(mode: usize, entwine: bool, search_count: usize) -> Self {
        Self { mode, entwine, search_count, searched: false, revealed: 0,
            selected: Vec::new(), selected_stable: Vec::new(), public_viewers: Vec::new(),
            resolution_checkpoint: 0 }
    }
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, _: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.min == 0 {
            assert_eq!(context.options.len(), 1, "only Entwine is optional");
            return if self.entwine { vec![context.options[0].index] } else { vec![] };
        }
        assert_eq!(context.options.len(), 2, "Journey has exactly two modes");
        assert!(context.options.iter().any(|option| option.index == self.mode && option.legal));
        vec![self.mode]
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        panic!("land permission must not ask a resolution-time may question")
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(context.player, A);
        assert_eq!((context.min, context.max), (0, Some(2)));
        assert_eq!(game.player(A).unwrap().land_plays_per_turn, 1, "search precedes the entwined land grant");
        self.searched = true;
        let legal: Vec<_> = context.candidates.iter().filter(|candidate| candidate.legal).collect();
        assert_eq!(legal.len(), 3, "nonbasic lands and the opponent's library are excluded");
        self.selected = legal.into_iter().take(self.search_count).map(|candidate| candidate.id).collect();
        self.selected_stable = self.selected.iter().map(|id| game.object(*id).unwrap().stable_id).collect();
        self.selected.clone()
    }
    fn view_cards(&mut self, game: &GameState, viewer: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
        if !context.public { return; }
        assert!(self.searched, "the reveal follows the explicit search selection");
        assert_eq!(cards, self.selected.as_slice(), "reveal exactly the selected identities");
        assert_eq!((context.viewer, context.subject, context.zone), (viewer, A, Zone::Library));
        assert!(game.player(A).unwrap().hand.is_empty(), "reveal happens before any selected card moves to hand");
        assert!(cards.iter().all(|id| game.object(*id).is_some_and(|object| object.owner == A && object.zone == Zone::Library)));
        assert!(!game.crypto_audit_operations_since(self.resolution_checkpoint).iter().any(|operation|
            matches!(operation, HiddenInfoOperation::LibraryShuffle { .. })), "reveal precedes shuffle");
        assert!(!self.public_viewers.contains(&viewer), "each player sees this exact revealed group once");
        self.public_viewers.push(viewer);
        self.revealed = self.revealed.max(cards.len());
    }
    fn decide_mana_payment(&mut self, _: &GameState, context: &ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm { plan_id: context.plan.id, request_hash: context.plan.request_hash }
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) {
    game.turn.priority_player = Some(A);
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id)).unwrap();
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() { return; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    panic!("cast did not complete");
}
fn play(game: &mut GameState, choices: &mut Choices) {
    game.turn.priority_player = Some(A);
    let id = land(game, A, Zone::Hand, true);
    perform(SpecialAction::PlayLand { card_id: id }, game, A, choices).unwrap();
}
fn expire(game: &mut GameState) {
    game.cleanup_restrictions_end_of_turn();
    game.update_cant_effects();
}

#[test]
fn summer_bloom_allows_zero_through_three_optional_actual_plays_then_expires() {
    for definition in bodies("Summer Bloom") { for used_before in [0, 1] { for actual_extra in 0..=3 {
        let mut game = game();
        let mut choices = Choices::new(0, false, 0);
        for _ in 0..used_before { play(&mut game, &mut choices); }
        cast(&mut game, &definition, &mut choices);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.player(A).unwrap().land_plays_per_turn, 4);
        assert_eq!(game.player(B).unwrap().land_plays_per_turn, 1);
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, used_before);
        for _ in 0..actual_extra { play(&mut game, &mut choices); }
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, used_before + actual_extra);
        expire(&mut game);
        assert_eq!(game.player(A).unwrap().land_plays_per_turn, 1);
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, used_before + actual_extra);
    } } }
}

#[test]
fn summer_bloom_cap_stacks_without_reset_and_never_grants_off_turn_plays() {
    for definition in bodies("Summer Bloom") {
        let mut game = game();
        let mut choices = Choices::new(0, false, 0);
        play(&mut game, &mut choices);
        for _ in 0..2 {
            cast(&mut game, &definition, &mut choices);
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        }
        assert_eq!(game.player(A).unwrap().land_plays_per_turn, 7);
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, 1);
        let witness = land(&mut game, A, Zone::Hand, true);
        game.turn.active_player = B;
        assert!(can_perform_check(&SpecialAction::PlayLand { card_id: witness }, &game, A).is_err());
        game.turn.active_player = A;
        for _ in 0..6 { play(&mut game, &mut choices); }
        assert!(can_perform_check(&SpecialAction::PlayLand { card_id: witness }, &game, A).is_err());
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, 7);
        expire(&mut game);
        assert_eq!(game.player(A).unwrap().land_plays_per_turn, 1);
    }
}

#[test]
fn journey_either_mode_or_paid_entwine_keeps_search_reveal_order_cost_and_cap() {
    for definition in bodies("Journey of Discovery") {
        for (mode, entwine) in [(0, false), (1, false), (0, true)] { for search_count in 0..=2 {
            let mut game = game();
            for _ in 0..3 { land(&mut game, A, Zone::Library, true); }
            let nonbasic = land(&mut game, A, Zone::Library, false);
            let opponents = land(&mut game, B, Zone::Library, true);
            let mut choices = Choices::new(mode, entwine, search_count);
            let mana_before = game.player(A).unwrap().mana_pool.total();
            cast(&mut game, &definition, &mut choices);
            assert_eq!(mana_before - game.player(A).unwrap().mana_pool.total(), if entwine { 6 } else { 3 });
            assert_eq!(game.stack.last().unwrap().chosen_modes, Some(if entwine { vec![0, 1] } else { vec![mode] }));
            // A seeded clone supplies the expected native shuffle permutation;
            // the actual audit record, rather than mere order inequality, proves
            // exactly one shuffle through the intended player's library owner.
            game.set_random_seed(0x4a4f_5552_4e45_5901);
            let mut expected_shuffle = game.clone();
            let library_before = game.player(A).unwrap().library.to_vec();
            let opponent_before = game.player(B).unwrap().library.to_vec();
            let random_before = game.irreversible_random_count();
            choices.resolution_checkpoint = game.crypto_audit_checkpoint();
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            let searches = entwine || mode == 0;
            let remaining: Vec<_> = library_before.iter().copied().filter(|id| !choices.selected.contains(id)).collect();
            let shuffles: Vec<_> = game.crypto_audit_operations_since(choices.resolution_checkpoint).into_iter().filter(|operation|
                matches!(operation, HiddenInfoOperation::LibraryShuffle { .. })).collect();
            assert_eq!(shuffles.len(), usize::from(searches), "even finding zero cards still shuffles; the grant-only mode never does");
            if searches {
                expected_shuffle.player_mut(A).unwrap().library = remaining.clone().into();
                expected_shuffle.shuffle_player_library(A);
                let HiddenInfoOperation::LibraryShuffle { player, before_order, after_order, random_count_before, random_count_after, .. } = &shuffles[0] else { unreachable!() };
                assert_eq!(*player, A);
                assert_eq!(before_order, &remaining, "all selected cards left before the shuffle boundary");
                assert_eq!(after_order, &expected_shuffle.player(A).unwrap().library.to_vec(), "exact fixed-seed native permutation");
                assert_eq!(after_order, &game.player(A).unwrap().library.to_vec());
                assert_eq!((*random_count_before, *random_count_after), (random_before, random_before + 1));
                assert_eq!(game.random_seed(), expected_shuffle.random_seed());
            } else {
                assert_eq!(game.player(A).unwrap().library.to_vec(), library_before);
            }
            assert_eq!(game.irreversible_random_count(), random_before + u64::from(searches));
            assert_eq!(game.player(B).unwrap().library.to_vec(), opponent_before);
            let hand_stable: Vec<_> = game.player(A).unwrap().hand.iter().map(|id| game.object(*id).unwrap().stable_id).collect();
            assert_eq!(hand_stable.len(), choices.selected_stable.len());
            assert!(hand_stable.iter().all(|stable| choices.selected_stable.contains(stable)), "only exact selected card identities move to hand");
            for stable in &choices.selected_stable {
                let id = game.find_object_by_stable_id(*stable).unwrap();
                assert_eq!(game.object(id).unwrap().zone, Zone::Hand);
            }
            for id in &remaining { assert_eq!(game.object(*id).unwrap().zone, Zone::Library); }
            assert_eq!(choices.public_viewers, if searches && search_count > 0 { vec![A, B] } else { vec![] });
            assert_eq!(choices.searched, searches);
            assert_eq!(game.player(A).unwrap().hand.len(), if searches { search_count } else { 0 });
            assert_eq!(choices.revealed, if searches { search_count } else { 0 });
            assert_eq!(game.object(nonbasic).unwrap().zone, Zone::Library);
            assert_eq!(game.object(opponents).unwrap().zone, Zone::Library);
            let cap = if entwine || mode == 1 { 3 } else { 1 };
            assert_eq!(game.player(A).unwrap().land_plays_per_turn, cap);
            assert_eq!(game.player(B).unwrap().land_plays_per_turn, 1);
            for _ in 0..cap { play(&mut game, &mut choices); }
            let witness = land(&mut game, A, Zone::Hand, true);
            assert!(can_perform_check(&SpecialAction::PlayLand { card_id: witness }, &game, A).is_err());
            expire(&mut game);
            assert_eq!(game.player(A).unwrap().land_plays_per_turn, 1);
        } }
    }
}
