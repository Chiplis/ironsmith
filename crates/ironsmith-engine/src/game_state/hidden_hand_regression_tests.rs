//! Peer-match regressions for choices that read hidden hand cards (see
//! `game_state::hidden_hand_choices`): a peer holding placeholders must offer,
//! validate and replay the owner's choice instead of skipping, erroring or
//! rejecting it because its local filter result is empty.

use crate::card::CardBuilder;
use crate::decision::DecisionMaker;
use crate::decisions::context::{SelectObjectsContext, SelectionRevealPolicy};
use crate::effects::{EffectExecutor, ExecutionContext};
use crate::game_state::GameState;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::types::CardType;
use crate::zone::Zone;

/// Answers every object choice with its first `take` legal candidates and
/// records the contexts it was asked.
#[derive(Debug, Default)]
struct RecordingChooseDm {
    take: usize,
    asked: Vec<SelectObjectsContext>,
}

impl DecisionMaker for RecordingChooseDm {
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.asked.push(ctx.clone());
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(self.take)
            .collect()
    }
}

fn setup() -> (GameState, PlayerId, ObjectId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.new_object_id();
    (game, alice, source)
}

/// A hand card Alice's peer holds only as a placeholder.
fn hidden_hand_card(game: &mut GameState, owner: PlayerId, slot: u16) -> ObjectId {
    game.create_hidden_card_placeholder(owner, Zone::Hand, slot, format!("slot-{slot}"))
}

fn creature_in(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
    let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), "Bear")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_card(&card, owner, zone)
}

#[test]
fn reveal_from_hand_offers_placeholder_and_opens_it_publicly() {
    let (mut game, alice, source) = setup();
    let placeholder = hidden_hand_card(&mut game, alice, 0);

    let mut dm = RecordingChooseDm {
        take: 1,
        ..Default::default()
    };
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    let outcome = crate::effects::RevealFromHandEffect::new(1, Some(CardType::Creature))
        .execute(&mut game, &mut ctx)
        .expect("a peer must replay the owner's reveal instead of failing");

    assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
    assert_eq!(dm.asked.len(), 1);
    assert_eq!(dm.asked[0].reveal_policy, SelectionRevealPolicy::Public);
    assert!(dm.asked[0].require_explicit_choice);
    assert!(game.is_publicly_revealed_hidden_card(placeholder));
}

#[test]
fn imprint_asks_even_when_only_placeholders_could_match() {
    let (mut game, alice, source) = setup();
    let placeholder = hidden_hand_card(&mut game, alice, 0);

    let mut dm = RecordingChooseDm::default();
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    crate::effects::cards::ImprintFromHandEffect::nonartifact_nonland()
        .execute(&mut game, &mut ctx)
        .expect("imprint resolves");

    assert_eq!(dm.asked.len(), 1, "the owner's imprint prompt must exist on every peer");
    assert!(
        dm.asked[0]
            .candidates
            .iter()
            .any(|candidate| candidate.id == placeholder)
    );
    assert_eq!(dm.asked[0].reveal_policy, SelectionRevealPolicy::Public);
}

#[test]
fn filtered_hand_costs_count_placeholders_as_payable() {
    let (mut game, alice, source) = setup();
    hidden_hand_card(&mut game, alice, 0);

    let blue = crate::color::ColorSet::BLUE;
    for (label, cost) in [
        ("discard a creature card", crate::costs::Cost::discard(1, Some(CardType::Creature))),
        ("exile a blue card from hand", crate::costs::Cost::exile_from_hand(1, Some(blue))),
    ] {
        let effect = cost.effect_ref().expect("effect-backed cost");
        assert!(
            effect
                .0
                .can_execute_as_cost_with_reason(
                    &game,
                    source,
                    alice,
                    crate::costs::PaymentReason::CastSpell,
                )
                .is_ok(),
            "{label}: a peer must not reject the owner's payment with a hidden card"
        );
    }
}

#[test]
fn filtered_hand_costs_stay_unpayable_without_hidden_cards() {
    let (mut game, alice, source) = setup();
    let _land = {
        let card = CardBuilder::new(CardId::from_raw(90), "Land")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, alice, Zone::Hand)
    };

    let cost = crate::costs::Cost::discard(1, Some(CardType::Creature));
    let effect = cost.effect_ref().expect("effect-backed cost");
    assert!(
        effect
            .0
            .can_execute_as_cost_with_reason(
                &game,
                source,
                alice,
                crate::costs::PaymentReason::CastSpell,
            )
            .is_err()
    );
}

#[test]
fn reveal_card_land_condition_becomes_an_interactive_reveal() {
    // Snarl / Temple of the Dragon Queen: "you may reveal a <quality> card
    // from your hand ... enters tapped unless you revealed one [or ...]".
    let (_, alice, source) = setup();
    let filter = crate::filter::ObjectFilter::default().with_type(CardType::Creature);
    for condition in [
        crate::effect::Condition::YouHaveCardInHandMatching(filter.clone()),
        crate::effect::Condition::Or(
            Box::new(crate::effect::Condition::YouHaveCardInHandMatching(filter.clone())),
            Box::new(crate::effect::Condition::YouControl(
                crate::filter::ObjectFilter::creature(),
            )),
        ),
    ] {
        let ability =
            crate::static_abilities::EntersTappedUnlessCondition::new(condition, String::new());
        let replacement = crate::static_abilities::StaticAbilityKind::generate_replacement_effect(
            &ability, source, alice,
        )
        .expect("replacement");
        assert!(matches!(
            replacement.replacement,
            crate::replacement::ReplacementAction::InteractiveRevealCardOrEnterTapped { .. }
        ));
    }
}

#[test]
fn reveal_or_enter_tapped_offers_placeholders() {
    let (mut game, alice, _) = setup();
    let placeholder = hidden_hand_card(&mut game, alice, 0);
    let filter = crate::filter::ObjectFilter::default().with_type(CardType::Land);

    let (cards, hidden) =
        crate::events::processing::hand_replacement_choice_candidates(&game, alice, &filter);
    assert!(hidden, "the prompt must exist on every peer");
    assert_eq!(cards, vec![placeholder]);
}

#[test]
fn all_matching_hand_instruction_lets_owner_reveal_first() {
    let (mut game, alice, source) = setup();
    let placeholder = hidden_hand_card(&mut game, alice, 0);
    let _public_creature = creature_in(&mut game, alice, Zone::Battlefield);
    let filter = crate::filter::ObjectFilter::default()
        .in_zone(Zone::Hand)
        .with_type(CardType::Creature);

    let mut dm = RecordingChooseDm {
        take: 1,
        ..Default::default()
    };
    let filter_ctx = crate::filter::FilterContext::new(alice).with_source(source);
    assert!(game.settle_hidden_hand_all_matching(&mut dm, source, &filter, &filter_ctx));
    assert_eq!(dm.asked.len(), 1);
    assert_eq!(dm.asked[0].reveal_policy, SelectionRevealPolicy::Public);
    assert!(game.is_publicly_revealed_hidden_card(placeholder));
}
