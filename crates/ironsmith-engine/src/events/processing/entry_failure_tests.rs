use super::*;
use crate::effect::{Effect, Value};
use crate::ids::{CardId, PlayerId};

fn setup() -> (GameState, ObjectId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let card = crate::card::CardBuilder::new(CardId::new(), "Entry program source")
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let entrant = game.create_object_from_card(&card, alice, Zone::Hand);
    game.take_pending_trigger_events();
    (game, entrant, alice)
}

fn program(fails: bool) -> crate::resolution::ResolutionProgram {
    let mut effects = vec![Effect::gain_life(2)];
    if fails {
        effects.push(Effect::lose_life(Value::X));
    }
    crate::resolution::ResolutionProgram::from_effects(effects)
}

#[test]
fn entry_program_failure_cannot_retain_an_earlier_instruction() {
    for fails in [false, true] {
        let (mut game, entrant, alice) = setup();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result =
            game.execute_entry_programs(entrant, alice, vec![program(fails)], None, &mut dm);
        assert_eq!(result.is_none(), fails);
        assert!(!dm.awaiting_choice());
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        if fails {
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[test]
fn central_entry_failure_restores_program_consequences_and_one_shot() {
    for fails in [false, true] {
        let (mut game, entrant, alice) = setup();
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(program(fails)),
            ),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result =
            game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm);
        assert!(!dm.awaiting_choice());
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        if fails {
            assert!(game.battlefield.is_empty());
            assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        } else {
            let entered = result.expect("valid entry program must complete").new_id;
            assert!(game.battlefield.contains(&entered));
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_none()
            );
        }
    }
}

fn gain_doubler(
    game: &mut GameState,
    source: ObjectId,
    alice: PlayerId,
) -> crate::replacement::ReplacementEffectId {
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::WouldGainLifeMatcher::new(crate::target::PlayerFilter::Specific(alice)),
            ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2)),
        ))
}

#[test]
fn entry_program_list_error_restores_preceding_program_and_one_shot() {
    for fails in [false, true] {
        let (mut game, entrant, alice) = setup();
        let one_shot = gain_doubler(&mut game, entrant, alice);
        let first = crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life(2)]);
        let mut second = vec![Effect::gain_life(1)];
        if fails {
            second.push(Effect::lose_life(Value::X));
        }
        let second = crate::resolution::ResolutionProgram::from_effects(second);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result =
            game.execute_entry_programs(entrant, alice, vec![first, second], None, &mut dm);
        assert_eq!(result.is_none(), fails);
        assert!(!dm.awaiting_choice());
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 25 }
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some(),
            fails,
        );
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        if fails {
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

struct ProgramAnswers {
    pause: bool,
    calls: usize,
    pending: bool,
}

impl crate::decision::DecisionMaker for ProgramAnswers {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert!(!self.pending);
        self.calls += 1;
        self.pending = self.pause && self.calls == 2;
        !self.pending
    }

    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}

#[test]
fn entry_program_list_pause_restores_preceding_program_and_replays_once() {
    let (mut game, entrant, alice) = setup();
    let one_shot = gain_doubler(&mut game, entrant, alice);
    let programs = vec![
        crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::gain_life(2),
            Effect::may(vec![Effect::gain_life(1)]),
        ]),
        crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::gain_life(3),
            Effect::may(vec![Effect::gain_life(4)]),
        ]),
    ];
    let mut dm = ProgramAnswers {
        pause: true,
        calls: 0,
        pending: false,
    };
    let result = game.execute_entry_programs(entrant, alice, programs.clone(), None, &mut dm);
    assert!(result.is_none());
    assert!(dm.pending);
    assert_eq!(dm.calls, 2);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = ProgramAnswers {
        pause: false,
        calls: 0,
        pending: false,
    };
    let result = game.execute_entry_programs(entrant, alice, programs, None, &mut replay);
    assert!(result.is_some());
    assert!(!replay.pending);
    assert_eq!(replay.calls, 2);
    assert_eq!(game.player(alice).unwrap().life, 32);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_none()
    );
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
}
