use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::triggers::{Trigger, TriggerQueue};
use crate::{
    Ability, CardId, CardType, Color, Effect, ManaCost, ManaSymbol, ObjectFilter, PlayerFilter,
    Zone,
};

struct ColorChoice {
    suspend: bool,
    pending: bool,
    seen: usize,
}
impl DecisionMaker for ColorChoice {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
    fn decide_colors(
        &mut self,
        game: &GameState,
        _: &crate::decisions::context::ColorsContext,
    ) -> Vec<Color> {
        self.seen += 1;
        let player = game.player(PlayerId::from_index(0)).unwrap();
        assert_eq!(
            player.mana_pool.green, 1,
            "the original activation already produced mana before its mana trigger"
        );
        assert_eq!(player.life, 20, "ordinary triggers must wait for priority");
        assert!(
            game.stack.is_empty(),
            "neither mana ability goes onto the stack"
        );
        self.pending = self.suspend;
        vec![Color::Red]
    }
}
fn setup(trigger_effects: Vec<Effect>) -> (GameState, ObjectId, ObjectId, ManaPaymentRequest) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.priority_player = None;
    let source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Manual producer")
            .card_types(vec![CardType::Land])
            .with_ability(Ability::mana(
                crate::cost::TotalCost::free(),
                vec![ManaSymbol::Green],
            ))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let event = Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::land());
    let observer = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Mana and ordinary observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(event.clone(), trigger_effects))
            .with_ability(Ability::triggered(event, vec![Effect::gain_life(9)]))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let request = ManaPaymentRequest::new(
        alice,
        observer,
        crate::costs::PaymentReason::Effect,
        ManaCost::from_symbols(vec![ManaSymbol::Green, ManaSymbol::Red]),
    );
    game.take_pending_trigger_events();
    (game, source, observer, request)
}

#[test]
fn manual_mana_trigger_choice_rolls_back_the_activation_and_resumes_without_ordinary_triggers() {
    let (mut game, source, observer, request) = setup(vec![Effect::add_mana_of_any_color(1)]);
    let mut pending = ColorChoice {
        suspend: true,
        pending: false,
        seen: 0,
    };
    assert!(
        activate_mana_during_payment_with_outputs(&mut game, &request, source, 0, &mut pending)
            .unwrap()
            .is_none()
    );
    assert_eq!(pending.seen, 1);
    assert!(pending.pending);
    assert!(!game.is_tapped(source));
    assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
    assert_eq!(game.player(request.payer).unwrap().life, 20);
    assert!(game.stack.is_empty());
    let mut dm = ColorChoice {
        suspend: false,
        pending: false,
        seen: 0,
    };
    let outputs =
        activate_mana_during_payment_with_outputs(&mut game, &request, source, 0, &mut dm)
            .unwrap()
            .unwrap();
    assert_eq!(dm.seen, 1);
    assert!(!outputs.is_empty());
    assert!(game.is_tapped(source));
    let player = game.player(request.payer).unwrap();
    assert_eq!(
        (player.mana_pool.green, player.mana_pool.red, player.life),
        (1, 1, 20)
    );
    assert!(game.stack.is_empty());
    let mut queue = TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue.entries.len(),
        1,
        "only the ordinary trigger survives completion"
    );
    assert_eq!(queue.entries[0].source, observer);
    game.validate_zone_consistency().unwrap();
}

#[test]
fn manual_mana_trigger_resource_failure_preserves_source_and_production() {
    let (mut game, source, _, request) = setup(vec![
        Effect::add_mana(vec![ManaSymbol::Red]),
        Effect::new(crate::effects::CreateTokenEffect::you(
            crate::cards::tokens::treasure_token_definition(),
            1,
        )),
    ]);
    game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits {
        max_created_tokens: 0,
        ..Default::default()
    });
    let mut dm = ColorChoice {
        suspend: false,
        pending: false,
        seen: 0,
    };
    let result = activate_mana_during_payment_with_outputs(&mut game, &request, source, 0, &mut dm);
    assert!(
        matches!(
            result,
            Err(crate::special_actions::ActionError::ExecutionFailure {
                error: crate::effects::ExecutionError::ResourceLimitExceeded { .. },
                ..
            })
        ),
        "trigger-path resource failure must propagate: {result:?}"
    );
    assert!(!game.is_tapped(source));
    assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
    assert_eq!(game.battlefield.len(), 2);
    assert!(game.stack.is_empty());
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn error_from_actual_immediate_trigger_choice_restores_the_completed_activation() {
    let (mut game, source, observer, request) = setup(vec![Effect::add_mana_of_any_color(1)]);
    struct FailChoice {
        source: ObjectId,
        observer: ObjectId,
        seen: usize,
    }
    impl DecisionMaker for FailChoice {
        fn planned_mana_output(
            &mut self,
            game: &GameState,
            choice: &crate::mana_payment::ManaProductionChoice,
        ) -> Result<Option<Vec<ManaSymbol>>, String> {
            assert_eq!(choice.source, self.observer);
            assert!(game.is_tapped(self.source));
            assert_eq!(game.player(choice.player).unwrap().mana_pool.green, 1);
            self.seen += 1;
            Err("intentional immediate-trigger choice failure".into())
        }
    }
    let mut dm = FailChoice {
        source,
        observer,
        seen: 0,
    };
    let result = activate_mana_during_payment_with_outputs(&mut game, &request, source, 0, &mut dm);
    assert_eq!(
        dm.seen, 1,
        "the failure must follow actual production, rather than a planning probe"
    );
    assert!(
        matches!(result,Err(crate::special_actions::ActionError::ExecutionFailure{
        error:crate::effects::ExecutionError::InternalError(ref message),..
    }) if message=="intentional immediate-trigger choice failure")
    );
    assert!(!game.is_tapped(source));
    assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
    assert!(game.stack.is_empty());
    assert!(game.take_pending_trigger_events().is_empty());
}
