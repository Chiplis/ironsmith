//! Basic land type transformation effect.
//!
//! Used for cards like Grixis Illusionist:
//! "{T}: Target land becomes the basic land type of your choice until end of turn."
//!
//! Also supports fixed-subtype variants such as:
//! "{T}: Target land becomes an Island until end of turn."

use crate::continuous::Modification;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::types::Subtype;

/// Effect: target land becomes one basic land type of the controller's choice.
pub type BecomeBasicLandTypeChoiceEffect = ironsmith_core::BecomeBasicLandTypeChoiceEffect;

fn subtype_options() -> [(Subtype, ManaSymbol, &'static str); 5] {
    [
        (Subtype::Plains, ManaSymbol::White, "Plains"),
        (Subtype::Island, ManaSymbol::Blue, "Island"),
        (Subtype::Swamp, ManaSymbol::Black, "Swamp"),
        (Subtype::Mountain, ManaSymbol::Red, "Mountain"),
        (Subtype::Forest, ManaSymbol::Green, "Forest"),
    ]
}

impl EffectExecutor for BecomeBasicLandTypeChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let options = subtype_options().into_iter().filter(|(subtype, _, _)|
            self.allowed_subtypes.is_empty() || self.allowed_subtypes.contains(subtype)).collect::<Vec<_>>();
        if self.allowed_subtypes.iter().any(|subtype| !subtype.is_basic_land_type()) || options.is_empty() {
            return Err(ExecutionError::InternalError("invalid basic land subtype choice set".into()));
        }
        let (subtype, _, _) = if let Some(subtype) = self.fixed_subtype.or_else(|| (options.len() == 1).then(|| options[0].0)) {
            options.iter().copied()
                .find(|(candidate, _, _)| *candidate == subtype)
                .ok_or_else(|| ExecutionError::InternalError("invalid fixed basic land subtype".into()))?
        } else {
            let chooser = crate::effects::helpers::resolve_player_filter_as_chooser(
                game,
                &self.chooser,
                ctx,
            )?;

            let displayed: Vec<SelectableOption> = options
                .iter()
                .enumerate()
                .map(|(idx, (_, _, label))| SelectableOption::new(idx, *label))
                .collect();
            let choice_ctx = SelectOptionsContext::new(
                chooser,
                Some(ctx.source),
                "Choose a basic land type",
                displayed,
                1,
                1,
            );
            let choices = ctx.decision_maker.decide_options(game, &choice_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let [chosen] = choices.as_slice() else {
                return Err(ExecutionError::InternalError("basic land selection must contain exactly one index".into()));
            };
            let Some(chosen) = Some(*chosen).filter(|idx| *idx < options.len()) else {
                return Err(ExecutionError::InternalError("invalid basic land type selection".into()));
            };

            options[chosen]
        };
        // CR305.7 changes only land subtypes, not Creature/Artifact/etc
        // subtypes. Intrinsic basic-land mana is derived at the layer boundary.
        let apply = if self.preserve_other_types {
            crate::effects::ApplyContinuousEffect::with_spec(self.target.clone(),
                Modification::AddSubtypes(vec![subtype]), self.duration.clone())
        } else {
            crate::effects::ApplyContinuousEffect::with_spec(self.target.clone(),
                Modification::SetSubtypes(vec![subtype]), self.duration.clone())
                .with_additional_modification(Modification::RemoveLandRulesTextAbilities)
        };

        apply.execute(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::ability::AbilityKind;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::ids::{CardId, PlayerId};
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::test_prelude::*;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::types::{CardType, Subtype};
    #[cfg(ironsmith_runtime_parser_tests)]
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

    #[cfg(ironsmith_runtime_parser_tests)]
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

    #[cfg(ironsmith_runtime_parser_tests)]
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

#[cfg(test)]
mod basic_land_rule_boundary_tests {
    use super::*;
    use crate::{CardId,CardType,PlayerId,Zone,Supertype};
    use crate::card::{CardBuilder,PowerToughness};
    use crate::ability::Ability;
    use crate::continuous::{EffectTarget,Modification};
    use crate::decision::{DecisionMaker,SelectFirstDecisionMaker};
    use crate::effects::{ApplyContinuousEffect,EffectContext,EffectExecutor};
    use crate::effect::Until;
    use crate::static_abilities::{StaticAbility,StaticAbilityId};
    use crate::target::ChooseSpec;
    const A:PlayerId=PlayerId(0);
    fn board() -> (GameState,crate::ObjectId) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let card=CardBuilder::new(CardId::new(),"Printed flying forest")
            .card_types(vec![CardType::Artifact,CardType::Land,CardType::Creature])
            .subtypes(vec![Subtype::Elf,Subtype::Forest,Subtype::Desert])
            .supertypes(vec![Supertype::Snow,Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(3,4)).build();
        let mut definition=crate::cards::CardDefinition::new(card);
        definition.abilities.push(Ability::static_ability(StaticAbility::flying()));
        let id=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        (game,id)
    }
    fn grant(game:&mut GameState,id:crate::ObjectId,ability:StaticAbility) {
        ApplyContinuousEffect::new(EffectTarget::Specific(id),Modification::AddAbility(ability),Until::Forever)
            .execute(game,&mut EffectContext::new_default(id,A)).unwrap();
    }
    #[test]
    fn replacing_basic_land_type_removes_printed_but_never_external_grants_or_other_type_families() {
        let (mut game,id)=board();grant(&mut game,id,StaticAbility::haste());
        BecomeBasicLandTypeChoiceEffect::fixed(ChooseSpec::SpecificObject(id),Subtype::Island,Until::EndOfTurn)
            .execute(&mut game,&mut EffectContext::new_default(id,A)).unwrap();
        grant(&mut game,id,StaticAbility::vigilance());
        let chars=game.calculated_characteristics(id).unwrap();
        for kind in [CardType::Artifact,CardType::Land,CardType::Creature] {assert!(chars.card_types.contains(&kind));}
        for st in [Supertype::Legendary,Supertype::Snow] {assert!(chars.supertypes.contains(&st));}
        assert!(chars.subtypes.contains(&Subtype::Elf)&&chars.subtypes.contains(&Subtype::Island));
        assert!(!chars.subtypes.contains(&Subtype::Forest)&&!chars.subtypes.contains(&Subtype::Desert));
        assert_eq!((chars.power,chars.toughness),(Some(3),Some(4)));
        assert!(!game.current_has_static_ability_id(id,StaticAbilityId::Flying));
        for ab in [StaticAbilityId::Haste,StaticAbilityId::Vigilance] {assert!(game.current_has_static_ability_id(id,ab));}
        assert!(chars.abilities.contains(&Ability::basic_land_mana(Subtype::Island).unwrap()));
        assert!(!chars.abilities.contains(&Ability::basic_land_mana(Subtype::Forest).unwrap()));
        crate::turn::execute_cleanup_step(&mut game);game.refresh_continuous_state().unwrap();
        assert!(game.current_has_static_ability_id(id,StaticAbilityId::Flying));
        assert!(game.current_subtypes(id).unwrap().contains(&Subtype::Desert));
        assert!(game.current_has_static_ability_id(id,StaticAbilityId::Haste));
    }
    #[test]
    fn additional_land_type_keeps_text_old_land_types_and_both_intrinsic_mana_abilities() {
        let (mut game,id)=board();
        BecomeBasicLandTypeChoiceEffect::new(ChooseSpec::SpecificObject(id),Until::EndOfTurn)
            .with_options(vec![Subtype::Island],true)
            .execute(&mut game,&mut EffectContext::new_default(id,A)).unwrap();
        let chars=game.calculated_characteristics(id).unwrap();
        for st in [Subtype::Forest,Subtype::Island,Subtype::Desert,Subtype::Elf] {assert!(chars.subtypes.contains(&st));}
        assert!(game.current_has_static_ability_id(id,StaticAbilityId::Flying));
        for st in [Subtype::Forest,Subtype::Island] {assert!(chars.abilities.contains(&Ability::basic_land_mana(st).unwrap()));}
    }
    struct ExactOptions;
    impl DecisionMaker for ExactOptions {
        fn decide_options(&mut self,_:&GameState,ctx:&SelectOptionsContext)->Vec<usize> {
            assert_eq!(ctx.options.len(),2);assert_eq!(ctx.options[0].description,"Plains");assert_eq!(ctx.options[1].description,"Island");vec![1]
        }
    }
    #[test]
    fn authored_or_alternative_offers_only_the_named_land_types() {
        let (mut game,id)=board();let mut dm=ExactOptions;
        BecomeBasicLandTypeChoiceEffect::new(ChooseSpec::SpecificObject(id),Until::EndOfTurn)
            .with_options(vec![Subtype::Plains,Subtype::Island],false)
            .execute(&mut game,&mut EffectContext::new(id,A,&mut dm)).unwrap();
        assert!(game.current_subtypes(id).unwrap().contains(&Subtype::Island));
        let bad=BecomeBasicLandTypeChoiceEffect::new(ChooseSpec::SpecificObject(id),Until::Forever).with_options(vec![Subtype::Desert],false);
        assert!(bad.execute(&mut game,&mut EffectContext::new(id,A,&mut SelectFirstDecisionMaker)).is_err());
    }
    struct InvalidOptions(Vec<usize>);
    impl DecisionMaker for InvalidOptions {
        fn decide_options(&mut self,_:&GameState,_:&SelectOptionsContext)->Vec<usize> {self.0.clone()}
    }
    #[test]
    fn invalid_multi_missing_and_out_of_range_choices_do_not_change_the_land() {
        for choices in [vec![],vec![0,1],vec![1,1],vec![5]] {
            let (mut game,id)=board();let before=game.current_subtypes(id);let mut dm=InvalidOptions(choices);
            let effect=BecomeBasicLandTypeChoiceEffect::new(ChooseSpec::SpecificObject(id),Until::Forever);
            assert!(effect.execute(&mut game,&mut EffectContext::new(id,A,&mut dm)).is_err());
            assert_eq!(game.current_subtypes(id),before);
        }
    }

    #[test]
    fn independently_registered_grants_survive_land_rules_loss_but_printed_restrictions_do_not() {
        let (mut game,id)=board();
        game.object_mut(id).unwrap().abilities_mut().push(Ability::static_ability(StaticAbility::cant_attack()));
        game.grant_temporary_static_ability_to_object_until_end_of_turn(id,StaticAbilityId::Haste);
        game.refresh_continuous_state().unwrap();assert!(!game.can_attack(id));
        BecomeBasicLandTypeChoiceEffect::fixed(ChooseSpec::SpecificObject(id),Subtype::Island,Until::EndOfTurn)
            .execute(&mut game,&mut EffectContext::new_default(id,A)).unwrap();
        game.refresh_continuous_state().unwrap();assert!(game.can_attack(id));
        assert!(game.current_has_static_ability_id(id,StaticAbilityId::Haste));
        assert!(!game.current_has_static_ability_id(id,StaticAbilityId::Flying));
        let chars=game.calculated_characteristics(id).unwrap();
        assert!(chars.abilities.iter().enumerate().any(|(slot,ability)|
            matches!(ability.kind,crate::ability::AbilityKind::Static(ref a) if a.id()==StaticAbilityId::Haste)
                && matches!(chars.abilities.origin(slot),Some(crate::continuous::AbilityOrigin::Temporary(_)))));
        crate::turn::execute_cleanup_step(&mut game);game.refresh_continuous_state().unwrap();
        assert!(!game.can_attack(id));assert!(!game.current_has_static_ability_id(id,StaticAbilityId::Haste));
    }
    #[test]
    fn copy_base_preserves_registered_grants_but_copied_text_is_still_lost_to_basic_land_replacement() {
        let (mut game,id)=board();
        let donor_card=CardBuilder::new(CardId::new(),"Copied land text")
            .card_types(vec![CardType::Land,CardType::Creature]).subtypes(vec![Subtype::Forest,Subtype::Elf])
            .power_toughness(PowerToughness::fixed(3,4)).build();
        let mut donor_def=crate::cards::CardDefinition::new(donor_card);
        donor_def.abilities.push(Ability::static_ability(StaticAbility::vigilance()));
        let donor=game.create_object_from_definition(&donor_def,A,Zone::Battlefield);
        game.grant_temporary_static_ability_to_object_until_end_of_turn(id,StaticAbilityId::Haste);
        ApplyContinuousEffect::new_runtime(EffectTarget::Specific(id),crate::effects::continuous::RuntimeModification::CopyOf {
            source:ChooseSpec::SpecificObject(donor),preserve_source_abilities:false,name_override:None,
            name_override_surface:None,add_supertypes:vec![],copy_exception_surface:None,
        },Until::Forever).execute(&mut game,&mut EffectContext::new_default(id,A)).unwrap();
        assert!(game.current_has_static_ability_id(id,StaticAbilityId::Haste));
        assert!(game.current_has_static_ability_id(id,StaticAbilityId::Vigilance));
        BecomeBasicLandTypeChoiceEffect::fixed(ChooseSpec::SpecificObject(id),Subtype::Island,Until::Forever)
            .execute(&mut game,&mut EffectContext::new_default(id,A)).unwrap();
        assert!(game.current_has_static_ability_id(id,StaticAbilityId::Haste));
        assert!(!game.current_has_static_ability_id(id,StaticAbilityId::Vigilance));
        ApplyContinuousEffect::new(EffectTarget::Specific(id),Modification::RemoveAllAbilities,Until::EndOfTurn)
            .execute(&mut game,&mut EffectContext::new_default(id,A)).unwrap();
        assert!(!game.current_has_static_ability_id(id,StaticAbilityId::Haste),"ordinary layer6 removal still removes registered grants");
    }

}
