use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::effects::{ExecutionContext, ExileEffect};
use crate::{Ability, CardId, CardType, ChooseSpec, Color, Effect, StaticAbility};

struct Choices {
    suspend: bool,
    pending: bool,
    worlds: Vec<(usize, usize)>,
}
impl crate::decision::DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
    fn decide_colors(
        &mut self,
        game: &GameState,
        _: &crate::decisions::context::ColorsContext,
    ) -> Vec<Color> {
        self.worlds.push((
            game.battlefield.len(),
            game.player(PlayerId::from_index(0)).unwrap().hand.len(),
        ));
        self.pending = self.suspend;
        vec![Color::Blue]
    }
}
fn game() -> GameState {
    GameState::new(vec!["A".into(), "B".into()], 20)
}
fn permanent(game: &mut GameState, owner: PlayerId, zone: Zone, choose: bool) -> ObjectId {
    let mut definition = CardDefinitionBuilder::new(CardId::new(), "Duration resource")
        .card_types(vec![CardType::Artifact]);
    if choose {
        definition = definition.with_ability(Ability::static_ability(
            StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
        ));
    }
    game.create_object_from_definition(&definition.build(), owner, zone)
}
fn track(game: &mut GameState, source: ObjectId, object: ObjectId, zone: Zone) {
    game.add_exiled_with_source_link_returning_to(source, object, zone);
    game.mark_return_exiled_when_source_leaves(source);
}
fn choices(suspend: bool) -> Choices {
    Choices {
        suspend,
        pending: false,
        worlds: Vec::new(),
    }
}
fn depart(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let effect = Effect::new(ExileEffect::with_spec(ChooseSpec::SpecificObject(source)));
    let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), dm);
    crate::effects::execute_effect(game, &effect, &mut ctx).unwrap();
}

#[test]
fn duration_return_instruction_boundary_restores_choice_and_original_together() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, alice, Zone::Battlefield, false);
    let returned = permanent(&mut game, alice, Zone::Exile, true);
    let stable = game.object(returned).unwrap().stable_id;
    track(&mut game, source, returned, Zone::Battlefield);
    let mut dm = choices(true);
    depart(&mut game, source, &mut dm);
    assert!(dm.pending);
    assert_eq!(dm.worlds, vec![(0, 0)]);
    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.object(returned).unwrap().zone, Zone::Exile);
    assert!(
        game.exile_tracking
            .return_exiled_when_source_leaves
            .contains(&source)
    );
    assert!(!game.has_pending_duration_end_returns());
    assert!(game.chosen_color(returned).is_none());
    let mut dm = choices(false);
    depart(&mut game, source, &mut dm);
    assert!(game.object(source).is_none());
    let new_id = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(new_id).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.chosen_color(new_id), Some(Color::Blue));
    assert_eq!(dm.worlds, vec![(0, 0)]);
    assert!(!game.has_pending_duration_end_returns());
    game.validate_zone_consistency().unwrap();
}

#[test]
fn mixed_duration_return_choices_precede_hand_arrivals_and_keep_individual_causes() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sources = [
        permanent(&mut game, alice, Zone::Battlefield, false),
        permanent(&mut game, bob, Zone::Battlefield, false),
    ];
    let cards = [
        permanent(&mut game, bob, Zone::Exile, true),
        permanent(&mut game, alice, Zone::Exile, false),
    ];
    track(&mut game, sources[0], cards[0], Zone::Battlefield);
    track(&mut game, sources[1], cards[1], Zone::Hand);
    // This native owner queues one actual ending event, retaining both sources.
    game.return_exiled_for_sources_leave(&sources);
    game.take_pending_trigger_events();
    let mut dm = choices(false);
    game.process_pending_duration_end_returns(&mut dm).unwrap();
    assert_eq!(
        dm.worlds,
        vec![(2, 0)],
        "the hand original must not precede entry preparation"
    );
    let events = game.turn_store.action_history.iter().map(|record| record.event.clone()).collect::<Vec<_>>();
    let changes = events
        .iter()
        .filter_map(|e| e.downcast::<crate::events::ZoneChangeEvent>())
        .filter(|c| c.from == Zone::Exile)
        .collect::<Vec<_>>();
    assert!(
        changes.iter().any(
            |c| c.to == Zone::Battlefield && c.cause.duration_return_source == Some(sources[0])
        )
    );
    assert!(
        changes
            .iter()
            .any(|c| c.to == Zone::Hand && c.cause.duration_return_source == Some(sources[1]))
    );
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|id| game.object(**id).unwrap().owner == bob)
            .count(),
        2
    );
    let batches = events
        .iter()
        .filter(|e| {
            e.downcast::<crate::events::ZoneChangeEvent>()
                .is_some_and(|c| c.from == Zone::Exile)
        })
        .map(|e| e.simultaneous_batch())
        .collect::<Vec<_>>();
    assert!(batches.len() >= 2 && batches[0].is_some());
    assert!(batches.iter().all(|batch| *batch == batches[0]));
    game.validate_zone_consistency().unwrap();
}

#[test]
fn separate_duration_endings_remain_separate_in_a_native_clone() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, alice, Zone::Battlefield, false);
    let first = permanent(&mut game, alice, Zone::Exile, false);
    track(&mut game, source, first, Zone::Hand);
    game.return_exiled_for_source_leave(source);
    let mut branch = game.clone();
    let second = permanent(&mut branch, alice, Zone::Exile, true);
    track(&mut branch, source, second, Zone::Battlefield);
    branch.return_exiled_for_source_leave(source);
    assert_eq!(
        game.auxiliary_tracking.pending_duration_end_returns.len(),
        1
    );
    assert_eq!(
        branch.auxiliary_tracking.pending_duration_end_returns.len(),
        2
    );
    let mut dm = choices(false);
    branch
        .process_pending_duration_end_returns(&mut dm)
        .unwrap();
    assert_eq!(
        dm.worlds,
        vec![(1, 1)],
        "a later return must see the earlier hand arrival"
    );
    assert_eq!(game.object(first).unwrap().zone, Zone::Exile);
    assert!(game.player(alice).unwrap().hand.is_empty());
}
