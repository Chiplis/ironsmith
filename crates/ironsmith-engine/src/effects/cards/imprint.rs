//! Imprint effect implementation.
//!
//! Imprint exiles a card from a zone (typically hand) and associates it with
//! the source permanent. Used by Chrome Mox, Isochron Scepter, etc.

use crate::effect::EffectOutcome;
use crate::effects::CompletedEffectOutputs;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ObjectFilter;
use crate::zone::Zone;

/// Effect that exiles a card from hand and imprints it on the source permanent.
///
/// This is an optional effect ("you may exile"). If the player chooses not to
/// exile anything, no card is imprinted.
///
/// # Fields
///
/// * `filter` - Filter for which cards can be imprinted (e.g., nonartifact, nonland)
///
/// # Example
///
/// ```ignore
/// // Chrome Mox: "you may exile a nonartifact, nonland card from your hand"
/// let effect = ImprintFromHandEffect::new(
///     ObjectFilter::any()
///         .exclude_card_type(CardType::Artifact)
///         .exclude_card_type(CardType::Land)
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImprintFromHandEffect {
    /// Filter for valid cards to imprint.
    pub filter: ObjectFilter,
}

impl ImprintFromHandEffect {
    /// Create a new imprint from hand effect with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Create an imprint effect for nonartifact, nonland cards.
    pub fn nonartifact_nonland() -> Self {
        use crate::types::CardType;
        Self::new(
            ObjectFilter::default()
                .without_type(CardType::Artifact)
                .without_type(CardType::Land),
        )
    }
}

impl EffectExecutor for ImprintFromHandEffect {
    fn canonical_program(&self) -> Option<crate::effect::Effect> {
        use crate::effect::{ChoiceCount, Effect};
        use crate::target::{ChooseSpec, PlayerFilter};
        const SELECTED: &str = "__imprint_selected";
        const EXILED: &str = "__imprint_arrived";
        let filter = ObjectFilter {
            zone: Some(Zone::Hand),
            owner: Some(PlayerFilter::You),
            any_of: vec![self.filter.clone()],
            ..ObjectFilter::default()
        };
        let mut choose = crate::effects::ChooseObjectsEffect::new(
            filter,
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            SELECTED,
        )
        .with_description("choose a card to exile and imprint");
        choose.public_selection = true;
        let mut exile = crate::effects::ExileEffect::with_spec(ChooseSpec::tagged(SELECTED));
        exile.remember_on_source = true;
        Some(Effect::new(crate::effects::composition::ScopedProgram {
            players: vec![],
            object_tags: vec![SELECTED.into(), EXILED.into()],
            effects: vec![
                Effect::new(choose),
                Effect::new(crate::effects::composition::OutcomeBranchProgram {
                    producer: Effect::new(crate::effects::composition::ExactTaggedObjects {
                        tag: SELECTED.into(),
                        zone: Zone::Hand,
                    }),
                    test: crate::effects::composition::OutcomeBranchTest::HasObjects,
                    if_true: vec![Effect::new(exile).tag(EXILED)],
                    if_false: vec![],
                }),
            ],
            result_objects: (EXILED.into(), Zone::Exile),
            retain_result_snapshots: true,
        }))
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.canonical_program()
            .unwrap()
            .0
            .execute_child_with_outputs(game, ctx)
    }
}

#[cfg(test)]
mod composition_regressions {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::{Effect, OutcomeValue};
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::{CardId, PlayerId};

    #[test]
    fn hand_selection_keeps_authored_filter_constraints() {
        for filter in [
            ObjectFilter::default().in_zone(Zone::Graveyard),
            ObjectFilter::default().owned_by(PlayerFilter::Opponent),
        ] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let card = CardBuilder::new(CardId::new(), "Imprint filter fixture").build();
            let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
            let hand = game.create_object_from_card(&card, alice, Zone::Hand);
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ImprintFromHandEffect::new(filter)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.object(hand).unwrap().zone, Zone::Hand);
            assert!(game.get_imprinted_cards(source).is_empty());
        }
    }

    #[test]
    fn original_exile_result_survives_an_added_move() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Imprint receipt fixture").build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let hand = game.create_object_from_card(&card, alice, Zone::Hand);
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(hand),
                    Some(Zone::Hand),
                    Some(Zone::Exile),
                ),
                ReplacementAction::Additionally(vec![Effect::new(
                    crate::effects::MoveToZoneEffect::new(
                        ChooseSpec::tagged("it"),
                        Zone::Graveyard,
                        false,
                    ),
                )]),
            ));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = ImprintFromHandEffect::new(ObjectFilter::default())
            .execute(&mut game, &mut ctx)
            .unwrap();
        let OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected original exile receipt");
        };
        assert_eq!(ids.len(), 1);
        assert!(
            game.object(ids[0]).is_none(),
            "summary names the original arrival, not its successor"
        );
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        assert!(game.exile.is_empty());
    }
}
