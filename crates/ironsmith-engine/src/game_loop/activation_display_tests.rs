use super::priority_cast::*;
use crate::ability::Ability;
use crate::cards::CardDefinitionBuilder;
use crate::decision::SelectFirstDecisionMaker;
use crate::effect::{Effect, Until};
use crate::ids::CardId;

#[test]
fn activation_target_prompt_quotes_the_selected_ability() {
    let damage_text = "{R}, Sacrifice Goblin Legionnaire: It deals 2 damage to any target.";
    let prevention_text = "{W}, Sacrifice Goblin Legionnaire: Prevent the next 2 damage that would be dealt to any target this turn.";
    for (index, expected, target_description) in [
        (0, damage_text, "target for damage"),
        (1, prevention_text, "target to protect"),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let programs = [
            vec![Effect::deal_damage(2, ChooseSpec::AnyTarget)],
            vec![Effect::prevent_damage(
                2,
                ChooseSpec::AnyTarget,
                Until::EndOfTurn,
            )],
        ];
        let mut definition = CardDefinitionBuilder::new(CardId::new(), "Goblin Legionnaire")
            .card_types(vec![CardType::Creature])
            .with_ability(Ability::activated(
                crate::cost::TotalCost::free(),
                programs[0].clone(),
            ))
            .with_ability(Ability::activated(
                crate::cost::TotalCost::free(),
                programs[1].clone(),
            ))
            .build();
        definition.canonical_text = format!("{damage_text}\n{prevention_text}");
        definition.ability_labels = vec![damage_text.into(), prevention_text.into()];
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let requirements =
            extract_target_requirements(&game, &programs[index], alice, Some(source));
        let pending = PendingActivation::new(
            source,
            index,
            None,
            alice,
            ProvNodeId::default(),
            ActivationStage::ChoosingTargets,
            programs[index].clone().into(),
            requirements,
            None,
            vec![],
            crate::costs::PaymentReason::ActivateAbility,
            vec![],
            vec![],
            Default::default(),
            0,
            false,
            false,
            snapshot.stable_id,
            snapshot,
            "Goblin Legionnaire".into(),
            None,
            false,
            false,
            vec![],
            None,
            vec![],
        );
        let mut state = PriorityLoopState::new(2);
        let progress = continue_activation(
            &mut game,
            &mut TriggerQueue::new(),
            &mut state,
            pending,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("expected target selection");
        };
        let context = crate::decisions::context::enrich_display_hints(&game, context);
        assert_eq!(
            context.context_text(),
            Some(expected.trim_end_matches('.')),
            "the prompt must quote only ability {index}"
        );
        let crate::decisions::context::DecisionContext::Targets(targets) = context else {
            panic!("expected targets");
        };
        assert_eq!(targets.requirements[0].description, target_description);
        assert!(
            targets.requirements[0]
                .legal_targets
                .contains(&Target::Player(PlayerId::from_index(1)))
        );
        assert_eq!(
            state.pending_activation.as_ref().unwrap().ability_index,
            index
        );
    }
}
