use super::*;
use crate::card::CardBuilder;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::decision::DecisionMaker;
use crate::decisions::context::{BooleanContext, ViewCardsContext};
use crate::effect::{Effect, EffectId, EffectPredicate};
use crate::ids::CardId;
use crate::snapshot::ObjectSnapshot;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[derive(Debug, Default)]
struct RevealAnswers {
    answer: Option<bool>,
    pending: bool,
    offered: Vec<BooleanContext>,
    public_views: usize,
}

impl DecisionMaker for RevealAnswers {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.offered.push(ctx.clone());
        self.pending = self.answer.is_none();
        self.answer.unwrap_or(false)
    }

    fn awaiting_choice(&self) -> bool {
        self.pending
    }

    fn view_cards(
        &mut self,
        _game: &GameState,
        _viewer: PlayerId,
        _cards: &[ObjectId],
        ctx: &ViewCardsContext,
    ) {
        self.public_views += usize::from(ctx.public);
    }
}

fn identity(matches: bool) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Looked-at card")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![if matches { Subtype::Elf } else { Subtype::Bear }])
        .build()
}

fn fixture(
    known: bool,
    matches: bool,
) -> (GameState, PlayerId, ObjectId, ObjectId, ConditionalEffect) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source_card = CardBuilder::new(CardId::new(), "Kinship source")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let top = game.create_hidden_card_placeholder(alice, Zone::Library, 0, "top-0".into());
    if known {
        game.reveal_hidden_card_with_definition(top, &identity(matches))
            .unwrap();
    }
    let filter = ObjectFilter {
        zone: None,
        shares_creature_type_with_source: true,
        ..ObjectFilter::creature()
    };
    let effect = ConditionalEffect::if_only(
        Condition::TaggedObjectMatches("looked".into(), filter),
        vec![
            Effect::with_id(
                0,
                Effect::may_single(Effect::new(crate::effects::RevealTaggedEffect::new(
                    "looked",
                ))),
            ),
            Effect::if_then(
                EffectId(0),
                EffectPredicate::Happened,
                vec![Effect::gain_life(2)],
            ),
        ],
    );
    (game, alice, source, top, effect)
}

fn resolve(
    game: &mut GameState,
    alice: PlayerId,
    source: ObjectId,
    top: ObjectId,
    effect: &ConditionalEffect,
    dm: &mut RevealAnswers,
) -> Result<EffectOutcome, ExecutionError> {
    let snapshot = ObjectSnapshot::from_object(game.object(top).unwrap(), game);
    let mut ctx = ExecutionContext::new(source, alice, dm);
    ctx.set_tagged_objects("looked", vec![snapshot]);
    let outcome = effect.execute(game, &mut ctx);
    assert!(
        ctx.optional_identity_guard.is_none(),
        "the guard must not leak into later offers"
    );
    outcome
}

#[test]
fn hidden_conditional_reveal_keeps_both_views_at_the_offer() {
    for known in [false, true] {
        let (mut game, alice, source, top, effect) = fixture(known, true);
        let mut dm = RevealAnswers::default();
        let outcome = resolve(&mut game, alice, source, top, &effect, &mut dm).unwrap();
        assert!(dm.pending);
        assert_eq!(dm.offered.len(), 1);
        assert!(dm.offered[0].can_accept);
        assert_eq!(dm.public_views, 0);
        assert!(outcome.events.is_empty());
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(game.hidden_identity_obligations().is_empty());
    }
}

#[test]
fn hidden_conditional_reveal_decline_discloses_no_identity_claim() {
    for known in [false, true] {
        for matches in [false, true] {
            let (mut game, alice, source, top, effect) = fixture(known, matches);
            let mut dm = RevealAnswers {
                answer: Some(false),
                ..Default::default()
            };
            resolve(&mut game, alice, source, top, &effect, &mut dm).unwrap();
            assert_eq!(dm.offered.len(), 1);
            assert_eq!(dm.offered[0].can_accept, !known || matches);
            assert_eq!(dm.public_views, 0);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert!(game.hidden_identity_obligations().is_empty());
            assert!(game.hidden_claim_subjects().is_empty());
        }
    }
}

#[test]
fn hidden_conditional_reveal_accepts_with_a_verifiable_identity_obligation() {
    for known in [false, true] {
        let (mut game, alice, source, top, effect) = fixture(known, true);
        let mut dm = RevealAnswers {
            answer: Some(true),
            ..Default::default()
        };
        resolve(&mut game, alice, source, top, &effect, &mut dm).unwrap();
        assert_eq!(dm.offered.len(), 1);
        assert_eq!(dm.public_views, 2);
        assert_eq!(game.player(alice).unwrap().life, 22);
        assert_eq!(game.hidden_identity_obligations().len(), 1);
        assert!(
            game.hidden_identity_obligation_violation(top, &identity(true))
                .is_none()
        );
        assert!(
            game.hidden_identity_obligation_violation(top, &identity(false))
                .is_some()
        );
    }
}

#[test]
fn hidden_conditional_reveal_rejects_a_known_mismatch_without_side_effects() {
    let (mut game, alice, source, top, effect) = fixture(true, false);
    let mut dm = RevealAnswers {
        answer: Some(true),
        ..Default::default()
    };
    assert_eq!(
        resolve(&mut game, alice, source, top, &effect, &mut dm),
        Err(ExecutionError::InvalidTarget)
    );
    assert!(!dm.offered[0].can_accept);
    assert_eq!(dm.public_views, 0);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(game.hidden_identity_obligations().is_empty());
}

#[test]
fn hidden_conditional_reveal_keeps_the_same_offer_after_public_opening() {
    let (mut game, alice, source, top, effect) = fixture(true, true);
    game.mark_hidden_cards_publicly_revealed(&[top]);
    let mut dm = RevealAnswers::default();
    resolve(&mut game, alice, source, top, &effect, &mut dm).unwrap();
    assert!(dm.pending);
    assert_eq!(dm.offered.len(), 1);
    assert!(dm.offered[0].can_accept);
}

#[test]
fn hidden_conditional_reveal_claim_keeps_the_source_types_at_resolution() {
    let (mut game, alice, source, top, effect) = fixture(false, true);
    let mut dm = RevealAnswers {
        answer: Some(true),
        ..Default::default()
    };
    resolve(&mut game, alice, source, top, &effect, &mut dm).unwrap();
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    assert!(game.object(source).is_none());
    assert!(
        game.hidden_identity_obligation_violation(top, &identity(true))
            .is_none()
    );
    assert!(
        game.hidden_identity_obligation_violation(top, &identity(false))
            .is_some()
    );
}
