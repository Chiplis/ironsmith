use crate::cards::ParseAnnotations;
#[cfg(test)]
use crate::cards::builders::CardDefinition;
use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
use crate::parse_trace;

use super::document_parser;
use super::effect_pipeline::{LoweredCardDocument, NormalizedCardAst, ParsedCardAst};
use super::ir::RewriteSemanticDocument;
use super::lower;

pub(crate) fn parse_text_to_semantic_document(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(RewriteSemanticDocument, ParseAnnotations), CardTextError> {
    document_parser::parse_text_to_semantic_document(builder, text, allow_unsupported)
}

pub(crate) fn parse_semantic_document(
    doc: RewriteSemanticDocument,
) -> Result<ParsedCardAst, CardTextError> {
    super::semantic_document::parse_semantic_document(doc)
}

pub(crate) fn prepare_parsed_document(
    ast: ParsedCardAst,
) -> Result<NormalizedCardAst, CardTextError> {
    lower::prepare_parsed_card_ast_for_lowering(ast)
}

pub(crate) fn lower_prepared_document_with_facts(
    ast: NormalizedCardAst,
) -> Result<LoweredCardDocument, CardTextError> {
    lower::lower_normalized_card_ast_with_facts(ast)
}

#[cfg(test)]
pub(crate) fn parse_text_with_annotations_lowered(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let lowered = parse_text_with_annotations_lowered_with_facts(builder, text, allow_unsupported)?;
    Ok((lowered.definition, lowered.annotations))
}

pub(crate) fn parse_text_with_annotations_lowered_with_facts(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<LoweredCardDocument, CardTextError> {
    let (doc, _) = parse_text_to_semantic_document(builder, text, allow_unsupported)?;
    let parsed = {
        let _scope = parse_trace::scope("semantic parse");
        parse_semantic_document(doc)?
    };
    let prepared = {
        let _scope = parse_trace::scope("prepare lowering input");
        prepare_parsed_document(parsed)?
    };
    let lowered = {
        let _scope = parse_trace::scope("lower runtime definition");
        lower_prepared_document_with_facts(prepared)?
    };
    Ok(lowered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardId;
    use crate::ability::AbilityKind;
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::runtime_backend::effect_pipeline::NormalizedCardItem;
    use crate::runtime_backend::semantic::ParsedCardItem;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn document_semantic_facts_drive_pipeline_rewrites() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Semantic Facts Pipeline")
            .card_types(vec![CardType::Instant]);
        let text = "Return target nonland permanent you don't control to its owner's hand.\nOverload {1}{U}";

        let (semantic, _) = parse_text_to_semantic_document(builder, text.to_string(), false)?;
        assert_eq!(
            semantic.overload_items.as_ref().map(|items| items.len()),
            Some(1)
        );

        let parsed = parse_semantic_document(semantic)?;
        assert_eq!(
            parsed
                .overload_branch
                .as_ref()
                .map(|branch| branch.items.len()),
            Some(1)
        );

        let prepared = prepare_parsed_document(parsed)?;
        assert_eq!(
            prepared
                .overload_branch
                .as_ref()
                .map(|branch| branch.items.len()),
            Some(1)
        );

        let lowered = lower_prepared_document_with_facts(prepared)?;
        let Some(AlternativeCastingMethod::Overload { effects, .. }) =
            lowered.definition.alternative_casts.first()
        else {
            panic!("expected overload alternative cast")
        };
        assert!(!effects.is_empty());
        assert!(!format!("{effects:#?}").contains("Target("));
        Ok(())
    }

    #[test]
    fn time_vault_turn_skip_followup_survives_preparation() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Time Vault Variant");
        let text = "If you would begin your turn while this artifact is tapped, you may skip that turn instead. If you do, untap this artifact.";
        let (semantic, _) = parse_text_to_semantic_document(builder, text.to_string(), false)?;
        let parsed = parse_semantic_document(semantic)?;
        prepare_parsed_document(parsed)?;
        Ok(())
    }

    #[test]
    fn land_type_copy_and_haste_duration_survives_pipeline() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Land Type Copy Variant")
            .card_types(vec![CardType::Sorcery]);
        let text = "Choose a nonbasic land type. Each land you control of that type becomes a copy of target creature you control until end of turn and gains haste until end of turn.";
        let (semantic, _) = parse_text_to_semantic_document(builder, text.to_string(), false)?;
        let parsed = parse_semantic_document(semantic)?;
        let prepared = prepare_parsed_document(parsed)?;
        let lowered = lower_prepared_document_with_facts(prepared)?;
        let spell_effect = lowered
            .definition
            .spell_effect
            .expect("copy-and-haste spell effect");
        let effects = spell_effect.to_vec();
        let haste = effects
            .iter()
            .find_map(|effect| {
                let apply = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
                let is_haste = apply.modification.as_ref().is_some_and(|modification| {
                    matches!(
                        modification,
                        crate::continuous::Modification::AddAbility(ability)
                            if ability.id() == crate::static_abilities::StaticAbilityId::Haste
                    )
                });
                is_haste.then_some(apply)
            })
            .expect("typed haste grant");
        assert_eq!(haste.until, crate::effect::Until::EndOfTurn);
        Ok(())
    }

    #[test]
    fn delayed_schedule_is_typed_before_lowering_finishes() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Typed Delayed Schedule")
            .card_types(vec![CardType::Sorcery]);
        let (definition, _) = parse_text_with_annotations_lowered(
            builder,
            "At the beginning of your next upkeep, draw a card.".to_string(),
            false,
        )?;

        assert!(definition.abilities.is_empty());
        let debug = format!("{definition:#?}");
        assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
        assert!(debug.contains("start_next_turn: true"), "{debug}");
        Ok(())
    }

    #[test]
    fn kicked_counter_replacement_is_typed_before_lowering_finishes() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Typed Kicked Counter")
            .card_types(vec![CardType::Instant]);
        let (definition, _) = parse_text_with_annotations_lowered(
            builder,
            "Kicker {2}\nCounter target spell if its mana value is 3 or less. If this spell was kicked, counter that spell if its mana value is 7 or less instead.".to_string(),
            false,
        )?;

        let spell_effect = definition.spell_effect.expect("expected spell effects");
        let [segment] = spell_effect.segments.as_slice() else {
            panic!("expected one resolution segment, got {spell_effect:#?}");
        };
        let base_conditional = segment
            .default_effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
            .expect("expected the base mana-value gate");
        let crate::effect::Condition::TaggedObjectMatches(base_tag, base_filter) =
            &base_conditional.condition
        else {
            panic!("expected a tagged base spell filter, got {base_conditional:#?}");
        };
        assert!(matches!(
            base_filter.mana_value.as_ref(),
            Some(crate::target::Comparison::LessThanOrEqual(3))
        ));

        let kicked_branch = segment
            .self_replacements
            .iter()
            .find(|branch| branch.condition == crate::effect::Condition::ThisSpellWasKicked)
            .expect("expected kicked replacement branch");
        let conditional = kicked_branch
            .replacement_effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
            .expect("expected a conditional kicked counter effect");
        let crate::effect::Condition::TaggedObjectMatches(kicked_tag, kicked_filter) =
            &conditional.condition
        else {
            panic!("expected a tagged spell filter, got {conditional:#?}");
        };
        assert!(matches!(
            kicked_filter.mana_value.as_ref(),
            Some(crate::target::Comparison::LessThanOrEqual(7))
        ));
        assert_eq!(base_tag, kicked_tag, "both gates must share one target tag");
        Ok(())
    }

    #[test]
    fn line_semantic_facts_survive_parsed_and_normalized_stages() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Line Facts Pipeline")
            .card_types(vec![CardType::Creature]);
        let text = "Whenever this creature becomes tapped during your turn, draw a card. Do this only once each turn.";

        let (semantic, _) = parse_text_to_semantic_document(builder, text.to_string(), false)?;
        let parsed = parse_semantic_document(semantic)?;
        let [ParsedCardItem::Line(parsed_line)] = parsed.items.as_slice() else {
            panic!("expected one parsed line, got {:?}", parsed.items);
        };
        assert!(
            parsed_line
                .semantic_facts
                .triggered_ability
                .becomes_tapped_during_your_turn
        );
        assert_eq!(
            parsed_line
                .semantic_facts
                .triggered_ability
                .frequency
                .do_this_limit_each_turn,
            Some(1)
        );
        let expected_facts = parsed_line.semantic_facts.clone();

        let prepared = prepare_parsed_document(parsed)?;
        let [NormalizedCardItem::Line(prepared_line)] = prepared.items.as_slice() else {
            panic!("expected one normalized line, got {:?}", prepared.items);
        };
        assert_eq!(prepared_line.semantic_facts, expected_facts);

        let definition = lower_prepared_document_with_facts(prepared)?.definition;
        let [ability] = definition.abilities.as_slice() else {
            panic!(
                "expected one triggered ability, got {:?}",
                definition.abilities
            );
        };
        assert_eq!(ability.functional_zones, vec![Zone::Battlefield]);
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("expected triggered ability, got {ability:?}");
        };
        assert_eq!(
            triggered.intervening_if,
            Some(crate::ConditionExpr::And(
                Box::new(crate::ConditionExpr::YourTurn),
                Box::new(crate::ConditionExpr::DoThisMaxTimesEachTurn(1)),
            ))
        );
        Ok(())
    }

    #[test]
    fn line_semantic_facts_drive_static_zones_and_this_spell_reduction_cap()
    -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Capped Reduction")
            .card_types(vec![CardType::Creature]);
        let (definition, _) = parse_text_with_annotations_lowered(
            builder,
            "This spell costs {1} less to cast for each creature type among creatures you control. This effect can't reduce the amount of mana this spell costs by more than {5}.".to_string(),
            false,
        )?;
        let (ability, reduction) = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => match &static_ability.payload {
                    ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) => {
                        Some((ability, reduction))
                    }
                    _ => None,
                },
                _ => None,
            })
            .expect("expected a this-spell cost reduction");
        assert_eq!(
            ability.functional_zones,
            vec![
                Zone::Hand,
                Zone::Stack,
                Zone::Graveyard,
                Zone::Exile,
                Zone::Library,
                Zone::Command,
            ]
        );
        let crate::effect::Value::Min(_, cap) = &reduction.amount else {
            panic!("expected capped reduction, got {:?}", reduction.amount);
        };
        assert!(matches!(cap.as_ref(), crate::effect::Value::Fixed(5)));

        let library_builder = CardDefinitionBuilder::new(CardId::new(), "Library Permission")
            .card_types(vec![CardType::Creature]);
        let (library_definition, _) = parse_text_with_annotations_lowered(
            library_builder,
            "While you're searching your library, you may cast this card from your library."
                .to_string(),
            false,
        )?;
        let library_ability = library_definition
            .abilities
            .iter()
            .find(|ability| matches!(ability.kind, AbilityKind::Static(_)))
            .expect("expected library permission static ability");
        assert_eq!(library_ability.functional_zones, vec![Zone::Library]);
        Ok(())
    }
}
