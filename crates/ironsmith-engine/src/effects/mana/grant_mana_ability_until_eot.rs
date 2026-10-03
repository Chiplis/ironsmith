//! Grant a temporary mana ability to a player until end of turn.
//!
//! This models effects like Channel:
//! "Until end of turn, any time you could activate a mana ability, you may pay 1 life.
//! If you do, add {C}."

use crate::ability::ActivatedAbility;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, GrantedManaAbility};

#[derive(Debug, Clone, PartialEq)]
pub struct GrantManaAbilityUntilEotEffect {
    pub ability: ActivatedAbility,
}

impl GrantManaAbilityUntilEotEffect {
    pub fn new(ability: ActivatedAbility) -> Self {
        Self { ability }
    }
}

impl EffectExecutor for GrantManaAbilityUntilEotEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        crate::ability::visit_activated_owned_effects(&self.ability, visitor);
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let expires_end_of_turn = game.turn.turn_number;
        game.effect_store
            .granted_mana_abilities
            .push(GrantedManaAbility {
                controller: ctx.controller,
                ability: self.ability.clone(),
                expires_end_of_turn,
            });
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod owned_cost_tests {
    use super::*;
    #[test]
    fn retained_effect_model_mana_grant_alternative_costs_preserve_all_template_branches() {
        fn collect(effect: &crate::effect::Effect, ids: &mut Vec<crate::CardId>) {
            effect.visit_card_definitions(&mut |definition| ids.push(definition.card.id));
            effect.visit_child_effects(&mut |child| collect(child, ids));
        }
        let templates = ["First native payment", "Second native payment"].map(|name| {
            crate::cards::builders::CardDefinitionBuilder::new(crate::CardId::new(), name)
                .token()
                .card_types(vec![crate::types::CardType::Creature])
                .flying()
                .build()
        });
        let ids = templates.each_ref().map(|definition| definition.card.id);
        let first_flying = templates[0]
            .abilities
            .iter()
            .find_map(|ability| {
                if let crate::ability::AbilityKind::Static(value) = &ability.kind {
                    Some(value.instance_id())
                } else {
                    None
                }
            })
            .unwrap();
        let branches = templates
            .into_iter()
            .map(|definition| {
                let effect = crate::effect::Effect::new(crate::effects::MayEffect::new(vec![
                    crate::effect::Effect::new(crate::effects::CreateTokenEffect::one(definition)),
                ]));
                crate::cost::TotalCost::from_costs(vec![
                    crate::costs::Cost::life(1),
                    crate::costs::Cost::from_model(ironsmith_core::Cost::Effect(effect)).unwrap(),
                ])
            })
            .collect();
        let ability = crate::ability::Ability::activated(
            crate::cost::TotalCost::one_of(branches),
            Vec::<crate::effect::Effect>::new(),
        );
        let crate::ability::AbilityKind::Activated(mut ability) = ability.kind else {
            panic!("activated prototype")
        };

        let spend_template = crate::cards::builders::CardDefinitionBuilder::new(
            crate::CardId::new(),
            "Native mana spend template",
        )
        .token()
        .card_types(vec![crate::types::CardType::Creature])
        .flying()
        .build();
        let spend_id = spend_template.card.id;
        let spend_identity = spend_template
            .abilities
            .iter()
            .find_map(|ability| {
                if let crate::ability::AbilityKind::Static(value) = &ability.kind {
                    Some(value.instance_id())
                } else {
                    None
                }
            })
            .unwrap();
        ability.mana_usage_restrictions.push(
            crate::ability::ManaUsageRestriction::PaymentTransaction {
                restriction: None,
                on_spend: vec![crate::ability::ManaSpendPayload {
                    predicate: crate::ability::ManaPaymentPredicate::Any,
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        crate::effect::Effect::new(crate::effects::CreateTokenEffect::one(
                            spend_template,
                        )),
                    ]),
                    choices: vec![],
                }],
            },
        );

        ability.mana_output = Some(vec![crate::mana::ManaSymbol::Green]);
        let effect = crate::effect::Effect::new(GrantManaAbilityUntilEotEffect::new(ability));
        let mut reached = Vec::new();
        collect(&effect, &mut reached);
        assert_eq!(
            reached,
            vec![ids[0], ids[1], spend_id],
            "all alternative payment templates are reached in model order"
        );
        let alice = crate::PlayerId::from_index(0);
        let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source_definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::CardId::new(),
            "Mana grant source",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .build();
        let source =
            game.create_object_from_definition(&source_definition, alice, crate::Zone::Battlefield);
        let mut context = crate::effects::EffectContext::new_default(source, alice);
        crate::effects::execute_effect(&mut game, &effect, &mut context).unwrap();
        let retained = game.effect_store.granted_mana_abilities[0].ability.clone();
        assert_eq!(
            retained.mana_output,
            Some(vec![crate::mana::ManaSymbol::Green])
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut cost_context = crate::costs::CostContext::new(source, alice, &mut dm);
        for cost in retained.mana_cost.as_one_of().unwrap()[0].costs() {
            cost.pay(&mut game, &mut cost_context).unwrap();
        }

        let crate::ability::ManaUsageRestriction::PaymentTransaction { on_spend, .. } =
            &retained.mana_usage_restrictions[0]
        else {
            panic!("retained mana spend payload")
        };
        let template = &on_spend[0].effects.all_effects()[0]
            .downcast_ref::<crate::effects::CreateTokenEffect>()
            .unwrap()
            .token;
        assert_eq!(template.card.id, spend_id);
        assert!(template.abilities.iter().any(|ability|matches!(&ability.kind,crate::ability::AbilityKind::Static(value)if value.instance_id()==spend_identity)));

        assert_eq!(game.player(alice).unwrap().life, 19);
        assert_eq!(game.battlefield.len(), 2);
        let token = game.object(game.battlefield[1]).unwrap();
        assert_eq!(token.card, None, "created tokens are not physical cards");
        assert_eq!(token.name, "First native payment");
        assert!(token.abilities.iter().any(|ability|matches!(&ability.kind,crate::ability::AbilityKind::Static(value)if value.instance_id()==first_flying)));
    }
}
