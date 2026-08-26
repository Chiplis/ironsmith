use crate::cards::ParseAnnotations;
#[cfg(test)]
use crate::cards::builders::CardDefinition;
use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
use crate::lowering::CardAstMaterializer;
use crate::model::ParsedCardAst;
use crate::parse_context::ParseContext;
use crate::parse_trace;

use super::document_parser;
use super::effect_pipeline::{LoweredCardDocument, NormalizedCardAst};
use super::ir::RewriteSemanticDocument;
use super::lower;

pub fn parse_text_to_semantic_document_with_context(
    context: &mut ParseContext,
    builder: CardDefinitionBuilder,
    text: String,
) -> Result<(RewriteSemanticDocument, ParseAnnotations), CardTextError> {
    document_parser::parse_text_to_semantic_document_with_context(context, builder, text)
}

pub fn parse_semantic_document(
    doc: RewriteSemanticDocument,
) -> Result<ParsedCardAst, CardTextError> {
    crate::semantic_document::parse_semantic_document(doc)
}

pub fn prepare_parsed_document(ast: ParsedCardAst) -> Result<NormalizedCardAst, CardTextError> {
    lower::prepare_parsed_card_ast_for_lowering(ast)
}

pub fn lower_prepared_document_with_facts(
    ast: NormalizedCardAst,
) -> Result<LoweredCardDocument, CardTextError> {
    lower::lower_normalized_card_ast_with_facts(ast)
}

#[cfg(test)]
pub fn parse_text_with_annotations_lowered(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let mut context = crate::parse_context_for_builder(&builder, &text, allow_unsupported);
    let lowered =
        parse_text_with_annotations_lowered_with_facts_context(&mut context, builder, text)?;
    Ok((lowered.definition, lowered.annotations))
}

#[cfg(test)]
fn parse_text_to_semantic_document(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(RewriteSemanticDocument, ParseAnnotations), CardTextError> {
    let mut context = crate::parse_context_for_builder(&builder, &text, allow_unsupported);
    parse_text_to_semantic_document_with_context(&mut context, builder, text)
}

pub fn parse_text_with_annotations_lowered_with_facts_context(
    context: &mut ParseContext,
    builder: CardDefinitionBuilder,
    text: String,
) -> Result<LoweredCardDocument, CardTextError> {
    let parsed = crate::canonical_pipeline::parse_card_ast_with_context(context, builder, text)?;
    crate::lowering::lower_card_ast(&mut RuntimeCardAstMaterializer, parsed)
}

struct RuntimeCardAstMaterializer;

impl CardAstMaterializer for RuntimeCardAstMaterializer {
    type RuntimeDocument = LoweredCardDocument;
    type Error = CardTextError;

    fn materialize(&mut self, ast: ParsedCardAst) -> Result<Self::RuntimeDocument, Self::Error> {
        let prepared = {
            let _scope = parse_trace::scope("prepare lowering input");
            prepare_parsed_document(ast)?
        };
        let _scope = parse_trace::scope("lower runtime definition");
        lower_prepared_document_with_facts(prepared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardId;
    use crate::ability::AbilityKind;
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::effect_pipeline::NormalizedCardItem;
    use crate::model::compiler_semantic::ParsedCardItem;
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
        fn find_haste_until(effect: &crate::effect::Effect) -> Option<crate::effect::Until> {
            if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
                && apply.modification.as_ref().is_some_and(|modification| {
                    matches!(
                        modification,
                        crate::continuous::Modification::AddAbility(ability)
                            if ability.id() == crate::static_abilities::StaticAbilityId::Haste
                    )
                })
            {
                return Some(apply.until.clone());
            }
            let mut found = None;
            effect.visit_child_effects(&mut |child| {
                if found.is_none() {
                    found = find_haste_until(child);
                }
            });
            found
        }

        let haste_until = effects
            .iter()
            .find_map(find_haste_until)
            .expect("typed haste grant");
        assert_eq!(haste_until, crate::effect::Until::EndOfTurn);
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
    fn as_enters_statement_materializes_in_authored_ability_order() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Entry Choice Variant")
            .card_types(vec![CardType::Creature]);
        let (definition, _) = parse_text_with_annotations_lowered(
            builder,
            "As this creature enters, choose a nonland card name.\nSpells with the chosen name can't be cast."
                .to_string(),
            false,
        )?;

        assert!(
            definition.spell_effect.is_none(),
            "an as-enters action on a permanent is not a spell-resolution program"
        );
        let [entry_choice, restriction] = definition.abilities.as_slice() else {
            panic!(
                "expected the authored two abilities, got {:#?}",
                definition.abilities
            );
        };
        let AbilityKind::Static(entry_choice) = &entry_choice.kind else {
            panic!("expected a typed as-enters static ability")
        };
        assert!(matches!(
            &entry_choice.payload,
            ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram { .. }
        ));
        let AbilityKind::Static(restriction) = &restriction.kind else {
            panic!("expected the cast restriction to remain second")
        };
        assert!(matches!(
            &restriction.payload,
            ironsmith_core::StaticAbilityPayload::RuleRestriction { .. }
        ));
        Ok(())
    }

    #[test]
    fn flash_cleanup_statement_survives_the_static_line_chunk() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Flash Cleanup Aura")
            .card_types(vec![CardType::Enchantment]);
        let (definition, _) = parse_text_with_annotations_lowered(
            builder,
            "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step."
                .to_string(),
            false,
        )?;

        let spell_effect = definition
            .spell_effect
            .expect("cleanup condition must remain a spell-resolution program");
        let debug = format!("{spell_effect:#?}");
        assert!(debug.contains("ConditionalEffect"), "{debug}");
        assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
        assert!(debug.contains("BeginningOfNextCleanupStep"), "{debug}");
        Ok(())
    }

    #[test]
    fn aura_attachment_preserves_prior_flash_cleanup_program() -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Flash Cleanup Aura")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![crate::types::Subtype::Aura]);
        let (definition, _) = parse_text_with_annotations_lowered(
            builder,
            "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.\nEnchant creature"
                .to_string(),
            false,
        )?;

        let spell_effect = definition
            .spell_effect
            .expect("Aura should retain attachment and cleanup effects");
        let debug = format!("{spell_effect:#?}");
        assert!(debug.contains("AttachToEffect"), "{debug}");
        assert!(debug.contains("ConditionalEffect"), "{debug}");
        assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
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
        let base_tag = match &base_conditional.condition {
            crate::effect::Condition::ValueComparison {
                left: crate::effect::Value::ManaValueOf(base_spec),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: crate::effect::Value::Fixed(3),
            } => {
                let crate::target::ChooseSpec::Tagged(base_tag) = base_spec.base() else {
                    panic!("expected a tagged base spell reference, got {base_spec:#?}");
                };
                base_tag
            }
            crate::effect::Condition::TaggedObjectMatches(base_tag, base_filter)
                if matches!(
                    base_filter.mana_value.as_ref(),
                    Some(crate::target::Comparison::LessThanOrEqual(3))
                ) =>
            {
                base_tag
            }
            _ => {
                panic!("expected a tagged base spell mana-value gate, got {base_conditional:#?}")
            }
        };

        let kicked_branch = segment
            .self_replacements
            .iter()
            .find(|branch| {
                matches!(
                    branch.condition,
                    crate::effect::Condition::ThisSpellWasKicked
                        | crate::effect::Condition::TurnHistory(
                            ironsmith_core::TurnHistoryCondition::SourceWasKicked { .. }
                        )
                )
            })
            .unwrap_or_else(|| panic!("expected kicked replacement branch, got {segment:#?}"));
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
