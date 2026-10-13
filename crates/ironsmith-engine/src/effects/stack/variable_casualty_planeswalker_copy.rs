//! Variable casualty support for planeswalker spells such as "Casualty X".

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::types::{CardType, Supertype};

pub type VariableCasualtyPlaneswalkerCopyEffect =
    ironsmith_core::VariableCasualtyPlaneswalkerCopyEffect;

/// The creature sacrificed for the casualty cost while the spell was cast
/// (CR 702.153a: the sacrifice is an additional cost, 601.2f-h). Spell-cost
/// sacrifices are recorded under a typed optional-cost receipt binding on the spell.
fn casualty_sacrificed_creature<'a>(
    game: &'a GameState,
    ctx: &'a ExecutionContext,
    cost_ref: &crate::cost::OptionalCostRef,
) -> Option<&'a ObjectSnapshot> {
    let receipt = ironsmith_core::tag::optional_cost_sacrifice_receipt(cost_ref);
    let from_tags =
        |tags: &'a std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>| {
            tags.get(&receipt)?
                .iter()
                .find(|snapshot| snapshot.card_types.contains(&CardType::Creature))
        };
    from_tags(&ctx.tagged_objects).or_else(|| {
        game.object(ctx.source)
            .and_then(|spell| from_tags(&spell.cast_tagged_objects))
            .or_else(|| {
                game.turn_store
                    .cast_spell_lki
                    .get(&ctx.source)
                    .and_then(|lki| from_tags(&lki.0.cast_tagged_objects))
            })
    })
}

impl EffectExecutor for VariableCasualtyPlaneswalkerCopyEffect {
    fn canonical_program(&self) -> Option<crate::effect::Effect> {
        Some(crate::effect::Effect::new(
            crate::effects::CopySpellEffect::single(crate::target::ChooseSpec::Source)
                .removed_supertype(Supertype::Legendary)
                .with_set_base_loyalty(crate::effect::Value::PowerOf(Box::new(
                    crate::target::ChooseSpec::tagged(
                        ironsmith_core::tag::optional_cost_sacrifice_receipt(&self.cost_ref),
                    ),
                ))),
        ))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // The copy's starting loyalty is the sacrificed creature's power as it
        // last existed on the battlefield.
        let Some(sacrificed) = casualty_sacrificed_creature(game, ctx, &self.cost_ref) else {
            return Ok(EffectOutcome::impossible());
        };
        let loyalty = sacrificed.power.unwrap_or(0).max(0) as u32;

        crate::effects::CopySpellEffect::single(crate::target::ChooseSpec::Source)
            .removed_supertype(Supertype::Legendary)
            .with_set_base_loyalty(loyalty)
            .execute(game, ctx)
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
}
