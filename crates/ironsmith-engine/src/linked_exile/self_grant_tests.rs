use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::continuous::{EffectTarget, Modification};
use crate::effects::{ApplyContinuousEffect, ExecutionContext};
use crate::{Ability, CardId, CardType, Effect, GameState, PlayerId, TotalCost, Until, Zone};

#[test]
fn only_printed_self_grants_inherit_the_captured_pair_owner() {
    for printed in [false, true] {
        for to_source in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let alice = PlayerId::from_index(0);
            let definition = CardDefinitionBuilder::new(CardId::new(), "Grant owner")
                .card_types(vec![CardType::Artifact])
                .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let other = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let pair = LinkedExilePair {
                definition: ironsmith_core::LinkedExileDefinition([84; 32]),
                pair: 0,
            };
            let owner = LinkedExileOwner {
                host: source,
                pair,
                acquisition: if printed {
                    LinkedExileAcquisition::Printed
                } else {
                    LinkedExileAcquisition::EffectGroup(
                        game.effect_store.continuous_effects.next_effect_group_id(),
                    )
                },
            };
            let reader = Ability::activated(
                TotalCost::free(),
                crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life(1)])
                    .with_linked_exile_pair(pair),
            );
            let effect = Effect::new(ApplyContinuousEffect::new(
                if to_source {
                    EffectTarget::Source
                } else {
                    EffectTarget::Specific(other)
                },
                Modification::AddAbilityGeneric(reader),
                Until::Forever,
            ));
            let mut ctx = ExecutionContext::new_default(source, alice)
                .with_linked_exile_owner(Some(owner.clone()));
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
            let host = if to_source { source } else { other };
            let chars = game.current_characteristics(host).unwrap();
            let origin = chars.abilities.origin(0).unwrap();
            assert!(matches!(origin, AbilityOrigin::Effect { .. }));
            let acquired = LinkedExileOwner::capture(host, Some(pair), Some(origin)).unwrap();
            assert_eq!(acquired == owner, printed && to_source);
            let wrong_pair = LinkedExilePair { pair: 1, ..pair };
            assert_ne!(
                LinkedExileOwner::capture(host, Some(wrong_pair), Some(origin))
                    .unwrap()
                    .acquisition,
                LinkedExileAcquisition::Printed
            );
        }
    }
}
