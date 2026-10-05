use super::*;
use ironsmith::decisions::context::{NumberContext, TextInputContext, SelectionRevealPolicy};
struct Decisions {
    x: u32,
    name: &'static str,
    named: bool,
    shown: Vec<ObjectId>,
    pause_opening: bool,
    pending: bool,
    openings: Vec<Vec<ObjectId>>,
}
impl Decisions {
    fn new(x: u32, name: &'static str) -> Self {
        Self { x, name, named: false, shown: Vec::new(), pause_opening: false, pending: false, openings: Vec::new() }
    }
}
impl DecisionMaker for Decisions {
    fn decide_number(&mut self, _: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value { self.x } else { context.min }
    }
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert!(context.requirements.iter().any(|requirement| requirement.legal_targets.contains(&Target::Player(B))));
        vec![Target::Player(B)]
    }
    fn decide_text(&mut self, _: &GameState, _: &TextInputContext) -> String {
        self.named = true;
        self.name.into()
    }
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(context.reveal_policy, SelectionRevealPolicy::Public,
            "the only object prompt is the mandatory opening of the chosen IDs");
        assert_eq!(context.min, context.max.unwrap());
        let ids: Vec<_> = context.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
        assert_eq!(ids.len(), context.min);
        self.openings.push(ids.clone());
        self.pending = self.pause_opening;
        ids
    }
    fn view_cards(&mut self, game: &GameState, viewer: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
        assert!(self.named, "the name precedes the reveal");
        assert!(context.public);
        assert_eq!(context.subject, B);
        assert!(cards.iter().all(|id| game.object(*id).is_some_and(|object| object.zone == Zone::Hand)),
            "every reveal precedes the discard");
        if viewer == A { self.shown = cards.to_vec(); }
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn activate(game: &mut GameState, source: ObjectId, index: usize, dm: &mut Decisions) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let before = game.player(A).unwrap().mana_pool.total();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility { source, ability_index: index }), dm).unwrap();
    for _ in 0..64 {
        if state.pending_activation.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.player(A).unwrap().mana_pool.total(), before - dm.x);
    assert!(game.is_tapped(source));
    assert_eq!(game.stack.last().unwrap().targets, vec![Target::Player(B)]);
}
fn hand(game: &mut GameState, names: &[&str]) -> Vec<(ObjectId, ironsmith::ids::StableId, String)> {
    names.iter().map(|name| {
        let definition = compile_to_runtime_definition(name, "Type: Land", false).unwrap();
        let id = game.create_object_from_definition(&definition, B, Zone::Hand);
        (id, game.object(id).unwrap().stable_id, name.to_string())
    }).collect()
}
#[test]
fn nebuchadnezzar_pays_x_then_reveals_distinct_cards_and_discards_only_matching_revealed_names() {
    for definition in definitions("Nebuchadnezzar") {
        for x in [0, 1, 2, 9] {
            for leave in [false, true] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.remove_summoning_sickness(source);
                let index = definition.abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
                let cards = hand(&mut game, &["Island", "Mountain", "Island", "Island", "Mountain"]);
                game.turn.active_player = B;
                assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action, LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index)));
                game.turn.active_player = A;
                let mut dm = Decisions::new(x, "Island");
                activate(&mut game, source, index, &mut dm);
                if leave { game.move_object_by_effect(source, Zone::Exile).unwrap(); }
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.shown.len(), (x as usize).min(cards.len()));
                assert_eq!(dm.shown.iter().copied().collect::<std::collections::HashSet<_>>().len(), dm.shown.len());
                for (id, stable, name) in &cards {
                    let current = game.find_object_by_stable_id(*stable).unwrap();
                    let expected = if dm.shown.contains(id) && name == "Island" { Zone::Graveyard } else { Zone::Hand };
                    assert_eq!(game.object(current).unwrap().zone, expected, "{name}, x={x}, revealed={:?}", dm.shown);
                }
                assert_eq!(game.irreversible_random_count(), u64::from(x > 0));
            }
        }
    }
}
#[test]
fn subsequent_name_choices_do_not_union_with_earlier_activations() {
    for definition in definitions("Nebuchadnezzar") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(ability) => Some(ability), _ => None }).unwrap();
        for name in ["Island", "Mountain"] {
            let cards = hand(&mut game, &["Island", "Mountain"]);
            game.push_to_stack(StackEntry::ability(source, A, ability.effects.clone()).with_x(9).with_targets(vec![Target::Player(B)]));
            let mut dm = Decisions::new(9, name);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            for (_, stable, card_name) in cards {
                let id = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(id).unwrap().zone, if card_name == name { Zone::Graveyard } else { Zone::Hand });
            }
        }
    }
}
#[test]
fn hidden_subset_opening_suspends_the_whole_program_and_reuses_random_authority() {
    for definition in definitions("Nebuchadnezzar") {
      for peer in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut cards = Vec::new();
        let mut identities = Vec::new();
        for (slot, name) in ["Island", "Mountain", "Island"].iter().enumerate() {
            let card = compile_to_runtime_definition(name, "Type: Land", false).unwrap();
            let id = game.create_hidden_card_placeholder(B, Zone::Hand, slot as u16, format!("card-{slot}"));
            if !peer { game.reveal_hidden_card_with_definition(id, &card).unwrap(); }
            cards.push((id, game.object(id).unwrap().stable_id, name.to_string()));
            identities.push((id, card));
        }
        let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(ability) => Some(ability), _ => None }).unwrap();
        game.queue_transcript_random_seeds([112233]);
        game.push_to_stack(StackEntry::ability(source, A, ability.effects.clone()).with_x(2).with_targets(vec![Target::Player(B)]));
        let before_seed = game.random_seed();
        let mut dm = Decisions::new(2, "Island");
        dm.pause_opening = true;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.pending);
        assert_eq!(dm.openings.len(), 1);
        assert_eq!(dm.openings[0].len(), 2);
        assert!(dm.shown.is_empty());
        assert_eq!(game.irreversible_random_count(), 0);
        assert_eq!(game.random_seed(), before_seed);
        assert_eq!(game.player(B).unwrap().hand.len(), 3);
        assert_eq!(game.stack.len(), 1);
        assert!(cards.iter().all(|(id, _, _)| !game.is_publicly_revealed_hidden_card(*id)));
        if peer {
            for (id, card) in &identities {
                if dm.openings[0].contains(id) { game.reveal_hidden_card_with_definition(*id, card).unwrap(); }
            }
        }
        dm.pending = false;
        dm.pause_opening = false;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(game.irreversible_random_count(), 1);
        assert_eq!(game.random_seed(), 112233);
        assert_eq!(dm.openings[0], dm.openings[1]);
        assert_eq!(dm.shown, dm.openings[0]);
        for (id, stable, name) in cards {
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, if dm.shown.contains(&id) && name == "Island" { Zone::Graveyard } else { Zone::Hand });
        }
    }
  }
}
#[test]
fn empty_hand_still_names_a_card_without_consuming_randomness_or_discarding() {
    for definition in definitions("Nebuchadnezzar") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(ability) => Some(ability), _ => None }).unwrap();
        game.push_to_stack(StackEntry::ability(source, A, ability.effects.clone()).with_x(5).with_targets(vec![Target::Player(B)]));
        let mut dm = Decisions::new(5, "Island");
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.named);
        assert!(dm.shown.is_empty());
        assert!(dm.openings.is_empty());
        assert_eq!(game.irreversible_random_count(), 0);
        assert!(game.player(B).unwrap().graveyard.is_empty());
    }
}
