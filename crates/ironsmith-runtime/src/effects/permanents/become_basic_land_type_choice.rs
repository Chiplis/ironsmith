//! Basic land type transformation effect.
//!
//! Used for cards like Grixis Illusionist:
//! "{T}: Target land becomes the basic land type of your choice until end of turn."
//!
//! Also supports fixed-subtype variants such as:
//! "{T}: Target land becomes an Island until end of turn."

use crate::ability::Ability;
use crate::continuous::Modification;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::Subtype;

/// Effect: target land becomes one basic land type of the controller's choice.
#[derive(Debug, Clone, PartialEq)]
pub struct BecomeBasicLandTypeChoiceEffect {
    pub target: ChooseSpec,
    pub until: Until,
    pub chooser: PlayerFilter,
    pub fixed_subtype: Option<Subtype>,
}

impl BecomeBasicLandTypeChoiceEffect {
    pub fn new(target: ChooseSpec, until: Until) -> Self {
        Self {
            target,
            until,
            chooser: PlayerFilter::You,
            fixed_subtype: None,
        }
    }

    pub fn fixed(target: ChooseSpec, subtype: Subtype, until: Until) -> Self {
        Self {
            target,
            until,
            chooser: PlayerFilter::You,
            fixed_subtype: Some(subtype),
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    fn subtype_options() -> [(Subtype, ManaSymbol, &'static str); 5] {
        [
            (Subtype::Plains, ManaSymbol::White, "Plains"),
            (Subtype::Island, ManaSymbol::Blue, "Island"),
            (Subtype::Swamp, ManaSymbol::Black, "Swamp"),
            (Subtype::Mountain, ManaSymbol::Red, "Mountain"),
            (Subtype::Forest, ManaSymbol::Green, "Forest"),
        ]
    }

    fn mana_ability_for(subtype: Subtype) -> Ability {
        Ability::basic_land_mana(subtype).expect("basic land subtype should map to a mana ability")
    }
}

impl EffectExecutor for BecomeBasicLandTypeChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (subtype, _, _) = if let Some(subtype) = self.fixed_subtype {
            Self::subtype_options()
                .into_iter()
                .find(|(candidate, _, _)| *candidate == subtype)
                .expect("fixed basic land subtype must be one of the five basic land types")
        } else {
            let chooser = resolve_player_filter(game, &self.chooser, ctx)?;

            let options: Vec<SelectableOption> = Self::subtype_options()
                .iter()
                .enumerate()
                .map(|(idx, (_, _, label))| SelectableOption::new(idx, *label))
                .collect();
            let choice_ctx = SelectOptionsContext::new(
                chooser,
                Some(ctx.source),
                "Choose a basic land type",
                options,
                1,
                1,
            );
            let chosen = ctx
                .decision_maker
                .decide_options(game, &choice_ctx)
                .into_iter()
                .next();
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let Some(chosen) = chosen.filter(|idx| *idx < Self::subtype_options().len()) else {
                return Ok(EffectOutcome::count(0));
            };

            Self::subtype_options()[chosen]
        };
        let mana_ability = Self::mana_ability_for(subtype);

        let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
            self.target.clone(),
            Modification::SetSubtypes(vec![subtype]),
            self.until.clone(),
        );
        // Rule intent: type change replaces the land's abilities with the basic land mana ability.
        apply = apply.with_additional_modification(Modification::SetAbilities(vec![mana_ability]));

        apply.execute(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    struct ChooseIslandDm;
    impl DecisionMaker for ChooseIslandDm {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            // Island option index in BecomeBasicLandTypeChoiceEffect::subtype_options()
            let _ = ctx;
            vec![1]
        }
    }

    #[test]
    fn become_basic_land_type_choice_sets_subtype_and_replaces_mana_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let land_def = CardDefinitionBuilder::new(CardId::new(), "Weird Land")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Desert])
            .parse_text("{T}: Add {C}{C}.")
            .expect("land text should parse");

        let land_id = game.create_object_from_definition(&land_def, alice, Zone::Battlefield);
        let source = game.new_object_id();

        let mut dm = ChooseIslandDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = BecomeBasicLandTypeChoiceEffect::new(
            ChooseSpec::SpecificObject(land_id),
            Until::EndOfTurn,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("execute become basic land type choice");

        let subtypes = game.calculated_subtypes(land_id);
        assert!(
            subtypes.contains(&Subtype::Island),
            "expected land to be an Island, got {subtypes:?}"
        );

        let chars = game
            .calculated_characteristics(land_id)
            .expect("calculate characteristics");
        let mana_symbols: Vec<Vec<ManaSymbol>> = chars
            .abilities
            .iter()
            .filter_map(|a| match &a.kind {
                AbilityKind::Activated(act) if act.is_mana_ability() => {
                    Some(act.mana_symbols().to_vec())
                }
                _ => None,
            })
            .collect();

        assert!(
            mana_symbols
                .iter()
                .any(|syms| syms == &vec![ManaSymbol::Blue]),
            "expected island mana ability, got {mana_symbols:?}"
        );
        assert!(
            !mana_symbols
                .iter()
                .any(|syms| syms == &vec![ManaSymbol::Colorless, ManaSymbol::Colorless]),
            "expected old {{C}}{{C}} mana ability to be removed, got {mana_symbols:?}"
        );
    }

    #[test]
    fn fixed_basic_land_type_sets_subtype_and_replaces_mana_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let land_def = CardDefinitionBuilder::new(CardId::new(), "Weird Land")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Desert])
            .parse_text("{T}: Add {C}{C}.")
            .expect("land text should parse");

        let land_id = game.create_object_from_definition(&land_def, alice, Zone::Battlefield);
        let source = game.new_object_id();

        let mut dm = ChooseIslandDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = BecomeBasicLandTypeChoiceEffect::fixed(
            ChooseSpec::SpecificObject(land_id),
            Subtype::Forest,
            Until::EndOfTurn,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("execute fixed become basic land type");

        let subtypes = game.calculated_subtypes(land_id);
        assert!(
            subtypes.contains(&Subtype::Forest),
            "expected land to be a Forest, got {subtypes:?}"
        );

        let chars = game
            .calculated_characteristics(land_id)
            .expect("calculate characteristics");
        let mana_symbols: Vec<Vec<ManaSymbol>> = chars
            .abilities
            .iter()
            .filter_map(|a| match &a.kind {
                AbilityKind::Activated(act) if act.is_mana_ability() => {
                    Some(act.mana_symbols().to_vec())
                }
                _ => None,
            })
            .collect();

        assert!(
            mana_symbols
                .iter()
                .any(|syms| syms == &vec![ManaSymbol::Green]),
            "expected forest mana ability, got {mana_symbols:?}"
        );
        assert!(
            !mana_symbols
                .iter()
                .any(|syms| syms == &vec![ManaSymbol::Colorless, ManaSymbol::Colorless]),
            "expected old {{C}}{{C}} mana ability to be removed, got {mana_symbols:?}"
        );
    }
}
