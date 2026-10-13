//! Variable casualty support for planeswalker spells such as "Casualty X".

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
#[cfg(test)]
use crate::snapshot::ObjectSnapshot;
use crate::types::Supertype;
#[cfg(test)]
use crate::types::CardType;

pub type VariableCasualtyPlaneswalkerCopyEffect =
    ironsmith_core::VariableCasualtyPlaneswalkerCopyEffect;

impl EffectExecutor for VariableCasualtyPlaneswalkerCopyEffect {
    fn canonical_program(&self) -> Option<crate::effect::Effect> {
        const POWER: crate::effect::EffectId = crate::effect::EffectId(0xFFFF_FD02);
        Some(crate::effect::Effect::new(
            crate::effects::composition::receipt_binding::WithOptionalCostPower {
                cost_ref: self.cost_ref.clone(),
                result: POWER,
                body: crate::effect::Effect::new(
                    crate::effects::CopySpellEffect::single(crate::target::ChooseSpec::Source)
                        .removed_supertype(Supertype::Legendary)
                        .with_set_base_loyalty(crate::effect::Value::EffectValue(POWER)),
                ),
            },
        ))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.canonical_program().expect("receipt-bound copy program").0.execute(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::StackEntry;
    use crate::{CardId, PlayerId, Zone};

    #[test]
    fn real_cast_payment_records_only_the_casualty_sacrifice() {
        use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
        struct PayOptional;
        impl DecisionMaker for PayOptional {
            fn decide_options(
                &mut self,
                game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                if ctx.description.starts_with("Choose optional costs for") {
                    return ctx
                        .options
                        .iter()
                        .filter(|option| option.legal)
                        .take(1)
                        .map(|option| option.index)
                        .collect();
                }
                SelectFirstDecisionMaker.decide_options(game, ctx)
            }
            fn decide_objects(
                &mut self,
                game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<crate::ids::ObjectId> {
                SelectFirstDecisionMaker.decide_objects(game, ctx)
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Paid copy fixture")
                .card_types(vec![CardType::Planeswalker])
                .build(),
            player,
            Zone::Hand,
        );
        for power in [2, 7] {
            game.create_object_from_card(
                &CardBuilder::new(CardId::new(), "Cost creature")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(power, power))
                    .build(),
                player,
                Zone::Battlefield,
            );
        }
        let cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(
            crate::target::ObjectFilter::creature().you_control(),
        ));
        let spell = game.object_mut(source).unwrap();
        spell.additional_cost = cost.clone().into();
        let cost_ref = crate::cost::OptionalCostRef::with_discriminator(
            ironsmith_core::OptionalCostKind::Casualty,
            "printed-0",
        );
        let mut optional = crate::cost::OptionalCost::custom("Casualty", cost);
        optional.reference = cost_ref.clone();
        spell.optional_costs.push(optional);
        let provenance = ExecutionContext::new_default(source, player).provenance;
        let cast = crate::game_loop::cast_spell_from_resolving_effect_with_outputs(
            &mut game,
            source,
            Zone::Hand,
            player,
            &crate::alternative_cast::CastingMethod::Normal,
            true,
            None,
            provenance,
            &mut PayOptional,
        )
        .unwrap()
        .expect("the cast completes");
        let spell = game.object(cast.new_id).unwrap();
        assert_eq!(
            spell
                .cast_tagged_objects
                .get(&ironsmith_core::tag::optional_cost_sacrifice_receipt(
                    &cost_ref
                ))
                .expect("a typed payment receipt")
                .iter()
                .map(|snapshot| snapshot.power)
                .collect::<Vec<_>>(),
            vec![Some(7)]
        );
        let mut ctx = ExecutionContext::new_default(cast.new_id, player);
        let copied = VariableCasualtyPlaneswalkerCopyEffect::new()
            .with_cost_ref(cost_ref)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            game.object(copied.objects().unwrap()[0])
                .unwrap()
                .base_loyalty,
            Some(7)
        );
    }

    #[test]
    fn casualty_uses_its_actual_receipt_and_the_override_remains_copiable() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Copy fixture")
                .card_types(vec![CardType::Planeswalker])
                .supertypes(vec![Supertype::Legendary])
                .build(),
            player,
            Zone::Stack,
        );
        game.object_mut(source).unwrap().base_loyalty = Some(3);
        game.stack.push(StackEntry::new(source, player));
        let creature = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Paid creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(7, 7))
                .build(),
            player,
            Zone::Battlefield,
        );
        let paid = ObjectSnapshot::from_object(game.object(creature).unwrap(), &game);
        let mut unrelated = paid.clone();
        unrelated.power = Some(2);
        let mut ctx = ExecutionContext::new_default(source, player);
        ctx.set_tagged_objects("sacrifice_cost_0", vec![unrelated.clone()]);
        ctx.set_tagged_objects(
            ironsmith_core::tag::SacrificeCostTag::OriginalResult(0).key(),
            vec![unrelated],
        );
        assert!(
            VariableCasualtyPlaneswalkerCopyEffect::new()
                .execute(&mut game, &mut ctx)
                .unwrap()
                .objects()
                .is_none()
        );
        ctx.set_tagged_objects(
            ironsmith_core::tag::optional_cost_sacrifice_receipt(&"Casualty".into()),
            vec![paid],
        );
        let copied = VariableCasualtyPlaneswalkerCopyEffect::new()
            .execute(&mut game, &mut ctx)
            .unwrap();
        let copy = game.object(copied.objects().unwrap()[0]).unwrap().clone();
        assert_eq!(copy.base_loyalty, Some(7));
        assert!(!copy.supertypes.contains(&Supertype::Legendary));
        assert!(
            !copy
                .counters
                .contains_key(&crate::object::CounterType::Loyalty)
        );
        let second = crate::object::Object::spell_copy_of(&copy, game.new_object_id(), player);
        assert_eq!(second.base_loyalty, Some(7));
    }
    #[test]
    fn canonical_and_direct_copy_bind_the_same_printed_receipt_and_restore_x() {
        for canonical in [false, true] {
            for live_receipt in [false, true] {
                let mut game = crate::tests::test_helpers::setup_two_player_game();
                let player = PlayerId(0);
                let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Receipt copy")
                    .card_types(vec![CardType::Planeswalker]).supertypes(vec![Supertype::Legendary]).build(), player, Zone::Stack);
                game.stack.push(StackEntry::new(source, player));
                let creature = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Payment")
                    .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(7, 7)).build(), player, Zone::Battlefield);
                let receipt = ObjectSnapshot::from_object(game.object(creature).unwrap(), &game);
                let reference = crate::cost::OptionalCostRef::with_discriminator(ironsmith_core::OptionalCostKind::Casualty, "printed-1");
                let key = ironsmith_core::tag::optional_cost_sacrifice_receipt(&reference);
                let effect = VariableCasualtyPlaneswalkerCopyEffect::new().with_cost_ref(reference);
                let mut ctx = ExecutionContext::new_default(source, player).with_x(42);
                ctx.store_outcome(crate::effect::EffectId(0xFFFF_FD02), EffectOutcome::count(99));
                let missing = if canonical { effect.canonical_program().unwrap().0.execute(&mut game, &mut ctx) }
                    else { effect.execute(&mut game, &mut ctx) }.unwrap();
                assert!(missing.objects().is_none());
                if live_receipt { game.object_mut(source).unwrap().cast_tagged_objects.insert(key, vec![receipt]); }
                else { ctx.set_tagged_objects(key, vec![receipt]); }
                let copied = if canonical { effect.canonical_program().unwrap().0.execute(&mut game, &mut ctx) }
                    else { effect.execute(&mut game, &mut ctx) }.unwrap();
                assert_eq!(game.object(copied.objects().unwrap()[0]).unwrap().base_loyalty, Some(7));
                assert_eq!(ctx.x_value, Some(42));
                assert_eq!(ctx.get_outcome(crate::effect::EffectId(0xFFFF_FD02)).unwrap().count_or_zero(), 99);
            }
        }
    }

}
