use super::*;
use crate::runtime_backend::semantic::ParsedManaRestriction;

pub(crate) fn apply_pending_mana_restrictions(
    parsed: &mut ParsedAbility,
    restrictions: &[ParsedManaRestriction],
) -> Result<(), CardTextError> {
    let AbilityKind::Activated(ability) = parsed.kind_mut() else {
        return Err(CardTextError::InvariantViolation(
            "rewrite activated lowering expected activated ability kind".to_string(),
        ));
    };
    for restriction in restrictions {
        apply_pending_mana_restriction(ability, restriction);
    }
    Ok(())
}

pub(crate) fn parse_next_spell_cost_reduction_sentence_rewrite(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let parsed = activated_line_grammar::parse_next_spell_cost_reduction_tokens(tokens)?;

    Some(EffectAst::subject_verb_reduce_next_spell_cost_this_turn(
        crate::cards::builders::PlayerAst::You,
        parsed.spell_filter,
        parsed.reduction,
    ))
}

pub(crate) fn parse_each_player_and_their_creatures_damage_sentence_rewrite(
    _effect_text: &str,
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let parsed = effect_grammar::parse_each_player_creatures_damage_tokens(tokens)?;

    let mut filter = crate::filter::ObjectFilter::default();
    filter.card_types = vec![crate::types::CardType::Creature];
    filter.controller = Some(crate::PlayerFilter::IteratedPlayer);

    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_damage(
                parsed.amount.clone(),
                crate::cards::builders::TargetAst::Player(
                    crate::PlayerFilter::IteratedPlayer,
                    None,
                ),
            ),
            EffectAst::subject_verb_damage_each(parsed.amount, filter),
        ],
    }])
}

#[allow(dead_code)]
pub(crate) fn lower_rewrite_document(
    doc: RewriteSemanticDocument,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let parsed = rewrite_document_to_parsed_card_ast(doc)?;
    let ast = prepare_parsed_card_ast_for_lowering(parsed)?;
    lower_normalized_card_ast(ast)
}

#[allow(dead_code)]
pub(crate) fn lower_parsed_card_ast(
    ast: ParsedCardAst,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    lower_normalized_card_ast(prepare_parsed_card_ast_for_lowering(ast)?)
}

pub(crate) fn lower_normalized_card_ast(
    ast: NormalizedCardAst,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let lowered = lower_normalized_card_ast_with_facts(ast)?;
    Ok((lowered.definition, lowered.annotations))
}

pub(crate) fn lower_normalized_card_ast_with_facts(
    ast: NormalizedCardAst,
) -> Result<LoweredCardDocument, CardTextError> {
    let NormalizedCardAst {
        mut builder,
        mut annotations,
        items,
        overload_branch,
        semantic_facts,
        allow_unsupported,
    } = ast;
    let overload_ast = overload_branch.map(|branch| NormalizedCardAst {
        builder: builder.clone(),
        annotations: ParseAnnotations::default(),
        items: branch.items,
        overload_branch: None,
        semantic_facts: Default::default(),
        allow_unsupported,
    });

    let mut level_abilities = Vec::new();
    let mut level_activated_lines = Vec::new();
    let mut last_restrictable_ability: Option<usize> = None;
    let mut state = RewriteLoweredCardState::default();

    for item in items {
        match item {
            NormalizedCardItem::Line(line) => {
                rewrite_lower_line_ast(
                    &mut builder,
                    &mut state,
                    &mut annotations,
                    line,
                    allow_unsupported,
                    &mut last_restrictable_ability,
                )?;
            }
            NormalizedCardItem::Modal(modal) => {
                let abilities_before = builder.abilities.len();
                builder = rewrite_lower_parsed_modal(builder, modal, allow_unsupported)?;
                rewrite_update_last_restrictable_ability(
                    &builder,
                    abilities_before,
                    &mut last_restrictable_ability,
                );
            }
            NormalizedCardItem::LevelAbility(level) => {
                let lowered = rewrite_lower_level_ability_ast(level)?;
                level_abilities.push(lowered.level_ability);
                level_activated_lines.extend(lowered.activated_lines);
            }
        }
    }

    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }
    for line in level_activated_lines {
        rewrite_lower_line_ast(
            &mut builder,
            &mut state,
            &mut annotations,
            line,
            allow_unsupported,
            &mut last_restrictable_ability,
        )?;
    }

    builder = rewrite_finalize_lowered_card(builder, &mut state);
    if let Some(overload_ast) = overload_ast {
        let overloaded = lower_normalized_card_ast_with_facts(overload_ast)?;
        let overload_effects = overloaded
            .definition
            .spell_effect
            .unwrap_or_default()
            .to_vec();
        for method in &mut builder.alternative_casts {
            if let crate::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
                method
            {
                *effects = overload_effects.clone();
            }
        }
    }
    Ok(LoweredCardDocument {
        definition: builder.build(),
        annotations,
        semantic_facts,
    })
}
