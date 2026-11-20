use super::*;
use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::ids::CardId;
use crate::replacement::EventModification;
use crate::target::PlayerFilter;

/// Counts replacement-order prompts and always takes the first option.
#[derive(Default)]
struct CountingChooser {
    prompts: usize,
}

impl DecisionMaker for CountingChooser {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.prompts += 1;
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.index)
            .take(ctx.min.max(1))
            .collect()
    }
}

fn setup() -> (GameState, PlayerId) {
    (
        GameState::new(vec!["Alice".into(), "Bob".into()], 20),
        PlayerId::from_index(0),
    )
}

/// One permanent per replacement, as with token copies of a doubler.
fn register(
    game: &mut GameState,
    controller: PlayerId,
    matcher: impl crate::events::ReplacementMatcher + 'static,
    action: ReplacementAction,
) {
    let card = crate::card::CardBuilder::new(CardId::new(), "Replacement source")
        .card_types(vec![crate::types::CardType::Enchantment])
        .build();
    let source = game.create_object_from_card(&card, controller, Zone::Battlefield);
    game.effect_store
        .replacement_effects
        .add_effect(ReplacementEffect::with_matcher(source, controller, matcher, action));
}

fn final_event(result: TraitEventResult) -> Event {
    match result {
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => event,
        TraitEventResult::Expanded { original, .. } => final_event(*original),
        other => panic!("replacement processing did not finish (needs input: {})", other.requires_replacement_input()),
    }
}

fn token_count(game: &mut GameState, dm: &mut CountingChooser, controller: PlayerId) -> u128 {
    let proposal = crate::events::CreateTokensEvent::with_cause(controller, 1, EventCause::effect());
    let event = final_event(
        process_with_dm(game, Event::new_with_provenance(proposal, Default::default()), dm)
            .expect("token proposal resolves"),
    );
    crate::events::downcast_event::<crate::events::CreateTokensEvent>(event.inner())
        .expect("token event")
        .total_count()
}

fn life_gain(game: &mut GameState, dm: &mut CountingChooser, player: PlayerId, amount: u32) -> u32 {
    let event = final_event(
        process_with_dm(
            game,
            Event::new_with_provenance(crate::events::LifeGainEvent::new(player, amount), Default::default()),
            dm,
        )
        .expect("life gain resolves"),
    );
    crate::events::downcast_event::<crate::events::LifeGainEvent>(event.inner())
        .expect("life gain event")
        .amount
}

#[test]
fn identical_token_doublers_apply_without_an_order_choice() {
    let (mut game, alice) = setup();
    for _ in 0..20 {
        register(
            &mut game,
            alice,
            crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
            ReplacementAction::Double,
        );
    }
    // A different card with a different scale factor still commutes.
    register(
        &mut game,
        alice,
        crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
        ReplacementAction::Modify(EventModification::Multiply(3)),
    );
    let mut dm = CountingChooser::default();
    assert_eq!(token_count(&mut game, &mut dm, alice), 3 << 20);
    assert_eq!(dm.prompts, 0);

    // Unrepresentable totals saturate without one prompt per doubler, and the
    // creation budget bounds what is actually created.
    for _ in 0..20 {
        register(
            &mut game,
            alice,
            crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
            ReplacementAction::Double,
        );
    }
    assert_eq!(
        token_count(&mut game, &mut dm, alice),
        u128::from(u32::MAX),
        "2^40 * 3 tokens saturates the token count representation"
    );
    assert_eq!(dm.prompts, 0);
    game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits {
        max_created_tokens: 5,
        ..Default::default()
    });
    let token = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Doubled token")
        .token()
        .card_types(vec![crate::types::CardType::Artifact])
        .build();
    let source = game.new_object_id();
    let before = game.battlefield.len();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::effects::EffectExecutor::execute(
        &crate::effects::CreateTokenEffect::you(token, 1),
        &mut game,
        &mut ctx,
    )
    .expect("creation past the budget creates what fits");
    assert_eq!(game.battlefield.len() - before, 5, "the doubled count clamps to the budget");
}

#[test]
fn offsets_commute_but_mixed_scale_and_offset_still_ask() {
    let (mut game, alice) = setup();
    for addend in [1, 2, 3] {
        register(
            &mut game,
            alice,
            crate::events::WouldGainLifeMatcher::you(),
            ReplacementAction::Modify(EventModification::Add(addend)),
        );
    }
    let mut dm = CountingChooser::default();
    assert_eq!(life_gain(&mut game, &mut dm, alice, 1), 7);
    assert_eq!(dm.prompts, 0);

    // 2(q + 6) differs from 2q + 6: the player must order these.
    register(&mut game, alice, crate::events::WouldGainLifeMatcher::you(), ReplacementAction::Double);
    let mut dm = CountingChooser::default();
    life_gain(&mut game, &mut dm, alice, 1);
    assert!(dm.prompts > 0);
}

/// "If you would gain 4 or more life": its applicability depends on the
/// amount, so it may start applying part way through a batch.
#[derive(Debug, Clone)]
struct GainAtLeastFour;

impl crate::events::ReplacementMatcher for GainAtLeastFour {
    fn may_match_event_kind(&self, kind: EventKind) -> bool {
        kind == EventKind::LifeGain
    }
    fn matches_prepared_event(
        &self,
        event: &dyn GameEventType,
        _ctx: &crate::events::context::PreparedEventContext,
    ) -> bool {
        crate::events::downcast_event::<crate::events::LifeGainEvent>(event)
            .is_some_and(|gain| gain.amount >= 4)
    }
    fn display(&self) -> String {
        "If you would gain 4 or more life".into()
    }
}

#[test]
fn amount_sensitive_bystanders_keep_the_order_choice() {
    let (mut game, alice) = setup();
    for _ in 0..2 {
        register(&mut game, alice, crate::events::WouldGainLifeMatcher::you(), ReplacementAction::Double);
    }
    register(&mut game, alice, GainAtLeastFour, ReplacementAction::Modify(EventModification::Add(1)));
    let mut dm = CountingChooser::default();
    life_gain(&mut game, &mut dm, alice, 1);
    assert!(dm.prompts > 0);
}
